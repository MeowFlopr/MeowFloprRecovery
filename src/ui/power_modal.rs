use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::{M3Shapes, M3Theme};
use crate::hardware::power::RebootTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerModalTarget {
    System,
    Recovery,
    Bootloader,
    FastbootD,
    PowerOff,
    Close,
}

pub struct PowerModalMetrics {
    pub card_x: f32,
    pub card_y: f32,
    pub card_w: f32,
    pub card_h: f32,
    pub row_x: f32,
    pub row_w: f32,
    pub row_h: f32,
    pub row_gap: f32,
    pub rows_start_y: f32,
    pub cancel_y: f32,
    pub cancel_h: f32,
}

impl Default for PowerModalMetrics {
    fn default() -> Self {
        let screen_w = 1220.0;
        let card_w = 1080.0;
        let card_x = (screen_w - card_w) / 2.0; // 70.0
        let card_y = 520.0;
        let card_h = 1600.0;

        let row_w = card_w - 96.0; // 984.0
        let row_x = card_x + 48.0; // 118.0
        let row_h = 170.0;
        let row_gap = 24.0;
        let rows_start_y = card_y + 250.0;

        let cancel_h = 110.0;
        let cancel_y = card_y + card_h - cancel_h - 48.0;

        Self {
            card_x,
            card_y,
            card_w,
            card_h,
            row_x,
            row_w,
            row_h,
            row_gap,
            rows_start_y,
            cancel_y,
            cancel_h,
        }
    }
}

pub fn hit_test(x: i32, y: i32) -> Option<PowerModalTarget> {
    let m = PowerModalMetrics::default();
    let fx = x as f32;
    let fy = y as f32;

    // Check if clicked outside modal card -> Close modal
    if fx < m.card_x || fx > m.card_x + m.card_w || fy < m.card_y || fy > m.card_y + m.card_h {
        return Some(PowerModalTarget::Close);
    }

    // Top right close button (x: card_x + card_w - 120 .. card_x + card_w - 40, y: card_y + 40 .. card_y + 120)
    let close_x = m.card_x + m.card_w - 120.0;
    let close_y = m.card_y + 40.0;
    if fx >= close_x && fx <= close_x + 80.0 && fy >= close_y && fy <= close_y + 80.0 {
        return Some(PowerModalTarget::Close);
    }

    // Check rows
    let row_targets = [
        PowerModalTarget::System,
        PowerModalTarget::Recovery,
        PowerModalTarget::Bootloader,
        PowerModalTarget::FastbootD,
        PowerModalTarget::PowerOff,
    ];

    for (i, target) in row_targets.iter().enumerate() {
        let ry = m.rows_start_y + (i as f32) * (m.row_h + m.row_gap);
        if fx >= m.row_x && fx <= m.row_x + m.row_w && fy >= ry && fy <= ry + m.row_h {
            return Some(*target);
        }
    }

    // Check Cancel button
    if fx >= m.row_x && fx <= m.row_x + m.row_w && fy >= m.cancel_y && fy <= m.cancel_y + m.cancel_h {
        return Some(PowerModalTarget::Close);
    }

    None
}

pub fn render_power_modal(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    pressed: Option<PowerModalTarget>,
) {
    let m = PowerModalMetrics::default();

    // 1. Semi-transparent backdrop scrim (Dark Dimming)
    let scrim = Color::from_rgba8(0, 0, 0, 190);
    let mut scrim_paint = tiny_skia::Paint::default();
    scrim_paint.set_color(scrim);
    canvas.pixmap.fill_rect(
        tiny_skia::Rect::from_xywh(0.0, 0.0, 1220.0, 2712.0).unwrap(),
        &scrim_paint,
        tiny_skia::Transform::identity(),
        None,
    );

    // 2. Elevated Modal Sheet
    canvas.draw_rounded_rect(
        m.card_x,
        m.card_y,
        m.card_w,
        m.card_h,
        56.0,
        Some(theme.surface_container_high),
        Some((theme.outline_variant, 2.0)),
    );

    // 3. Modal Header: Icon + Title + Subtitle
    let icon_cx = m.card_x + 96.0;
    let icon_cy = m.card_y + 110.0;
    canvas.draw_circle(icon_cx, icon_cy, 48.0, theme.primary_container);
    draw_icon(&mut canvas.pixmap, M3Icon::Power, icon_cx, icon_cy, 48.0, theme.primary);

    fonts.draw_text(
        &mut canvas.pixmap,
        "Power & Reboot",
        icon_cx + 72.0,
        icon_cy - 6.0,
        42.0,
        theme.on_surface,
    );

    fonts.draw_text(
        &mut canvas.pixmap,
        "Choose target reboot mode or shutdown",
        icon_cx + 72.0,
        icon_cy + 32.0,
        24.0,
        theme.on_surface_variant,
    );

    // Close button (X) in top right
    let close_cx = m.card_x + m.card_w - 80.0;
    let close_cy = m.card_y + 80.0;
    canvas.draw_circle(close_cx, close_cy, 36.0, theme.surface_container_highest);
    draw_icon(&mut canvas.pixmap, M3Icon::Close, close_cx, close_cy, 36.0, theme.on_surface);

    // 4. Action Rows (System, Recovery, Bootloader, FastbootD, PowerOff)
    struct RowInfo<'a> {
        target: PowerModalTarget,
        title: &'a str,
        subtitle: &'a str,
        icon: M3Icon,
        is_danger: bool,
    }

    let rows = [
        RowInfo {
            target: PowerModalTarget::System,
            title: "Reboot to System",
            subtitle: "Normal boot into Android / HyperOS 3",
            icon: M3Icon::Restart,
            is_danger: false,
        },
        RowInfo {
            target: PowerModalTarget::Recovery,
            title: "Reboot Recovery",
            subtitle: "Restart MeowFloprRecovery OS",
            icon: M3Icon::Shield,
            is_danger: false,
        },
        RowInfo {
            target: PowerModalTarget::Bootloader,
            title: "Bootloader / Fastboot",
            subtitle: "MediaTek LittleKernel hardware fastboot",
            icon: M3Icon::Terminal,
            is_danger: false,
        },
        RowInfo {
            target: PowerModalTarget::FastbootD,
            title: "FastbootD Mode",
            subtitle: "Userspace dynamic partition fastboot",
            icon: M3Icon::Settings,
            is_danger: false,
        },
        RowInfo {
            target: PowerModalTarget::PowerOff,
            title: "Power Off",
            subtitle: "Complete hardware power shutdown",
            icon: M3Icon::Power,
            is_danger: true,
        },
    ];

    for (i, row) in rows.iter().enumerate() {
        let ry = m.rows_start_y + (i as f32) * (m.row_h + m.row_gap);
        let is_pressed = pressed == Some(row.target);

        let bg = if is_pressed {
            if row.is_danger {
                theme.error_container
            } else {
                theme.primary_container
            }
        } else {
            theme.surface_container
        };

        let stroke = if is_pressed {
            if row.is_danger {
                Some((theme.error, 3.0))
            } else {
                Some((theme.primary, 3.0))
            }
        } else {
            Some((theme.outline_variant, 1.5))
        };

        canvas.draw_rounded_rect(m.row_x, ry, m.row_w, m.row_h, 32.0, Some(bg), stroke);

        // Row Icon Container
        let row_icon_cx = m.row_x + 72.0;
        let row_icon_cy = ry + (m.row_h / 2.0);
        let icon_bg = if row.is_danger {
            theme.error_container
        } else {
            theme.surface_container_low
        };
        canvas.draw_circle(row_icon_cx, row_icon_cy, 44.0, icon_bg);

        let icon_fg = if row.is_danger {
            theme.error
        } else {
            theme.primary
        };
        draw_icon(&mut canvas.pixmap, row.icon, row_icon_cx, row_icon_cy, 44.0, icon_fg);

        // Titles
        let title_x = row_icon_cx + 64.0;
        let title_y = ry + 68.0;
        let title_fg = if is_pressed && row.is_danger {
            theme.error
        } else {
            theme.on_surface
        };
        fonts.draw_text(&mut canvas.pixmap, row.title, title_x, title_y, 34.0, title_fg);

        let sub_y = ry + 118.0;
        fonts.draw_text(
            &mut canvas.pixmap,
            row.subtitle,
            title_x,
            sub_y,
            22.0,
            theme.on_surface_variant,
        );
    }

    // 5. Cancel Button at Bottom
    let cancel_pressed = pressed == Some(PowerModalTarget::Close);
    let cancel_bg = if cancel_pressed {
        theme.surface_container_high
    } else {
        theme.surface_container
    };
    canvas.draw_pill(
        m.row_x,
        m.cancel_y,
        m.row_w,
        m.cancel_h,
        cancel_bg,
        Some((theme.outline_variant, 1.5)),
    );

    let cancel_label = "Cancel";
    let text_w = fonts.measure_text(cancel_label, 32.0);
    let text_x = m.row_x + (m.row_w - text_w) / 2.0;
    let text_y = m.cancel_y + (m.cancel_h / 2.0) + 10.0;
    fonts.draw_text(&mut canvas.pixmap, cancel_label, text_x, text_y, 32.0, theme.on_surface);
}
