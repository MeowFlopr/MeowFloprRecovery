use std::fs;
use std::path::{Path, PathBuf};
use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::slider::SwipeSlider;
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size_str: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionTarget {
    InitBoot,
    Boot,
    VendorBoot,
    Dtbo,
}

impl PartitionTarget {
    pub fn name(&self) -> &'static str {
        match self {
            PartitionTarget::InitBoot => "init_boot",
            PartitionTarget::Boot => "boot",
            PartitionTarget::VendorBoot => "vendor_boot",
            PartitionTarget::Dtbo => "dtbo",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            PartitionTarget::InitBoot => "Init Boot",
            PartitionTarget::Boot => "Boot (Kernel)",
            PartitionTarget::VendorBoot => "Vendor Boot",
            PartitionTarget::Dtbo => "DTBO",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerAction {
    Back,
    ItemClick(usize),
    FlashConfirm,
    FlashCancel,
}

pub struct InstallerState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_file: Option<FileEntry>,
    pub target_partition: PartitionTarget,
    pub flash_slider: SwipeSlider,
}

impl Default for InstallerState {
    fn default() -> Self {
        let card_w = 1040.0;
        let box_x = (1220.0 - card_w) / 2.0 + 48.0;
        let box_w = card_w - 96.0; // 944.0
        let slider_y = 650.0 + 580.0; // 1230.0
        let slider_h = 130.0;

        let flash_slider = SwipeSlider::new(box_x, slider_y, box_w, slider_h, "Swipe to Flash >>>");

        let mut s = Self {
            current_dir: PathBuf::from("/sdcard"),
            entries: Vec::new(),
            selected_file: None,
            target_partition: PartitionTarget::InitBoot,
            flash_slider,
        };
        s.load_dir();
        s
    }
}

impl InstallerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_spring(&mut self) -> bool {
        if self.selected_file.is_some() {
            return self.flash_slider.tick_spring();
        }
        false
    }

    pub fn load_dir(&mut self) {
        self.entries.clear();

        // If not root, add parent directory entry
        if self.current_dir.parent().is_some() && self.current_dir != Path::new("/") {
            self.entries.push(FileEntry {
                name: ".. (Parent Directory)".to_string(),
                is_dir: true,
                size_str: "Folder".to_string(),
                path: self.current_dir.parent().unwrap().to_path_buf(),
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }

                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        dirs.push(FileEntry {
                            name,
                            is_dir: true,
                            size_str: "Folder".to_string(),
                            path,
                        });
                    } else {
                        let size_bytes = meta.len();
                        let size_str = if size_bytes > 1024 * 1024 {
                            format!("{:.1} MB", (size_bytes as f64) / (1024.0 * 1024.0))
                        } else {
                            format!("{:.1} KB", (size_bytes as f64) / 1024.0)
                        };

                        files.push(FileEntry {
                            name,
                            is_dir: false,
                            size_str,
                            path,
                        });
                    }
                }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            self.entries.extend(dirs);
            self.entries.extend(files);
        } else {
            // If /sdcard isn't available, show empty notice
            self.entries.push(FileEntry {
                name: "Storage not mounted or empty (/sdcard)".to_string(),
                is_dir: false,
                size_str: "".to_string(),
                path: PathBuf::new(),
            });
        }
    }

    pub fn enter_entry(&mut self, idx: usize) {
        if let Some(entry) = self.entries.get(idx).cloned() {
            if entry.is_dir {
                self.current_dir = entry.path;
                self.load_dir();
            } else if entry.name.ends_with(".zip") || entry.name.ends_with(".img") || entry.name.ends_with(".apk") {
                let name_lower = entry.name.to_lowercase();
                self.target_partition = if name_lower.contains("init_boot") || name_lower.contains("boot_init") || name_lower.contains("kernelsu") {
                    PartitionTarget::InitBoot
                } else if name_lower.contains("vendor_boot") {
                    PartitionTarget::VendorBoot
                } else if name_lower.contains("dtbo") {
                    PartitionTarget::Dtbo
                } else {
                    PartitionTarget::Boot
                };
                self.selected_file = Some(entry);
                self.flash_slider.reset();
            }
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<InstallerAction> {
        let fx = tx as f32;
        let fy = ty as f32;

        // If modal confirmation is open
        if let Some(ref file_entry) = self.selected_file {
            let is_img = file_entry.name.ends_with(".img");
            let card_w = 1040.0;
            let box_x = (1220.0 - card_w) / 2.0 + 48.0;
            let box_w = card_w - 96.0;

            let card_y = 650.0;
            let box_y = card_y + 190.0;
            let box_h = 210.0;

            // When flashing .img, partition pills are selectable
            if is_img && (state == TouchState::Released || state == TouchState::Pressed) {
                let part_sel_y = box_y + box_h + 20.0;
                let pill_y = part_sel_y + 42.0;
                let pill_h = 68.0;

                let targets = [
                    (PartitionTarget::InitBoot, box_x),
                    (PartitionTarget::Boot, box_x + 238.0),
                    (PartitionTarget::VendorBoot, box_x + 476.0),
                    (PartitionTarget::Dtbo, box_x + 714.0),
                ];

                for (target, px) in targets {
                    if fx >= px && fx <= (px + 225.0) && fy >= pill_y && fy <= (pill_y + pill_h) {
                        self.target_partition = target;
                        return None;
                    }
                }
            }

            let slider_y = if is_img { box_y + box_h + 155.0 } else { box_y + box_h + 35.0 };
            let slider_h = 130.0;
            self.flash_slider.y = slider_y;
            self.flash_slider.height = slider_h;

            let cancel_y = slider_y + slider_h + 25.0;
            let cancel_h = 105.0;

            // Cancel Button
            if state == TouchState::Released && fx >= box_x && fx <= (box_x + box_w) && fy >= cancel_y && fy <= (cancel_y + cancel_h) {
                self.selected_file = None;
                self.flash_slider.reset();
                return Some(InstallerAction::FlashCancel);
            }

            // Swipe to Flash Slider
            if self.flash_slider.handle_touch(tx, ty, state) {
                // DO NOT take selected_file here! Keep it for main.rs to read.
                self.flash_slider.reset();
                return Some(InstallerAction::FlashConfirm);
            }

            return None;
        }

        if state == TouchState::Released {
            // Top Bar Back (x: 40..160, y: 70..170)
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                return Some(InstallerAction::Back);
            }

            // Bottom Back Pill (x: 40..340, y: 2460..2580)
            if fx >= 40.0 && fx <= 340.0 && fy >= 2460.0 && fy <= 2580.0 {
                return Some(InstallerAction::Back);
            }

            // Item rows list: y: 210..2420
            let list_start_y = 210.0;
            let row_h = 120.0;
            let row_gap = 14.0;

            if fy >= list_start_y && fy <= 2420.0 && fx >= 40.0 && fx <= 1180.0 {
                let rel_y = fy - list_start_y;
                let idx = (rel_y / (row_h + row_gap)) as usize;
                if idx < self.entries.len() {
                    return Some(InstallerAction::ItemClick(idx));
                }
            }
        }

        None
    }
}

pub fn render_installer(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &InstallerState,
    pressed: Option<InstallerAction>,
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
    let is_back_pressed = pressed == Some(InstallerAction::Back);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    fonts.draw_text(
        &mut canvas.pixmap,
        "Install / Flash Package",
        back_cx + 64.0,
        back_cy - 4.0,
        38.0,
        theme.on_surface,
    );

    let path_str = state.current_dir.to_string_lossy();
    let display_path = if path_str.len() > 45 {
        format!("...{}", &path_str[path_str.len() - 42..])
    } else {
        path_str.to_string()
    };

    fonts.draw_text(
        &mut canvas.pixmap,
        &display_path,
        back_cx + 64.0,
        back_cy + 34.0,
        22.0,
        theme.primary,
    );

    // =========================================================================
    // 2. FILE LIST ROWS
    // =========================================================================
    let list_start_y = 210.0;
    let row_h = 120.0;
    let row_gap = 14.0;
    let max_visible = 15;

    for (i, entry) in state.entries.iter().take(max_visible).enumerate() {
        let ry = list_start_y + (i as f32) * (row_h + row_gap);
        let is_row_pressed = pressed == Some(InstallerAction::ItemClick(i));

        let bg = if is_row_pressed {
            theme.primary_container
        } else {
            theme.surface_container_high
        };

        canvas.draw_rounded_rect(
            margin,
            ry,
            content_w,
            row_h,
            24.0,
            Some(bg),
            Some((theme.outline_variant, 1.0)),
        );

        // Icon
        let icon_cx = margin + 60.0;
        let icon_cy = ry + (row_h / 2.0);
        let icon = if entry.is_dir {
            M3Icon::Folder
        } else if entry.name.ends_with(".zip") {
            M3Icon::Install
        } else {
            M3Icon::File
        };

        let icon_color = if entry.is_dir {
            theme.primary
        } else if entry.name.ends_with(".zip") {
            theme.secondary
        } else {
            theme.on_surface_variant
        };

        draw_icon(&mut canvas.pixmap, icon, icon_cx, icon_cy, 40.0, icon_color);

        // Title
        let text_x = icon_cx + 56.0;
        let title_y = ry + 54.0;
        let name_trimmed = if entry.name.len() > 40 {
            format!("{}...", &entry.name[..38])
        } else {
            entry.name.clone()
        };

        fonts.draw_text(
            &mut canvas.pixmap,
            &name_trimmed,
            text_x,
            title_y,
            28.0,
            theme.on_surface,
        );

        // Subtitle (size / folder)
        let sub_y = ry + 94.0;
        fonts.draw_text(
            &mut canvas.pixmap,
            &entry.size_str,
            text_x,
            sub_y,
            20.0,
            theme.on_surface_variant,
        );
    }

    // =========================================================================
    // 3. BOTTOM BAR
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

    // =========================================================================
    // 4. CONFIRMATION MODAL WITH INTERACTIVE SWIPESLIDER
    // =========================================================================
    if let Some(ref selected) = state.selected_file {
        let is_img = selected.name.ends_with(".img");

        // Scrim
        let scrim = Color::from_rgba8(0, 0, 0, 195);
        let mut sp = tiny_skia::Paint::default();
        sp.set_color(scrim);
        canvas.pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 1220.0, 2712.0).unwrap(),
            &sp,
            tiny_skia::Transform::identity(),
            None,
        );

        // Dialog Card
        let card_w = 1040.0;
        let card_x = (screen_w - card_w) / 2.0;
        let card_y = 650.0;
        let card_h = if is_img { 1180.0 } else { 1050.0 };

        canvas.draw_rounded_rect(
            card_x,
            card_y,
            card_w,
            card_h,
            48.0,
            Some(theme.surface_container_high),
            Some((theme.outline_variant, 2.0)),
        );

        // Header
        let icon_cx = card_x + 96.0;
        let icon_cy = card_y + 100.0;
        canvas.draw_circle(icon_cx, icon_cy, 48.0, theme.primary_container);
        draw_icon(&mut canvas.pixmap, M3Icon::Install, icon_cx, icon_cy, 48.0, theme.primary);

        let title = if is_img { "Confirm Image Flash" } else { "Confirm Flash" };
        let sub = if is_img { "Choose target partition and swipe to flash" } else { "Swipe slider to flash package to device" };
        fonts.draw_text(&mut canvas.pixmap, title, icon_cx + 72.0, icon_cy - 4.0, 38.0, theme.on_surface);
        fonts.draw_text(&mut canvas.pixmap, sub, icon_cx + 72.0, icon_cy + 34.0, 22.0, theme.on_surface_variant);

        // Package / Image Box Info
        let box_y = card_y + 190.0;
        let box_w = card_w - 96.0;
        let box_x = card_x + 48.0;
        let box_h = 210.0;
        canvas.draw_rounded_rect(box_x, box_y, box_w, box_h, 32.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.5)));

        let box_title = if is_img { "Selected Image File:" } else { "Selected Package:" };
        fonts.draw_text(&mut canvas.pixmap, box_title, box_x + 36.0, box_y + 50.0, 22.0, theme.on_surface_variant);
        fonts.draw_text(&mut canvas.pixmap, &selected.name, box_x + 36.0, box_y + 98.0, 30.0, theme.primary);
        fonts.draw_text(&mut canvas.pixmap, &format!("Size: {}", selected.size_str), box_x + 36.0, box_y + 144.0, 22.0, theme.on_surface);
        fonts.draw_text(&mut canvas.pixmap, &format!("Path: {}", selected.path.display()), box_x + 36.0, box_y + 184.0, 18.0, theme.on_surface_variant);

        // If flashing .img, render partition selector pills
        if is_img {
            let part_sel_y = box_y + box_h + 20.0;
            fonts.draw_text(&mut canvas.pixmap, "Target Partition to Flash:", box_x + 8.0, part_sel_y + 24.0, 22.0, theme.on_surface);

            let pill_y = part_sel_y + 42.0;
            let pill_h = 68.0;
            let pill_w = 225.0;

            let targets = [
                (PartitionTarget::InitBoot, box_x),
                (PartitionTarget::Boot, box_x + 238.0),
                (PartitionTarget::VendorBoot, box_x + 476.0),
                (PartitionTarget::Dtbo, box_x + 714.0),
            ];

            for (target, px) in targets {
                let is_sel = state.target_partition == target;
                let bg = if is_sel { theme.primary } else { theme.surface_container };
                let fg = if is_sel { theme.on_primary } else { theme.on_surface };
                let border = if is_sel { None } else { Some((theme.outline_variant, 1.5)) };

                canvas.draw_pill(px, pill_y, pill_w, pill_h, bg, border);
                let label = target.display_label();
                let lw = fonts.measure_text(label, 22.0);
                fonts.draw_text(&mut canvas.pixmap, label, px + (pill_w - lw) / 2.0, pill_y + 42.0, 22.0, fg);
            }
        }

        // Interactive SwipeSlider for Flash confirmation
        state.flash_slider.render(canvas, theme, fonts);

        // Cancel Button
        let cancel_y = state.flash_slider.y + state.flash_slider.height + 25.0;
        let cancel_h = 105.0;
        canvas.draw_pill(box_x, cancel_y, box_w, cancel_h, theme.surface_container, Some((theme.outline_variant, 1.5)));
        let cancel_lbl = "Cancel";
        let cw = fonts.measure_text(cancel_lbl, 28.0);
        fonts.draw_text(&mut canvas.pixmap, cancel_lbl, box_x + (box_w - cw) / 2.0, cancel_y + 64.0, 28.0, theme.on_surface);
    }
}
