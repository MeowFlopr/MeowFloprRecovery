#ifndef MRR_DISPLAY_DRM_H
#define MRR_DISPLAY_DRM_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

bool display_drm_init(uint32_t *width, uint32_t *height);
void display_drm_cleanup(void);

uint32_t *display_drm_get_fb(void);
uint32_t display_drm_get_stride(void);
uint32_t display_drm_get_refresh_rate(void);

void display_drm_flip(void);

void display_drm_set_brightness(int val);
int display_drm_get_brightness(void);

void display_drm_set_power(bool on);
bool display_drm_is_power(void);

bool display_drm_screenshot(const char *filepath);

#ifdef __cplusplus
}
#endif

#endif // MRR_DISPLAY_DRM_H
