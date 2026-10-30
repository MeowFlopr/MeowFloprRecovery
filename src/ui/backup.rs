use std::fs;
use std::path::PathBuf;
use tiny_skia::Color;

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::slider::SwipeSlider;
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupTab {
    Backup,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageTarget {
    Internal,
    UsbOtg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    Lz4,
    Gzip,
    None,
}

#[derive(Debug, Clone)]
pub struct PartitionItem {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub size_mb: u64,
    pub is_checked: bool,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct BackupArchive {
    pub name: String,
    pub path: PathBuf,
    pub date_str: String,
    pub size_mb: u64,
    pub partitions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupAction {
    Back,
    SelectTab(BackupTab),
    TogglePartition(usize),
    ToggleStorageTarget,
    CycleCompression,
    ToggleSha256,
    SelectArchive(usize),
    ConfirmBackup,
    ConfirmRestore,
}

pub struct BackupState {
    pub current_tab: BackupTab,
    pub partitions: Vec<PartitionItem>,
    pub storage_target: StorageTarget,
    pub compression: CompressionMode,
    pub generate_sha256: bool,
    pub backup_slider: SwipeSlider,
    pub restore_slider: SwipeSlider,
    pub archives: Vec<BackupArchive>,
    pub selected_archive_idx: Option<usize>,
}

impl Default for BackupState {
    fn default() -> Self {
        let margin = 40.0;
        let content_w = 1220.0 - (margin * 2.0);
        let slider_x = margin + 40.0;
        let slider_w = content_w - 80.0;
        let slider_h = 130.0;

        let backup_slider = SwipeSlider::new(slider_x, 2280.0, slider_w, slider_h, "Swipe to Backup >>>")
            .with_accent(Color::from_rgba8(123, 208, 255, 255));
        let restore_slider = SwipeSlider::new(slider_x, 2280.0, slider_w, slider_h, "Swipe to Restore >>>")
            .with_accent(Color::from_rgba8(208, 188, 255, 255));

        let partitions = vec![
            PartitionItem { id: "boot", name: "Boot Kernel (boot_a/b)", desc: "Linux 6.1 GKI kernel image", size_mb: 64, is_checked: true, is_critical: false },
            PartitionItem { id: "vendor_boot", name: "Vendor Boot (vendor_boot)", desc: "Ramdisk, DTB & hardware modules", size_mb: 64, is_checked: true, is_critical: false },
            PartitionItem { id: "dtbo", name: "DTBO & Init_boot", desc: "Device tree overlay & early boot", size_mb: 16, is_checked: true, is_critical: false },
            PartitionItem { id: "super", name: "Super (System, Vendor, Product)", desc: "All core OS filesystem partitions (~9 GB)", size_mb: 9216, is_checked: false, is_critical: false },
            PartitionItem { id: "persist", name: "Persist (Sensors & Calibrations)", desc: "Touch, light, camera & Wi-Fi calibs", size_mb: 64, is_checked: true, is_critical: true },
            PartitionItem { id: "nvram", name: "NVRAM / NVDATA / Protect", desc: "Modem baseband, radio & IMEI calibrations", size_mb: 128, is_checked: true, is_critical: true },
            PartitionItem { id: "metadata", name: "Metadata & Misc", desc: "FBE v2 encryption metadata & bootloader control", size_mb: 32, is_checked: false, is_critical: false },
        ];

        let mut s = Self {
            current_tab: BackupTab::Backup,
            partitions,
            storage_target: StorageTarget::Internal,
            compression: CompressionMode::Lz4,
            generate_sha256: true,
            backup_slider,
            restore_slider,
            archives: Vec::new(),
            selected_archive_idx: None,
        };
        s.scan_archives();
        s
    }
}

impl BackupState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_spring(&mut self) -> bool {
        match self.current_tab {
            BackupTab::Backup => self.backup_slider.tick_spring(),
            BackupTab::Restore => self.restore_slider.tick_spring(),
        }
    }

    pub fn total_selected_size_mb(&self) -> u64 {
        self.partitions.iter().filter(|p| p.is_checked).map(|p| p.size_mb).sum()
    }

    pub fn reset_sliders(&mut self) {
        self.backup_slider.reset();
        self.restore_slider.reset();
    }

    pub fn scan_archives(&mut self) {
        self.archives.clear();
        let paths = [
            PathBuf::from("/sdcard/MRR/BACKUPS"),
            PathBuf::from("/mnt/media_rw/usb_otg/MRR/BACKUPS"),
            PathBuf::from("/usb_otg/MRR/BACKUPS"),
        ];

        for dir in &paths {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let mut size_bytes = 0u64;
                        let mut parts = Vec::new();
                        if let Ok(sub) = fs::read_dir(&path) {
                            for f in sub.flatten() {
                                if let Ok(meta) = f.metadata() {
                                    size_bytes += meta.len();
                                    parts.push(f.file_name().to_string_lossy().to_string());
                                }
                            }
                        }
                        let date_str = if let Ok(meta) = entry.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                                    let days = dur.as_secs() / 86400;
                                    let year = 1970 + days / 365;
                                    let month = ((days % 365) / 30) + 1;
                                    let day = ((days % 365) % 30) + 1;
                                    format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(31))
                                } else {
                                    "Unknown".to_string()
                                }
                            } else {
                                "Unknown".to_string()
                            }
                        } else {
                            "Unknown".to_string()
                        };
                        self.archives.push(BackupArchive {
                            name,
                            path,
                            date_str,
                            size_mb: size_bytes / (1024 * 1024),
                            partitions: parts,
                        });
                    }
                }
            }
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<BackupAction> {
        let fx = tx as f32;
        let fy = ty as f32;
        let margin = 40.0;
        let content_w = 1220.0 - (margin * 2.0);

        if state == TouchState::Released {
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                return Some(BackupAction::Back);
            }
            if fx >= 40.0 && fx <= 340.0 && fy >= 2460.0 && fy <= 2580.0 {
                return Some(BackupAction::Back);
            }
        }

        if state == TouchState::Released && fy >= 200.0 && fy <= 280.0 {
            let tab_w = (content_w - 16.0) / 2.0;
            if fx >= margin && fx <= margin + tab_w {
                self.current_tab = BackupTab::Backup;
                self.reset_sliders();
                return Some(BackupAction::SelectTab(BackupTab::Backup));
            } else if fx >= margin + tab_w + 16.0 && fx <= margin + tab_w * 2.0 + 16.0 {
                self.current_tab = BackupTab::Restore;
                self.reset_sliders();
                self.scan_archives();
                return Some(BackupAction::SelectTab(BackupTab::Restore));
            }
        }

        if self.current_tab == BackupTab::Backup {
            if state == TouchState::Released {
                if fy >= 300.0 && fy <= 370.0 && fx >= margin && fx <= margin + 450.0 {
                    self.storage_target = match self.storage_target {
                        StorageTarget::Internal => StorageTarget::UsbOtg,
                        StorageTarget::UsbOtg => StorageTarget::Internal,
                    };
                    return Some(BackupAction::ToggleStorageTarget);
                }

                if fy >= 300.0 && fy <= 370.0 && fx >= margin + 470.0 && fx <= margin + 820.0 {
                    self.compression = match self.compression {
                        CompressionMode::Lz4 => CompressionMode::Gzip,
                        CompressionMode::Gzip => CompressionMode::None,
                        CompressionMode::None => CompressionMode::Lz4,
                    };
                    return Some(BackupAction::CycleCompression);
                }

                if fy >= 300.0 && fy <= 370.0 && fx >= margin + 840.0 && fx <= 1220.0 - margin {
                    self.generate_sha256 = !self.generate_sha256;
                    return Some(BackupAction::ToggleSha256);
                }

                let row_h = 135.0;
                let start_y = 400.0;
                let gap = 14.0;

                for (i, p) in self.partitions.iter_mut().enumerate() {
                    let cy = start_y + (i as f32) * (row_h + gap);
                    if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + row_h {
                        p.is_checked = !p.is_checked;
                        return Some(BackupAction::TogglePartition(i));
                    }
                }
            }

            if self.backup_slider.handle_touch(tx, ty, state) {
                self.backup_slider.reset();
                return Some(BackupAction::ConfirmBackup);
            }
        }

        if self.current_tab == BackupTab::Restore {
            if state == TouchState::Released {
                let item_h = 160.0;
                let start_y = 320.0;
                let gap = 16.0;

                for (i, _arc) in self.archives.iter().enumerate() {
                    let cy = start_y + (i as f32) * (item_h + gap);
                    if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + item_h {
                        self.selected_archive_idx = Some(i);
                        self.restore_slider.reset();
                        return Some(BackupAction::SelectArchive(i));
                    }
                }
            }

            if self.selected_archive_idx.is_some() && self.restore_slider.handle_touch(tx, ty, state) {
                self.restore_slider.reset();
                return Some(BackupAction::ConfirmRestore);
            }
        }

        None
    }
}

pub fn render_backup(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &BackupState,
    pressed: Option<BackupAction>,
) {
    canvas.clear(theme.background);

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

    let back_cx = margin + 44.0;
    let back_cy = 115.0;
    let is_back_pressed = pressed == Some(BackupAction::Back);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    fonts.draw_text(&mut canvas.pixmap, "Backup & Restore", back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, "Partition snapshots & recovery archives", back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    let tab_y = 200.0;
    let tab_h = 76.0;
    let tab_w = (content_w - 16.0) / 2.0;

    let tabs = [("Backup (Создание)", BackupTab::Backup), ("Restore (Восстановление)", BackupTab::Restore)];

    for (i, (title, tab_enum)) in tabs.iter().enumerate() {
        let tx = margin + (i as f32) * (tab_w + 16.0);
        let is_selected = state.current_tab == *tab_enum;

        let (bg, fg, border) = if is_selected {
            (theme.tertiary_container, theme.tertiary, Some((theme.tertiary, 2.0)))
        } else {
            (theme.surface_container, theme.on_surface_variant, Some((theme.outline_variant, 1.0)))
        };

        canvas.draw_pill(tx, tab_y, tab_w, tab_h, bg, border);
        let tw = fonts.measure_text(title, 25.0);
        fonts.draw_text(&mut canvas.pixmap, title, tx + (tab_w - tw) / 2.0, tab_y + 48.0, 25.0, fg);
    }

    match state.current_tab {
        BackupTab::Backup => {
            let chip_y = 300.0;
            let chip_h = 70.0;

            let (target_lbl, target_accent) = match state.storage_target {
                StorageTarget::Internal => ("Хранилище: /sdcard", theme.primary),
                StorageTarget::UsbOtg => ("Хранилище: USB-OTG", theme.tertiary),
            };
            canvas.draw_pill(margin, chip_y, 440.0, chip_h, theme.surface_container_high, Some((theme.outline_variant, 1.5)));
            draw_icon(&mut canvas.pixmap, M3Icon::Storage, margin + 35.0, chip_y + 35.0, 26.0, target_accent);
            fonts.draw_text(&mut canvas.pixmap, target_lbl, margin + 65.0, chip_y + 45.0, 21.0, theme.on_surface);

            let comp_lbl = match state.compression {
                CompressionMode::Lz4 => "Сжатие: LZ4 (Fast)",
                CompressionMode::Gzip => "Сжатие: Gzip (Max)",
                CompressionMode::None => "Сжатие: Без сжатия",
            };
            canvas.draw_pill(margin + 460.0, chip_y, 350.0, chip_h, theme.surface_container_high, Some((theme.outline_variant, 1.5)));
            fonts.draw_text(&mut canvas.pixmap, comp_lbl, margin + 485.0, chip_y + 45.0, 21.0, theme.on_surface);

            let (sha_bg, sha_fg) = if state.generate_sha256 {
                (theme.primary_container, theme.primary)
            } else {
                (theme.surface_container_high, theme.on_surface_variant)
            };
            canvas.draw_pill(margin + 830.0, chip_y, content_w - 830.0, chip_h, sha_bg, Some((theme.outline_variant, 1.5)));
            let sha_lbl = if state.generate_sha256 { "SHA256: ON" } else { "SHA256: OFF" };
            fonts.draw_text(&mut canvas.pixmap, sha_lbl, margin + 865.0, chip_y + 45.0, 21.0, sha_fg);

            let row_h = 135.0;
            let start_y = 390.0;
            let gap = 14.0;

            for (i, p) in state.partitions.iter().enumerate() {
                let cy = start_y + (i as f32) * (row_h + gap);
                let bg = if p.is_checked { theme.surface_container_high } else { theme.surface_container };
                let border = if p.is_checked { Some((theme.tertiary, 1.5)) } else { Some((theme.outline_variant, 1.0)) };

                canvas.draw_rounded_rect(margin, cy, content_w, row_h, 24.0, Some(bg), border);

                let chk_cx = margin + 50.0;
                let chk_cy = cy + row_h / 2.0;
                let chk_color = if p.is_checked { theme.tertiary } else { theme.surface_container_highest };
                canvas.draw_circle(chk_cx, chk_cy, 30.0, chk_color);
                if p.is_checked {
                    draw_icon(&mut canvas.pixmap, M3Icon::Check, chk_cx, chk_cy, 30.0, theme.background);
                }

                fonts.draw_text(&mut canvas.pixmap, p.name, chk_cx + 46.0, cy + 54.0, 27.0, theme.on_surface);
                fonts.draw_text(&mut canvas.pixmap, p.desc, chk_cx + 46.0, cy + 96.0, 19.0, theme.on_surface_variant);

                let size_str = if p.size_mb >= 1024 {
                    format!("{:.1} GB", (p.size_mb as f32) / 1024.0)
                } else {
                    format!("{} MB", p.size_mb)
                };
                let sw = fonts.measure_text(&size_str, 22.0);
                let badge_x = screen_w - margin - sw - 40.0;
                fonts.draw_text(&mut canvas.pixmap, &size_str, badge_x, cy + 54.0, 22.0, theme.tertiary);

                if p.is_critical {
                    fonts.draw_text(&mut canvas.pixmap, "IMEI/Калибровки", badge_x - 90.0, cy + 96.0, 18.0, Color::from_rgba8(255, 170, 80, 255));
                }
            }

            let total_mb = state.total_selected_size_mb();
            let total_str = if total_mb >= 1024 {
                format!("Размер дампа: {:.2} ГБ • Доступно на диске: 213 ГБ", total_mb as f32 / 1024.0)
            } else {
                format!("Размер дампа: {} МБ • Доступно на диске: 213 ГБ", total_mb)
            };
            let sum_y = 1460.0;
            canvas.draw_rounded_rect(margin, sum_y, content_w, 90.0, 24.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.0)));
            fonts.draw_text(&mut canvas.pixmap, &total_str, margin + 40.0, sum_y + 55.0, 23.0, theme.on_surface);

            state.backup_slider.render(canvas, theme, fonts);
        }

        BackupTab::Restore => {
            if state.archives.is_empty() {
                let empty_y = 400.0;
                let empty_h = 400.0;
                canvas.draw_rounded_rect(margin, empty_y, content_w, empty_h, 32.0, Some(theme.surface_container), Some((theme.outline_variant, 1.0)));
                draw_icon(&mut canvas.pixmap, M3Icon::Backup, screen_w / 2.0, empty_y + 140.0, 70.0, theme.on_surface_variant);
                let e_msg = "Резервные копии не найдены";
                let ew = fonts.measure_text(e_msg, 32.0);
                fonts.draw_text(&mut canvas.pixmap, e_msg, (screen_w - ew) / 2.0, empty_y + 240.0, 32.0, theme.on_surface);
                let e_sub = "Создайте копию во вкладке 'Backup' (/sdcard/MRR/BACKUPS/)";
                let esw = fonts.measure_text(e_sub, 22.0);
                fonts.draw_text(&mut canvas.pixmap, e_sub, (screen_w - esw) / 2.0, empty_y + 290.0, 22.0, theme.on_surface_variant);
            } else {
                let start_y = 320.0;
                let item_h = 160.0;
                let gap = 16.0;

                for (i, arc) in state.archives.iter().enumerate() {
                    let cy = start_y + (i as f32) * (item_h + gap);
                    let is_sel = state.selected_archive_idx == Some(i);

                    let bg = if is_sel { theme.primary_container } else { theme.surface_container_high };
                    let border = if is_sel { Some((theme.primary, 2.0)) } else { Some((theme.outline_variant, 1.0)) };

                    canvas.draw_rounded_rect(margin, cy, content_w, item_h, 28.0, Some(bg), border);

                    draw_icon(&mut canvas.pixmap, M3Icon::Folder, margin + 60.0, cy + item_h / 2.0, 48.0, if is_sel { theme.primary } else { theme.tertiary });

                    fonts.draw_text(&mut canvas.pixmap, &arc.name, margin + 110.0, cy + 60.0, 30.0, theme.on_surface);
                    let meta_desc = format!("Дата: {} • Размер: {} МБ • Разделов: {}", arc.date_str, arc.size_mb, arc.partitions.len());
                    fonts.draw_text(&mut canvas.pixmap, &meta_desc, margin + 110.0, cy + 105.0, 21.0, theme.on_surface_variant);

                    if is_sel {
                        draw_icon(&mut canvas.pixmap, M3Icon::Check, screen_w - margin - 60.0, cy + item_h / 2.0, 36.0, theme.primary);
                    }
                }

                if state.selected_archive_idx.is_some() {
                    state.restore_slider.render(canvas, theme, fonts);
                }
            }
        }
    }

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

