#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/ioctl.h>
#include <sys/sysmacros.h>
#include <errno.h>
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <drm/drm_fourcc.h>

#include "display_drm.h"

static int s_drm_fd = -1;
static uint32_t s_width = 1220;
static uint32_t s_height = 2712;
static uint32_t s_stride = 1220;
static uint32_t *s_mapped_fb = NULL;
static size_t s_fb_size = 0;
static uint32_t s_fb_id = 0;
static uint32_t s_dumb_handle = 0;
static uint32_t s_conn_id = 0;
static uint32_t s_crtc_id = 0;
static uint32_t s_mode_blob_id = 0;

static uint32_t s_plane0_id = 0;
static uint32_t s_plane1_id = 0;
static uint32_t s_refresh_rate = 144;
static uint32_t s_panel_max_hz = 144;

static uint32_t s_prop_p0_fb = 0;
static uint32_t s_prop_p1_fb = 0;
static uint32_t s_prop_conn_crtc = 0;
static uint32_t s_prop_crtc_active = 0;
static uint32_t s_prop_crtc_mode = 0;

static int s_user_brightness = 2500;
static bool s_screen_power = true;

uint32_t display_drm_get_refresh_rate(void) {
    return s_refresh_rate;
}

uint32_t *display_drm_get_fb(void) {
    return s_mapped_fb;
}

uint32_t display_drm_get_stride(void) {
    return s_stride;
}

static void write_brightness_raw(int val) {
    FILE *f = fopen("/sys/class/leds/lcd-backlight/brightness", "w");
    if (f) {
        fprintf(f, "%d\n", val);
        fclose(f);
    }
}

void display_drm_set_brightness(int val) {
    if (val < 100) val = 100;
    if (val > 10000) val = 10000;
    s_user_brightness = val;
    if (s_screen_power) {
        write_brightness_raw(val);
    }
}

int display_drm_get_brightness(void) {
    return s_user_brightness;
}

void display_drm_set_power(bool on) {
    s_screen_power = on;
    if (on) {
        write_brightness_raw(s_user_brightness);
    } else {
        write_brightness_raw(0);
    }
    printf("[MRR-DISPLAY] Screen power set to %s (level=%d)\n", on ? "ON" : "OFF", on ? s_user_brightness : 0);
}

bool display_drm_is_power(void) {
    return s_screen_power;
}

static uint32_t find_prop(int fd, uint32_t obj_id, uint32_t obj_type, const char *name) {
    drmModeObjectProperties *props = drmModeObjectGetProperties(fd, obj_id, obj_type);
    if (!props) return 0;
    uint32_t found_id = 0;
    for (uint32_t i = 0; i < props->count_props; i++) {
        drmModePropertyRes *p = drmModeGetProperty(fd, props->props[i]);
        if (p) {
            if (strcmp(p->name, name) == 0) {
                found_id = p->prop_id;
                drmModeFreeProperty(p);
                break;
            }
            drmModeFreeProperty(p);
        }
    }
    drmModeFreeObjectProperties(props);
    return found_id;
}

void display_drm_flip(void) {
    if (s_drm_fd < 0 || !s_fb_id) return;

    drmModeDirtyFB(s_drm_fd, s_fb_id, NULL, 0);

    drmModeAtomicReq *req = drmModeAtomicAlloc();
    if (!req) return;

    if (s_prop_p0_fb && s_plane0_id) {
        drmModeAtomicAddProperty(req, s_plane0_id, s_prop_p0_fb, s_fb_id);
    }
    if (s_prop_p1_fb && s_plane1_id && s_plane1_id != s_plane0_id) {
        drmModeAtomicAddProperty(req, s_plane1_id, s_prop_p1_fb, s_fb_id);
    }

    drmModeAtomicCommit(s_drm_fd, req, DRM_MODE_ATOMIC_NONBLOCK, NULL);
    drmModeAtomicFree(req);
}

bool display_drm_init(uint32_t *width, uint32_t *height) {
    printf("[MRR-DISPLAY] Initializing Dual-Pipe Atomic DRM on /dev/dri/card0...\n");

    s_drm_fd = open("/dev/dri/card0", O_RDWR | O_CLOEXEC);
    if (s_drm_fd < 0) {
        perror("[MRR-DISPLAY] open /dev/dri/card0 failed");
        return false;
    }

    drmSetMaster(s_drm_fd);
    drmSetClientCap(s_drm_fd, DRM_CLIENT_CAP_UNIVERSAL_PLANES, 1);
    drmSetClientCap(s_drm_fd, DRM_CLIENT_CAP_ATOMIC, 1);

    drmModeRes *res = drmModeGetResources(s_drm_fd);
    if (!res) {
        perror("[MRR-DISPLAY] drmModeGetResources failed");
        close(s_drm_fd);
        s_drm_fd = -1;
        return false;
    }

    drmModeConnector *conn = NULL;
    for (int i = 0; i < res->count_connectors; i++) {
        drmModeConnector *c = drmModeGetConnector(s_drm_fd, res->connectors[i]);
        if (c && c->connection == DRM_MODE_CONNECTED && c->count_modes > 0) {
            conn = c;
            break;
        }
        if (c) drmModeFreeConnector(c);
    }

    if (!conn) {
        fprintf(stderr, "[MRR-DISPLAY] No connected connector found!\n");
        drmModeFreeResources(res);
        close(s_drm_fd);
        s_drm_fd = -1;
        return false;
    }

    s_conn_id = conn->connector_id;
    drmModeModeInfo mode = conn->modes[0]; // Native bootloader mode

    s_panel_max_hz = 144; // Xiaomi 14T AMOLED 144Hz panel spec
    for (int m = 0; m < conn->count_modes; m++) {
        printf("[MRR-DISPLAY] Mode %d: %ux%u @ %uHz (flags=0x%x)\n",
               m, conn->modes[m].hdisplay, conn->modes[m].vdisplay,
               conn->modes[m].vrefresh, conn->modes[m].flags);
        if (conn->modes[m].vrefresh > s_panel_max_hz) {
            s_panel_max_hz = conn->modes[m].vrefresh;
        }
    }
    s_refresh_rate = s_panel_max_hz;
    s_width = mode.hdisplay;   // 1220
    s_height = mode.vdisplay; // 2712
    s_crtc_id = res->crtcs[0]; // 115

    printf("[MRR-DISPLAY] Active Mode: %ux%u @ %uHz (Panel HW: %uHz) on Conn=%u, CRTC=%u\n",
           s_width, s_height, mode.vrefresh, s_panel_max_hz, s_conn_id, s_crtc_id);

    // 1. Create dumb buffer
    struct drm_mode_create_dumb create_dumb = {0};
    create_dumb.width = s_width;
    create_dumb.height = s_height;
    create_dumb.bpp = 32;
    if (ioctl(s_drm_fd, DRM_IOCTL_MODE_CREATE_DUMB, &create_dumb) < 0) {
        perror("[MRR-DISPLAY] CREATE_DUMB failed");
        return false;
    }
    s_dumb_handle = create_dumb.handle;
    s_stride = create_dumb.pitch / 4;
    s_fb_size = create_dumb.size;

    // 2. Add Framebuffer (DRM_FORMAT_ARGB8888)
    uint32_t handles[4] = { s_dumb_handle };
    uint32_t pitches[4] = { create_dumb.pitch };
    uint32_t offsets[4] = { 0 };
    if (drmModeAddFB2(s_drm_fd, s_width, s_height, DRM_FORMAT_ARGB8888, handles, pitches, offsets, &s_fb_id, 0) != 0) {
        perror("[MRR-DISPLAY] drmModeAddFB2 failed");
        return false;
    }

    // 3. Map dumb buffer
    struct drm_mode_map_dumb map_dumb = {0};
    map_dumb.handle = s_dumb_handle;
    if (ioctl(s_drm_fd, DRM_IOCTL_MODE_MAP_DUMB, &map_dumb) < 0) {
        perror("[MRR-DISPLAY] MAP_DUMB failed");
        return false;
    }

    s_mapped_fb = (uint32_t *)mmap(NULL, s_fb_size, PROT_READ | PROT_WRITE, MAP_SHARED, s_drm_fd, map_dumb.offset);
    if (s_mapped_fb == MAP_FAILED) {
        perror("[MRR-DISPLAY] mmap failed");
        s_mapped_fb = NULL;
        return false;
    }

    // Clear buffer to Material 3 Dark Surface (#141218)
    for (size_t i = 0; i < s_stride * s_height; i++) {
        s_mapped_fb[i] = 0xFF141218;
    }

    // 4. Create mode blob
    if (drmModeCreatePropertyBlob(s_drm_fd, &mode, sizeof(mode), &s_mode_blob_id) != 0) {
        perror("[MRR-DISPLAY] CreatePropertyBlob failed");
        return false;
    }

    // 5. Query Planes & Properties
    drmModePlaneRes *planes = drmModeGetPlaneResources(s_drm_fd);
    if (!planes || planes->count_planes < 1) {
        fprintf(stderr, "[MRR-DISPLAY] No planes available!\n");
        return false;
    }

    s_plane0_id = planes->planes[0];
    s_plane1_id = (planes->count_planes > 1) ? planes->planes[1] : s_plane0_id;

    s_prop_crtc_active = find_prop(s_drm_fd, s_crtc_id, DRM_MODE_OBJECT_CRTC, "ACTIVE");
    s_prop_crtc_mode = find_prop(s_drm_fd, s_crtc_id, DRM_MODE_OBJECT_CRTC, "MODE_ID");
    s_prop_conn_crtc = find_prop(s_drm_fd, s_conn_id, DRM_MODE_OBJECT_CONNECTOR, "CRTC_ID");

    s_prop_p0_fb = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "FB_ID");
    uint32_t p0_crtc = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_ID");
    uint32_t p0_src_x = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_X");
    uint32_t p0_src_y = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_Y");
    uint32_t p0_src_w = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_W");
    uint32_t p0_src_h = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_H");
    uint32_t p0_crtc_x = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_X");
    uint32_t p0_crtc_y = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_Y");
    uint32_t p0_crtc_w = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_W");
    uint32_t p0_crtc_h = find_prop(s_drm_fd, s_plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_H");

    uint32_t half_w = s_width / 2; // 610 px for MTK dual layer mixer

    drmModeAtomicReq *req = drmModeAtomicAlloc();
    if (s_prop_conn_crtc) drmModeAtomicAddProperty(req, s_conn_id, s_prop_conn_crtc, s_crtc_id);
    if (s_prop_crtc_mode) drmModeAtomicAddProperty(req, s_crtc_id, s_prop_crtc_mode, s_mode_blob_id);
    if (s_prop_crtc_active) drmModeAtomicAddProperty(req, s_crtc_id, s_prop_crtc_active, 1);

    // Plane 0: Left Half
    if (s_prop_p0_fb) drmModeAtomicAddProperty(req, s_plane0_id, s_prop_p0_fb, s_fb_id);
    if (p0_crtc) drmModeAtomicAddProperty(req, s_plane0_id, p0_crtc, s_crtc_id);
    if (p0_src_x) drmModeAtomicAddProperty(req, s_plane0_id, p0_src_x, 0);
    if (p0_src_y) drmModeAtomicAddProperty(req, s_plane0_id, p0_src_y, 0);
    if (p0_src_w) drmModeAtomicAddProperty(req, s_plane0_id, p0_src_w, half_w << 16);
    if (p0_src_h) drmModeAtomicAddProperty(req, s_plane0_id, p0_src_h, s_height << 16);
    if (p0_crtc_x) drmModeAtomicAddProperty(req, s_plane0_id, p0_crtc_x, 0);
    if (p0_crtc_y) drmModeAtomicAddProperty(req, s_plane0_id, p0_crtc_y, 0);
    if (p0_crtc_w) drmModeAtomicAddProperty(req, s_plane0_id, p0_crtc_w, half_w);
    if (p0_crtc_h) drmModeAtomicAddProperty(req, s_plane0_id, p0_crtc_h, s_height);

    // Plane 1: Right Half
    s_prop_p1_fb = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "FB_ID");
    uint32_t p1_crtc = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_ID");
    uint32_t p1_src_x = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_X");
    uint32_t p1_src_y = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_Y");
    uint32_t p1_src_w = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_W");
    uint32_t p1_src_h = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_H");
    uint32_t p1_crtc_x = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_X");
    uint32_t p1_crtc_y = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_Y");
    uint32_t p1_crtc_w = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_W");
    uint32_t p1_crtc_h = find_prop(s_drm_fd, s_plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_H");

    if (s_prop_p1_fb) drmModeAtomicAddProperty(req, s_plane1_id, s_prop_p1_fb, s_fb_id);
    if (p1_crtc) drmModeAtomicAddProperty(req, s_plane1_id, p1_crtc, s_crtc_id);
    if (p1_src_x) drmModeAtomicAddProperty(req, s_plane1_id, p1_src_x, half_w << 16);
    if (p1_src_y) drmModeAtomicAddProperty(req, s_plane1_id, p1_src_y, 0);
    if (p1_src_w) drmModeAtomicAddProperty(req, s_plane1_id, p1_src_w, half_w << 16);
    if (p1_src_h) drmModeAtomicAddProperty(req, s_plane1_id, p1_src_h, s_height << 16);
    if (p1_crtc_x) drmModeAtomicAddProperty(req, s_plane1_id, p1_crtc_x, half_w);
    if (p1_crtc_y) drmModeAtomicAddProperty(req, s_plane1_id, p1_crtc_y, 0);
    if (p1_crtc_w) drmModeAtomicAddProperty(req, s_plane1_id, p1_crtc_w, half_w);
    if (p1_crtc_h) drmModeAtomicAddProperty(req, s_plane1_id, p1_crtc_h, s_height);

    int ret = drmModeAtomicCommit(s_drm_fd, req, DRM_MODE_ATOMIC_ALLOW_MODESET, NULL);
    if (ret != 0) {
        printf("[MRR-DISPLAY] Atomic commit failed with code %d\n", ret);
        drmModeAtomicFree(req);
        return false;
    }
    printf("[MRR-DISPLAY] Dual-pipe atomic modeset committed successfully!\n");
    drmModeAtomicFree(req);

    // 6. Set Backlight Brightness
    display_drm_set_power(true);

    drmModeFreePlaneResources(planes);
    drmModeFreeConnector(conn);
    drmModeFreeResources(res);

    if (width) *width = s_width;
    if (height) *height = s_height;

    printf("[MRR-DISPLAY] Dual-pipe DRM Display initialized successfully (%ux%u)!\n", s_width, s_height);
    return true;
}

bool display_drm_screenshot(const char *filepath) {
    if (!s_mapped_fb) return false;
    FILE *f = fopen(filepath, "wb");
    if (!f) return false;
    fprintf(f, "P6\n%u %u\n255\n", s_width, s_height);
    for (uint32_t y = 0; y < s_height; y++) {
        for (uint32_t x = 0; x < s_width; x++) {
            uint32_t px = s_mapped_fb[y * s_stride + x];
            uint8_t rgb[3];
            rgb[0] = (px >> 16) & 0xFF; // R
            rgb[1] = (px >> 8) & 0xFF;  // G
            rgb[2] = px & 0xFF;         // B
            fwrite(rgb, 1, 3, f);
        }
    }
    fclose(f);
    printf("[MRR-DISPLAY] Screenshot saved to %s\n", filepath);
    return true;
}

void display_drm_cleanup(void) {
    if (s_mapped_fb && s_fb_size) {
        munmap(s_mapped_fb, s_fb_size);
        s_mapped_fb = NULL;
    }
    if (s_mode_blob_id && s_drm_fd >= 0) {
        drmModeDestroyPropertyBlob(s_drm_fd, s_mode_blob_id);
        s_mode_blob_id = 0;
    }
    if (s_fb_id && s_drm_fd >= 0) {
        drmModeRmFB(s_drm_fd, s_fb_id);
        s_fb_id = 0;
    }
    if (s_dumb_handle && s_drm_fd >= 0) {
        struct drm_mode_destroy_dumb destroy_dumb = { .handle = s_dumb_handle };
        ioctl(s_drm_fd, DRM_IOCTL_MODE_DESTROY_DUMB, &destroy_dumb);
        s_dumb_handle = 0;
    }
    if (s_drm_fd >= 0) {
        close(s_drm_fd);
        s_drm_fd = -1;
    }
    printf("[MRR-DISPLAY] Display backend cleaned up.\n");
}
