use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

/// Material 3 Expressive Interactive Swipe-to-Confirm Slider
#[derive(Debug, Clone)]
pub struct SwipeSlider {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: String,
    pub progress: f32, // 0.0 .. 1.0
    pub is_dragging: bool,
    pub confirmed: bool,
    pub accent_color: Option<Color>,
    pub is_danger: bool,
}

impl SwipeSlider {
    pub fn new(x: f32, y: f32, width: f32, height: f32, label: &str) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label: label.to_string(),
            progress: 0.0,
            is_dragging: false,
            confirmed: false,
            accent_color: None,
            is_danger: false,
        }
    }

    pub fn with_danger(mut self, is_danger: bool) -> Self {
        self.is_danger = is_danger;
        self
    }

    pub fn with_accent(mut self, accent: Color) -> Self {
        self.accent_color = Some(accent);
        self
    }

    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.is_dragging = false;
        self.confirmed = false;
    }

    /// Advances spring physics when user releases early.
    /// Returns true if a frame re-render is needed.
    pub fn tick_spring(&mut self) -> bool {
        if !self.is_dragging && !self.confirmed && self.progress > 0.0 {
            // Smooth organic exponential spring damping
            if self.progress < 0.03 {
                self.progress = 0.0;
            } else {
                self.progress = (self.progress * 0.72 - 0.01).max(0.0);
            }
            return true;
        }
        false
    }

    /// Handles touch input. Returns true ONLY when the slider is swiped to >= 85% and confirmed.
    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> bool {
        let fx = tx as f32;
        let fy = ty as f32;

        let padding = 8.0;
        let thumb_r = (self.height - (padding * 2.0)) / 2.0;
        let start_cx = self.x + padding + thumb_r;
        let end_cx = self.x + self.width - padding - thumb_r;
        let travel = end_cx - start_cx;

        // Expanded vertical hit-box (+-100px) for initiating drag
        let in_y_range = fy >= (self.y - 100.0) && fy <= (self.y + self.height + 100.0);

        match state {
            TouchState::Pressed => {
                // If not currently dragging, check if user tapped near thumb or start of track
                if !self.is_dragging {
                    if !in_y_range {
                        return false;
                    }

                    let current_cx = start_cx + travel * self.progress;
                    let near_thumb = (fx - current_cx).abs() <= (thumb_r + 70.0);
                    let near_start = fx >= (self.x - 40.0) && fx <= (start_cx + 70.0);
                    if near_thumb || near_start {
                        self.is_dragging = true;
                    }
                }

                // If currently dragging, update progress smoothly (do not drop if user drifts vertically)
                if self.is_dragging {
                    let target_progress = ((fx - start_cx) / travel).clamp(0.0, 1.0);
                    self.progress = target_progress;

                    // Trigger confirmation threshold: 85% of width (per spec)
                    if self.progress >= 0.85 {
                        self.progress = 1.0;
                        self.confirmed = true;
                        self.is_dragging = false;
                        return true;
                    }
                }
            }
            TouchState::Released => {
                if self.is_dragging && !self.confirmed {
                    self.is_dragging = false;
                    // Will snap back via tick_spring
                }
            }
        }

        false
    }

    pub fn render(&self, canvas: &mut Canvas, theme: &M3Theme, fonts: &FontRenderer) {
        let padding = 8.0;
        let thumb_r = (self.height - (padding * 2.0)) / 2.0;
        let start_cx = self.x + padding + thumb_r;
        let end_cx = self.x + self.width - padding - thumb_r;
        let travel = end_cx - start_cx;
        let current_cx = start_cx + travel * self.progress;

        let accent = if let Some(acc) = self.accent_color {
            acc
        } else if self.is_danger {
            theme.error
        } else {
            theme.primary
        };

        let container_tone = if self.is_danger {
            theme.error_container
        } else {
            theme.surface_container_high
        };

        // 1. Background Track (Capsule Pill)
        canvas.draw_pill(
            self.x,
            self.y,
            self.width,
            self.height,
            container_tone,
            Some((theme.outline_variant, 1.5)),
        );

        // 2. Active Progress Fill
        if self.progress > 0.02 {
            let fill_w = (current_cx - self.x + thumb_r + padding).clamp(self.height, self.width);
            let fill_alpha = if self.is_danger { 180 } else { 120 };
            let fill_color = Color::from_rgba8(
                (accent.red() * 255.0) as u8,
                (accent.green() * 255.0) as u8,
                (accent.blue() * 255.0) as u8,
                fill_alpha,
            );
            canvas.draw_pill(self.x, self.y, fill_w, self.height, fill_color, None);
        }

        // 3. Central Hint Text (fades out as thumb advances)
        let label_alpha = ((1.0 - (self.progress * 1.4)).max(0.0) * 255.0) as u8;
        if label_alpha > 10 {
            let text_color = Color::from_rgba8(
                (theme.on_surface.red() * 255.0) as u8,
                (theme.on_surface.green() * 255.0) as u8,
                (theme.on_surface.blue() * 255.0) as u8,
                label_alpha,
            );
            let text_size = 28.0;
            let tw = fonts.measure_text(&self.label, text_size);
            let text_x = self.x + (self.width - tw) / 2.0;
            let text_y = self.y + (self.height / 2.0) + 9.0;
            fonts.draw_text(&mut canvas.pixmap, &self.label, text_x, text_y, text_size, text_color);
        }

        // 4. Thumb Glow & Drag Elevation (when user holds it)
        let thumb_cy = self.y + (self.height / 2.0);
        if self.is_dragging {
            canvas.draw_circle(
                current_cx,
                thumb_cy,
                thumb_r + 6.0,
                Color::from_rgba8(
                    (accent.red() * 255.0) as u8,
                    (accent.green() * 255.0) as u8,
                    (accent.blue() * 255.0) as u8,
                    80,
                ),
            );
        }

        // 5. Thumb Circle / Squircle
        let thumb_fill = if self.is_danger {
            theme.error
        } else {
            theme.primary
        };
        canvas.draw_circle(current_cx, thumb_cy, thumb_r, thumb_fill);

        // 6. Thumb Chevron Arrows Icon (inside circle)
        let on_thumb_color = if self.is_danger {
            Color::from_rgba8(96, 0, 0, 255)
        } else {
            theme.on_primary
        };

        if self.confirmed {
            draw_icon(&mut canvas.pixmap, M3Icon::Check, current_cx, thumb_cy, thumb_r * 1.1, on_thumb_color);
        } else {
            // Draw double chevron >>>
            draw_double_chevron(&mut canvas.pixmap, current_cx, thumb_cy, thumb_r * 0.9, on_thumb_color);
        }
    }
}

fn draw_double_chevron(pixmap: &mut PixmapMut, cx: f32, cy: f32, size: f32, color: Color) {
    use tiny_skia::{Paint, PathBuilder, Stroke, Transform};

    let s = size / 2.0;
    let stroke_w = (size * 0.14).max(3.0);

    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(color);
    stroke_paint.anti_alias = true;

    let mut stroke_opts = Stroke::default();
    stroke_opts.width = stroke_w;
    stroke_opts.line_cap = tiny_skia::LineCap::Round;
    stroke_opts.line_join = tiny_skia::LineJoin::Round;

    let mut pb = PathBuilder::new();

    // First chevron
    let c1_x = cx - s * 0.35;
    pb.move_to(c1_x - s * 0.25, cy - s * 0.55);
    pb.line_to(c1_x + s * 0.25, cy);
    pb.line_to(c1_x - s * 0.25, cy + s * 0.55);

    // Second chevron
    let c2_x = cx + s * 0.35;
    pb.move_to(c2_x - s * 0.25, cy - s * 0.55);
    pb.line_to(c2_x + s * 0.25, cy);
    pb.line_to(c2_x - s * 0.25, cy + s * 0.55);

    if let Some(p) = pb.finish() {
        pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
    }
}
