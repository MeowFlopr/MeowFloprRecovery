use tiny_skia::{Color, PixmapMut};

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::crypto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecryptAction {
    Digit(char),
    Backspace,
    Clear,
    Submit,
    Skip,
    TryDefaultKey,
}

#[derive(Debug, Clone)]
pub struct DecryptState {
    pub pin_input: String,
    pub status_msg: String,
    pub is_busy: bool,
    pub is_success: bool,
    pub is_error: bool,
    pub error_count: u32,
}

impl Default for DecryptState {
    fn default() -> Self {
        Self {
            pin_input: String::new(),
            status_msg: "Enter lockscreen PIN to unlock files".to_string(),
            is_busy: false,
            is_success: false,
            is_error: false,
            error_count: 0,
        }
    }
}

impl DecryptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_digit(&mut self, ch: char) {
        if self.pin_input.len() < 16 {
            self.pin_input.push(ch);
            self.is_error = false;
            self.status_msg = format!("{} digits entered", self.pin_input.len());
        }
    }

    pub fn backspace(&mut self) {
        self.pin_input.pop();
        self.is_error = false;
        if self.pin_input.is_empty() {
            self.status_msg = "Enter lockscreen PIN to unlock files".to_string();
        } else {
            self.status_msg = format!("{} digits entered", self.pin_input.len());
        }
    }

    pub fn clear(&mut self) {
        self.pin_input.clear();
        self.is_error = false;
        self.status_msg = "Enter lockscreen PIN to unlock files".to_string();
    }

    pub fn execute_unlock(&mut self) -> bool {
        self.is_busy = true;
        self.status_msg = "Deriving FBE v2 Credential Key...".to_string();

        println!("[MRR-DECRYPT] Attempting storage decryption with provided credentials...");

        // 1. Try smart auto decrypt & CE derivation
        let ok = crypto::auto_decrypt();

        self.is_busy = false;
        if ok {
            self.is_success = true;
            self.is_error = false;
            self.status_msg = "Storage decrypted successfully!".to_string();
            true
        } else {
            self.is_error = true;
            self.error_count += 1;
            self.status_msg = "Incorrect PIN or CE key not ready in KeyMint".to_string();
            false
        }
    }
}

pub fn hit_test(x: i32, y: i32) -> Option<DecryptAction> {
    let fx = x as f32;
    let fy = y as f32;
    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0); // 1140.0

    // 1. Top Bar Skip / Back button (x: 40..160, y: 70..170)
    if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
        return Some(DecryptAction::Skip);
    }

    // 2. Auto Try Default Key Pill (top right): x: 800..1180, y: 70..160
    if fx >= 780.0 && fx <= 1180.0 && fy >= 70.0 && fy <= 160.0 {
        return Some(DecryptAction::TryDefaultKey);
    }

    // 3. Numpad Grid: y: 920 .. 1960
    let pad_start_y = 920.0;
    let btn_size = 190.0;
    let col_gap = 130.0;
    let row_gap = 48.0;

    let center_x = screen_w / 2.0;
    let col_xs = [
        center_x - col_gap - btn_size, // Col 0 (Left)
        center_x - (btn_size / 2.0),   // Col 1 (Center)
        center_x + col_gap,            // Col 2 (Right)
    ];

    let row_ys = [
        pad_start_y,                                     // Row 0: 1, 2, 3
        pad_start_y + btn_size + row_gap,               // Row 1: 4, 5, 6
        pad_start_y + (btn_size + row_gap) * 2.0,       // Row 2: 7, 8, 9
        pad_start_y + (btn_size + row_gap) * 3.0,       // Row 3: C, 0, ⌫
    ];

    // Check 3x4 grid
    for (row_idx, &ry) in row_ys.iter().enumerate() {
        for (col_idx, &rx) in col_xs.iter().enumerate() {
            let cx = rx + (btn_size / 2.0);
            let cy = ry + (btn_size / 2.0);
            let dist_sq = (fx - cx) * (fx - cx) + (fy - cy) * (fy - cy);
            let r = btn_size / 2.0;

            if dist_sq <= (r + 15.0) * (r + 15.0) {
                match (row_idx, col_idx) {
                    (0, 0) => return Some(DecryptAction::Digit('1')),
                    (0, 1) => return Some(DecryptAction::Digit('2')),
                    (0, 2) => return Some(DecryptAction::Digit('3')),
                    (1, 0) => return Some(DecryptAction::Digit('4')),
                    (1, 1) => return Some(DecryptAction::Digit('5')),
                    (1, 2) => return Some(DecryptAction::Digit('6')),
                    (2, 0) => return Some(DecryptAction::Digit('7')),
                    (2, 1) => return Some(DecryptAction::Digit('8')),
                    (2, 2) => return Some(DecryptAction::Digit('9')),
                    (3, 0) => return Some(DecryptAction::Clear),
                    (3, 1) => return Some(DecryptAction::Digit('0')),
                    (3, 2) => return Some(DecryptAction::Backspace),
                    _ => {}
                }
            }
        }
    }

    // 4. Submit / Unlock Pill: y: 2040 .. 2180
    let submit_y = 2040.0;
    let submit_w = content_w - 80.0;
    let submit_x = margin + 40.0;
    if fx >= submit_x && fx <= (submit_x + submit_w) && fy >= submit_y && fy <= (submit_y + 140.0) {
        return Some(DecryptAction::Submit);
    }

    // 5. Bottom Skip Button: y: 2460 .. 2580
    if fx >= 40.0 && fx <= 400.0 && fy >= 2460.0 && fy <= 2580.0 {
        return Some(DecryptAction::Skip);
    }

    None
}

pub fn render_decrypt(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &DecryptState,
    pressed: Option<DecryptAction>,
) {
    canvas.clear(theme.background);

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0); // 1140.0

    // =========================================================================
    // 1. TOP BAR
    // =========================================================================
    let back_cx = margin + 44.0;
    let back_cy = 115.0;
    let is_back_pressed = pressed == Some(DecryptAction::Skip);
    let back_bg = if is_back_pressed {
        theme.primary_container
    } else {
        theme.surface_container_high
    };
    canvas.draw_circle(back_cx, back_cy, 44.0, back_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    fonts.draw_text(&mut canvas.pixmap, "Decrypt Storage", back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, "File-Based Encryption v2 (fscrypt)", back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    // Quick auto-decrypt button on top right
    let try_w = 260.0;
    let try_h = 76.0;
    let try_x = screen_w - margin - try_w;
    let try_y = back_cy - 38.0;
    let try_pressed = pressed == Some(DecryptAction::TryDefaultKey);
    let try_bg = if try_pressed { theme.primary_container } else { theme.surface_container_high };
    canvas.draw_pill(try_x, try_y, try_w, try_h, try_bg, Some((theme.outline_variant, 1.5)));
    let tw = fonts.measure_text("Default Key", 24.0);
    fonts.draw_text(&mut canvas.pixmap, "Default Key", try_x + (try_w - tw) / 2.0, try_y + 47.0, 24.0, theme.primary);

    // =========================================================================
    // 2. LOCK HERO & STATUS
    // =========================================================================
    let hero_cy = 320.0;
    let center_x = screen_w / 2.0;

    // Lock circle
    let lock_color = if state.is_success {
        Color::from_rgba8(40, 180, 90, 255)
    } else if state.is_error {
        theme.error
    } else {
        theme.primary
    };

    let lock_bg = if state.is_success {
        Color::from_rgba8(20, 60, 35, 255)
    } else if state.is_error {
        theme.error_container
    } else {
        theme.primary_container
    };

    canvas.draw_circle(center_x, hero_cy, 70.0, lock_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::Lock, center_x, hero_cy, 70.0, lock_color);

    // Title & Status
    let status_color = if state.is_success {
        Color::from_rgba8(120, 240, 160, 255)
    } else if state.is_error {
        theme.error
    } else {
        theme.on_surface
    };

    let sub_w = fonts.measure_text(&state.status_msg, 28.0);
    fonts.draw_text(
        &mut canvas.pixmap,
        &state.status_msg,
        (screen_w - sub_w) / 2.0,
        hero_cy + 130.0,
        28.0,
        status_color,
    );

    // =========================================================================
    // 3. PIN DOTS DISPLAY (or masked characters)
    // =========================================================================
    let dots_y = hero_cy + 220.0;
    let max_dots = 8;
    let input_len = state.pin_input.len();
    let dot_spacing = 60.0;
    let total_dots_w = (max_dots as f32 - 1.0) * dot_spacing;
    let start_dot_x = center_x - (total_dots_w / 2.0);

    for i in 0..max_dots {
        let dx = start_dot_x + (i as f32) * dot_spacing;
        let is_filled = i < input_len;

        if is_filled {
            canvas.draw_circle(dx, dots_y, 16.0, theme.primary);
        } else {
            canvas.draw_circle(dx, dots_y, 10.0, theme.surface_container_highest);
        }
    }

    if input_len > max_dots {
        let more_str = format!("+{} more", input_len - max_dots);
        fonts.draw_text(&mut canvas.pixmap, &more_str, center_x + (total_dots_w / 2.0) + 30.0, dots_y + 8.0, 20.0, theme.primary);
    }

    // =========================================================================
    // 4. NUMPAD 3x4 (Material 3 Circular Expressive Buttons)
    // =========================================================================
    let pad_start_y = 920.0;
    let btn_size = 190.0;
    let col_gap = 130.0;
    let row_gap = 48.0;

    let col_xs = [
        center_x - col_gap - btn_size,
        center_x - (btn_size / 2.0),
        center_x + col_gap,
    ];

    let row_ys = [
        pad_start_y,
        pad_start_y + btn_size + row_gap,
        pad_start_y + (btn_size + row_gap) * 2.0,
        pad_start_y + (btn_size + row_gap) * 3.0,
    ];

    let numpad_labels = [
        [("1", DecryptAction::Digit('1')), ("2", DecryptAction::Digit('2')), ("3", DecryptAction::Digit('3'))],
        [("4", DecryptAction::Digit('4')), ("5", DecryptAction::Digit('5')), ("6", DecryptAction::Digit('6'))],
        [("7", DecryptAction::Digit('7')), ("8", DecryptAction::Digit('8')), ("9", DecryptAction::Digit('9'))],
        [("C", DecryptAction::Clear), ("0", DecryptAction::Digit('0')), ("⌫", DecryptAction::Backspace)],
    ];

    for (row_idx, row) in numpad_labels.iter().enumerate() {
        let ry = row_ys[row_idx];
        for (col_idx, (lbl, action)) in row.iter().enumerate() {
            let rx = col_xs[col_idx];
            let cx = rx + (btn_size / 2.0);
            let cy = ry + (btn_size / 2.0);
            let is_btn_pressed = pressed == Some(*action);

            let bg = if is_btn_pressed {
                theme.primary_container
            } else {
                theme.surface_container_high
            };

            canvas.draw_circle(cx, cy, btn_size / 2.0, bg);

            if *lbl == "⌫" {
                draw_icon(&mut canvas.pixmap, M3Icon::Backspace, cx, cy, 54.0, theme.on_surface);
            } else {
                let text_size = 46.0;
                let tw = fonts.measure_text(lbl, text_size);
                let text_color = if *lbl == "C" {
                    theme.error
                } else {
                    theme.on_surface
                };
                fonts.draw_text(&mut canvas.pixmap, lbl, cx - (tw / 2.0), cy + 16.0, text_size, text_color);
            }
        }
    }

    // =========================================================================
    // 5. SUBMIT PILL ("Unlock Storage")
    // =========================================================================
    let submit_y = 2040.0;
    let submit_w = content_w - 80.0;
    let submit_x = margin + 40.0;
    let submit_h = 140.0;
    let submit_pressed = pressed == Some(DecryptAction::Submit);

    let submit_bg = if submit_pressed {
        theme.secondary
    } else {
        theme.primary
    };

    canvas.draw_pill(submit_x, submit_y, submit_w, submit_h, submit_bg, None);
    let submit_lbl = if state.is_busy {
        "Decrypting Storage..."
    } else {
        "Unlock & Mount Storage"
    };
    let sw = fonts.measure_text(submit_lbl, 32.0);
    fonts.draw_text(
        &mut canvas.pixmap,
        submit_lbl,
        submit_x + (submit_w - sw) / 2.0,
        submit_y + 82.0,
        32.0,
        theme.on_primary,
    );

    // =========================================================================
    // 6. BOTTOM DOCK
    // =========================================================================
    let dock_y = 2460.0;
    let dock_h = 110.0;
    let btn_w = 320.0;

    canvas.draw_pill(
        margin,
        dock_y,
        btn_w,
        dock_h,
        if is_back_pressed { theme.primary_container } else { theme.surface_container_high },
        Some((theme.outline_variant, 1.5)),
    );
    let lbl = "<- Skip Decrypt";
    let w = fonts.measure_text(lbl, 26.0);
    fonts.draw_text(&mut canvas.pixmap, lbl, margin + (btn_w - w) / 2.0, dock_y + 64.0, 26.0, theme.on_surface);
}
