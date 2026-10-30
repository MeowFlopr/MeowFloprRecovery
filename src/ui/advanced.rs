use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::i18n::Language;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::hardware;
use crate::platform::TouchState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedSubScreen {
    Menu,
    RootManager,
    FileManager,
    DmesgViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootActionType {
    BackupStockBoot,
    InstallSukiSU,
    InstallMagisk,
    RestoreStockBoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmEntry {
    pub name: String,
    pub is_dir: bool,
    pub size_str: String,
    pub mode: u32,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvancedAction {
    Back,
    OpenRootManager,
    OpenFileManager,
    FixSELinux,
    ExportLogs,
    OpenDmesg,
    RootAction(RootActionType),
    FmItemClick(usize),
    ChmodSelected(u32),
    DeleteSelected,
    DmesgRefresh,
    DmesgScrollUp,
    DmesgScrollDown,
}

pub struct AdvancedState {
    pub subscreen: AdvancedSubScreen,
    pub detected_root: String,
    pub status_message: String,
    pub active_slot: String,
    // File Manager
    pub fm_path: PathBuf,
    pub fm_entries: Vec<FmEntry>,
    pub selected_file: Option<FmEntry>,
    // Dmesg Viewer
    pub dmesg_lines: Vec<String>,
    pub dmesg_scroll_offset: usize,
    pub is_dragging_dmesg: bool,
    pub last_touch_y: Option<f32>,
}

impl Default for AdvancedState {
    fn default() -> Self {
        let active_slot = hardware::slots::get_current_slot();
        let mut s = Self {
            subscreen: AdvancedSubScreen::Menu,
            detected_root: "Stock (Official HyperOS Kernel)".to_string(),
            status_message: "Ready".to_string(),
            active_slot,
            fm_path: PathBuf::from("/sdcard"),
            fm_entries: Vec::new(),
            selected_file: None,
            dmesg_lines: Vec::new(),
            dmesg_scroll_offset: 0,
            is_dragging_dmesg: false,
            last_touch_y: None,
        };
        s.detect_current_root();
        s.load_fm_dir();
        s
    }
}

impl AdvancedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh_dmesg(&mut self) {
        let output = Command::new("dmesg").output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            self.dmesg_lines = text.lines().map(|l| l.to_string()).collect();
        } else {
            self.dmesg_lines = vec![
                "[DMESG] Unable to read kernel ring buffer".to_string(),
                "[DMESG] /system/bin/dmesg failed or requires root".to_string(),
            ];
        }
        self.dmesg_scroll_offset = 0;
    }

    pub fn detect_current_root(&mut self) {
        if Path::new("/system/bin/magisk").exists() || Path::new("/data/adb/magisk").exists() {
            self.detected_root = "Magisk v28.x Active".to_string();
        } else if Path::new("/data/adb/ksu").exists() {
            self.detected_root = "KernelSU Next Active".to_string();
        } else if Path::new("/data/adb/ap").exists() {
            self.detected_root = "APatch Active".to_string();
        } else {
            self.detected_root = "Stock Kernel (Official AVB 2.0)".to_string();
        }
    }

    pub fn load_fm_dir(&mut self) {
        self.fm_entries.clear();

        if self.fm_path.parent().is_some() && self.fm_path != Path::new("/") {
            self.fm_entries.push(FmEntry {
                name: ".. (Parent Directory)".to_string(),
                is_dir: true,
                size_str: "DIR".to_string(),
                mode: 0o755,
                path: self.fm_path.parent().unwrap().to_path_buf(),
            });
        }

        if let Ok(read_dir) = fs::read_dir(&self.fm_path) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') && name != ".transforms" {
                    continue;
                }

                if let Ok(meta) = entry.metadata() {
                    let mode = meta.permissions().mode() & 0o777;
                    if meta.is_dir() {
                        dirs.push(FmEntry {
                            name,
                            is_dir: true,
                            size_str: "DIR".to_string(),
                            mode,
                            path,
                        });
                    } else {
                        let size = meta.len();
                        let size_str = if size > 1024 * 1024 {
                            format!("{:.1} MB", size as f32 / (1024.0 * 1024.0))
                        } else if size > 1024 {
                            format!("{:.1} KB", size as f32 / 1024.0)
                        } else {
                            format!("{} B", size)
                        };

                        files.push(FmEntry {
                            name,
                            is_dir: false,
                            size_str,
                            mode,
                            path,
                        });
                    }
                }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            self.fm_entries.extend(dirs);
            self.fm_entries.extend(files);
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<AdvancedAction> {
        let fx = tx as f32;
        let fy = ty as f32;
        let margin = 40.0;

        // Dmesg drag scrolling
        if self.subscreen == AdvancedSubScreen::DmesgViewer {
            if fy >= 200.0 && fy <= 2400.0 && fx >= margin && fx <= 1220.0 - margin {
                match state {
                    TouchState::Pressed => {
                        if let Some(last_y) = self.last_touch_y {
                            let delta = fy - last_y;
                            if delta.abs() > 15.0 {
                                let lines_delta = (delta.abs() / 24.0).round() as usize;
                                if delta > 0.0 {
                                    let total = self.dmesg_lines.len();
                                    if self.dmesg_scroll_offset + lines_delta < total {
                                        self.dmesg_scroll_offset += lines_delta;
                                    }
                                } else {
                                    self.dmesg_scroll_offset = self.dmesg_scroll_offset.saturating_sub(lines_delta);
                                }
                                self.last_touch_y = Some(fy);
                                return None;
                            }
                        } else {
                            self.last_touch_y = Some(fy);
                            self.is_dragging_dmesg = true;
                            return None;
                        }
                    }
                    TouchState::Released => {
                        self.is_dragging_dmesg = false;
                        self.last_touch_y = None;
                    }
                }
            }
        }

        if state == TouchState::Released {
            // 1. Back button
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                if self.subscreen != AdvancedSubScreen::Menu {
                    self.subscreen = AdvancedSubScreen::Menu;
                    self.selected_file = None;
                    return None;
                }
                return Some(AdvancedAction::Back);
            }
            if fx >= 40.0 && fx <= 340.0 && fy >= 2460.0 && fy <= 2580.0 {
                if self.subscreen != AdvancedSubScreen::Menu {
                    self.subscreen = AdvancedSubScreen::Menu;
                    self.selected_file = None;
                    return None;
                }
                return Some(AdvancedAction::Back);
            }

            match self.subscreen {
                AdvancedSubScreen::Menu => {
                    let card_h = 160.0;
                    let start_y = 220.0;
                    let gap = 24.0;

                    let actions = [
                        AdvancedAction::OpenRootManager,
                        AdvancedAction::OpenFileManager,
                        AdvancedAction::OpenDmesg,
                        AdvancedAction::FixSELinux,
                        AdvancedAction::ExportLogs,
                    ];

                    for (i, action) in actions.iter().enumerate() {
                        let cy = start_y + (i as f32) * (card_h + gap);
                        if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + card_h {
                            match action {
                                AdvancedAction::OpenRootManager => {
                                    self.subscreen = AdvancedSubScreen::RootManager;
                                    self.detect_current_root();
                                    return None;
                                }
                                AdvancedAction::OpenFileManager => {
                                    self.subscreen = AdvancedSubScreen::FileManager;
                                    self.load_fm_dir();
                                    return None;
                                }
                                AdvancedAction::OpenDmesg => {
                                    self.subscreen = AdvancedSubScreen::DmesgViewer;
                                    self.refresh_dmesg();
                                    return None;
                                }
                                _ => return Some(action.clone()),
                            }
                        }
                    }
                }

                AdvancedSubScreen::RootManager => {
                    let card_h = 150.0;
                    let start_y = 380.0;
                    let gap = 20.0;

                    let root_actions = [
                        RootActionType::BackupStockBoot,
                        RootActionType::InstallSukiSU,
                        RootActionType::InstallMagisk,
                        RootActionType::RestoreStockBoot,
                    ];

                    for (i, act) in root_actions.iter().enumerate() {
                        let cy = start_y + (i as f32) * (card_h + gap);
                        if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + card_h {
                            return Some(AdvancedAction::RootAction(*act));
                        }
                    }
                }

                AdvancedSubScreen::FileManager => {
                    if let Some(ref sel) = self.selected_file.clone() {
                        let btn_y = 1250.0;
                        let btn_h = 90.0;
                        let bw = 240.0;

                        // chmod 0755
                        if fy >= btn_y && fy <= btn_y + btn_h && fx >= margin + 40.0 && fx <= margin + 40.0 + bw {
                            let _ = fs::set_permissions(&sel.path, fs::Permissions::from_mode(0o755));
                            self.status_message = "chmod 0755 applied".to_string();
                            self.selected_file = None;
                            self.load_fm_dir();
                            return Some(AdvancedAction::ChmodSelected(0o755));
                        }

                        // chmod 0644
                        let c2_x = margin + 40.0 + bw + 20.0;
                        if fy >= btn_y && fy <= btn_y + btn_h && fx >= c2_x && fx <= c2_x + bw {
                            let _ = fs::set_permissions(&sel.path, fs::Permissions::from_mode(0o644));
                            self.status_message = "chmod 0644 applied".to_string();
                            self.selected_file = None;
                            self.load_fm_dir();
                            return Some(AdvancedAction::ChmodSelected(0o644));
                        }

                        // Delete
                        let del_x = c2_x + bw + 20.0;
                        if fy >= btn_y && fy <= btn_y + btn_h && fx >= del_x && fx <= del_x + bw {
                            if sel.is_dir {
                                let _ = fs::remove_dir_all(&sel.path);
                            } else {
                                let _ = fs::remove_file(&sel.path);
                            }
                            self.status_message = format!("Deleted: {}", sel.name);
                            self.selected_file = None;
                            self.load_fm_dir();
                            return Some(AdvancedAction::DeleteSelected);
                        }

                        // Cancel / Close modal
                        let cl_y = btn_y + 110.0;
                        if fy >= cl_y && fy <= cl_y + 80.0 {
                            self.selected_file = None;
                        }
                    } else {
                        let item_h = 110.0;
                        let start_y = 280.0;
                        let gap = 12.0;

                        for (i, entry) in self.fm_entries.iter().enumerate() {
                            let cy = start_y + (i as f32) * (item_h + gap);
                            if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + item_h {
                                if entry.is_dir {
                                    self.fm_path = entry.path.clone();
                                    self.load_fm_dir();
                                    return Some(AdvancedAction::FmItemClick(i));
                                } else {
                                    self.selected_file = Some(entry.clone());
                                    return Some(AdvancedAction::FmItemClick(i));
                                }
                            }
                        }
                    }
                }

                AdvancedSubScreen::DmesgViewer => {
                    // Top Refresh button: x: 1040..1180, y: 70..170
                    if fx >= 1040.0 && fx <= 1180.0 && fy >= 70.0 && fy <= 170.0 {
                        self.refresh_dmesg();
                        return Some(AdvancedAction::DmesgRefresh);
                    }
                    // Bottom dock scroll controls: y: 2460..2580
                    if fy >= 2460.0 && fy <= 2580.0 {
                        // Scroll Up: x: 380..740
                        if fx >= 380.0 && fx <= 740.0 {
                            let total = self.dmesg_lines.len();
                            if self.dmesg_scroll_offset + 30 < total {
                                self.dmesg_scroll_offset += 30;
                            }
                            return Some(AdvancedAction::DmesgScrollUp);
                        }
                        // Scroll Down: x: 760..1180
                        if fx >= 760.0 && fx <= 1180.0 {
                            self.dmesg_scroll_offset = self.dmesg_scroll_offset.saturating_sub(30);
                            return Some(AdvancedAction::DmesgScrollDown);
                        }
                    }
                }
            }
        }

        None
    }
}

pub fn render_advanced(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &AdvancedState,
    lang: Language,
) {
    canvas.clear(theme.background);

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

    // =========================================================================
    // 1. TOP APP BAR
    // =========================================================================
    let back_cx = margin + 44.0;
    let back_cy = 115.0;
    canvas.draw_circle(back_cx, back_cy, 44.0, theme.surface_container_high);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    let (title, sub) = match state.subscreen {
        AdvancedSubScreen::Menu => match lang {
            Language::English => ("Advanced Tools", "Root management, file manager & logs"),
            Language::Russian => ("Дополнительно", "Менеджер root, файлы и логи ядра"),
        },
        AdvancedSubScreen::RootManager => match lang {
            Language::English => ("Root Manager", "Patch kernel boot / init_boot directly"),
            Language::Russian => ("Менеджер Root", "Патчинг boot / init_boot через magiskboot"),
        },
        AdvancedSubScreen::FileManager => ("File Manager", state.fm_path.to_str().unwrap_or("/")),
        AdvancedSubScreen::DmesgViewer => match lang {
            Language::English => ("Kernel Dmesg Log", "Raw Linux kernel ring buffer stream"),
            Language::Russian => ("Kernel Dmesg Log", "Системный лог ядра Linux"),
        },
    };

    fonts.draw_text(&mut canvas.pixmap, title, back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, sub, back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    // Refresh button if in Dmesg Viewer
    if state.subscreen == AdvancedSubScreen::DmesgViewer {
        let clr_w = 150.0;
        let clr_x = screen_w - margin - clr_w;
        canvas.draw_pill(clr_x, 75.0, clr_w, 75.0, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
        let ref_lbl = match lang {
            Language::English => "Refresh",
            Language::Russian => "Обновить",
        };
        let rw = fonts.measure_text(ref_lbl, 22.0);
        fonts.draw_text(&mut canvas.pixmap, ref_lbl, clr_x + (clr_w - rw) / 2.0, 120.0, 22.0, theme.primary);
    }

    // =========================================================================
    // 2. SUBSCREEN CONTENT
    // =========================================================================
    match state.subscreen {
        AdvancedSubScreen::Menu => {
            let cards = match lang {
                Language::English => [
                    ("Root Manager", "Magisk, KernelSU / SukiSU boot patcher", M3Icon::Root, theme.primary_container, theme.primary),
                    ("File Manager", "Explore /sdcard, permissions (chmod), delete", M3Icon::Folder, theme.secondary_container, theme.secondary),
                    ("Kernel Dmesg Viewer", "View live Linux kernel console messages", M3Icon::Terminal, theme.tertiary_container, theme.tertiary),
                    ("Fix SELinux Contexts", "Restore default security contexts on /data", M3Icon::Shield, theme.surface_container_highest, theme.on_surface),
                    ("Export System Logs", "Dump dmesg & recovery.log to /sdcard/MRR/logs/", M3Icon::Copy, theme.surface_container_highest, theme.on_surface),
                ],
                Language::Russian => [
                    ("Root Manager", "Патчер ядра: Magisk, KernelSU / SukiSU", M3Icon::Root, theme.primary_container, theme.primary),
                    ("Файловый менеджер", "Просмотр файлов, chmod 0755/0644, удаление", M3Icon::Folder, theme.secondary_container, theme.secondary),
                    ("Kernel Dmesg Viewer", "Просмотр системного лога ядра Linux", M3Icon::Terminal, theme.tertiary_container, theme.tertiary),
                    ("Fix SELinux Contexts", "Восстановление меток безопасности SELinux", M3Icon::Shield, theme.surface_container_highest, theme.on_surface),
                    ("Экспорт логов", "Сохранить dmesg и логи в /sdcard/MRR/logs/", M3Icon::Copy, theme.surface_container_highest, theme.on_surface),
                ],
            };

            let card_h = 160.0;
            let start_y = 220.0;
            let gap = 24.0;

            for (i, (lbl, desc, icon, c_bg, acc)) in cards.iter().enumerate() {
                let cy = start_y + (i as f32) * (card_h + gap);
                canvas.draw_rounded_rect(margin, cy, content_w, card_h, 32.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.5)));

                let icx = margin + 70.0;
                let icy = cy + card_h / 2.0;
                canvas.draw_circle(icx, icy, 44.0, *c_bg);
                draw_icon(&mut canvas.pixmap, *icon, icx, icy, 40.0, *acc);

                fonts.draw_text(&mut canvas.pixmap, lbl, icx + 65.0, cy + 62.0, 32.0, theme.on_surface);
                fonts.draw_text(&mut canvas.pixmap, desc, icx + 65.0, cy + 108.0, 22.0, theme.on_surface_variant);

                draw_icon(&mut canvas.pixmap, M3Icon::ChevronRight, screen_w - margin - 50.0, icy, 32.0, theme.on_surface_variant);
            }
        }

        AdvancedSubScreen::RootManager => {
            // Status hero card
            let hero_y = 200.0;
            let hero_h = 140.0;
            canvas.draw_rounded_rect(margin, hero_y, content_w, hero_h, 28.0, Some(theme.surface_container_high), Some((theme.primary, 1.5)));

            let icx = margin + 60.0;
            let icy = hero_y + hero_h / 2.0;
            canvas.draw_circle(icx, icy, 40.0, theme.primary_container);
            draw_icon(&mut canvas.pixmap, M3Icon::Root, icx, icy, 36.0, theme.primary);

            let status_lbl = match lang {
                Language::English => "Current Kernel Root Status:",
                Language::Russian => "Текущий статус Root ядра:",
            };
            fonts.draw_text(&mut canvas.pixmap, status_lbl, icx + 60.0, hero_y + 54.0, 24.0, theme.on_surface_variant);
            fonts.draw_text(&mut canvas.pixmap, &state.detected_root, icx + 60.0, hero_y + 98.0, 30.0, theme.primary);

            // Action Cards
            let r_cards = match lang {
                Language::English => [
                    ("Backup Stock Boot Partition", "Dump active boot/init_boot to /sdcard/MRR/stock_boot.img", theme.secondary),
                    ("Install SukiSU / KernelSU", "Inject ksu into active boot via magiskboot tool", theme.primary),
                    ("Install Magisk Root", "Patch active boot ramdisk with Magisk daemon", theme.tertiary),
                    ("Restore Stock Boot Partition", "Flash original unpatched boot to uninstall root", theme.error),
                ],
                Language::Russian => [
                    ("Бэкап стокового Boot раздела", "Дамп активного boot/init_boot в /sdcard/MRR/stock_boot.img", theme.secondary),
                    ("Установить SukiSU / KernelSU", "Внедрить модуль ksu в boot через magiskboot", theme.primary),
                    ("Установить Magisk Root", "Пропатчить ramdisk загрузочного образа Magisk'ом", theme.tertiary),
                    ("Восстановить стоковый Boot", "Прошить чистый образ ядра для удаления root", theme.error),
                ],
            };

            let card_h = 150.0;
            let start_y = 380.0;
            let gap = 20.0;

            for (i, (lbl, desc, acc)) in r_cards.iter().enumerate() {
                let cy = start_y + (i as f32) * (card_h + gap);
                canvas.draw_rounded_rect(margin, cy, content_w, card_h, 28.0, Some(theme.surface_container_high), Some((*acc, 1.5)));

                draw_icon(&mut canvas.pixmap, M3Icon::Root, margin + 60.0, cy + card_h / 2.0, 38.0, *acc);
                fonts.draw_text(&mut canvas.pixmap, lbl, margin + 115.0, cy + 60.0, 29.0, theme.on_surface);
                fonts.draw_text(&mut canvas.pixmap, desc, margin + 115.0, cy + 104.0, 20.0, theme.on_surface_variant);
            }
        }

        AdvancedSubScreen::FileManager => {
            // Path breadcrumb bar
            let p_y = 195.0;
            canvas.draw_pill(margin, p_y, content_w, 64.0, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
            let path_str = state.fm_path.to_string_lossy();
            fonts.draw_text(&mut canvas.pixmap, &path_str, margin + 30.0, p_y + 42.0, 22.0, theme.primary);

            // Items list: y: 280 .. 2350
            let item_h = 110.0;
            let start_y = 280.0;
            let gap = 12.0;

            for (i, entry) in state.fm_entries.iter().take(18).enumerate() {
                let cy = start_y + (i as f32) * (item_h + gap);
                canvas.draw_rounded_rect(margin, cy, content_w, item_h, 22.0, Some(theme.surface_container), Some((theme.outline_variant, 1.0)));

                let icon = if entry.is_dir { M3Icon::Folder } else { M3Icon::File };
                let ic_color = if entry.is_dir { theme.tertiary } else { theme.on_surface_variant };
                draw_icon(&mut canvas.pixmap, icon, margin + 45.0, cy + item_h / 2.0, 32.0, ic_color);

                fonts.draw_text(&mut canvas.pixmap, &entry.name, margin + 85.0, cy + 48.0, 25.0, theme.on_surface);
                let mode_str = format!("0{:o} • {}", entry.mode, entry.size_str);
                fonts.draw_text(&mut canvas.pixmap, &mode_str, margin + 85.0, cy + 86.0, 19.0, theme.on_surface_variant);
            }

            // File Details & Chmod Modal if a file is tapped
            if let Some(ref sel) = state.selected_file {
                let m_y = 800.0;
                let m_h = 600.0;
                canvas.draw_rounded_rect(margin + 20.0, m_y, content_w - 40.0, m_h, 36.0, Some(theme.surface_container_highest), Some((theme.primary, 2.5)));

                draw_icon(&mut canvas.pixmap, M3Icon::File, margin + 70.0, m_y + 70.0, 48.0, theme.primary);
                fonts.draw_text(&mut canvas.pixmap, &sel.name, margin + 120.0, m_y + 65.0, 32.0, theme.on_surface);
                let full_p = sel.path.to_string_lossy();
                fonts.draw_text(&mut canvas.pixmap, &full_p, margin + 120.0, m_y + 105.0, 20.0, theme.on_surface_variant);

                let perm_str = format!("Mode: 0{:o} • Size: {}", sel.mode, sel.size_str);
                fonts.draw_text(&mut canvas.pixmap, &perm_str, margin + 50.0, m_y + 190.0, 25.0, theme.on_surface);

                // Quick buttons: chmod 0755, chmod 0644, Delete
                let btn_y = m_y + 260.0;
                let btn_h = 80.0;
                let bw = 240.0;

                // chmod 0755
                canvas.draw_pill(margin + 50.0, btn_y, bw, btn_h, theme.primary, None);
                fonts.draw_text(&mut canvas.pixmap, "chmod 0755", margin + 85.0, btn_y + 50.0, 22.0, theme.on_primary);

                // chmod 0644
                let c2_x = margin + 50.0 + bw + 20.0;
                canvas.draw_pill(c2_x, btn_y, bw, btn_h, theme.surface_container_high, Some((theme.outline_variant, 1.5)));
                fonts.draw_text(&mut canvas.pixmap, "chmod 0644", c2_x + 35.0, btn_y + 50.0, 22.0, theme.on_surface);

                // Delete
                let del_x = c2_x + bw + 20.0;
                canvas.draw_pill(del_x, btn_y, bw, btn_h, theme.error_container, Some((theme.error, 1.5)));
                let del_lbl = match lang {
                    Language::English => "Delete",
                    Language::Russian => "Удалить",
                };
                fonts.draw_text(&mut canvas.pixmap, del_lbl, del_x + 55.0, btn_y + 50.0, 22.0, theme.error);

                // Close
                let cl_y = btn_y + 110.0;
                canvas.draw_pill(margin + 50.0, cl_y, content_w - 100.0, 70.0, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
                let cl_lbl = match lang {
                    Language::English => "Close",
                    Language::Russian => "Закрыть",
                };
                fonts.draw_text(&mut canvas.pixmap, cl_lbl, screen_w / 2.0 - 45.0, cl_y + 45.0, 24.0, theme.on_surface_variant);
            }
        }

        AdvancedSubScreen::DmesgViewer => {
            // Dmesg terminal box: y: 200 .. 2420
            let d_y = 200.0;
            let d_h = 2220.0;
            canvas.draw_rounded_rect(
                margin,
                d_y,
                content_w,
                d_h,
                20.0,
                Some(Color::from_rgba8(14, 16, 22, 255)),
                Some((theme.outline_variant, 1.5)),
            );

            let line_h = 32.0;
            let visible_count = ((d_h - 40.0) / line_h) as usize;
            let total = state.dmesg_lines.len();
            let end_idx = total.saturating_sub(state.dmesg_scroll_offset);
            let start_idx = end_idx.saturating_sub(visible_count);

            let mut cur_y = d_y + 36.0;
            for i in start_idx..end_idx {
                if let Some(line) = state.dmesg_lines.get(i) {
                    let col = if line.contains("error") || line.contains("fail") {
                        theme.error
                    } else if line.contains("warn") {
                        theme.secondary
                    } else {
                        Color::from_rgba8(220, 224, 230, 255)
                    };

                    let disp = if line.len() > 72 { format!("{}...", &line[..69]) } else { line.clone() };
                    fonts.draw_text(&mut canvas.pixmap, &disp, margin + 20.0, cur_y, 20.0, col);
                    cur_y += line_h;
                }
            }
        }
    }

    // =========================================================================
    // 3. BOTTOM DOCK
    // =========================================================================
    let dock_y = 2460.0;
    let dock_h = 110.0;
    let btn_w = 300.0;

    canvas.draw_pill(margin, dock_y, btn_w, dock_h, theme.surface_container_high, Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, margin + 48.0, dock_y + dock_h / 2.0, 36.0, theme.on_surface);
    let lbl = match lang {
        Language::English => "BACK",
        Language::Russian => "НАЗАД",
    };
    fonts.draw_text(&mut canvas.pixmap, lbl, margin + 96.0, dock_y + 68.0, 24.0, theme.on_surface);

    if state.subscreen == AdvancedSubScreen::DmesgViewer {
        let sc_w = 340.0;
        let sc_x1 = margin + btn_w + 30.0;
        let sc_x2 = sc_x1 + sc_w + 20.0;

        canvas.draw_pill(sc_x1, dock_y, sc_w, dock_h, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
        fonts.draw_text(&mut canvas.pixmap, "PAGE UP", sc_x1 + 100.0, dock_y + 68.0, 22.0, theme.primary);

        canvas.draw_pill(sc_x2, dock_y, sc_w, dock_h, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
        fonts.draw_text(&mut canvas.pixmap, "PAGE DOWN", sc_x2 + 85.0, dock_y + 68.0, 22.0, theme.primary);
    }
}
