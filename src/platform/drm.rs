use tiny_skia::Pixmap;

extern "C" {
    fn display_drm_init(width: *mut u32, height: *mut u32) -> bool;
    fn display_drm_cleanup();
    fn display_drm_get_fb() -> *mut u32;
    fn display_drm_get_stride() -> u32;
    fn display_drm_get_refresh_rate() -> u32;
    fn display_drm_flip();
    fn display_drm_set_brightness(val: libc::c_int);
    fn display_drm_get_brightness() -> libc::c_int;
    fn display_drm_set_power(on: bool);
    fn display_drm_is_power() -> bool;
    fn display_drm_screenshot(filepath: *const libc::c_char) -> bool;
}

pub struct DrmDisplay {
    width: u32,
    height: u32,
    stride: u32,
    refresh_rate: u32,
    fb_ptr: *mut u32,
}

impl DrmDisplay {
    pub fn init() -> Result<Self, &'static str> {
        let mut w: u32 = 0;
        let mut h: u32 = 0;

        let ok = unsafe { display_drm_init(&mut w as *mut u32, &mut h as *mut u32) };
        if !ok {
            return Err("Failed to initialize DRM display hardware");
        }

        let stride = unsafe { display_drm_get_stride() };
        let refresh_rate = unsafe { display_drm_get_refresh_rate() };
        let fb_ptr = unsafe { display_drm_get_fb() };

        if fb_ptr.is_null() {
            return Err("DRM mapped framebuffer pointer is null");
        }

        Ok(Self {
            width: w,
            height: h,
            stride,
            refresh_rate,
            fb_ptr,
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

        unsafe {
            display_drm_flip();
        }
    }

    pub fn set_brightness(&self, val: i32) {
        unsafe { display_drm_set_brightness(val as libc::c_int) };
    }

    pub fn get_brightness(&self) -> i32 {
        unsafe { display_drm_get_brightness() as i32 }
    }

    pub fn set_screen_power(&self, on: bool) {
        unsafe { display_drm_set_power(on) };
    }

    pub fn is_screen_power(&self) -> bool {
        unsafe { display_drm_is_power() }
    }

    pub fn screenshot(&self, filepath: &str) -> bool {
        let c_str = match std::ffi::CString::new(filepath) {
            Ok(s) => s,
            Err(_) => return false,
        };
        unsafe { display_drm_screenshot(c_str.as_ptr()) }
    }
}

impl Drop for DrmDisplay {
    fn drop(&mut self) {
        unsafe {
            display_drm_cleanup();
        }
    }
}
