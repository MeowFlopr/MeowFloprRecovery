use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::slider::SwipeSlider;
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeTab {
    AdvancedWipe,
    FactoryReset,
    FormatData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeAction {
    Back,
    SelectTab(WipeTab),
    ToggleDalvik,
    ToggleCache,
    ToggleSystem,
    ToggleVendor,
    ToggleData,
    ToggleInternalStorage,
    ToggleMetadata,
    TypeKey(char),
    BackspaceKey,
    ClearKey,
    Confirmed(WipeTab),
}

pub struct WipeState {
    pub current_tab: WipeTab,
    // Advanced Wipe checkboxes
    pub wipe_dalvik: bool,
    pub wipe_cache: bool,
    pub wipe_system: bool,
    pub wipe_vendor: bool,
    pub wipe_data: bool,
    pub wipe_internal: bool,
    pub wipe_metadata: bool,
    // Interactive SwipeSliders
    pub adv_slider: SwipeSlider,
    pub reset_slider: SwipeSlider,
    pub format_slider: SwipeSlider,
    // Format Data confirmation prompt
    pub format_input: String,
    pub status_message: String,
}

impl Default for WipeState {
    fn default() -> Self {
        let margin = 40.0;
        let content_w = 1220.0 - (margin * 2.0); // 1140.0
        let slider_x = margin + 40.0;
        let slider_w = content_w - 80.0; // 1060.0
        let slider_h = 130.0;

        let adv_slider = SwipeSlider::new(slider_x, 2280.0, slider_w, slider_h, "Swipe to Wipe Selected >>>")
            .with_accent(Color::from_rgba8(255, 110, 110, 255));
        let reset_slider = SwipeSlider::new(slider_x, 1080.0, slider_w, slider_h, "Swipe to Factory Reset >>>")
            .with_accent(Color::from_rgba8(255, 170, 80, 255));
        let format_slider = SwipeSlider::new(slider_x, 1540.0, slider_w, slider_h, "Swipe to FORMAT DATA >>>")
            .with_danger(true);

        Self {
            current_tab: WipeTab::AdvancedWipe,
            wipe_dalvik: true,
            wipe_cache: true,
            wipe_system: false,
            wipe_vendor: false,
            wipe_data: false,
            wipe_internal: false,
            wipe_metadata: false,
            adv_slider,
            reset_slider,
            format_slider,
            format_input: String::new(),
            status_message: "Select partitions to wipe or choose Factory Reset".to_string(),
        }
    }
}

impl WipeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick_spring(&mut self) -> bool {
        match self.current_tab {
            WipeTab::AdvancedWipe => self.adv_slider.tick_spring(),
            WipeTab::FactoryReset => self.reset_slider.tick_spring(),
            WipeTab::FormatData => self.format_slider.tick_spring(),
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<WipeAction> {
        let fx = tx as f32;
        let fy = ty as f32;
        let margin = 40.0;

        // 1. Back button (Top Bar: x: 40..160, y: 70..170, or Bottom Bar: y: 2460..2580)
        if state == TouchState::Released {
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                return Some(WipeAction::Back);
            }
            if fx >= 40.0 && fx <= 340.0 && fy >= 2460.0 && fy <= 2580.0 {
                return Some(WipeAction::Back);
            }
        }

        // 2. Tab Bar Selection (y: 200 .. 290)
        if state == TouchState::Released && fy >= 200.0 && fy <= 290.0 {
            let tab_w = (1140.0 - 24.0) / 3.0; // ~372.0
            if fx >= margin && fx <= margin + tab_w {
                self.current_tab = WipeTab::AdvancedWipe;
                return Some(WipeAction::SelectTab(WipeTab::AdvancedWipe));
            } else if fx >= margin + tab_w + 12.0 && fx <= margin + tab_w * 2.0 + 12.0 {
                self.current_tab = WipeTab::FactoryReset;
                return Some(WipeAction::SelectTab(WipeTab::FactoryReset));
            } else if fx >= margin + tab_w * 2.0 + 24.0 && fx <= margin + tab_w * 3.0 + 24.0 {
                self.current_tab = WipeTab::FormatData;
                return Some(WipeAction::SelectTab(WipeTab::FormatData));
            }
        }

        // 3. Checkboxes in Advanced Wipe (y: 330 .. 1560)
        if self.current_tab == WipeTab::AdvancedWipe && state == TouchState::Released {
            let row_h = 135.0;
            let start_y = 330.0;
            let gap = 16.0;

            for i in 0..7 {
                let cy = start_y + (i as f32) * (row_h + gap);
                if fx >= margin && fx <= 1220.0 - margin && fy >= cy && fy <= cy + row_h {
                    match i {
                        0 => {
                            self.wipe_dalvik = !self.wipe_dalvik;
                            return Some(WipeAction::ToggleDalvik);
                        }
                        1 => {
                            self.wipe_cache = !self.wipe_cache;
                            return Some(WipeAction::ToggleCache);
                        }
                        2 => {
                            self.wipe_data = !self.wipe_data;
                            return Some(WipeAction::ToggleData);
                        }
                        3 => {
                            self.wipe_internal = !self.wipe_internal;
                            return Some(WipeAction::ToggleInternalStorage);
                        }
                        4 => {
                            self.wipe_system = !self.wipe_system;
                            return Some(WipeAction::ToggleSystem);
                        }
                        5 => {
                            self.wipe_vendor = !self.wipe_vendor;
                            return Some(WipeAction::ToggleVendor);
                        }
                        6 => {
                            self.wipe_metadata = !self.wipe_metadata;
                            return Some(WipeAction::ToggleMetadata);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 4. Format Data On-Screen Confirmation Keypad
        if self.current_tab == WipeTab::FormatData && state == TouchState::Released {
            // Keypad area: y: 1150 .. 1450
            if fy >= 1150.0 && fy <= 1450.0 {
                let kw = 200.0;
                let kh = 90.0;
                let k_gap = 20.0;
                let row1_y = 1180.0;
                let row2_y = 1290.0;

                // Row 1: 'Y', 'E', 'S'
                let kx_y = margin + 120.0;
                let kx_e = kx_y + kw + k_gap;
                let kx_s = kx_e + kw + k_gap;

                if fy >= row1_y && fy <= row1_y + kh {
                    if fx >= kx_y && fx <= kx_y + kw {
                        if self.format_input.len() < 8 {
                            self.format_input.push('y');
                        }
                        return Some(WipeAction::TypeKey('y'));
                    }
                    if fx >= kx_e && fx <= kx_e + kw {
                        if self.format_input.len() < 8 {
                            self.format_input.push('e');
                        }
                        return Some(WipeAction::TypeKey('e'));
                    }
                    if fx >= kx_s && fx <= kx_s + kw {
                        if self.format_input.len() < 8 {
                            self.format_input.push('s');
                        }
                        return Some(WipeAction::TypeKey('s'));
                    }
                }

                // Row 2: 'CLEAR', 'BACKSPACE'
                let c_w = 340.0;
                let cx_clear = margin + 180.0;
                let cx_bksp = cx_clear + c_w + 30.0;

                if fy >= row2_y && fy <= row2_y + kh {
                    if fx >= cx_clear && fx <= cx_clear + c_w {
                        self.format_input.clear();
                        return Some(WipeAction::ClearKey);
                    }
                    if fx >= cx_bksp && fx <= cx_bksp + c_w {
                        self.format_input.pop();
                        return Some(WipeAction::BackspaceKey);
                    }
                }
            }
        }

        // 5. Interactive SwipeSliders
        let confirmed = match self.current_tab {
            WipeTab::AdvancedWipe => self.adv_slider.handle_touch(tx, ty, state),
            WipeTab::FactoryReset => self.reset_slider.handle_touch(tx, ty, state),
            WipeTab::FormatData => {
                // Slider is unlocked ONLY when word 'yes' is typed
                if self.format_input.trim().eq_ignore_ascii_case("yes") {
                    self.format_slider.handle_touch(tx, ty, state)
                } else {
                    false
                }
            }
        };

        if confirmed {
            return Some(WipeAction::Confirmed(self.current_tab));
        }

        None
    }
}

pub fn render_wipe(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &WipeState,
    pressed: Option<WipeAction>,
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
    let is_back_pressed = pressed == Some(WipeAction::Back);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    fonts.draw_text(&mut canvas.pixmap, "Wipe & Clean", back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, "Selective wipe, factory reset & format data", back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    // =========================================================================
    // 2. TAB SELECTOR BAR (3 Expressive Segmented Tabs: y: 200 .. 280)
    // =========================================================================
    let tab_y = 200.0;
    let tab_h = 76.0;
    let tab_gap = 12.0;
    let tab_w = (content_w - (tab_gap * 2.0)) / 3.0; // ~365.0

    let tabs = [
        ("Advanced Wipe", WipeTab::AdvancedWipe),
        ("Factory Reset", WipeTab::FactoryReset),
        ("Format Data", WipeTab::FormatData),
    ];

    for (i, (title, tab_enum)) in tabs.iter().enumerate() {
        let tx = margin + (i as f32) * (tab_w + tab_gap);
        let is_selected = state.current_tab == *tab_enum;

        let (bg, fg, border) = if is_selected {
            if *tab_enum == WipeTab::FormatData {
                (theme.error_container, theme.error, Some((theme.error, 2.0)))
            } else {
                (theme.primary_container, theme.primary, Some((theme.primary, 2.0)))
            }
        } else {
            (theme.surface_container, theme.on_surface_variant, Some((theme.outline_variant, 1.0)))
        };

        canvas.draw_pill(tx, tab_y, tab_w, tab_h, bg, border);
        let tw = fonts.measure_text(title, 24.0);
        fonts.draw_text(&mut canvas.pixmap, title, tx + (tab_w - tw) / 2.0, tab_y + 48.0, 24.0, fg);
    }

    // =========================================================================
    // 3. TAB CONTENT
    // =========================================================================
    match state.current_tab {
        WipeTab::AdvancedWipe => {
            // Checkbox items
            let items = [
                ("Dalvik / ART Cache", "Recompiles runtime bytecode; fixes crashes", state.wipe_dalvik, false),
                ("Cache Partition", "Temporary OTA and recovery logs", state.wipe_cache, false),
                ("Data (/data)", "Installed apps & user app databases", state.wipe_data, true),
                ("Internal Storage", "/sdcard photos, videos, music & downloads", state.wipe_internal, true),
                ("System (/system)", "Core Android OS binaries & framework", state.wipe_system, true),
                ("Vendor (/vendor)", "Hardware device drivers & HAL services", state.wipe_vendor, true),
                ("Metadata (/metadata)", "FBE v2 encryption key storage directory", state.wipe_metadata, true),
            ];

            let row_h = 135.0;
            let start_y = 300.0;
            let gap = 14.0;

            for (i, (name, desc, is_checked, is_danger)) in items.iter().enumerate() {
                let cy = start_y + (i as f32) * (row_h + gap);

                let card_bg = if *is_checked {
                    if *is_danger {
                        Color::from_rgba8(48, 18, 18, 255)
                    } else {
                        theme.surface_container_high
                    }
                } else {
                    theme.surface_container
                };

                let card_border = if *is_checked && *is_danger {
                    Some((theme.error, 2.0))
                } else {
                    Some((theme.outline_variant, 1.0))
                };

                canvas.draw_rounded_rect(margin, cy, content_w, row_h, 24.0, Some(card_bg), card_border);

                // Checkbox circle
                let chk_cx = margin + 50.0;
                let chk_cy = cy + row_h / 2.0;
                let chk_color = if *is_checked {
                    if *is_danger { theme.error } else { theme.primary }
                } else {
                    theme.surface_container_highest
                };
                canvas.draw_circle(chk_cx, chk_cy, 30.0, chk_color);

                if *is_checked {
                    draw_icon(&mut canvas.pixmap, M3Icon::Check, chk_cx, chk_cy, 30.0, theme.on_primary);
                }

                // Text labels
                let title_color = if *is_checked && *is_danger { theme.error } else { theme.on_surface };
                fonts.draw_text(&mut canvas.pixmap, name, chk_cx + 46.0, cy + 54.0, 27.0, title_color);
                fonts.draw_text(&mut canvas.pixmap, desc, chk_cx + 46.0, cy + 96.0, 19.0, theme.on_surface_variant);

                if *is_danger && *is_checked {
                    let badge_x = content_w - 90.0;
                    draw_icon(&mut canvas.pixmap, M3Icon::Warning, badge_x, cy + 50.0, 28.0, theme.error);
                }
            }

            // Warning pill if high-risk partitions selected
            if state.wipe_data || state.wipe_internal || state.wipe_metadata {
                let warn_y = 1380.0;
                canvas.draw_rounded_rect(
                    margin,
                    warn_y,
                    content_w,
                    110.0,
                    24.0,
                    Some(Color::from_rgba8(50, 15, 15, 255)),
                    Some((theme.error, 1.5)),
                );
                draw_icon(&mut canvas.pixmap, M3Icon::Warning, margin + 44.0, warn_y + 55.0, 36.0, theme.error);
                fonts.draw_text(&mut canvas.pixmap, "ВНИМАНИЕ: Выбраны критические разделы!", margin + 80.0, warn_y + 46.0, 22.0, theme.error);
                fonts.draw_text(&mut canvas.pixmap, "Удаление Data сотрёт все приложения. Metadata уничтожит ключи FBE v2.", margin + 80.0, warn_y + 80.0, 18.0, theme.on_surface);
            }

            // Swipe to Wipe Selected
            state.adv_slider.render(canvas, theme, fonts);
        }

        WipeTab::FactoryReset => {
            let card_y = 330.0;
            let card_h = 600.0;
            canvas.draw_rounded_rect(margin, card_y, content_w, card_h, 36.0, Some(theme.surface_container_high), Some((Color::from_rgba8(255, 170, 80, 255), 2.0)));

            let icon_cx = margin + 80.0;
            let icon_cy = card_y + 80.0;
            canvas.draw_circle(icon_cx, icon_cy, 48.0, Color::from_rgba8(70, 40, 10, 255));
            draw_icon(&mut canvas.pixmap, M3Icon::Wipe, icon_cx, icon_cy, 48.0, Color::from_rgba8(255, 180, 80, 255));

            fonts.draw_text(&mut canvas.pixmap, "Factory Reset (Сброс к заводу)", icon_cx + 70.0, icon_cy - 4.0, 36.0, theme.on_surface);
            fonts.draw_text(&mut canvas.pixmap, "Стандартная очистка без потери личных файлов", icon_cx + 70.0, icon_cy + 34.0, 22.0, theme.on_surface_variant);

            let text_y = card_y + 190.0;
            fonts.draw_text(&mut canvas.pixmap, "* Очищает базы данных приложений (/data/data, /data/app)", margin + 40.0, text_y, 25.0, theme.on_surface);
            fonts.draw_text(&mut canvas.pixmap, "* Очищает системные настройки (/data/system)", margin + 40.0, text_y + 55.0, 25.0, theme.on_surface);
            fonts.draw_text(&mut canvas.pixmap, "* Очищает кэш компилятора ART Dalvik и /cache", margin + 40.0, text_y + 110.0, 25.0, theme.on_surface);

            let safe_card_y = text_y + 180.0;
            canvas.draw_rounded_rect(
                margin + 20.0,
                safe_card_y,
                content_w - 40.0,
                140.0,
                24.0,
                Some(Color::from_rgba8(15, 45, 25, 255)),
                Some((Color::from_rgba8(120, 240, 160, 255), 1.5)),
            );
            draw_icon(&mut canvas.pixmap, M3Icon::Shield, margin + 65.0, safe_card_y + 70.0, 38.0, Color::from_rgba8(120, 240, 160, 255));
            fonts.draw_text(&mut canvas.pixmap, "ЛИЧНЫЕ ДАННЫЕ В БЕЗОПАСНОСТИ:", margin + 110.0, safe_card_y + 52.0, 24.0, Color::from_rgba8(120, 240, 160, 255));
            fonts.draw_text(&mut canvas.pixmap, "Внутренняя память (/data/media/0, Фото, Видео, Загрузки) НЕ затрагивается!", margin + 110.0, safe_card_y + 95.0, 20.0, theme.on_surface);

            // Swipe to Factory Reset Slider
            state.reset_slider.render(canvas, theme, fonts);
        }

        WipeTab::FormatData => {
            let card_y = 310.0;
            let card_h = 580.0;
            canvas.draw_rounded_rect(margin, card_y, content_w, card_h, 36.0, Some(theme.error_container), Some((theme.error, 3.0)));

            let icon_cx = margin + 80.0;
            let icon_cy = card_y + 80.0;
            canvas.draw_circle(icon_cx, icon_cy, 48.0, Color::from_rgba8(90, 20, 20, 255));
            draw_icon(&mut canvas.pixmap, M3Icon::Warning, icon_cx, icon_cy, 52.0, theme.error);

            fonts.draw_text(&mut canvas.pixmap, "DANGER: Format Data", icon_cx + 70.0, icon_cy - 4.0, 38.0, theme.error);
            fonts.draw_text(&mut canvas.pixmap, "Полное пересоздание файловой системы F2FS", icon_cx + 70.0, icon_cy + 36.0, 23.0, theme.on_surface);

            let text_y = card_y + 180.0;
            fonts.draw_text(&mut canvas.pixmap, "ВНИМАНИЕ: ФОРМАТИРОВАНИЕ НЕОБРАТИМО!", margin + 40.0, text_y, 26.0, theme.error);
            fonts.draw_text(&mut canvas.pixmap, "Форматирование userdata полностью уничтожает:", margin + 40.0, text_y + 45.0, 23.0, theme.on_surface);
            fonts.draw_text(&mut canvas.pixmap, "* ВСЕ личные фото, видео, музыку и документы (/sdcard)", margin + 60.0, text_y + 90.0, 23.0, theme.error);
            fonts.draw_text(&mut canvas.pixmap, "* Ключи шифрования FBE v2 и установленные приложения", margin + 60.0, text_y + 135.0, 23.0, theme.error);
            fonts.draw_text(&mut canvas.pixmap, "Применяется при смене HyperOS на кастомную AOSP прошивку", margin + 40.0, text_y + 195.0, 22.0, theme.on_surface_variant);
            fonts.draw_text(&mut canvas.pixmap, "или при полном повреждении структур шифрования.", margin + 40.0, text_y + 235.0, 22.0, theme.on_surface_variant);

            // =================================================================
            // On-Screen "Type yes" Protection Box (y: 930 .. 1480)
            // =================================================================
            let prompt_y = 930.0;
            canvas.draw_rounded_rect(
                margin,
                prompt_y,
                content_w,
                550.0,
                32.0,
                Some(theme.surface_container_high),
                Some((theme.outline_variant, 1.5)),
            );

            fonts.draw_text(&mut canvas.pixmap, "Защита от случайного нажатия:", margin + 40.0, prompt_y + 55.0, 26.0, theme.on_surface);
            fonts.draw_text(&mut canvas.pixmap, "Для разблокировки слайдера введите слово \"yes\":", margin + 40.0, prompt_y + 95.0, 22.0, theme.on_surface_variant);

            // Input display box
            let input_box_y = prompt_y + 130.0;
            let input_box_h = 75.0;
            let is_unlocked = state.format_input.trim().eq_ignore_ascii_case("yes");

            let (in_bg, in_border, in_fg) = if is_unlocked {
                (Color::from_rgba8(20, 60, 30, 255), theme.primary, Color::from_rgba8(120, 240, 160, 255))
            } else {
                (theme.surface_container_highest, theme.outline_variant, theme.on_surface)
            };

            canvas.draw_rounded_rect(margin + 40.0, input_box_y, content_w - 80.0, input_box_h, 20.0, Some(in_bg), Some((in_border, 2.0)));
            let display_str = if state.format_input.is_empty() {
                "нажмите буквы Y - E - S ниже...".to_string()
            } else {
                state.format_input.clone()
            };
            fonts.draw_text(&mut canvas.pixmap, &display_str, margin + 70.0, input_box_y + 50.0, 28.0, in_fg);

            // Virtual Keyboard Keys (Row 1: Y, E, S)
            let kw = 200.0;
            let kh = 90.0;
            let k_gap = 20.0;
            let row1_y = input_box_y + 105.0;

            let kx_y = margin + 120.0;
            let kx_e = kx_y + kw + k_gap;
            let kx_s = kx_e + kw + k_gap;

            let keys = [("Y", kx_y), ("E", kx_e), ("S", kx_s)];
            for (ch, kx) in keys {
                canvas.draw_pill(kx, row1_y, kw, kh, theme.surface_container_highest, Some((theme.outline_variant, 1.5)));
                fonts.draw_text(&mut canvas.pixmap, ch, kx + kw / 2.0 - 12.0, row1_y + 58.0, 36.0, theme.on_surface);
            }

            // Row 2: Clear, Backspace
            let row2_y = row1_y + kh + 20.0;
            let c_w = 340.0;
            let cx_clear = margin + 180.0;
            let cx_bksp = cx_clear + c_w + 30.0;

            canvas.draw_pill(cx_clear, row2_y, c_w, kh, theme.surface_container_highest, Some((theme.outline_variant, 1.5)));
            let clr_lbl = "Очистить";
            let clrw = fonts.measure_text(clr_lbl, 24.0);
            fonts.draw_text(&mut canvas.pixmap, clr_lbl, cx_clear + (c_w - clrw) / 2.0, row2_y + 56.0, 24.0, theme.on_surface);

            canvas.draw_pill(cx_bksp, row2_y, c_w, kh, theme.surface_container_highest, Some((theme.outline_variant, 1.5)));
            let del_lbl = "<-- Стереть";
            let delw = fonts.measure_text(del_lbl, 24.0);
            fonts.draw_text(&mut canvas.pixmap, del_lbl, cx_bksp + (c_w - delw) / 2.0, row2_y + 56.0, 24.0, theme.on_surface);

            // Red Danger Swipe Slider (Unlocked when "yes" is entered)
            if is_unlocked {
                state.format_slider.render(canvas, theme, fonts);
            } else {
                // Render locked dummy track
                let locked_y = 1540.0;
                let locked_w = content_w - 80.0;
                canvas.draw_pill(margin + 40.0, locked_y, locked_w, 130.0, theme.surface_container, Some((theme.outline_variant, 1.0)));
                let lock_hint = "[ Заблокировано: введите \"yes\" выше для форматирования ]";
                let lhw = fonts.measure_text(lock_hint, 24.0);
                fonts.draw_text(&mut canvas.pixmap, lock_hint, margin + 40.0 + (locked_w - lhw) / 2.0, locked_y + 74.0, 24.0, theme.on_surface_variant);
            }
        }
    }

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
