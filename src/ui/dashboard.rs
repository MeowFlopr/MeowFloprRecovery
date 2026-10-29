use tiny_skia::{Color, FillRule, Paint, Transform};

use super::canvas::Canvas;
use super::i18n::Language;
use super::icons::{draw_icon, M3Icon};
use super::shapes::{M3ShapeType, ShapeRenderer};
use super::text::FontRenderer;
use super::theme::{M3Shapes, M3Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTarget {
    HeroCard,
    Install,
    Wipe,
    Backup,
    Decrypt,
    Mount,
    Advanced,
    Terminal,
    Settings,
    Power,
    DockBack,
    DockTerminal,
    DockPower,
}

pub struct DashboardState<'a> {
    pub language: Language,
    pub slot: &'a str,
    pub battery_percent: u32,
    pub battery_temp: i32,
    pub is_decrypted: bool,
    pub storage_free_gb: u32,
    pub storage_total_gb: u32,
    pub time_str: &'a str,
    pub active_shape: Option<M3ShapeType>,
    pub pressed_target: Option<ActionTarget>,
}

pub fn hit_test(x: i32, y: i32) -> Option<ActionTarget> {
    let fx = x as f32;
    let fy = y as f32;
    let margin = 40.0;
    let screen_w = 1220.0;
    let content_w = screen_w - (margin * 2.0);

    if fx >= margin && fx <= margin + 260.0 && fy >= 60.0 && fy <= 140.0 {
        return Some(ActionTarget::Decrypt);
    }

    if fx >= margin && fx <= margin + content_w && fy >= 170.0 && fy <= 510.0 {
        return Some(ActionTarget::HeroCard);
    }

    let grid_y = 550.0;
    let grid_gap = 24.0;
    let card_w = (content_w - grid_gap) / 2.0;
    let card_h = 380.0;

    let targets = [
        ActionTarget::Install,
        ActionTarget::Wipe,
        ActionTarget::Backup,
        ActionTarget::Mount,
        ActionTarget::Advanced,
        ActionTarget::Terminal,
        ActionTarget::Settings,
        ActionTarget::Power,
    ];

    for (i, target) in targets.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let cx = margin + col * (card_w + grid_gap);
        let cy = grid_y + row * (card_h + grid_gap);

        if fx >= cx && fx <= cx + card_w && fy >= cy && fy <= cy + card_h {
            return Some(*target);
        }
    }

    let dock_y = 2460.0;
    if fy >= dock_y && fy <= dock_y + 120.0 {
        let back_x = margin + 24.0;
        let back_w = 280.0;
        if fx >= back_x && fx <= back_x + back_w {
            return Some(ActionTarget::DockBack);
        }

        let center_w = 420.0;
        let center_x = margin + (content_w - center_w) / 2.0;
        if fx >= center_x && fx <= center_x + center_w {
            return Some(ActionTarget::DockTerminal);
        }

        let pwr_x = screen_w - margin - 24.0 - 280.0;
        let pwr_w = 280.0;
        if fx >= pwr_x && fx <= pwr_x + pwr_w {
            return Some(ActionTarget::DockPower);
        }
    }

    None
}

pub fn render_dashboard(
    canvas: &mut Canvas,
    theme: &M3Theme,
    shapes: &M3Shapes,
    fonts: &FontRenderer,
    state: &DashboardState,
) {
    canvas.clear(theme.background);

    let lang = state.language;
    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

    let status_y = 70.0;

    let (dec_bg, dec_text, dec_icon_color) = if state.is_decrypted {
        (
            Color::from_rgba8(20, 50, 32, 255),
            match lang {
                Language::English => "DECRYPTED",
                Language::Russian => "РАСШИФРОВАНО",
            },
            Color::from_rgba8(120, 240, 160, 255),
        )
    } else {
        (
            Color::from_rgba8(60, 24, 24, 255),
            match lang {
                Language::English => "ENCRYPTED",
                Language::Russian => "ЗАБЛОКИРОВАНО",
            },
            Color::from_rgba8(255, 140, 140, 255),
        )
    };
    canvas.draw_pill(margin, status_y, 250.0, 56.0, dec_bg, Some((theme.outline_variant, 1.0)));
    draw_icon(&mut canvas.pixmap, M3Icon::Shield, margin + 35.0, status_y + 28.0, 24.0, dec_icon_color);
    fonts.draw_text(&mut canvas.pixmap, dec_text, margin + 65.0, status_y + 36.0, 18.0, dec_icon_color);

    let bat_x = screen_w - margin - 260.0;
    canvas.draw_pill(
        bat_x,
        status_y,
        140.0,
        56.0,
        theme.surface_container,
        Some((theme.outline_variant, 1.0)),
    );
    draw_icon(&mut canvas.pixmap, M3Icon::Battery, bat_x + 30.0, status_y + 28.0, 22.0, theme.on_surface);
    let bat_str = format!("{}%", state.battery_percent);
    fonts.draw_text(&mut canvas.pixmap, &bat_str, bat_x + 60.0, status_y + 36.0, 19.0, theme.on_surface);

    let time_x = screen_w - margin - 100.0;
    canvas.draw_pill(
        time_x,
        status_y,
        100.0,
        56.0,
        theme.surface_container,
        Some((theme.outline_variant, 1.0)),
    );
    fonts.draw_text(&mut canvas.pixmap, state.time_str, time_x + 22.0, status_y + 36.0, 19.0, theme.on_surface);

    let hero_y = 170.0;
    let hero_h = 340.0;

    let hero_bg = if state.pressed_target == Some(ActionTarget::HeroCard) {
        theme.surface_container_high
    } else {
        theme.surface_container
    };

    canvas.draw_rounded_rect(
        margin,
        hero_y,
        content_w,
        hero_h,
        shapes.corner_card,
        Some(hero_bg),
        Some((theme.outline_variant, 1.0)),
    );

    let hero_shape = state.active_shape.unwrap_or(M3ShapeType::Cookie4);
    if let Some(path) = ShapeRenderer::build_shape(hero_shape, margin + 75.0, hero_y + 85.0, 90.0) {
        let mut paint = Paint::default();
        paint.set_color(theme.primary_container);
        paint.anti_alias = true;
        canvas.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }
    draw_icon(&mut canvas.pixmap, M3Icon::Storage, margin + 75.0, hero_y + 85.0, 44.0, theme.primary);

    let hero_title = match lang {
        Language::English => "Internal Storage (FBE v2)",
        Language::Russian => "Внутренний накопитель (FBE v2)",
    };
    fonts.draw_text(
        &mut canvas.pixmap,
        hero_title,
        margin + 145.0,
        hero_y + 75.0,
        28.0,
        theme.on_surface,
    );

    let storage_desc = match lang {
        Language::English => format!(
            "Available {} GB of {} GB • Slot {} • Dimensity 8300",
            state.storage_free_gb, state.storage_total_gb, state.slot
        ),
        Language::Russian => format!(
            "Свободно {} ГБ из {} ГБ • Слот {} • Dimensity 8300",
            state.storage_free_gb, state.storage_total_gb, state.slot
        ),
    };
    fonts.draw_text(
        &mut canvas.pixmap,
        &storage_desc,
        margin + 145.0,
        hero_y + 115.0,
        18.0,
        theme.on_surface_variant,
    );

    let progress = if state.storage_total_gb > 0 {
        (state.storage_total_gb - state.storage_free_gb) as f32 / state.storage_total_gb as f32
    } else {
        0.10
    };

    let bar_y = hero_y + 185.0;
    let bar_w = content_w - 60.0;
    canvas.draw_progress_bar(
        margin + 30.0,
        bar_y,
        bar_w,
        24.0,
        progress,
        theme.surface_container_highest,
        theme.primary,
    );

    let slot_chip_x = margin + 30.0;
    let slot_chip_y = hero_y + 245.0;
    canvas.draw_pill(
        slot_chip_x,
        slot_chip_y,
        180.0,
        48.0,
        theme.surface_container_high,
        Some((theme.outline_variant, 1.0)),
    );
    let slot_text = match lang {
        Language::English => format!("Active Slot {}", state.slot),
        Language::Russian => format!("Активен Слот {}", state.slot),
    };
    fonts.draw_text(&mut canvas.pixmap, &slot_text, slot_chip_x + 24.0, slot_chip_y + 31.0, 17.0, theme.on_surface);

    let fs_chip_x = slot_chip_x + 200.0;
    canvas.draw_pill(
        fs_chip_x,
        slot_chip_y,
        260.0,
        48.0,
        theme.surface_container_high,
        Some((theme.outline_variant, 1.0)),
    );
    fonts.draw_text(
        &mut canvas.pixmap,
        "FS: F2FS (inlinecrypt)",
        fs_chip_x + 24.0,
        slot_chip_y + 31.0,
        17.0,
        theme.on_surface_variant,
    );

    let grid_y = 550.0;
    let grid_gap = 24.0;
    let card_w = (content_w - grid_gap) / 2.0;
    let card_h = 380.0;

    let cards = match lang {
        Language::English => [
            ("Install", "Flash ZIP, Magisk & kernel", theme.primary_container, theme.primary, M3Icon::Install, M3ShapeType::Sunny8, ActionTarget::Install),
            ("Wipe", "Factory reset, cache & format", theme.error_container, theme.error, M3Icon::Wipe, M3ShapeType::ElongatedArch, ActionTarget::Wipe),
            ("Backup", "Create & restore snapshots", theme.tertiary_container, theme.tertiary, M3Icon::Backup, M3ShapeType::PuffyCloud, ActionTarget::Backup),
            ("Mount", "Partitions, USB MTP & Sideload", theme.secondary_container, theme.secondary, M3Icon::Mount, M3ShapeType::Cookie4, ActionTarget::Mount),
            ("Advanced", "Root Manager, files & logs", theme.primary_container, theme.primary, M3Icon::Advanced, M3ShapeType::Cookie4, ActionTarget::Advanced),
            ("Terminal", "Interactive UNIX /bin/sh shell", theme.tertiary_container, theme.tertiary, M3Icon::Terminal, M3ShapeType::Squircle, ActionTarget::Terminal),
            ("Settings", "Themes, brightness & A/B slot", theme.surface_container_highest, theme.on_surface, M3Icon::Settings, M3ShapeType::Cookie4, ActionTarget::Settings),
            ("Power", "System, Recovery, Bootloader", theme.error_container, theme.error, M3Icon::Power, M3ShapeType::Circle, ActionTarget::Power),
        ],
        Language::Russian => [
            ("Прошивка", "Установка ZIP, Magisk, ядер", theme.primary_container, theme.primary, M3Icon::Install, M3ShapeType::Sunny8, ActionTarget::Install),
            ("Очистка", "Сброс к заводским, кэш, format", theme.error_container, theme.error, M3Icon::Wipe, M3ShapeType::ElongatedArch, ActionTarget::Wipe),
            ("Бэкап", "Создание и восстановление дампа", theme.tertiary_container, theme.tertiary, M3Icon::Backup, M3ShapeType::PuffyCloud, ActionTarget::Backup),
            ("Монтирование", "Разделы, MTP USB и Sideload", theme.secondary_container, theme.secondary, M3Icon::Mount, M3ShapeType::Cookie4, ActionTarget::Mount),
            ("Инструменты", "Root Magisk, файлы, логи", theme.primary_container, theme.primary, M3Icon::Advanced, M3ShapeType::Cookie4, ActionTarget::Advanced),
            ("Терминал", "Командная строка /system/bin/sh", theme.tertiary_container, theme.tertiary, M3Icon::Terminal, M3ShapeType::Squircle, ActionTarget::Terminal),
            ("Настройки", "Яркость, темы, слот A/B", theme.surface_container_highest, theme.on_surface, M3Icon::Settings, M3ShapeType::Cookie4, ActionTarget::Settings),
            ("Питание", "Система, Fastboot, Bootloader", theme.error_container, theme.error, M3Icon::Power, M3ShapeType::Circle, ActionTarget::Power),
        ],
    };

    for (i, (title, sub, container_bg, accent_color, icon, signature_shape, target)) in cards.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;

        let cx = margin + col * (card_w + grid_gap);
        let cy = grid_y + row * (card_h + grid_gap);

        let is_pressed = state.pressed_target == Some(*target);
        let current_card_bg = if is_pressed {
            theme.surface_container_high
        } else {
            theme.surface_container
        };
        let current_outline = if is_pressed {
            (theme.primary, 1.5)
        } else {
            (theme.outline_variant, 1.0)
        };

        canvas.draw_rounded_rect(
            cx,
            cy,
            card_w,
            card_h,
            shapes.corner_card,
            Some(current_card_bg),
            Some(current_outline),
        );

        let active_shape = state.active_shape.unwrap_or(*signature_shape);
        let icon_cx = cx + 80.0;
        let icon_cy = cy + 90.0;
        let icon_size = 94.0;

        if let Some(path) = ShapeRenderer::build_shape(active_shape, icon_cx, icon_cy, icon_size) {
            let mut paint = Paint::default();
            paint.set_color(*container_bg);
            paint.anti_alias = true;
            canvas.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }

        draw_icon(&mut canvas.pixmap, *icon, icon_cx, icon_cy, 46.0, *accent_color);

        fonts.draw_text(
            &mut canvas.pixmap,
            title,
            cx + 42.0,
            cy + 250.0,
            30.0,
            theme.on_surface,
        );

        fonts.draw_text(
            &mut canvas.pixmap,
            sub,
            cx + 42.0,
            cy + 300.0,
            18.0,
            theme.on_surface_variant,
        );
    }

    let dock_y = 2460.0;
    let dock_h = 120.0;

    canvas.draw_pill(
        margin,
        dock_y,
        content_w,
        dock_h,
        theme.surface_container_high,
        Some((theme.outline_variant, 1.5)),
    );

    let back_x = margin + 24.0;
    let back_w = 280.0;
    let back_pressed = state.pressed_target == Some(ActionTarget::DockBack);
    canvas.draw_pill(
        back_x,
        dock_y + 18.0,
        back_w,
        84.0,
        if back_pressed { theme.surface_container_highest } else { theme.surface_container },
        Some((theme.outline_variant, 1.0)),
    );
    let back_str = match lang {
        Language::English => "<- Back",
        Language::Russian => "<- Назад",
    };
    fonts.draw_text(&mut canvas.pixmap, back_str, back_x + 85.0, dock_y + 70.0, 22.0, theme.on_surface);

    let center_w = 420.0;
    let center_x = margin + (content_w - center_w) / 2.0;
    let terminal_pressed = state.pressed_target == Some(ActionTarget::DockTerminal);
    canvas.draw_pill(
        center_x,
        dock_y + 18.0,
        center_w,
        84.0,
        if terminal_pressed { theme.secondary } else { theme.primary },
        None,
    );
    draw_icon(&mut canvas.pixmap, M3Icon::Terminal, center_x + 80.0, dock_y + 60.0, 36.0, theme.on_primary);
    let term_str = match lang {
        Language::English => "Terminal",
        Language::Russian => "Терминал",
    };
    fonts.draw_text(&mut canvas.pixmap, term_str, center_x + 130.0, dock_y + 70.0, 24.0, theme.on_primary);

    let pwr_x = screen_w - margin - 24.0 - 280.0;
    let pwr_pressed = state.pressed_target == Some(ActionTarget::DockPower);
    canvas.draw_pill(
        pwr_x,
        dock_y + 18.0,
        280.0,
        84.0,
        if pwr_pressed { theme.surface_container_highest } else { theme.surface_container },
        Some((theme.outline_variant, 1.0)),
    );
    draw_icon(&mut canvas.pixmap, M3Icon::Power, pwr_x + 50.0, dock_y + 60.0, 32.0, theme.error);
    let pwr_str = match lang {
        Language::English => "Power",
        Language::Russian => "Питание",
    };
    fonts.draw_text(&mut canvas.pixmap, pwr_str, pwr_x + 95.0, dock_y + 70.0, 22.0, theme.error);
}
