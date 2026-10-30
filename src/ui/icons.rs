use tiny_skia::{Color, FillRule, Paint, PathBuilder, PixmapMut, Stroke, Transform};

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
            // Downward arrow into a tray
            let mut pb = PathBuilder::new();
            // Arrow stem
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx, cy + s * 0.20);
            // Arrow head
            pb.move_to(cx - s * 0.40, cy - s * 0.15);
            pb.line_to(cx, cy + s * 0.25);
            pb.line_to(cx + s * 0.40, cy - s * 0.15);
            // Tray
            pb.move_to(cx - s * 0.55, cy + s * 0.35);
            pb.line_to(cx - s * 0.55, cy + s * 0.65);
            pb.line_to(cx + s * 0.55, cy + s * 0.65);
            pb.line_to(cx + s * 0.55, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Wipe => {
            // Trash can / broom
            let mut pb = PathBuilder::new();
            // Lid
            pb.move_to(cx - s * 0.55, cy - s * 0.40);
            pb.line_to(cx + s * 0.55, cy - s * 0.40);
            pb.move_to(cx - s * 0.25, cy - s * 0.40);
            pb.line_to(cx - s * 0.20, cy - s * 0.65);
            pb.line_to(cx + s * 0.20, cy - s * 0.65);
            pb.line_to(cx + s * 0.25, cy - s * 0.40);
            // Can body
            pb.move_to(cx - s * 0.45, cy - s * 0.30);
            pb.line_to(cx - s * 0.35, cy + s * 0.65);
            pb.line_to(cx + s * 0.35, cy + s * 0.65);
            pb.line_to(cx + s * 0.45, cy - s * 0.30);
            // Inner lines
            pb.move_to(cx, cy - s * 0.15);
            pb.line_to(cx, cy + s * 0.45);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Backup => {
            // Cloud with upward arrow
            let mut pb = PathBuilder::new();
            // Cloud shape
            pb.move_to(cx - s * 0.55, cy + s * 0.40);
            pb.cubic_to(cx - s * 0.85, cy + s * 0.35, cx - s * 0.85, cy - s * 0.15, cx - s * 0.50, cy - s * 0.25);
            pb.cubic_to(cx - s * 0.45, cy - s * 0.70, cx + s * 0.25, cy - s * 0.75, cx + s * 0.45, cy - s * 0.35);
            pb.cubic_to(cx + s * 0.85, cy - s * 0.25, cx + s * 0.85, cy + s * 0.25, cx + s * 0.55, cy + s * 0.40);
            // Arrow up
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
            // Unlocked padlock / Shield
            let mut pb = PathBuilder::new();
            // Shackle (open)
            pb.move_to(cx - s * 0.30, cy - s * 0.05);
            pb.line_to(cx - s * 0.30, cy - s * 0.45);
            pb.cubic_to(cx - s * 0.30, cy - s * 0.80, cx + s * 0.30, cy - s * 0.80, cx + s * 0.30, cy - s * 0.45);
            pb.line_to(cx + s * 0.30, cy - s * 0.25);
            // Lock body
            pb.move_to(cx - s * 0.45, cy - s * 0.05);
            pb.line_to(cx + s * 0.45, cy - s * 0.05);
            pb.line_to(cx + s * 0.45, cy + s * 0.60);
            pb.line_to(cx - s * 0.45, cy + s * 0.60);
            pb.close();
            // Keyhole
            pb.move_to(cx, cy + s * 0.15);
            pb.line_to(cx, cy + s * 0.38);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Mount => {
            // Storage drive / layers
            let mut pb = PathBuilder::new();
            let h = s * 0.30;
            // Top disk
            pb.move_to(cx - s * 0.60, cy - s * 0.40);
            pb.line_to(cx + s * 0.60, cy - s * 0.40);
            pb.line_to(cx + s * 0.60, cy - s * 0.40 + h);
            pb.line_to(cx - s * 0.60, cy - s * 0.40 + h);
            pb.close();
            // Middle disk
            pb.move_to(cx - s * 0.60, cy - s * 0.05);
            pb.line_to(cx + s * 0.60, cy - s * 0.05);
            pb.line_to(cx + s * 0.60, cy - s * 0.05 + h);
            pb.line_to(cx - s * 0.60, cy - s * 0.05 + h);
            pb.close();
            // Bottom disk
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
            // >_ Prompt
            let mut pb = PathBuilder::new();
            // Angle bracket >
            pb.move_to(cx - s * 0.50, cy - s * 0.45);
            pb.line_to(cx - s * 0.05, cy - s * 0.05);
            pb.line_to(cx - s * 0.50, cy + s * 0.35);
            // Underscore cursor _
            pb.move_to(cx + s * 0.05, cy + s * 0.35);
            pb.line_to(cx + s * 0.55, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Settings => {
            // Tune / Equalizer Sliders
            let mut pb = PathBuilder::new();
            // Line 1
            pb.move_to(cx - s * 0.55, cy - s * 0.35);
            pb.line_to(cx + s * 0.55, cy - s * 0.35);
            // Knob 1
            pb.move_to(cx - s * 0.15, cy - s * 0.55);
            pb.line_to(cx - s * 0.15, cy - s * 0.15);

            // Line 2
            pb.move_to(cx - s * 0.55, cy + s * 0.05);
            pb.line_to(cx + s * 0.55, cy + s * 0.05);
            // Knob 2
            pb.move_to(cx + s * 0.20, cy - s * 0.15);
            pb.line_to(cx + s * 0.20, cy + s * 0.25);

            // Line 3
            pb.move_to(cx - s * 0.55, cy + s * 0.45);
            pb.line_to(cx + s * 0.55, cy + s * 0.45);
            // Knob 3
            pb.move_to(cx - s * 0.30, cy + s * 0.25);
            pb.line_to(cx - s * 0.30, cy + s * 0.65);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Power => {
            // Power standby icon
            let mut pb = PathBuilder::new();
            // Circle arc
            pb.push_circle(cx, cy + s * 0.05, s * 0.55);
            // Line at top
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
            // Terminal bump
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
            // Tag with 'X' inside
            let mut pb = PathBuilder::new();
            pb.move_to(cx - s * 0.65, cy);
            pb.line_to(cx - s * 0.25, cy - s * 0.45);
            pb.line_to(cx + s * 0.60, cy - s * 0.45);
            pb.line_to(cx + s * 0.60, cy + s * 0.45);
            pb.line_to(cx - s * 0.25, cy + s * 0.45);
            pb.close();
            // X inside
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
            // Shackle
            pb.move_to(cx - s * 0.35, cy - s * 0.10);
            pb.line_to(cx - s * 0.35, cy - s * 0.35);
            pb.cubic_to(cx - s * 0.35, cy - s * 0.70, cx + s * 0.35, cy - s * 0.70, cx + s * 0.35, cy - s * 0.35);
            pb.line_to(cx + s * 0.35, cy - s * 0.10);
            // Body
            pb.move_to(cx - s * 0.50, cy - s * 0.10);
            pb.line_to(cx + s * 0.50, cy - s * 0.10);
            pb.line_to(cx + s * 0.50, cy + s * 0.60);
            pb.line_to(cx - s * 0.50, cy + s * 0.60);
            pb.close();
            // Keyhole
            pb.move_to(cx, cy + s * 0.15);
            pb.line_to(cx, cy + s * 0.35);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Warning => {
            // Triangle with exclamation mark
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - s * 0.65);
            pb.line_to(cx + s * 0.65, cy + s * 0.55);
            pb.line_to(cx - s * 0.65, cy + s * 0.55);
            pb.close();
            // Exclamation
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
            // Center line
            pb.move_to(cx, cy - s * 0.55);
            pb.line_to(cx, cy + s * 0.55);
            // Left branch
            pb.move_to(cx - s * 0.40, cy - s * 0.20);
            pb.line_to(cx - s * 0.40, cy + s * 0.05);
            pb.line_to(cx, cy + s * 0.25);
            // Right branch
            pb.move_to(cx + s * 0.40, cy - s * 0.20);
            pb.line_to(cx + s * 0.40, cy + s * 0.05);
            pb.line_to(cx, cy + s * 0.25);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Brightness => {
            let mut pb = PathBuilder::new();
            // Center sun circle
            pb.push_circle(cx, cy, s * 0.35);
            // 4 Rays
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
            // # symbol
            let mut pb = PathBuilder::new();
            // Vertical lines
            pb.move_to(cx - s * 0.22, cy - s * 0.60);
            pb.line_to(cx - s * 0.28, cy + s * 0.60);
            pb.move_to(cx + s * 0.28, cy - s * 0.60);
            pb.line_to(cx + s * 0.22, cy + s * 0.60);
            // Horizontal lines
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
            // Back sheet
            pb.move_to(cx - s * 0.20, cy - s * 0.50);
            pb.line_to(cx + s * 0.50, cy - s * 0.50);
            pb.line_to(cx + s * 0.50, cy + s * 0.20);
            // Front sheet
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
            // Circular arrow
            pb.move_to(cx + s * 0.50, cy);
            pb.cubic_to(cx + s * 0.50, cy + s * 0.55, cx - s * 0.50, cy + s * 0.55, cx - s * 0.50, cy);
            pb.cubic_to(cx - s * 0.50, cy - s * 0.55, cx + s * 0.30, cy - s * 0.55, cx + s * 0.45, cy - s * 0.20);
            // Arrowhead
            pb.line_to(cx + s * 0.25, cy - s * 0.20);
            pb.move_to(cx + s * 0.45, cy - s * 0.20);
            pb.line_to(cx + s * 0.45, cy - s * 0.45);

            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &stroke_paint, &stroke_opts, Transform::identity(), None);
            }
        }
        M3Icon::Advanced => {
            // Wrench / screwdriver crossed or toolbox
            let mut pb = PathBuilder::new();
            // Box outline
            pb.move_to(cx - s * 0.55, cy - s * 0.20);
            pb.line_to(cx + s * 0.55, cy - s * 0.20);
            pb.line_to(cx + s * 0.50, cy + s * 0.55);
            pb.line_to(cx - s * 0.50, cy + s * 0.55);
            pb.close();
            // Handle
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
