use tiny_skia::{Path, PathBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M3ShapeType {
    Circle,
    Squircle,
    Cookie4,
    Sunny8,
    Clover4,
    PuffyCloud,
    Clamshell,
    ElongatedArch,
}

pub struct ShapeRenderer;

impl ShapeRenderer {
    pub fn build_shape(shape: M3ShapeType, cx: f32, cy: f32, size: f32) -> Option<Path> {
        let r = size / 2.0;
        let mut pb = PathBuilder::new();

        match shape {
            M3ShapeType::Circle => {
                pb.push_circle(cx, cy, r);
            }
            M3ShapeType::Squircle => {
                let corner_r = r * 0.72;
                let x = cx - r;
                let y = cy - r;
                let w = size;
                let h = size;

                pb.move_to(x + corner_r, y);
                pb.line_to(x + w - corner_r, y);
                pb.cubic_to(x + w, y, x + w, y + corner_r * 0.5, x + w, y + corner_r);
                pb.line_to(x + w, y + h - corner_r);
                pb.cubic_to(x + w, y + h, x + w - corner_r * 0.5, y + h, x + w - corner_r, y + h);
                pb.line_to(x + corner_r, y + h);
                pb.cubic_to(x, y + h, x, y + h - corner_r * 0.5, x, y + h - corner_r);
                pb.line_to(x, y + corner_r);
                pb.cubic_to(x, y, x + corner_r * 0.5, y, x + corner_r, y);
                pb.close();
            }
            M3ShapeType::Cookie4 => {
                let steps = 64;
                for i in 0..=steps {
                    let theta = (i as f32 / steps as f32) * std::f32::consts::PI * 2.0;
                    let rad = r * (0.84 + 0.18 * (4.0 * theta).cos());
                    let px = cx + rad * theta.cos();
                    let py = cy + rad * theta.sin();
                    if i == 0 {
                        pb.move_to(px, py);
                    } else {
                        pb.line_to(px, py);
                    }
                }
                pb.close();
            }
            M3ShapeType::Sunny8 => {
                let steps = 72;
                for i in 0..=steps {
                    let theta = (i as f32 / steps as f32) * std::f32::consts::PI * 2.0;
                    let rad = r * (0.88 + 0.14 * (8.0 * theta).cos());
                    let px = cx + rad * theta.cos();
                    let py = cy + rad * theta.sin();
                    if i == 0 {
                        pb.move_to(px, py);
                    } else {
                        pb.line_to(px, py);
                    }
                }
                pb.close();
            }
            M3ShapeType::Clover4 => {
                let petal_r = r * 0.48;
                let offset = r * 0.44;
                pb.push_circle(cx - offset, cy, petal_r);
                pb.push_circle(cx + offset, cy, petal_r);
                pb.push_circle(cx, cy - offset, petal_r);
                pb.push_circle(cx, cy + offset, petal_r);
                pb.push_circle(cx, cy, offset * 1.1);
            }
            M3ShapeType::PuffyCloud => {
                let steps = 60;
                for i in 0..=steps {
                    let theta = (i as f32 / steps as f32) * std::f32::consts::PI * 2.0;
                    let rad = r * (0.86 + 0.16 * (6.0 * theta).cos());
                    let px = cx + rad * theta.cos();
                    let py = cy + rad * theta.sin();
                    if i == 0 {
                        pb.move_to(px, py);
                    } else {
                        pb.line_to(px, py);
                    }
                }
                pb.close();
            }
            M3ShapeType::Clamshell => {
                let w = size * 0.95;
                let h = size * 0.75;
                let x = cx - w / 2.0;
                let y = cy - h / 2.0;
                let bevel = r * 0.38;

                pb.move_to(x + bevel, y);
                pb.line_to(x + w - bevel, y);
                pb.line_to(x + w, y + bevel);
                pb.line_to(x + w, y + h - bevel);
                pb.line_to(x + w - bevel, y + h);
                pb.line_to(x + bevel, y + h);
                pb.line_to(x, y + h - bevel);
                pb.line_to(x, y + bevel);
                pb.close();
            }
            M3ShapeType::ElongatedArch => {
                let w = size * 0.80;
                let h = size * 1.18;
                let x = cx - w / 2.0;
                let y = cy - h / 2.0;
                let arch_r = w / 2.0;
                let bot_corner = 18.0;

                pb.move_to(x + bot_corner, y + h);
                pb.line_to(x + w - bot_corner, y + h);
                pb.quad_to(x + w, y + h, x + w, y + h - bot_corner);
                pb.line_to(x + w, y + arch_r);
                pb.cubic_to(x + w, y, x, y, x, y + arch_r);
                pb.line_to(x, y + h - bot_corner);
                pb.quad_to(x, y + h, x + bot_corner, y + h);
                pb.close();
            }
        }

        pb.finish()
    }
}
