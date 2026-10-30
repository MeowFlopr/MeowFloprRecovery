use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::hardware::fastbootd::FastbootStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastbootdAction {
    Exit,
    RebootSystem,
    RebootRecovery,
    RebootBootloader,
}

pub struct FastbootdMetrics {
    pub margin: f32,
    pub width: f32,
    pub hero_y: f32,
    pub hero_h: f32,
    pub term_y: f32,
    pub term_h: f32,
    pub btn_y: f32,
    pub btn_h: f32,
}

impl Default for FastbootdMetrics {
    fn default() -> Self {
        Self {
            margin: 70.0,
            width: 1080.0,
            hero_y: 230.0,
            hero_h: 460.0,
            term_y: 720.0,
            term_h: 1600.0,
            btn_y: 2360.0,
            btn_h: 140.0,
        }
    }
}

pub fn hit_test(x: i32, y: i32) -> Option<FastbootdAction> {
    let m = FastbootdMetrics::default();
    let fx = x as f32;
    let fy = y as f32;

    let back_cx = m.margin + 44.0;
    let back_cy = 130.0;
    if (fx - back_cx).powi(2) + (fy - back_cy).powi(2) <= 50.0f32.powi(2) {
        return Some(FastbootdAction::Exit);
    }

    if fy >= m.btn_y && fy <= m.btn_y + m.btn_h {
        let btn_w = (m.width - 32.0) / 3.0;

        let b1_x = m.margin;
        if fx >= b1_x && fx <= b1_x + btn_w {
            return Some(FastbootdAction::Exit);
        }

        let b2_x = b1_x + btn_w + 16.0;
        if fx >= b2_x && fx <= b2_x + btn_w {
            return Some(FastbootdAction::RebootSystem);
        }

        let b3_x = b2_x + btn_w + 16.0;
        if fx >= b3_x && fx <= b3_x + btn_w {
            return Some(FastbootdAction::RebootRecovery);
        }
    }

    None
}

pub fn render_fastbootd(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    status: &FastbootStatus,
    pressed: Option<FastbootdAction>,
) {
    let m = FastbootdMetrics::default();

    canvas.pixmap.fill(theme.background);

    let back_cx = m.margin + 44.0;
    let back_cy = 130.0;
    let back_pressed = pressed == Some(FastbootdAction::Exit);
    let back_bg = if back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };

    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(
        &mut canvas.pixmap,
        M3Icon::ArrowBack,
        back_cx,
        back_cy,
        40.0,
        theme.on_surface,
    );

    let title_x = back_cx + 64.0;
    let title_y = back_cy - 4.0;
    fonts.draw_text(
        &mut canvas.pixmap,
        "FastbootD Mode",
        title_x,
        title_y,
        42.0,
        theme.on_surface,
    );

    let sub_title = format!("Userspace Dynamic Fastboot • Active Slot: {}", status.current_slot);
    fonts.draw_text(
        &mut canvas.pixmap,
        &sub_title,
        title_x,
        title_y + 36.0,
        22.0,
        theme.on_surface_variant,
    );

    canvas.draw_rounded_rect(
        m.margin,
        m.hero_y,
        m.width,
        m.hero_h,
        36.0,
        Some(theme.surface_container),
        Some((theme.outline_variant, 1.5)),
    );

    let usb_cx = m.margin + 80.0;
    let usb_cy = m.hero_y + 80.0;
    let (icon_bg, icon_fg, badge_text) = if status.is_connected {
        (
            theme.primary_container,
            theme.primary,
            "CONNECTED (USB 3.0)",
        )
    } else {
        (
            theme.surface_container_high,
            theme.on_surface_variant,
            "LISTENING ON USB",
        )
    };

    canvas.draw_circle(usb_cx, usb_cy, 46.0, icon_bg);
    draw_icon(
        &mut canvas.pixmap,
        M3Icon::Usb,
        usb_cx,
        usb_cy,
        44.0,
        icon_fg,
    );

    let badge_x = m.margin + m.width - 290.0;
    let badge_y = m.hero_y + 46.0;
    canvas.draw_pill(
        badge_x,
        badge_y,
        250.0,
        56.0,
        icon_bg,
        Some((theme.outline_variant, 1.0)),
    );
    fonts.draw_text(
        &mut canvas.pixmap,
        badge_text,
        badge_x + 22.0,
        badge_y + 37.0,
        20.0,
        icon_fg,
    );

    let stat_x = usb_cx + 70.0;
    let stat_y = usb_cy + 8.0;
    fonts.draw_text(
        &mut canvas.pixmap,
        &status.status_text,
        stat_x,
        stat_y,
        28.0,
        theme.on_surface,
    );

    let track_x = m.margin + 44.0;
    let track_y = m.hero_y + 160.0;
    let track_w = m.width - 88.0;
    let track_h = 16.0;

    canvas.draw_pill(track_x, track_y, track_w, track_h, theme.surface_container_highest, None);
    if status.progress > 0.0 {
        let fill_w = track_w * status.progress.clamp(0.0, 1.0);
        canvas.draw_pill(track_x, track_y, fill_w, track_h, theme.primary, None);
    }

    let specs_y = track_y + 54.0;
    let info_rows = [
        ("Device:", "Xiaomi 14T (degas, MT6897)"),
        ("Protocol:", "Fastboot userspace v1.0 / v2.0"),
        ("Dynamic Partitions:", "liblp dm-linear mapped"),
        ("Active Command:", if status.last_command.is_empty() { "none" } else { &status.last_command }),
    ];

    for (idx, (k, v)) in info_rows.iter().enumerate() {
        let ry = specs_y + (idx as f32) * 44.0;
        fonts.draw_text(&mut canvas.pixmap, k, track_x, ry, 22.0, theme.on_surface_variant);
        fonts.draw_text(&mut canvas.pixmap, v, track_x + 240.0, ry, 22.0, theme.on_surface);
    }

    canvas.draw_rounded_rect(
        m.margin,
        m.term_y,
        m.width,
        m.term_h,
        32.0,
        Some(Color::from_rgba8(12, 14, 18, 255)),
        Some((theme.outline_variant, 1.5)),
    );

    let term_hdr_y = m.term_y + 44.0;
    fonts.draw_text(
        &mut canvas.pixmap,
        "Fastboot Console Log",
        m.margin + 36.0,
        term_hdr_y,
        24.0,
        theme.primary,
    );

    let line_start_y = term_hdr_y + 40.0;
    let max_lines = 32;
    let log_slice = if status.logs.len() > max_lines {
        &status.logs[status.logs.len() - max_lines..]
    } else {
        &status.logs[..]
    };

    for (i, line) in log_slice.iter().enumerate() {
        let ly = line_start_y + (i as f32) * 44.0;
        if ly > m.term_y + m.term_h - 20.0 {
            break;
        }

        let fg = if line.starts_with(">") {
            theme.primary
        } else if line.contains("FAIL") || line.contains("error") {
            theme.error
        } else if line.contains("OKAY") || line.contains("success") {
            Color::from_rgba8(120, 240, 160, 255)
        } else {
            theme.on_surface_variant
        };

        fonts.draw_text(&mut canvas.pixmap, line, m.margin + 36.0, ly, 20.0, fg);
    }

    let btn_w = (m.width - 32.0) / 3.0;

    let b1_x = m.margin;
    let b1_pressed = pressed == Some(FastbootdAction::Exit);
    let b1_bg = if b1_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_rounded_rect(b1_x, m.btn_y, btn_w, m.btn_h, 28.0, Some(b1_bg), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, b1_x + (btn_w / 2.0), m.btn_y + 44.0, 36.0, theme.primary);
    let l1_w = fonts.measure_text("Exit to Recovery", 22.0);
    fonts.draw_text(&mut canvas.pixmap, "Exit to Recovery", b1_x + (btn_w - l1_w) / 2.0, m.btn_y + 104.0, 22.0, theme.on_surface);

    let b2_x = b1_x + btn_w + 16.0;
    let b2_pressed = pressed == Some(FastbootdAction::RebootSystem);
    let b2_bg = if b2_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_rounded_rect(b2_x, m.btn_y, btn_w, m.btn_h, 28.0, Some(b2_bg), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::Restart, b2_x + (btn_w / 2.0), m.btn_y + 44.0, 36.0, theme.primary);
    let l2_w = fonts.measure_text("Reboot System", 22.0);
    fonts.draw_text(&mut canvas.pixmap, "Reboot System", b2_x + (btn_w - l2_w) / 2.0, m.btn_y + 104.0, 22.0, theme.on_surface);

    let b3_x = b2_x + btn_w + 16.0;
    let b3_pressed = pressed == Some(FastbootdAction::RebootRecovery);
    let b3_bg = if b3_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_rounded_rect(b3_x, m.btn_y, btn_w, m.btn_h, 28.0, Some(b3_bg), Some((theme.outline_variant, 1.5)));
    draw_icon(&mut canvas.pixmap, M3Icon::Terminal, b3_x + (btn_w / 2.0), m.btn_y + 44.0, 36.0, theme.primary);
    let l3_w = fonts.measure_text("Reboot Recovery", 22.0);
    fonts.draw_text(&mut canvas.pixmap, "Reboot Recovery", b3_x + (btn_w - l3_w) / 2.0, m.btn_y + 104.0, 22.0, theme.on_surface);
}
