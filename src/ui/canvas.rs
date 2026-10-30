use tiny_skia::{
    Color, FillRule, Paint, PathBuilder, PixmapMut, Rect, Transform,
};

pub struct Canvas<'a> {
    pub pixmap: PixmapMut<'a>,
}

impl<'a> Canvas<'a> {
    pub fn new(pixmap: PixmapMut<'a>) -> Self {
        Self { pixmap }
    }

    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color);
    }

    /// Draws a rounded rectangle with antialiased corners
    pub fn draw_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill: Option<Color>,
        stroke: Option<(Color, f32)>,
    ) {
        let r = radius.min(w / 2.0).min(h / 2.0);
        let mut pb = PathBuilder::new();

        // Build rounded rect path
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.quad_to(x + w, y, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.quad_to(x + w, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.quad_to(x, y + h, x, y + h - r);
        pb.line_to(x, y + r);
        pb.quad_to(x, y, x + r, y);
        pb.close();

        let path = match pb.finish() {
            Some(p) => p,
            None => return,
        };

        if let Some(fill_color) = fill {
            let mut paint = Paint::default();
            paint.set_color(fill_color);
            paint.anti_alias = true;
            self.pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        if let Some((stroke_color, width)) = stroke {
            let mut paint = Paint::default();
            paint.set_color(stroke_color);
            paint.anti_alias = true;
            let mut stroke_opts = tiny_skia::Stroke::default();
            stroke_opts.width = width;
            self.pixmap.stroke_path(
                &path,
                &paint,
                &stroke_opts,
                Transform::identity(),
                None,
            );
        }
    }

    /// Draws a full Material 3 Pill
    pub fn draw_pill(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Color,
        stroke: Option<(Color, f32)>,
    ) {
        let radius = h / 2.0;
        self.draw_rounded_rect(x, y, w, h, radius, Some(fill), stroke);
    }

    /// Draws a circle
    pub fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, fill: Color) {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(fill);
            paint.anti_alias = true;
            self.pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Draws an Expressive thick rounded progress bar
    pub fn draw_progress_bar(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        progress: f32, // 0.0 .. 1.0
        bg_color: Color,
        fg_color: Color,
    ) {
        let radius = h / 2.0;
        // Background track
        self.draw_rounded_rect(x, y, w, h, radius, Some(bg_color), None);

        // Progress fill
        let fill_w = (w * progress.clamp(0.0, 1.0)).max(h);
        self.draw_rounded_rect(x, y, fill_w, h, radius, Some(fg_color), None);
    }
}
