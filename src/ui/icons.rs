use tiny_skia::{Color, Paint, PathBuilder, PixmapMut, Stroke, Transform};

#[derive(Debug, Clone, Copy)]
pub enum M3Icon {
    Install,
    Wipe,
    Backup,
    Decrypt,
    Mount,
    Terminal,
    Settings,
    Power,
    Battery,
    Shield,
    Storage,
    ArrowBack,
    Close,
    Refresh,
    Check,
    Restart,
    Folder,
    File,
    Backspace,
    Lock,
    Warning,
    ChevronRight,
    Usb,
    Brightness,
    Root,
    Copy,
    Restore,
    Advanced,
}

pub fn draw_icon(pixmap: &mut PixmapMut, icon: M3Icon, cx: f32, cy: f32, size: f32, color: Color) {
    let s = size / 2.0;
    let stroke_w = (size * 0.10).max(3.0);

    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(color);
    stroke_paint.anti_alias = true;

    let mut stroke_opts = Stroke::default();
    stroke_opts.width = stroke_w;
    stroke_opts.line_cap = tiny_skia::LineCap::Round;
    stroke_opts.line_join = tiny_skia::LineJoin::Round;

    let mut fill_paint = Paint::default();
    fill_paint.set_color(color);
    fill_paint.anti_alias = true;

    match icon {
        M3Icon::Install => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx, cy + s * 0.20);
            pb.move_to(cx - s * 0.40, cy - s * 0.15);
            pb.line_to(cx, cy + s * 0.25);
            pb.line_to(cx + s * 0.40, cy - s * 0.15);
            pb.move_to(cx - s * 0.55, cy + s * 0.35);
            pb.line_to(cx - s * 0.55, cy + s * 0.65);
            pb.line_to(cx + s * 0.55, cy + s * 0.65);
            pb.line_to(cx + s * 0.55, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Wipe => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.55, cy - s * 0.40);
            pb.line_to(cx + s * 0.55, cy - s * 0.40);
            pb.move_to(cx - s * 0.25, cy - s * 0.40);
            pb.line_to(cx - s * 0.20, cy - s * 0.65);
            pb.line_to(cx + s * 0.20, cy - s * 0.65);
            pb.line_to(cx + s * 0.25, cy - s * 0.40);
            pb.move_to(cx - s * 0.45, cy - s * 0.30);
            pb.line_to(cx - s * 0.35, cy + s * 0.65);
            pb.line_to(cx + s * 0.35, cy + s * 0.65);
            pb.line_to(cx + s * 0.45, cy - s * 0.30);
            pb.move_to(cx, cy - s * 0.15);
            pb.line_to(cx, cy + s * 0.45);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Backup => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.55, cy + s * 0.40);
            pb.cubic_to(cx - s * 0.85, cy + s * 0.35, cx - s * 0.85, cy - s * 0.15, cx - s * 0.50, cy - s * 0.25);
            pb.cubic_to(cx - s * 0.45, cy - s * 0.70, cx + s * 0.25, cy - s * 0.75, cx + s * 0.45, cy - s * 0.35);
            pb.cubic_to(cx + s * 0.85, cy - s * 0.25, cx + s * 0.85, cy + s * 0.25, cx + s * 0.55, cy + s * 0.40);
            pb.move_to(cx, cy + s * 0.45);
            pb.line_to(cx, cy - s * 0.05);
            pb.move_to(cx - s * 0.25, cy + s * 0.15);
            pb.line_to(cx, cy - s * 0.10);
            pb.line_to(cx + s * 0.25, cy + s * 0.15);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Decrypt => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.30, cy - s * 0.05);
            pb.line_to(cx - s * 0.30, cy - s * 0.45);
            pb.cubic_to(cx - s * 0.30, cy - s * 0.80, cx + s * 0.30, cy - s * 0.80, cx + s * 0.30, cy - s * 0.45);
            pb.line_to(cx + s * 0.30, cy - s * 0.25);
            pb.move_to(cx - s * 0.45, cy - s * 0.05);
            pb.line_to(cx + s * 0.45, cy - s * 0.05);
            pb.line_to(cx + s * 0.45, cy + s * 0.60);
            pb.line_to(cx - s * 0.45, cy + s * 0.60);
            pb.close();
            pb.move_to(cx, cy + s * 0.15);
            pb.line_to(cx, cy + s * 0.38);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Mount => {
            let mut pb = PathBuilder::new();
            let h = s * 0.30;
            pb.move_to(cx - s * 0.60, cy - s * 0.40);
            pb.line_to(cx + s * 0.60, cy - s * 0.40);
            pb.line_to(cx + s * 0.60, cy - s * 0.40 + h);
            pb.line_to(cx - s * 0.60, cy - s * 0.40 + h);
            pb.close();
            pb.move_to(cx - s * 0.60, cy - s * 0.05);
            pb.line_to(cx + s * 0.60, cy - s * 0.05);
            pb.line_to(cx + s * 0.60, cy - s * 0.05 + h);
            pb.line_to(cx - s * 0.60, cy - s * 0.05 + h);
            pb.close();
            pb.move_to(cx - s * 0.60, cy + s * 0.30);
            pb.line_to(cx + s * 0.60, cy + s * 0.30);
            pb.line_to(cx + s * 0.60, cy + s * 0.30 + h);
            pb.line_to(cx - s * 0.60, cy + s * 0.30 + h);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Terminal => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.50, cy - s * 0.45);
            pb.line_to(cx - s * 0.05, cy - s * 0.05);
            pb.line_to(cx - s * 0.50, cy + s * 0.35);
            pb.move_to(cx + s * 0.05, cy + s * 0.35);
            pb.line_to(cx + s * 0.55, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Settings => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.55, cy - s * 0.35);
            pb.line_to(cx + s * 0.55, cy - s * 0.35);
            pb.move_to(cx - s * 0.15, cy - s * 0.55);
            pb.line_to(cx - s * 0.15, cy - s * 0.15);

            pb.move_to(cx - s * 0.55, cy + s * 0.05);
            pb.line_to(cx + s * 0.55, cy + s * 0.05);
            pb.move_to(cx + s * 0.20, cy - s * 0.15);
            pb.line_to(cx + s * 0.20, cy + s * 0.25);

            pb.move_to(cx - s * 0.55, cy + s * 0.45);
            pb.line_to(cx + s * 0.55, cy + s * 0.45);
            pb.move_to(cx - s * 0.30, cy + s * 0.25);
            pb.line_to(cx - s * 0.30, cy + s * 0.65);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Power => {
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy + s * 0.05, s * 0.55);
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx, cy + s * 0.05);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Battery => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.65, cy - s * 0.35);
            pb.line_to(cx + s * 0.45, cy - s * 0.35);
            pb.line_to(cx + s * 0.45, cy + s * 0.35);
            pb.line_to(cx - s * 0.65, cy + s * 0.35);
            pb.close();
            pb.move_to(cx + s * 0.45, cy - s * 0.15);
            pb.line_to(cx + s * 0.60, cy - s * 0.15);
            pb.line_to(cx + s * 0.60, cy + s * 0.15);
            pb.line_to(cx + s * 0.45, cy + s * 0.15);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Shield => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx + s * 0.55, cy - s * 0.40);
            pb.line_to(cx + s * 0.55, cy + s * 0.15);
            pb.cubic_to(cx + s * 0.55, cy + s * 0.50, cx, cy + s * 0.75, cx, cy + s * 0.75);
            pb.cubic_to(cx, cy + s * 0.75, cx - s * 0.55, cy + s * 0.50, cx - s * 0.55, cy + s * 0.15);
            pb.line_to(cx - s * 0.55, cy - s * 0.40);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Storage => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.60, cy - s * 0.30);
            pb.line_to(cx + s * 0.60, cy - s * 0.30);
            pb.line_to(cx + s * 0.60, cy + s * 0.30);
            pb.line_to(cx - s * 0.60, cy + s * 0.30);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::ArrowBack => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx + s * 0.40, cy);
            pb.line_to(cx - s * 0.40, cy);
            pb.move_to(cx - s * 0.10, cy - s * 0.40);
            pb.line_to(cx - s * 0.40, cy);
            pb.line_to(cx - s * 0.10, cy + s * 0.40);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Close => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.45, cy - s * 0.45);
            pb.line_to(cx + s * 0.45, cy + s * 0.45);
            pb.move_to(cx + s * 0.45, cy - s * 0.45);
            pb.line_to(cx - s * 0.45, cy + s * 0.45);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Refresh => {
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy, s * 0.55);
            pb.move_to(cx + s * 0.20, cy - s * 0.60);
            pb.line_to(cx + s * 0.55, cy - s * 0.45);
            pb.line_to(cx + s * 0.55, cy - s * 0.80);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Check => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.50, cy);
            pb.line_to(cx - s * 0.10, cy + s * 0.40);
            pb.line_to(cx + s * 0.55, cy - s * 0.40);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Restart => {
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy, s * 0.55);
            pb.move_to(cx, cy - s * 0.55);
            pb.line_to(cx, cy);
            pb.line_to(cx + s * 0.35, cy);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Folder => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.60, cy - s * 0.30);
            pb.line_to(cx - s * 0.20, cy - s * 0.30);
            pb.line_to(cx - s * 0.05, cy - s * 0.10);
            pb.line_to(cx + s * 0.60, cy - s * 0.10);
            pb.line_to(cx + s * 0.60, cy + s * 0.45);
            pb.line_to(cx - s * 0.60, cy + s * 0.45);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::File => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.45, cy - s * 0.55);
            pb.line_to(cx + s * 0.15, cy - s * 0.55);
            pb.line_to(cx + s * 0.45, cy - s * 0.25);
            pb.line_to(cx + s * 0.45, cy + s * 0.55);
            pb.line_to(cx - s * 0.45, cy + s * 0.55);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Backspace => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.65, cy);
            pb.line_to(cx - s * 0.25, cy - s * 0.45);
            pb.line_to(cx + s * 0.60, cy - s * 0.45);
            pb.line_to(cx + s * 0.60, cy + s * 0.45);
            pb.line_to(cx - s * 0.25, cy + s * 0.45);
            pb.close();
            pb.move_to(cx - s * 0.05, cy - s * 0.20);
            pb.line_to(cx + s * 0.35, cy + s * 0.20);
            pb.move_to(cx + s * 0.35, cy - s * 0.20);
            pb.line_to(cx - s * 0.05, cy + s * 0.20);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Lock => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.35, cy - s * 0.10);
            pb.line_to(cx - s * 0.35, cy - s * 0.35);
            pb.cubic_to(cx - s * 0.35, cy - s * 0.70, cx + s * 0.35, cy - s * 0.70, cx + s * 0.35, cy - s * 0.35);
            pb.line_to(cx + s * 0.35, cy - s * 0.10);
            pb.move_to(cx - s * 0.50, cy - s * 0.10);
            pb.line_to(cx + s * 0.50, cy - s * 0.10);
            pb.line_to(cx + s * 0.50, cy + s * 0.60);
            pb.line_to(cx - s * 0.50, cy + s * 0.60);
            pb.close();
            pb.move_to(cx, cy + s * 0.15);
            pb.line_to(cx, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Warning => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx + s * 0.65, cy + s * 0.55);
            pb.line_to(cx - s * 0.65, cy + s * 0.55);
            pb.close();
            pb.move_to(cx, cy - s * 0.25);
            pb.line_to(cx, cy + s * 0.15);
            pb.move_to(cx, cy + s * 0.32);
            pb.line_to(cx, cy + s * 0.38);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::ChevronRight => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.25, cy - s * 0.50);
            pb.line_to(cx + s * 0.25, cy);
            pb.line_to(cx - s * 0.25, cy + s * 0.50);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Usb => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - s * 0.55);
            pb.line_to(cx, cy + s * 0.55);
            pb.move_to(cx - s * 0.40, cy - s * 0.20);
            pb.line_to(cx - s * 0.40, cy + s * 0.05);
            pb.line_to(cx, cy + s * 0.25);
            pb.move_to(cx + s * 0.40, cy - s * 0.20);
            pb.line_to(cx + s * 0.40, cy + s * 0.05);
            pb.line_to(cx, cy + s * 0.25);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Brightness => {
            let mut pb = PathBuilder::new();
            pb.push_circle(cx, cy, s * 0.35);
            pb.move_to(cx, cy - s * 0.70);
            pb.line_to(cx, cy - s * 0.48);
            pb.move_to(cx, cy + s * 0.48);
            pb.line_to(cx, cy + s * 0.70);
            pb.move_to(cx - s * 0.70, cy);
            pb.line_to(cx - s * 0.48, cy);
            pb.move_to(cx + s * 0.48, cy);
            pb.line_to(cx + s * 0.70, cy);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Root => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.22, cy - s * 0.60);
            pb.line_to(cx - s * 0.28, cy + s * 0.60);
            pb.move_to(cx + s * 0.28, cy - s * 0.60);
            pb.line_to(cx + s * 0.22, cy + s * 0.60);
            pb.move_to(cx - s * 0.55, cy - s * 0.20);
            pb.line_to(cx + s * 0.55, cy - s * 0.20);
            pb.move_to(cx - s * 0.55, cy + s * 0.20);
            pb.line_to(cx + s * 0.55, cy + s * 0.20);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Copy => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.20, cy - s * 0.50);
            pb.line_to(cx + s * 0.50, cy - s * 0.50);
            pb.line_to(cx + s * 0.50, cy + s * 0.20);
            pb.move_to(cx - s * 0.50, cy - s * 0.20);
            pb.line_to(cx + s * 0.20, cy - s * 0.20);
            pb.line_to(cx + s * 0.20, cy + s * 0.50);
            pb.line_to(cx - s * 0.50, cy + s * 0.50);
            pb.close();

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Restore => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx + s * 0.50, cy);
            pb.cubic_to(cx + s * 0.50, cy + s * 0.55, cx - s * 0.50, cy + s * 0.55, cx - s * 0.50, cy);
            pb.cubic_to(cx - s * 0.50, cy - s * 0.55, cx + s * 0.30, cy - s * 0.55, cx + s * 0.45, cy - s * 0.20);
            pb.line_to(cx + s * 0.25, cy - s * 0.20);
            pb.move_to(cx + s * 0.45, cy - s * 0.20);
            pb.line_to(cx + s * 0.45, cy - s * 0.45);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Advanced => {
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.55, cy - s * 0.20);
            pb.line_to(cx + s * 0.55, cy - s * 0.20);
            pb.line_to(cx + s * 0.50, cy + s * 0.55);
            pb.line_to(cx - s * 0.50, cy + s * 0.55);
            pb.close();
            pb.move_to(cx - s * 0.25, cy - s * 0.20);
            pb.line_to(cx - s * 0.25, cy - s * 0.50);
            pb.line_to(cx + s * 0.25, cy - s * 0.50);
            pb.line_to(cx + s * 0.25, cy - s * 0.20);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
    }
}
