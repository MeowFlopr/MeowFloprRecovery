use std::ffi::CString;
use std::fs;
use std::path::Path;
use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

#[derive(Debug, Clone)]
pub struct MountItem {
    pub name: &'static str,
    pub path: &'static str,
    pub dev_block: &'static str,
    pub fstype: &'static str,
    pub is_mounted: bool,
    pub is_system_group: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountAction {
    Back,
    ToggleMount(usize),
    ToggleReadOnly,
    ToggleMtp,
    LaunchSideload,
}

pub struct MountState {
    pub partitions: Vec<MountItem>,
    pub mount_system_ro: bool,
    pub mtp_enabled: bool,
    pub status_message: String,
}

impl Default for MountState {
    fn default() -> Self {
        let mut partitions = vec![
            MountItem { name: "System (/system)", path: "/system", dev_block: "/dev/block/mapper/system", fstype: "erofs", is_mounted: false, is_system_group: true },
            MountItem { name: "Vendor (/vendor)", path: "/vendor", dev_block: "/dev/block/mapper/vendor", fstype: "erofs", is_mounted: false, is_system_group: true },
            MountItem { name: "Product (/product)", path: "/product", dev_block: "/dev/block/mapper/product", fstype: "erofs", is_mounted: false, is_system_group: true },
            MountItem { name: "System Ext (/system_ext)", path: "/system_ext", dev_block: "/dev/block/mapper/system_ext", fstype: "erofs", is_mounted: false, is_system_group: true },
            MountItem { name: "Data (/data)", path: "/data", dev_block: "/dev/block/mapper/userdata", fstype: "f2fs", is_mounted: false, is_system_group: false },
            MountItem { name: "Cache (/cache)", path: "/cache", dev_block: "/dev/block/by-name/rescue", fstype: "ext4", is_mounted: false, is_system_group: false },
            MountItem { name: "Metadata (/metadata)", path: "/metadata", dev_block: "/dev/block/by-name/metadata", fstype: "f2fs", is_mounted: false, is_system_group: false },
            MountItem { name: "Internal Storage (/sdcard)", path: "/sdcard", dev_block: "/data/media/0", fstype: "none", is_mounted: false, is_system_group: false },
            MountItem { name: "USB-OTG", path: "/usb_otg", dev_block: "/dev/block/sda1", fstype: "vfat", is_mounted: false, is_system_group: false },
        ];

        // Check initial mount status from /proc/mounts
        for p in &mut partitions {
            p.is_mounted = is_path_mounted(p.path);
        }

        Self {
            partitions,
            mount_system_ro: true,
            mtp_enabled: false,
            status_message: "Touch partition to mount or unmount".to_string(),
        }
    }
}

pub fn is_path_mounted(mount_point: &str) -> bool {
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let mut parts = line.split_whitespace();
            if let Some(target) = parts.nth(1) {
                if target == mount_point {
                    return true;
                }
            }
        }
    }
    false
}

impl MountState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh_mounts(&mut self) {
        for p in &mut self.partitions {
            p.is_mounted = is_path_mounted(p.path);
        }
    }

    pub fn toggle_mount_item(&mut self, idx: usize) {
        if idx >= self.partitions.len() {
            return;
        }

        let p = &mut self.partitions[idx];
        let currently_mounted = is_path_mounted(p.path);

        if currently_mounted {
            // Unmount
            let c_path = CString::new(p.path).unwrap_or_default();
            let ret = unsafe { libc::umount2(c_path.as_ptr(), libc::MNT_FORCE) };
            if ret == 0 {
                p.is_mounted = false;
                self.status_message = format!("Unmounted {}", p.name);
            } else {
                self.status_message = format!("Unmount failed for {}", p.path);
            }
        } else {
            // Ensure target directory exists
            let _ = fs::create_dir_all(p.path);

            let flags = if p.is_system_group && self.mount_system_ro {
                libc::MS_RDONLY
            } else if p.path == "/sdcard" {
                libc::MS_BIND
            } else {
                0
            };

            let c_dev = CString::new(p.dev_block).unwrap_or_default();
            let c_path = CString::new(p.path).unwrap_or_default();
            let c_fs = CString::new(p.fstype).unwrap_or_default();

            let mut ret = unsafe {
                libc::mount(c_dev.as_ptr(), c_path.as_ptr(), c_fs.as_ptr(), flags, std::ptr::null())
            };

            // Fallback filesystem types if erofs fails
            if ret != 0 && p.fstype == "erofs" {
                let c_ext4 = CString::new("ext4").unwrap();
                ret = unsafe {
                    libc::mount(c_dev.as_ptr(), c_path.as_ptr(), c_ext4.as_ptr(), flags, std::ptr::null())
                };
                if ret != 0 {
                    let c_f2fs = CString::new("f2fs").unwrap();
                    ret = unsafe {
                        libc::mount(c_dev.as_ptr(), c_path.as_ptr(), c_f2fs.as_ptr(), flags, std::ptr::null())
                    };
                }
            }

            if ret == 0 {
                p.is_mounted = true;
                self.status_message = format!("Mounted {}", p.name);
            } else {
                self.status_message = format!("Mount failed for {} (err: {})", p.path, std::io::Error::last_os_error());
            }
        }
    }

    pub fn toggle_mtp(&mut self) {
        self.mtp_enabled = !self.mtp_enabled;
        if self.mtp_enabled {
            // Setup Android USB gadget ConfigFS
            let _ = fs::write("/config/usb_gadget/g1/UDC", "none\n");
            let _ = fs::write("/config/usb_gadget/g1/configs/b.1/strings/0x409/configuration", "mtp_adb\n");
            let _ = fs::create_dir_all("/config/usb_gadget/g1/functions/mtp.gs0");
            let _ = std::os::unix::fs::symlink(
                "/config/usb_gadget/g1/functions/mtp.gs0",
                "/config/usb_gadget/g1/configs/b.1/f1",
            );
            // Re-bind UDC controller
            if let Ok(udc) = fs::read_to_string("/sys/class/udc") {
                if let Some(first_udc) = udc.lines().next() {
                    let _ = fs::write("/config/usb_gadget/g1/UDC", format!("{}\n", first_udc.trim()));
                }
            }
            self.status_message = "MTP Enabled: Device visible in Windows Explorer".to_string();
        } else {
            self.status_message = "MTP Disabled".to_string();
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<MountAction> {
        let fx = tx as f32;
        let fy = ty as f32;
        let margin = 40.0;
        let content_w = 1220.0 - (margin * 2.0);

        if state == TouchState::Released {
            // 1. Back button
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                return Some(MountAction::Back);
            }
            if fx >= 40.0 && fx <= 340.0 && fy >= 2460.0 && fy <= 2580.0 {
                return Some(MountAction::Back);
            }

            // 2. Control Cards (Read-Only Switch, Enable MTP, ADB Sideload)
            // y: 200 .. 340
            let card_y = 200.0;
            let card_h = 110.0;
            let btn_w = (content_w - 32.0) / 3.0; // ~368.0

            // Btn 1: Read-Only System toggle
            if fy >= card_y && fy <= card_y + card_h && fx >= margin && fx <= margin + btn_w {
                self.mount_system_ro = !self.mount_system_ro;
                return Some(MountAction::ToggleReadOnly);
            }

            // Btn 2: Enable MTP
            let mtp_x = margin + btn_w + 16.0;
            if fy >= card_y && fy <= card_y + card_h && fx >= mtp_x && fx <= mtp_x + btn_w {
                self.toggle_mtp();
                return Some(MountAction::ToggleMtp);
            }

            // Btn 3: ADB Sideload
            let side_x = mtp_x + btn_w + 16.0;
            if fy >= card_y && fy <= card_y + card_h && fx >= side_x && fx <= side_x + btn_w {
                return Some(MountAction::LaunchSideload);
            }

            // 3. Partition Checklist items: y: 360 .. 2300
            let row_h = 135.0;
            let start_y = 350.0;
            let gap = 14.0;

            for (i, _) in self.partitions.iter().enumerate() {
                let cy = start_y + (i as f32) * (row_h + gap);
                if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + row_h {
                    self.toggle_mount_item(i);
                    return Some(MountAction::ToggleMount(i));
                }
            }
        }

        None
    }
}

pub fn render_mount(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &MountState,
    pressed: Option<MountAction>,
) {
    canvas.clear(theme.background);

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0); // 1140.0

    // =========================================================================
    // 1. TOP APP BAR
    // =========================================================================
    let back_cx = margin + 44.0;
    let back_cy = 115.0;
    let is_back_pressed = pressed == Some(MountAction::Back);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    fonts.draw_text(&mut canvas.pixmap, "Mount & Storage", back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, "Mount partitions, MTP USB & ADB sideload", back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    // =========================================================================
    // 2. QUICK ACTIONS BAR (Read-Only Switch, MTP Toggle, Sideload)
    // =========================================================================
    let card_y = 195.0;
    let card_h = 125.0;
    let btn_w = (content_w - 32.0) / 3.0; // ~368.0

    // 1. Read-Only Toggle Card
    let (ro_bg, ro_fg, ro_text) = if state.mount_system_ro {
        (theme.surface_container_high, theme.primary, "System: Read-Only")
    } else {
        (theme.error_container, theme.error, "System: Read-Write!")
    };
    canvas.draw_rounded_rect(margin, card_y, btn_w, card_h, 24.0, Some(ro_bg), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::Lock, margin + 40.0, card_y + 40.0, 28.0, ro_fg);
    fonts.draw_text(&mut canvas.pixmap, ro_text, margin + 24.0, card_y + 88.0, 21.0, theme.on_surface);

    // 2. MTP Toggle Card
    let mtp_x = margin + btn_w + 16.0;
    let (mtp_bg, mtp_fg, mtp_text) = if state.mtp_enabled {
        (Color::from_rgba8(20, 60, 35, 255), Color::from_rgba8(120, 240, 160, 255), "MTP: Включён")
    } else {
        (theme.surface_container_high, theme.on_surface_variant, "Включить MTP")
    };
    canvas.draw_rounded_rect(mtp_x, card_y, btn_w, card_h, 24.0, Some(mtp_bg), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::Usb, mtp_x + 40.0, card_y + 40.0, 32.0, mtp_fg);
    fonts.draw_text(&mut canvas.pixmap, mtp_text, mtp_x + 24.0, card_y + 88.0, 21.0, theme.on_surface);

    // 3. ADB Sideload Card
    let side_x = mtp_x + btn_w + 16.0;
    canvas.draw_rounded_rect(side_x, card_y, btn_w, card_h, 24.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::Terminal, side_x + 40.0, card_y + 40.0, 28.0, theme.tertiary);
    fonts.draw_text(&mut canvas.pixmap, "ADB Sideload", side_x + 24.0, card_y + 88.0, 21.0, theme.on_surface);

    // =========================================================================
    // 3. PARTITION CHECKLIST (y: 350 .. 2350)
    // =========================================================================
    let row_h = 135.0;
    let start_y = 350.0;
    let gap = 14.0;

    for (i, p) in state.partitions.iter().enumerate() {
        let cy = start_y + (i as f32) * (row_h + gap);
        let bg = if p.is_mounted { theme.surface_container_high } else { theme.surface_container };
        let border = if p.is_mounted { Some((theme.primary, 1.5)) } else { Some((theme.outline_variant, 1.0)) };

        canvas.draw_rounded_rect(margin, cy, content_w, row_h, 24.0, Some(bg), border);

        let chk_cx = margin + 50.0;
        let chk_cy = cy + row_h / 2.0;
        let chk_color = if p.is_mounted { theme.primary } else { theme.surface_container_highest };
        canvas.draw_circle(chk_cx, chk_cy, 30.0, chk_color);
        if p.is_mounted {
            draw_icon(&mut canvas.pixmap, M3Icon::Check, chk_cx, chk_cy, 30.0, theme.on_primary);
        }

        fonts.draw_text(&mut canvas.pixmap, p.name, chk_cx + 46.0, cy + 54.0, 27.0, theme.on_surface);
        let status_desc = if p.is_mounted {
            format!("Примонтирован ({}) -> {}", p.fstype, p.path)
        } else {
            format!("Размонтирован • ФС: {}", p.fstype)
        };
        let status_color = if p.is_mounted { Color::from_rgba8(120, 240, 160, 255) } else { theme.on_surface_variant };
        fonts.draw_text(&mut canvas.pixmap, &status_desc, chk_cx + 46.0, cy + 96.0, 20.0, status_color);

        // Right side badge
        let badge_txt = if p.is_mounted { "MOUNTED" } else { "UNMOUNTED" };
        let bw = fonts.measure_text(badge_txt, 19.0);
        let bx = screen_w - margin - bw - 40.0;
        fonts.draw_text(&mut canvas.pixmap, badge_txt, bx, cy + 75.0, 19.0, status_color);
    }

    // Status Message Pill
    let msg_y = start_y + (state.partitions.len() as f32) * (row_h + gap) + 10.0;
    canvas.draw_pill(margin, msg_y, content_w, 75.0, theme.surface_container_highest, Some((theme.outline_variant, 1.0)));
    fonts.draw_text(&mut canvas.pixmap, &state.status_message, margin + 40.0, msg_y + 46.0, 21.0, theme.on_surface);

    // =========================================================================
    // 4. BOTTOM DOCK BACK
    // =========================================================================
    let dock_y = 2460.0;
    let dock_h = 110.0;
    let btn_w = 280.0;

    canvas.draw_pill(
        margin,
        dock_y,
        btn_w,
        dock_h,
        if is_back_pressed { theme.primary_container } else { theme.surface_container_high },
        Some((theme.outline_variant, 1.5)),
    );
    let lbl = "<- Dashboard";
    let w = fonts.measure_text(lbl, 26.0);
    fonts.draw_text(&mut canvas.pixmap, lbl, margin + (btn_w - w) / 2.0, dock_y + 64.0, 26.0, theme.on_surface);
}
