use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use tiny_skia::Pixmap;

const DRM_IOCTL_BASE: u8 = b'd';

const fn _ioc(dir: u32, type_: u8, nr: u8, size: usize) -> libc::c_int {
    ((dir << 30) | ((type_ as u32) << 8) | (nr as u32) | ((size as u32) << 16)) as libc::c_int
}

const fn _io(nr: u8) -> libc::c_int {
    _ioc(0, DRM_IOCTL_BASE, nr, 0)
}

const fn _iow<T>(nr: u8) -> libc::c_int {
    _ioc(1, DRM_IOCTL_BASE, nr, std::mem::size_of::<T>())
}

const fn _iowr<T>(nr: u8) -> libc::c_int {
    _ioc(3, DRM_IOCTL_BASE, nr, std::mem::size_of::<T>())
}

#[repr(C)]
struct DrmSetClientCap {
    capability: u64,
    value: u64,
}

#[repr(C)]
struct DrmModeCardRes {
    fb_id_ptr: u64,
    crtc_id_ptr: u64,
    connector_id_ptr: u64,
    encoder_id_ptr: u64,
    count_fbs: u32,
    count_crtcs: u32,
    count_connectors: u32,
    count_encoders: u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct DrmModeModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    type_: u32,
    name: [u8; 32],
}

#[repr(C)]
struct DrmModeGetConnector {
    encoders_ptr: u64,
    modes_ptr: u64,
    props_ptr: u64,
    prop_values_ptr: u64,
    count_modes: u32,
    count_props: u32,
    count_encoders: u32,
    encoder_id: u32,
    connector_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    pad: u32,
}

#[repr(C)]
struct DrmModeCreateDumb {
    height: u32,
    width: u32,
    bpp: u32,
    flags: u32,
    handle: u32,
    pitch: u32,
    size: u64,
}

#[repr(C)]
struct DrmModeDestroyDumb {
    handle: u32,
}

#[repr(C)]
struct DrmModeMapDumb {
    handle: u32,
    pad: u32,
    offset: u64,
}

#[repr(C)]
struct DrmModeFbCmd2 {
    fb_id: u32,
    width: u32,
    height: u32,
    pixel_format: u32,
    flags: u32,
    handles: [u32; 4],
    pitches: [u32; 4],
    offsets: [u32; 4],
    modifier: [u64; 4],
}

#[repr(C)]
struct DrmModeCreateBlob {
    data: u64,
    length: u32,
    blob_id: u32,
}

#[repr(C)]
struct DrmModeDestroyBlob {
    blob_id: u32,
}

#[repr(C)]
struct DrmModeGetPlaneRes {
    plane_id_ptr: u64,
    count_planes: u32,
}

#[repr(C)]
struct DrmModeObjGetProperties {
    props_ptr: u64,
    prop_values_ptr: u64,
    count_props: u32,
    obj_id: u32,
    obj_type: u32,
}

#[repr(C)]
struct DrmModeGetProperty {
    values_ptr: u64,
    enum_blob_ptr: u64,
    prop_id: u32,
    flags: u32,
    name: [u8; 32],
    count_values: u32,
    count_enum_blobs: u32,
}

#[repr(C)]
struct DrmModeAtomic {
    flags: u32,
    count_objs: u32,
    objs_ptr: u64,
    count_props_ptr: u64,
    props_ptr: u64,
    prop_values_ptr: u64,
    reserved: u64,
    user_data: u64,
}

#[repr(C)]
struct DrmModeFbDirtyCmd {
    fb_id: u32,
    flags: u32,
    color: u32,
    num_clips: u32,
    clips_ptr: u64,
}

const DRM_IOCTL_SET_MASTER: libc::c_int = _io(0x1e);
const DRM_IOCTL_SET_CLIENT_CAP: libc::c_int = _iow::<DrmSetClientCap>(0x0d);
const DRM_IOCTL_MODE_GETRESOURCES: libc::c_int = _iowr::<DrmModeCardRes>(0xa0);
const DRM_IOCTL_MODE_GETCONNECTOR: libc::c_int = _iowr::<DrmModeGetConnector>(0xa7);
const DRM_IOCTL_MODE_CREATE_DUMB: libc::c_int = _iowr::<DrmModeCreateDumb>(0xb2);
const DRM_IOCTL_MODE_MAP_DUMB: libc::c_int = _iowr::<DrmModeMapDumb>(0xb3);
const DRM_IOCTL_MODE_DESTROY_DUMB: libc::c_int = _iowr::<DrmModeDestroyDumb>(0xb4);
const DRM_IOCTL_MODE_ADDFB2: libc::c_int = _iowr::<DrmModeFbCmd2>(0xb8);
const DRM_IOCTL_MODE_RMFB: libc::c_int = _iowr::<u32>(0xaf);
const DRM_IOCTL_MODE_CREATEPROPBLOB: libc::c_int = _iowr::<DrmModeCreateBlob>(0xbd);
const DRM_IOCTL_MODE_DESTROYPROPBLOB: libc::c_int = _iowr::<DrmModeDestroyBlob>(0xbe);
const DRM_IOCTL_MODE_GETPLANERESOURCES: libc::c_int = _iowr::<DrmModeGetPlaneRes>(0xb5);
const DRM_IOCTL_MODE_OBJ_GETPROPERTIES: libc::c_int = _iowr::<DrmModeObjGetProperties>(0xb9);
const DRM_IOCTL_MODE_GETPROPERTY: libc::c_int = _iowr::<DrmModeGetProperty>(0xaa);
const DRM_IOCTL_MODE_ATOMIC_COMMIT: libc::c_int = _iowr::<DrmModeAtomic>(0xbc);
const DRM_IOCTL_MODE_DIRTYFB: libc::c_int = _iowr::<DrmModeFbDirtyCmd>(0xb1);

const DRM_CLIENT_CAP_UNIVERSAL_PLANES: u64 = 2;
const DRM_CLIENT_CAP_ATOMIC: u64 = 3;
const DRM_MODE_CONNECTED: u32 = 1;

const DRM_MODE_OBJECT_CRTC: u32 = 0xcccccccc;
const DRM_MODE_OBJECT_CONNECTOR: u32 = 0xc0c0c0c0;
const DRM_MODE_OBJECT_PLANE: u32 = 0xeeeeeeee;

const DRM_MODE_ATOMIC_NONBLOCK: u32 = 0x0200;
const DRM_MODE_ATOMIC_ALLOW_MODESET: u32 = 0x0100;
const DRM_FORMAT_ARGB8888: u32 = (b'A' as u32) | ((b'R' as u32) << 8) | ((b'2' as u32) << 16) | ((b'4' as u32) << 24);

struct AtomicRequest {
    objects: Vec<(u32, Vec<(u32, u64)>)>,
}

impl AtomicRequest {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    fn add_property(&mut self, obj_id: u32, prop_id: u32, value: u64) {
        if obj_id == 0 || prop_id == 0 {
            return;
        }
        for (existing_obj, props) in &mut self.objects {
            if *existing_obj == obj_id {
                props.push((prop_id, value));
                return;
            }
        }
        self.objects.push((obj_id, vec![(prop_id, value)]));
    }

    fn commit(&self, fd: RawFd, flags: u32) -> Result<(), i32> {
        let count_objs = self.objects.len() as u32;
        if count_objs == 0 {
            return Ok(());
        }

        let mut objs: Vec<u32> = Vec::with_capacity(self.objects.len());
        let mut count_props: Vec<u32> = Vec::with_capacity(self.objects.len());
        let mut props_storage: Vec<Vec<u32>> = Vec::with_capacity(self.objects.len());
        let mut values_storage: Vec<Vec<u64>> = Vec::with_capacity(self.objects.len());

        for (obj_id, prop_pairs) in &self.objects {
            objs.push(*obj_id);
            count_props.push(prop_pairs.len() as u32);
            let mut p_vec = Vec::with_capacity(prop_pairs.len());
            let mut v_vec = Vec::with_capacity(prop_pairs.len());
            for (p, v) in prop_pairs {
                p_vec.push(*p);
                v_vec.push(*v);
            }
            props_storage.push(p_vec);
            values_storage.push(v_vec);
        }

        let mut props_ptrs: Vec<u64> = props_storage
            .iter()
            .map(|v| v.as_ptr() as usize as u64)
            .collect();
        let mut values_ptrs: Vec<u64> = values_storage
            .iter()
            .map(|v| v.as_ptr() as usize as u64)
            .collect();

        let mut req = DrmModeAtomic {
            flags,
            count_objs,
            objs_ptr: objs.as_ptr() as usize as u64,
            count_props_ptr: count_props.as_ptr() as usize as u64,
            props_ptr: props_ptrs.as_mut_ptr() as usize as u64,
            prop_values_ptr: values_ptrs.as_mut_ptr() as usize as u64,
            reserved: 0,
            user_data: 0,
        };

        let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_ATOMIC_COMMIT, &mut req) };
        if ret != 0 {
            let err = unsafe { *libc::__errno_location() };
            return Err(err);
        }
        Ok(())
    }
}

fn find_property(fd: RawFd, obj_id: u32, obj_type: u32, name: &str) -> u32 {
    let mut get_props = DrmModeObjGetProperties {
        props_ptr: 0,
        prop_values_ptr: 0,
        count_props: 0,
        obj_id,
        obj_type,
    };

    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut get_props) } != 0 {
        return 0;
    }

    if get_props.count_props == 0 {
        return 0;
    }

    let mut prop_ids = vec![0u32; get_props.count_props as usize];
    let mut prop_vals = vec![0u64; get_props.count_props as usize];
    get_props.props_ptr = prop_ids.as_mut_ptr() as usize as u64;
    get_props.prop_values_ptr = prop_vals.as_mut_ptr() as usize as u64;

    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut get_props) } != 0 {
        return 0;
    }

    let target_bytes = name.as_bytes();

    for &p_id in &prop_ids {
        let mut get_prop = DrmModeGetProperty {
            values_ptr: 0,
            enum_blob_ptr: 0,
            prop_id: p_id,
            flags: 0,
            name: [0; 32],
            count_values: 0,
            count_enum_blobs: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPROPERTY, &mut get_prop) } == 0 {
            let len = get_prop.name.iter().position(|&c| c == 0).unwrap_or(32);
            if &get_prop.name[..len] == target_bytes {
                return p_id;
            }
        }
    }

    0
}

fn write_backlight_level(level: i32) {
    let paths = [
        "/sys/class/leds/lcd-backlight/brightness",
        "/sys/class/backlight/panel0-backlight/brightness",
    ];
    for p in &paths {
        if let Ok(mut f) = OpenOptions::new().write(true).open(p) {
            let _ = writeln!(f, "{}", level);
            return;
        }
    }
}

pub struct DrmDisplay {
    fd: File,
    width: u32,
    height: u32,
    stride: u32,
    refresh_rate: u32,
    fb_ptr: *mut u32,
    fb_size: usize,
    fb_id: u32,
    dumb_handle: u32,
    mode_blob_id: u32,
    plane0_id: u32,
    plane1_id: u32,
    prop_p0_fb: u32,
    prop_p1_fb: u32,
    brightness: i32,
    power_on: bool,
}

impl DrmDisplay {
    pub fn init() -> Result<Self, &'static str> {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/dri/card0")
        {
            Ok(f) => f,
            Err(_) => return Err("Failed to open /dev/dri/card0"),
        };

        let fd = file.as_raw_fd();

        unsafe {
            let _ = libc::ioctl(fd, DRM_IOCTL_SET_MASTER, 0);

            let mut cap_planes = DrmSetClientCap {
                capability: DRM_CLIENT_CAP_UNIVERSAL_PLANES,
                value: 1,
            };
            let _ = libc::ioctl(fd, DRM_IOCTL_SET_CLIENT_CAP, &mut cap_planes);

            let mut cap_atomic = DrmSetClientCap {
                capability: DRM_CLIENT_CAP_ATOMIC,
                value: 1,
            };
            let _ = libc::ioctl(fd, DRM_IOCTL_SET_CLIENT_CAP, &mut cap_atomic);
        }

        let mut res = DrmModeCardRes {
            fb_id_ptr: 0,
            crtc_id_ptr: 0,
            connector_id_ptr: 0,
            encoder_id_ptr: 0,
            count_fbs: 0,
            count_crtcs: 0,
            count_connectors: 0,
            count_encoders: 0,
            min_width: 0,
            max_width: 0,
            min_height: 0,
            max_height: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut res) } != 0 {
            return Err("Failed to query DRM card resources");
        }

        if res.count_connectors == 0 || res.count_crtcs == 0 {
            return Err("Zero connectors or CRTCs found on DRM card");
        }

        let mut connector_ids = vec![0u32; res.count_connectors as usize];
        let mut crtc_ids = vec![0u32; res.count_crtcs as usize];
        res.connector_id_ptr = connector_ids.as_mut_ptr() as usize as u64;
        res.crtc_id_ptr = crtc_ids.as_mut_ptr() as usize as u64;

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES, &mut res) } != 0 {
            return Err("Failed to read connector IDs");
        }

        let mut active_conn_id: u32 = 0;
        let mut active_mode = DrmModeModeInfo::default();
        let mut max_panel_hz: u32 = 144;

        for &conn_id in &connector_ids {
            let mut conn_info = DrmModeGetConnector {
                encoders_ptr: 0,
                modes_ptr: 0,
                props_ptr: 0,
                prop_values_ptr: 0,
                count_modes: 0,
                count_props: 0,
                count_encoders: 0,
                encoder_id: 0,
                connector_id: conn_id,
                connector_type: 0,
                connector_type_id: 0,
                connection: 0,
                mm_width: 0,
                mm_height: 0,
                subpixel: 0,
                pad: 0,
            };

            if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn_info) } != 0 {
                continue;
            }

            if conn_info.connection == DRM_MODE_CONNECTED && conn_info.count_modes > 0 {
                let mut modes = vec![DrmModeModeInfo::default(); conn_info.count_modes as usize];
                conn_info.modes_ptr = modes.as_mut_ptr() as usize as u64;

                if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR, &mut conn_info) } == 0 {
                    active_conn_id = conn_id;
                    active_mode = modes[0];
                    for m in &modes {
                        if m.vrefresh > max_panel_hz {
                            max_panel_hz = m.vrefresh;
                        }
                    }
                    break;
                }
            }
        }

        if active_conn_id == 0 {
            return Err("No connected display panel detected");
        }

        let width = active_mode.hdisplay as u32;
        let height = active_mode.vdisplay as u32;
        let crtc_id = crtc_ids[0];

        let mut create_dumb = DrmModeCreateDumb {
            height,
            width,
            bpp: 32,
            flags: 0,
            handle: 0,
            pitch: 0,
            size: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB, &mut create_dumb) } != 0 {
            return Err("Failed to create DRM dumb buffer");
        }

        let dumb_handle = create_dumb.handle;
        let stride = create_dumb.pitch / 4;
        let fb_size = create_dumb.size as usize;

        let mut fb_cmd = DrmModeFbCmd2 {
            fb_id: 0,
            width,
            height,
            pixel_format: DRM_FORMAT_ARGB8888,
            flags: 0,
            handles: [dumb_handle, 0, 0, 0],
            pitches: [create_dumb.pitch, 0, 0, 0],
            offsets: [0, 0, 0, 0],
            modifier: [0, 0, 0, 0],
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_ADDFB2, &mut fb_cmd) } != 0 {
            return Err("Failed to register DRM framebuffer (ADDFB2)");
        }
        let fb_id = fb_cmd.fb_id;

        let mut map_dumb = DrmModeMapDumb {
            handle: dumb_handle,
            pad: 0,
            offset: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB, &mut map_dumb) } != 0 {
            return Err("Failed to map DRM dumb buffer offset");
        }

        let fb_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                fb_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                map_dumb.offset as libc::off_t,
            )
        } as *mut u32;

        if fb_ptr == libc::MAP_FAILED as *mut u32 || fb_ptr.is_null() {
            return Err("Failed to mmap DRM framebuffer into process memory");
        }

        unsafe {
            for i in 0..(stride * height) as usize {
                *fb_ptr.add(i) = 0xFF141218;
            }
        }

        let mut create_blob = DrmModeCreateBlob {
            data: &active_mode as *const _ as usize as u64,
            length: std::mem::size_of::<DrmModeModeInfo>() as u32,
            blob_id: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_CREATEPROPBLOB, &mut create_blob) } != 0 {
            return Err("Failed to create DRM display mode blob");
        }
        let mode_blob_id = create_blob.blob_id;

        let mut plane_res = DrmModeGetPlaneRes {
            plane_id_ptr: 0,
            count_planes: 0,
        };

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut plane_res) } != 0
            || plane_res.count_planes == 0
        {
            return Err("Failed to query DRM plane resources");
        }

        let mut planes = vec![0u32; plane_res.count_planes as usize];
        plane_res.plane_id_ptr = planes.as_mut_ptr() as usize as u64;

        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETPLANERESOURCES, &mut plane_res) } != 0 {
            return Err("Failed to read DRM plane IDs");
        }

        let plane0_id = planes[0];
        let plane1_id = if planes.len() > 1 { planes[1] } else { plane0_id };

        let prop_crtc_active = find_property(fd, crtc_id, DRM_MODE_OBJECT_CRTC, "ACTIVE");
        let prop_crtc_mode = find_property(fd, crtc_id, DRM_MODE_OBJECT_CRTC, "MODE_ID");
        let prop_conn_crtc = find_property(fd, active_conn_id, DRM_MODE_OBJECT_CONNECTOR, "CRTC_ID");

        let prop_p0_fb = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "FB_ID");
        let p0_crtc = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_ID");
        let p0_src_x = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_X");
        let p0_src_y = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_Y");
        let p0_src_w = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_W");
        let p0_src_h = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "SRC_H");
        let p0_crtc_x = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_X");
        let p0_crtc_y = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_Y");
        let p0_crtc_w = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_W");
        let p0_crtc_h = find_property(fd, plane0_id, DRM_MODE_OBJECT_PLANE, "CRTC_H");

        let half_w = width / 2;

        let mut req = AtomicRequest::new();
        if prop_conn_crtc != 0 {
            req.add_property(active_conn_id, prop_conn_crtc, crtc_id as u64);
        }
        if prop_crtc_mode != 0 {
            req.add_property(crtc_id, prop_crtc_mode, mode_blob_id as u64);
        }
        if prop_crtc_active != 0 {
            req.add_property(crtc_id, prop_crtc_active, 1);
        }

        if prop_p0_fb != 0 {
            req.add_property(plane0_id, prop_p0_fb, fb_id as u64);
        }
        if p0_crtc != 0 {
            req.add_property(plane0_id, p0_crtc, crtc_id as u64);
        }
        if p0_src_x != 0 {
            req.add_property(plane0_id, p0_src_x, 0);
        }
        if p0_src_y != 0 {
            req.add_property(plane0_id, p0_src_y, 0);
        }
        if p0_src_w != 0 {
            req.add_property(plane0_id, p0_src_w, (half_w as u64) << 16);
        }
        if p0_src_h != 0 {
            req.add_property(plane0_id, p0_src_h, (height as u64) << 16);
        }
        if p0_crtc_x != 0 {
            req.add_property(plane0_id, p0_crtc_x, 0);
        }
        if p0_crtc_y != 0 {
            req.add_property(plane0_id, p0_crtc_y, 0);
        }
        if p0_crtc_w != 0 {
            req.add_property(plane0_id, p0_crtc_w, half_w as u64);
        }
        if p0_crtc_h != 0 {
            req.add_property(plane0_id, p0_crtc_h, height as u64);
        }

        let prop_p1_fb = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "FB_ID");
        let p1_crtc = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_ID");
        let p1_src_x = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_X");
        let p1_src_y = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_Y");
        let p1_src_w = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_W");
        let p1_src_h = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "SRC_H");
        let p1_crtc_x = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_X");
        let p1_crtc_y = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_Y");
        let p1_crtc_w = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_W");
        let p1_crtc_h = find_property(fd, plane1_id, DRM_MODE_OBJECT_PLANE, "CRTC_H");

        if prop_p1_fb != 0 {
            req.add_property(plane1_id, prop_p1_fb, fb_id as u64);
        }
        if p1_crtc != 0 {
            req.add_property(plane1_id, p1_crtc, crtc_id as u64);
        }
        if p1_src_x != 0 {
            req.add_property(plane1_id, p1_src_x, (half_w as u64) << 16);
        }
        if p1_src_y != 0 {
            req.add_property(plane1_id, p1_src_y, 0);
        }
        if p1_src_w != 0 {
            req.add_property(plane1_id, p1_src_w, (half_w as u64) << 16);
        }
        if p1_src_h != 0 {
            req.add_property(plane1_id, p1_src_h, (height as u64) << 16);
        }
        if p1_crtc_x != 0 {
            req.add_property(plane1_id, p1_crtc_x, half_w as u64);
        }
        if p1_crtc_y != 0 {
            req.add_property(plane1_id, p1_crtc_y, 0);
        }
        if p1_crtc_w != 0 {
            req.add_property(plane1_id, p1_crtc_w, half_w as u64);
        }
        if p1_crtc_h != 0 {
            req.add_property(plane1_id, p1_crtc_h, height as u64);
        }

        if let Err(_) = req.commit(fd, DRM_MODE_ATOMIC_ALLOW_MODESET) {
            return Err("Atomic modeset commit failed");
        }

        write_backlight_level(2500);

        Ok(Self {
            fd: file,
            width,
            height,
            stride,
            refresh_rate: max_panel_hz,
            fb_ptr,
            fb_size,
            fb_id,
            dumb_handle,
            mode_blob_id,
            plane0_id,
            plane1_id,
            prop_p0_fb,
            prop_p1_fb,
            brightness: 2500,
            power_on: true,
        })
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn refresh_rate(&self) -> u32 {
        self.refresh_rate
    }

    pub fn blit_and_flip(&mut self, pixmap: &Pixmap) {
        if self.fb_ptr.is_null() {
            return;
        }

        let pw = pixmap.width() as usize;
        let ph = pixmap.height() as usize;
        let stride = self.stride as usize;
        let src_bytes = pixmap.data();

        let dst_slice = unsafe {
            std::slice::from_raw_parts_mut(self.fb_ptr, stride * (self.height as usize))
        };

        let copy_w = pw.min(self.width as usize);
        let copy_h = ph.min(self.height as usize);

        let src_u32 = unsafe {
            std::slice::from_raw_parts(src_bytes.as_ptr() as *const u32, src_bytes.len() / 4)
        };

        if pw == stride && copy_w == stride {
            let total_pixels = copy_w * copy_h;
            let src_pixels = &src_u32[..total_pixels];
            let dst_pixels = &mut dst_slice[..total_pixels];
            for (src, dst) in src_pixels.iter().zip(dst_pixels.iter_mut()) {
                let val = *src;
                *dst = (val & 0xFF00FF00) | ((val & 0x000000FF) << 16) | ((val & 0x00FF0000) >> 16);
            }
        } else {
            for y in 0..copy_h {
                let src_row = &src_u32[y * pw..y * pw + copy_w];
                let dst_row = &mut dst_slice[y * stride..y * stride + copy_w];
                for (src, dst) in src_row.iter().zip(dst_row.iter_mut()) {
                    let val = *src;
                    *dst = (val & 0xFF00FF00) | ((val & 0x000000FF) << 16) | ((val & 0x00FF0000) >> 16);
                }
            }
        }

        let fd = self.fd.as_raw_fd();

        let mut dirty = DrmModeFbDirtyCmd {
            fb_id: self.fb_id,
            flags: 0,
            color: 0,
            num_clips: 0,
            clips_ptr: 0,
        };
        unsafe {
            let _ = libc::ioctl(fd, DRM_IOCTL_MODE_DIRTYFB, &mut dirty);
        }

        let mut req = AtomicRequest::new();
        if self.prop_p0_fb != 0 && self.plane0_id != 0 {
            req.add_property(self.plane0_id, self.prop_p0_fb, self.fb_id as u64);
        }
        if self.prop_p1_fb != 0 && self.plane1_id != 0 && self.plane1_id != self.plane0_id {
            req.add_property(self.plane1_id, self.prop_p1_fb, self.fb_id as u64);
        }

        let _ = req.commit(fd, DRM_MODE_ATOMIC_NONBLOCK);
    }

    pub fn set_brightness(&mut self, val: i32) {
        let level = val.clamp(100, 10000);
        self.brightness = level;
        if self.power_on {
            write_backlight_level(level);
        }
    }

    pub fn get_brightness(&self) -> i32 {
        self.brightness
    }

    pub fn set_screen_power(&mut self, on: bool) {
        self.power_on = on;
        if on {
            write_backlight_level(self.brightness);
        } else {
            write_backlight_level(0);
        }
    }

    pub fn is_screen_power(&self) -> bool {
        self.power_on
    }

    pub fn screenshot(&self, filepath: &str) -> bool {
        if self.fb_ptr.is_null() {
            return false;
        }

        let mut f = match File::create(filepath) {
            Ok(file) => file,
            Err(_) => return false,
        };

        if writeln!(f, "P6\n{} {}\n255", self.width, self.height).is_err() {
            return false;
        }

        let mut row_buf = Vec::with_capacity((self.width * 3) as usize);

        for y in 0..self.height {
            row_buf.clear();
            for x in 0..self.width {
                let px = unsafe { *self.fb_ptr.add((y * self.stride + x) as usize) };
                let r = ((px >> 16) & 0xFF) as u8;
                let g = ((px >> 8) & 0xFF) as u8;
                let b = (px & 0xFF) as u8;
                row_buf.push(r);
                row_buf.push(g);
                row_buf.push(b);
            }
            if f.write_all(&row_buf).is_err() {
                return false;
            }
        }

        true
    }
}

impl Drop for DrmDisplay {
    fn drop(&mut self) {
        let fd = self.fd.as_raw_fd();
        if !self.fb_ptr.is_null() && self.fb_size > 0 {
            unsafe {
                libc::munmap(self.fb_ptr as *mut libc::c_void, self.fb_size);
            }
            self.fb_ptr = ptr::null_mut();
        }

        if self.mode_blob_id != 0 {
            let mut destroy_blob = DrmModeDestroyBlob {
                blob_id: self.mode_blob_id,
            };
            unsafe {
                let _ = libc::ioctl(fd, DRM_IOCTL_MODE_DESTROYPROPBLOB, &mut destroy_blob);
            }
            self.mode_blob_id = 0;
        }

        if self.fb_id != 0 {
            let mut fb_id = self.fb_id;
            unsafe {
                let _ = libc::ioctl(fd, DRM_IOCTL_MODE_RMFB, &mut fb_id);
            }
            self.fb_id = 0;
        }

        if self.dumb_handle != 0 {
            let mut destroy_dumb = DrmModeDestroyDumb {
                handle: self.dumb_handle,
            };
            unsafe {
                let _ = libc::ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB, &mut destroy_dumb);
            }
            self.dumb_handle = 0;
        }
    }
}
