//! DRM Atomic Display Backend for Xiaomi 14T (MediaTek MT6897)
//! Provides safe Rust abstractions over the dual-pipe atomic DRM hardware plane.

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
    /// Initialize the atomic dual-pipe DRM display on `/dev/dri/card0`.
    pub fn init() -> Result<Self, &'static str> {
        let mut w: u32 = 0;
        let mut h: u32 = 0;

        // SAFETY: display_drm_init opens /dev/dri/card0 and assigns pointers.
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

    /// Blit a tiny-skia Pixmap into the DRM framebuffer and trigger an atomic flip.
    /// TinySkia produces premultiplied RGBA (R, G, B, A in byte order).
    /// DRM uses ARGB8888 (which on LE is B, G, R, A in byte order).
    /// We swizzle R and B during transfer to match the AMOLED panel hardware.
    pub fn blit_and_flip(&mut self, pixmap: &Pixmap) {
        if self.fb_ptr.is_null() {
            return;
        }

        let pw = pixmap.width() as usize;
        let ph = pixmap.height() as usize;
        let stride = self.stride as usize;
        let src_bytes = pixmap.data();

        // SAFETY: fb_ptr points to the mapped DRM dumb buffer of size stride * height * 4 bytes.
        let dst_slice = unsafe {
            std::slice::from_raw_parts_mut(self.fb_ptr, stride * (self.height as usize))
        };

        let copy_w = pw.min(self.width as usize);
        let copy_h = ph.min(self.height as usize);

        if pw == stride && copy_w == stride {
            // Ultra-fast 1D contiguous conversion with SIMD auto-vectorization
            let total_pixels = copy_w * copy_h;
            let src_chunks = src_bytes[..total_pixels * 4].chunks_exact(4);
            let dst_pixels = &mut dst_slice[..total_pixels];
            for (chunk, dst) in src_chunks.zip(dst_pixels.iter_mut()) {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                let a = chunk[3] as u32;
                *dst = (a << 24) | (r << 16) | (g << 8) | b;
            }
        } else {
            // Row-by-row conversion when dimensions differ
            for y in 0..copy_h {
                let src_row = &src_bytes[y * pw * 4..(y * pw + copy_w) * 4];
                let dst_row = &mut dst_slice[y * stride..y * stride + copy_w];
                for (chunk, dst) in src_row.chunks_exact(4).zip(dst_row.iter_mut()) {
                    let r = chunk[0] as u32;
                    let g = chunk[1] as u32;
                    let b = chunk[2] as u32;
                    let a = chunk[3] as u32;
                    *dst = (a << 24) | (r << 16) | (g << 8) | b;
                }
            }
        }

        // Trigger atomic page flip (or dirty fb notification)
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
