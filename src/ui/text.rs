use tiny_skia::{Color, PixmapMut};

static FONT_DATA: &[u8] = include_bytes!("../../assets/font.ttf");

pub struct FontRenderer {
    font: fontdue::Font,
}

impl Default for FontRenderer {
    fn default() -> Self {
        let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
            .expect("Failed to load embedded font");
        Self { font }
    }
}

impl FontRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw_text(
        &self,
        pixmap: &mut PixmapMut,
        text: &str,
        start_x: f32,
        baseline_y: f32,
        size_px: f32,
        color: Color,
    ) -> f32 {
        let mut pen_x = start_x;
        let pixmap_w = pixmap.width() as i32;
        let pixmap_h = pixmap.height() as i32;
        let pixels = pixmap.data_mut();

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size_px);
            let char_x = (pen_x + metrics.xmin as f32) as i32;
            let char_y = (baseline_y - metrics.height as f32 - metrics.ymin as f32) as i32;

            for row in 0..metrics.height {
                let py = char_y + row as i32;
                if py < 0 || py >= pixmap_h {
                    continue;
                }

                for col in 0..metrics.width {
                    let px = char_x + col as i32;
                    if px < 0 || px >= pixmap_w {
                        continue;
                    }

                    let alpha = bitmap[row * metrics.width + col];
                    if alpha == 0 {
                        continue;
                    }

                    let norm_alpha = (alpha as f32 / 255.0) * color.alpha();
                    let inv_alpha = 1.0 - norm_alpha;

                    let idx = ((py * pixmap_w + px) * 4) as usize;
                    if idx + 3 >= pixels.len() {
                        continue;
                    }

                    let dst_r = pixels[idx] as f32;
                    let dst_g = pixels[idx + 1] as f32;
                    let dst_b = pixels[idx + 2] as f32;

                    let src_r = color.red() * 255.0;
                    let src_g = color.green() * 255.0;
                    let src_b = color.blue() * 255.0;

                    pixels[idx] = (src_r * norm_alpha + dst_r * inv_alpha) as u8;
                    pixels[idx + 1] = (src_g * norm_alpha + dst_g * inv_alpha) as u8;
                    pixels[idx + 2] = (src_b * norm_alpha + dst_b * inv_alpha) as u8;
                    pixels[idx + 3] = 255;
                }
            }

            pen_x += metrics.advance_width;
        }

        pen_x
    }

    pub fn measure_text(&self, text: &str, size_px: f32) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, size_px);
            width += metrics.advance_width;
        }
        width
    }
}
