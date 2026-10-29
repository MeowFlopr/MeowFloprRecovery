use std::fs;
use std::path::Path;

use super::canvas::Canvas;
use super::i18n::Language;
use super::icons::{draw_icon, M3Icon};
use super::shapes::M3ShapeType;
use super::text::FontRenderer;
use super::theme::{M3Theme, ThemeId};
use crate::hardware;
use crate::platform::TouchState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenTimeout {
    Sec15,
    Sec30,
    Sec60,
    Min2,
    Never,
}

impl ScreenTimeout {
    pub const ALL: [ScreenTimeout; 5] = [
        ScreenTimeout::Sec15,
        ScreenTimeout::Sec30,
        ScreenTimeout::Sec60,
        ScreenTimeout::Min2,
        ScreenTimeout::Never,
    ];

    pub fn to_seconds(&self) -> u64 {
        match self {
            ScreenTimeout::Sec15 => 15,
            ScreenTimeout::Sec30 => 30,
            ScreenTimeout::Sec60 => 60,
            ScreenTimeout::Min2 => 120,
            ScreenTimeout::Never => u64::MAX,
        }
    }

    pub fn label(&self, lang: Language) -> &'static str {
        match lang {
            Language::English => match self {
                ScreenTimeout::Sec15 => "15s",
                ScreenTimeout::Sec30 => "30s",
                ScreenTimeout::Sec60 => "60s",
                ScreenTimeout::Min2 => "2m",
                ScreenTimeout::Never => "Never",
            },
            Language::Russian => match self {
                ScreenTimeout::Sec15 => "15 с",
                ScreenTimeout::Sec30 => "30 с",
                ScreenTimeout::Sec60 => "60 с",
                ScreenTimeout::Min2 => "2 мин",
                ScreenTimeout::Never => "Никогда",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    Back,
    SelectLanguage(Language),
    SelectTheme(ThemeId),
    SelectShape(Option<M3ShapeType>),
    SelectTimeout(ScreenTimeout),
    SwitchSlot,
    ToggleHaptic,
    SaveSettings,
}

pub struct SettingsState {
    pub language: Language,
    pub current_theme: ThemeId,
    pub current_shape: Option<M3ShapeType>,
    pub brightness_val: i32,
    pub brightness_progress: f32,
    pub is_dragging_brightness: bool,
    pub screen_timeout: ScreenTimeout,
    pub active_slot: String,
    pub haptic_feedback: bool,
    pub status_message: String,
}

const SETTINGS_FILE: &str = "/sdcard/MRR/settings.conf";

pub const MARGIN: f32 = 40.0;
pub const SCREEN_W: f32 = 1220.0;
pub const CONTENT_W: f32 = SCREEN_W - (MARGIN * 2.0);

pub const LANG_Y: f32 = 250.0;
pub const LANG_H: f32 = 80.0;

pub const BRIGHT_Y: f32 = 400.0;
pub const BRIGHT_H: f32 = 86.0;

pub const THEME_Y: f32 = 550.0;
pub const THEME_H: f32 = 80.0;

pub const SHAPE_Y: f32 = 700.0;
pub const SHAPE_H: f32 = 80.0;

pub const TIMEOUT_Y: f32 = 850.0;
pub const TIMEOUT_H: f32 = 80.0;

pub const SLOT_Y: f32 = 980.0;
pub const SLOT_H: f32 = 150.0;

pub const HAPTIC_Y: f32 = 1160.0;
pub const HAPTIC_H: f32 = 130.0;

pub const MSG_Y: f32 = 1320.0;
pub const MSG_H: f32 = 70.0;

pub const DOCK_Y: f32 = 2460.0;
pub const DOCK_H: f32 = 110.0;

impl Default for SettingsState {
    fn default() -> Self {
        let brightness_val = hardware::backlight::get_brightness().clamp(100, 4095);
        let brightness_progress = (brightness_val - 100) as f32 / 3995.0;
        let active_slot = hardware::slots::get_current_slot();

        let mut s = Self {
            language: Language::English,
            current_theme: ThemeId::ExpressiveDark,
            current_shape: None,
            brightness_val,
            brightness_progress,
            is_dragging_brightness: false,
            screen_timeout: ScreenTimeout::Sec60,
            active_slot,
            haptic_feedback: true,
            status_message: "Settings loaded".to_string(),
        };
        s.load_from_persist();
        s
    }
}

impl SettingsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_persist(&mut self) {
        if let Ok(content) = fs::read_to_string(SETTINGS_FILE) {
            for line in content.lines() {
                let mut parts = line.split('=');
                if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                    match k.trim() {
                        "lang" | "language" => {
                            self.language = Language::from_code(v.trim());
                        }
                        "theme" => {
                            self.current_theme = match v.trim() {
                                "AmoledPure" => ThemeId::AmoledPure,
                                "EmeraldGreen" => ThemeId::EmeraldGreen,
                                "SunsetAmber" => ThemeId::SunsetAmber,
                                "CyberpunkNeon" => ThemeId::CyberpunkNeon,
                                _ => ThemeId::ExpressiveDark,
                            };
                        }
                        "brightness" => {
                            if let Ok(b) = v.trim().parse::<i32>() {
                                self.brightness_val = b.clamp(100, 4095);
                                self.brightness_progress = (self.brightness_val - 100) as f32 / 3995.0;
                                hardware::backlight::set_brightness(self.brightness_val);
                            }
                        }
                        "timeout" => {
                            self.screen_timeout = match v.trim() {
                                "15" => ScreenTimeout::Sec15,
                                "30" => ScreenTimeout::Sec30,
                                "120" => ScreenTimeout::Min2,
                                "never" => ScreenTimeout::Never,
                                _ => ScreenTimeout::Sec60,
                            };
                        }
                        "haptic" => {
                            self.haptic_feedback = v.trim() == "1";
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn save_to_persist(&mut self) {
        let dir = Path::new("/sdcard/MRR");
        let _ = fs::create_dir_all(dir);

        let timeout_str = match self.screen_timeout {
            ScreenTimeout::Sec15 => "15",
            ScreenTimeout::Sec30 => "30",
            ScreenTimeout::Sec60 => "60",
            ScreenTimeout::Min2 => "120",
            ScreenTimeout::Never => "never",
        };

        let content = format!(
            "lang={}\ntheme={:?}\nbrightness={}\ntimeout={}\nhaptic={}\n",
            self.language.code(),
            self.current_theme,
            self.brightness_val,
            timeout_str,
            if self.haptic_feedback { 1 } else { 0 }
        );

        match fs::write(SETTINGS_FILE, content) {
            Ok(_) => {
                self.status_message = match self.language {
                    Language::English => "Saved to /sdcard/MRR/settings.conf".to_string(),
                    Language::Russian => "Сохранено в /sdcard/MRR/settings.conf".to_string(),
                };
            }
            Err(e) => {
                self.status_message = format!("Persist error: {}", e);
            }
        }
    }

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<SettingsAction> {
        let fx = tx as f32;
        let fy = ty as f32;

        if state == TouchState::Released {
            if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
                return Some(SettingsAction::Back);
            }
            if fx >= MARGIN && fx <= MARGIN + 320.0 && fy >= DOCK_Y && fy <= DOCK_Y + DOCK_H {
                return Some(SettingsAction::Back);
            }
        }

        if fy >= BRIGHT_Y - 20.0 && fy <= BRIGHT_Y + BRIGHT_H + 20.0 && fx >= MARGIN && fx <= MARGIN + CONTENT_W {
            match state {
                TouchState::Pressed => {
                    self.is_dragging_brightness = true;
                    self.brightness_progress = ((fx - MARGIN) / CONTENT_W).clamp(0.0, 1.0);
                    self.brightness_val = 100 + (self.brightness_progress * 3995.0) as i32;
                    hardware::backlight::set_brightness(self.brightness_val);
                    return None;
                }
                TouchState::Released => {
                    if self.is_dragging_brightness {
                        self.is_dragging_brightness = false;
                        self.save_to_persist();
                    }
                }
            }
        } else if state == TouchState::Released && self.is_dragging_brightness {
            self.is_dragging_brightness = false;
        }

        if state == TouchState::Released {
            if fy >= LANG_Y && fy <= LANG_Y + LANG_H {
                let l_gap = 16.0;
                let l_w = (CONTENT_W - l_gap) / 2.0;

                if fx >= MARGIN && fx <= MARGIN + l_w {
                    self.language = Language::English;
                    self.save_to_persist();
                    return Some(SettingsAction::SelectLanguage(Language::English));
                }
                if fx >= MARGIN + l_w + l_gap && fx <= MARGIN + (l_w * 2.0) + l_gap {
                    self.language = Language::Russian;
                    self.save_to_persist();
                    return Some(SettingsAction::SelectLanguage(Language::Russian));
                }
            }

            let th_gap = 12.0;
            let th_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;

            if fy >= THEME_Y && fy <= THEME_Y + THEME_H {
                for (i, t) in ThemeId::ALL.iter().enumerate() {
                    let kx = MARGIN + (i as f32) * (th_w + th_gap);
                    if fx >= kx && fx <= kx + th_w {
                        self.current_theme = *t;
                        self.save_to_persist();
                        return Some(SettingsAction::SelectTheme(*t));
                    }
                }
            }

            let sh_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;
            let shapes_list = [
                None,
                Some(M3ShapeType::Sunny8),
                Some(M3ShapeType::Cookie4),
                Some(M3ShapeType::Squircle),
                Some(M3ShapeType::ElongatedArch),
            ];

            if fy >= SHAPE_Y && fy <= SHAPE_Y + SHAPE_H {
                for (i, shp) in shapes_list.iter().enumerate() {
                    let kx = MARGIN + (i as f32) * (sh_w + th_gap);
                    if fx >= kx && fx <= kx + sh_w {
                        self.current_shape = *shp;
                        return Some(SettingsAction::SelectShape(*shp));
                    }
                }
            }

            let to_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;
            if fy >= TIMEOUT_Y && fy <= TIMEOUT_Y + TIMEOUT_H {
                for (i, t) in ScreenTimeout::ALL.iter().enumerate() {
                    let kx = MARGIN + (i as f32) * (to_w + th_gap);
                    if fx >= kx && fx <= kx + to_w {
                        self.screen_timeout = *t;
                        self.save_to_persist();
                        return Some(SettingsAction::SelectTimeout(*t));
                    }
                }
            }

            if fy >= SLOT_Y && fy <= SLOT_Y + SLOT_H && fx >= MARGIN && fx <= SCREEN_W - MARGIN {
                let target_slot = if self.active_slot == "A" { "B" } else { "A" };
                match hardware::slots::set_active_slot(target_slot) {
                    Ok(_) => {
                        self.active_slot = target_slot.to_string();
                        self.status_message = match self.language {
                            Language::English => format!("Active boot slot switched to Slot {}!", target_slot),
                            Language::Russian => format!("Активный слот переключён на Слот {}!", target_slot),
                        };
                    }
                    Err(e) => {
                        self.status_message = format!("Slot switch error: {}", e);
                    }
                }
                return Some(SettingsAction::SwitchSlot);
            }

            if fy >= HAPTIC_Y && fy <= HAPTIC_Y + HAPTIC_H && fx >= MARGIN && fx <= SCREEN_W - MARGIN {
                self.haptic_feedback = !self.haptic_feedback;
                self.save_to_persist();
                return Some(SettingsAction::ToggleHaptic);
            }
        }

        None
    }
}

pub fn render_settings(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &SettingsState,
    pressed: Option<SettingsAction>,
) {
    canvas.clear(theme.background);

    let lang = state.language;

    let back_cx = MARGIN + 44.0;
    let back_cy = 115.0;
    let is_back_pressed = pressed == Some(SettingsAction::Back);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    let title = match lang {
        Language::English => "Settings",
        Language::Russian => "Настройки",
    };
    let subtitle = match lang {
        Language::English => "Display, theme, language, A/B slot and persist",
        Language::Russian => "Дисплей, темы, язык, A/B слот и persist",
    };
    fonts.draw_text(&mut canvas.pixmap, title, back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, subtitle, back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    let lang_sec_title = match lang {
        Language::English => "SYSTEM LANGUAGE",
        Language::Russian => "ЯЗЫК ИНТЕРФЕЙСА",
    };
    fonts.draw_text(&mut canvas.pixmap, lang_sec_title, MARGIN, LANG_Y - 18.0, 22.0, theme.primary);

    let l_gap = 16.0;
    let l_w = (CONTENT_W - l_gap) / 2.0;

    let languages = [
        (Language::English, "English (Default)"),
        (Language::Russian, "Русский"),
    ];

    for (i, (l_code, label)) in languages.iter().enumerate() {
        let kx = MARGIN + (i as f32) * (l_w + l_gap);
        let is_sel = state.language == *l_code;

        let (bg, border, fg) = if is_sel {
            (theme.primary_container, Some((theme.primary, 2.5)), theme.primary)
        } else {
            (theme.surface_container_high, Some((theme.outline_variant, 1.0)), theme.on_surface)
        };

        canvas.draw_rounded_rect(kx, LANG_Y, l_w, LANG_H, 22.0, Some(bg), border);
        let nw = fonts.measure_text(label, 22.0);
        fonts.draw_text(&mut canvas.pixmap, label, kx + (l_w - nw) / 2.0, LANG_Y + 49.0, 22.0, fg);
    }

    let bright_sec_title = match lang {
        Language::English => "DISPLAY BRIGHTNESS (PWM)",
        Language::Russian => "ЯРКОСТЬ ДИСПЛЕЯ (ШИМ)",
    };
    fonts.draw_text(&mut canvas.pixmap, bright_sec_title, MARGIN, BRIGHT_Y - 18.0, 22.0, theme.primary);

    let bright_percent = ((state.brightness_val - 100) as f32 / 39.95).round() as i32;
    let b_info = format!("{}% ({} / 4095)", bright_percent, state.brightness_val);
    let bw = fonts.measure_text(&b_info, 20.0);
    fonts.draw_text(&mut canvas.pixmap, &b_info, SCREEN_W - MARGIN - bw, BRIGHT_Y - 18.0, 20.0, theme.on_surface_variant);

    canvas.draw_pill(MARGIN, BRIGHT_Y, CONTENT_W, BRIGHT_H, theme.surface_container_high, Some((theme.outline_variant, 1.5)));

    let fill_w = (CONTENT_W * state.brightness_progress).clamp(BRIGHT_H, CONTENT_W);
    canvas.draw_pill(MARGIN, BRIGHT_Y, fill_w, BRIGHT_H, theme.primary, None);

    draw_icon(&mut canvas.pixmap, M3Icon::Brightness, MARGIN + 44.0, BRIGHT_Y + BRIGHT_H / 2.0, 34.0, theme.on_primary);

    let thumb_cx = MARGIN + (CONTENT_W - BRIGHT_H) * state.brightness_progress + (BRIGHT_H / 2.0);
    canvas.draw_circle(thumb_cx, BRIGHT_Y + BRIGHT_H / 2.0, BRIGHT_H / 2.0 + 4.0, theme.on_primary);

    let theme_sec_title = match lang {
        Language::English => "COLOR THEME",
        Language::Russian => "ЦВЕТОВАЯ ТЕМА",
    };
    fonts.draw_text(&mut canvas.pixmap, theme_sec_title, MARGIN, THEME_Y - 18.0, 22.0, theme.primary);

    let th_gap = 12.0;
    let th_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;

    for (i, t) in ThemeId::ALL.iter().enumerate() {
        let kx = MARGIN + (i as f32) * (th_w + th_gap);
        let is_sel = state.current_theme == *t;

        let (bg, border, fg) = if is_sel {
            (theme.primary_container, Some((theme.primary, 2.5)), theme.primary)
        } else {
            (theme.surface_container_high, Some((theme.outline_variant, 1.0)), theme.on_surface)
        };

        canvas.draw_rounded_rect(kx, THEME_Y, th_w, THEME_H, 22.0, Some(bg), border);
        let name = t.name();
        let nw = fonts.measure_text(name, 19.0);
        fonts.draw_text(&mut canvas.pixmap, name, kx + (th_w - nw) / 2.0, THEME_Y + 49.0, 19.0, fg);
    }

    let shape_sec_title = match lang {
        Language::English => "MATERIAL 3 SHAPE PROFILE",
        Language::Russian => "ФОРМА СКРУГЛЕНИЙ MATERIAL 3",
    };
    fonts.draw_text(&mut canvas.pixmap, shape_sec_title, MARGIN, SHAPE_Y - 18.0, 22.0, theme.primary);

    let sh_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;
    let shapes_list = [
        (None, "Default"),
        (Some(M3ShapeType::Sunny8), "Sunny-8"),
        (Some(M3ShapeType::Cookie4), "Cookie-4"),
        (Some(M3ShapeType::Squircle), "Squircle"),
        (Some(M3ShapeType::ElongatedArch), "Arch"),
    ];

    for (i, (shp, label)) in shapes_list.iter().enumerate() {
        let kx = MARGIN + (i as f32) * (sh_w + th_gap);
        let is_sel = state.current_shape == *shp;

        let (bg, border, fg) = if is_sel {
            (theme.tertiary_container, Some((theme.tertiary, 2.5)), theme.tertiary)
        } else {
            (theme.surface_container_high, Some((theme.outline_variant, 1.0)), theme.on_surface)
        };

        canvas.draw_rounded_rect(kx, SHAPE_Y, sh_w, SHAPE_H, 22.0, Some(bg), border);
        let nw = fonts.measure_text(label, 20.0);
        fonts.draw_text(&mut canvas.pixmap, label, kx + (sh_w - nw) / 2.0, SHAPE_Y + 49.0, 20.0, fg);
    }

    let timeout_sec_title = match lang {
        Language::English => "SCREEN SLEEP TIMEOUT",
        Language::Russian => "ТАЙМАУТ ЗАСЫПАНИЯ ЭКРАНА",
    };
    fonts.draw_text(&mut canvas.pixmap, timeout_sec_title, MARGIN, TIMEOUT_Y - 18.0, 22.0, theme.primary);

    let to_w = (CONTENT_W - (th_gap * 4.0)) / 5.0;

    for (i, t) in ScreenTimeout::ALL.iter().enumerate() {
        let kx = MARGIN + (i as f32) * (to_w + th_gap);
        let is_sel = state.screen_timeout == *t;

        let (bg, border, fg) = if is_sel {
            (theme.secondary_container, Some((theme.secondary, 2.5)), theme.secondary)
        } else {
            (theme.surface_container_high, Some((theme.outline_variant, 1.0)), theme.on_surface)
        };

        canvas.draw_rounded_rect(kx, TIMEOUT_Y, to_w, TIMEOUT_H, 22.0, Some(bg), border);
        let lbl = t.label(lang);
        let nw = fonts.measure_text(lbl, 21.0);
        fonts.draw_text(&mut canvas.pixmap, lbl, kx + (to_w - nw) / 2.0, TIMEOUT_Y + 49.0, 21.0, fg);
    }

    canvas.draw_rounded_rect(MARGIN, SLOT_Y, CONTENT_W, SLOT_H, 28.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.5)));

    let slot_cx = MARGIN + 60.0;
    let slot_cy = SLOT_Y + SLOT_H / 2.0;
    canvas.draw_circle(slot_cx, slot_cy, 44.0, theme.primary_container);
    draw_icon(&mut canvas.pixmap, M3Icon::Restart, slot_cx, slot_cy, 40.0, theme.primary);

    let slot_card_title = match lang {
        Language::English => "Active Boot Slot",
        Language::Russian => "Текущий активный слот",
    };
    fonts.draw_text(&mut canvas.pixmap, slot_card_title, slot_cx + 60.0, SLOT_Y + 56.0, 28.0, theme.on_surface);

    let target_slot_str = if state.active_slot == "A" { "B" } else { "A" };
    let slot_desc = match lang {
        Language::English => format!("Active: Slot {} • Tap to switch to Slot {}", state.active_slot, target_slot_str),
        Language::Russian => format!("Активен Слот {} • Нажмите для переключения на Слот {}", state.active_slot, target_slot_str),
    };
    fonts.draw_text(&mut canvas.pixmap, &slot_desc, slot_cx + 60.0, SLOT_Y + 100.0, 20.0, theme.on_surface_variant);

    let badge_txt = format!("SLOT {}", state.active_slot);
    let sbw = fonts.measure_text(&badge_txt, 22.0);
    let sb_x = SCREEN_W - MARGIN - sbw - 40.0;
    canvas.draw_pill(sb_x - 16.0, SLOT_Y + 42.0, sbw + 32.0, 64.0, theme.primary, None);
    fonts.draw_text(&mut canvas.pixmap, &badge_txt, sb_x, SLOT_Y + 84.0, 22.0, theme.on_primary);

    canvas.draw_rounded_rect(MARGIN, HAPTIC_Y, CONTENT_W, HAPTIC_H, 28.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.5)));

    let hap_cx = MARGIN + 60.0;
    let hap_cy = HAPTIC_Y + HAPTIC_H / 2.0;
    canvas.draw_circle(hap_cx, hap_cy, 40.0, theme.secondary_container);
    draw_icon(&mut canvas.pixmap, M3Icon::Settings, hap_cx, hap_cy, 36.0, theme.secondary);

    let hap_card_title = match lang {
        Language::English => "Haptic Feedback (Vibration)",
        Language::Russian => "Тактильный отклик (Вибрация)",
    };
    fonts.draw_text(&mut canvas.pixmap, hap_card_title, hap_cx + 60.0, HAPTIC_Y + 54.0, 28.0, theme.on_surface);

    let hap_sub = match lang {
        Language::English => {
            if state.haptic_feedback { "Enabled for touches and gestures" } else { "Disabled" }
        }
        Language::Russian => {
            if state.haptic_feedback { "Включён при касаниях и жестах" } else { "Отключён" }
        }
    };
    fonts.draw_text(&mut canvas.pixmap, hap_sub, hap_cx + 60.0, HAPTIC_Y + 96.0, 20.0, theme.on_surface_variant);

    let sw_x = SCREEN_W - MARGIN - 120.0;
    let sw_y = HAPTIC_Y + 42.0;
    let (sw_bg, sw_knob_x) = if state.haptic_feedback {
        (theme.primary, sw_x + 38.0)
    } else {
        (theme.surface_container_highest, sw_x + 16.0)
    };
    canvas.draw_pill(sw_x, sw_y, 80.0, 46.0, sw_bg, None);
    canvas.draw_circle(sw_knob_x, sw_y + 23.0, 16.0, theme.on_surface);

    canvas.draw_pill(MARGIN, MSG_Y, CONTENT_W, MSG_H, theme.surface_container_highest, Some((theme.outline_variant, 1.0)));
    fonts.draw_text(&mut canvas.pixmap, &state.status_message, MARGIN + 40.0, MSG_Y + 44.0, 21.0, theme.on_surface);

    let btn_w = 320.0;
    canvas.draw_pill(MARGIN, DOCK_Y, btn_w, DOCK_H, theme.surface_container_high, Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, MARGIN + 48.0, DOCK_Y + DOCK_H / 2.0, 36.0, theme.on_surface);
    let back_lbl = match lang {
        Language::English => "BACK",
        Language::Russian => "НАЗАД",
    };
    fonts.draw_text(&mut canvas.pixmap, back_lbl, MARGIN + 96.0, DOCK_Y + 68.0, 24.0, theme.on_surface);
}
