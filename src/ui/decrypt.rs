use std::fs;
use std::path::Path;
use std::process::Command;
use tiny_skia::Color;

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
    InstallCompanion,
    RebootSystem,
    ToggleManualInput,
}

#[derive(Debug, Clone)]
pub struct DecryptState {
    pub pin_input: String,
    pub status_msg: String,
    pub is_busy: bool,
    pub is_success: bool,
    pub is_error: bool,
    pub error_count: u32,
    pub show_manual_pin: bool,
    pub companion_installed: bool,
}

impl Default for DecryptState {
    fn default() -> Self {
        Self {
            pin_input: String::new(),
            status_msg: "TEE RoT Active. Sync via Companion or enter PIN.".to_string(),
            is_busy: false,
            is_success: false,
            is_error: false,
            error_count: 0,
            show_manual_pin: false,
            companion_installed: false,
        }
    }
}

impl DecryptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_digit(&mut self, ch: char) {
        if self.pin_input.len() < 128 {
            self.pin_input.push(ch);
            self.is_error = false;
            self.status_msg = format!("{} digits entered", self.pin_input.len());
        }
    }

    pub fn backspace(&mut self) {
        self.pin_input.pop();
        self.is_error = false;
        if self.pin_input.is_empty() {
            self.status_msg = "Enter lockscreen PIN or 128-hex key".to_string();
        } else {
            self.status_msg = format!("{} digits entered", self.pin_input.len());
        }
    }

    pub fn clear(&mut self) {
        self.pin_input.clear();
        self.is_error = false;
        self.status_msg = "Enter lockscreen PIN or 128-hex key".to_string();
    }

    pub fn install_companion(&mut self) -> bool {
        self.is_busy = true;
        crypto::keys::ensure_persist_mounted();
        let persist_mrr = Path::new("/mnt/vendor/persist/mrr");
        let _ = fs::create_dir_all(persist_mrr);

        let zip_bytes = include_bytes!("../../assets/mrr-companion.zip");
        let apk_bytes = include_bytes!("../../assets/MrRCompanion.apk");
        let shim_bytes = include_bytes!("../../assets/vold_retrieve_shim.so");

        let _ = fs::write(persist_mrr.join("mrr-companion.zip"), zip_bytes);
        let _ = fs::write(persist_mrr.join("MrRCompanion.apk"), apk_bytes);
        let _ = fs::write(persist_mrr.join("vold_retrieve_shim.so"), shim_bytes);

        let service_sh = r#"#!/system/bin/sh
until [ "$(getprop sys.boot_completed)" = "1" ]; do
    sleep 3
done
SHIM=/mnt/vendor/persist/mrr/vold_retrieve_shim.so
if [ ! -f "$SHIM" ]; then
    SHIM=/data/local/tmp/vold_retrieve_shim.so
fi
if [ -f "$SHIM" ]; then
    RAW_KEYS=$(LD_PRELOAD="$SHIM" /system/bin/vold 2>&1 | grep 'RAW KEY' | awk '{print $NF}')
    DM_KEY=$(echo "$RAW_KEYS" | head -n 1)
    if [ -n "$DM_KEY" ] && [ ${#DM_KEY} -eq 128 ]; then
        mkdir -p /mnt/vendor/persist/mrr
        echo -n "$DM_KEY" > /mnt/vendor/persist/mrr/userdata.key
        chmod 600 /mnt/vendor/persist/mrr/userdata.key
    fi
fi
"#;
        let _ = fs::write(persist_mrr.join("service.sh"), service_sh);

        if Path::new("/data/adb").exists() {
            let _ = fs::create_dir_all("/data/adb/service.d");
            let _ = fs::write("/data/adb/service.d/mrr_sync.sh", service_sh);
            let _ = Command::new("chmod").args(["755", "/data/adb/service.d/mrr_sync.sh"]).status();
        }

        if Path::new("/sdcard").exists() {
            let _ = fs::write("/sdcard/MrRCompanion.apk", apk_bytes);
            let _ = fs::write("/sdcard/mrr-companion.zip", zip_bytes);
        }

        self.companion_installed = true;
        self.is_busy = false;
        self.status_msg = "Companion exported to /mnt/vendor/persist/mrr".to_string();
        true
    }

    pub fn execute_unlock(&mut self) -> bool {
        self.is_busy = true;
        self.status_msg = "Deriving FBE v2 Credential Key...".to_string();

        let ok = if !self.pin_input.is_empty() {
            if self.pin_input.len() == 128 && self.pin_input.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(raw_key) = crypto::keys::parse_hex_key(&self.pin_input) {
                    let _ = crypto::fscrypt::inject_master_key_to_kernel(&raw_key);
                }
            }
            crypto::auto_decrypt()
        } else {
            crypto::auto_decrypt()
        };

        self.is_busy = false;
        if ok {
            self.is_success = true;
            self.is_error = false;
            self.status_msg = "Storage decrypted successfully!".to_string();
            true
        } else {
            self.is_error = true;
            self.error_count += 1;
            self.status_msg = "Key derivation failed. Sync key from Android.".to_string();
            false
        }
    }
}

pub fn hit_test(x: i32, y: i32, show_manual: bool) -> Option<DecryptAction> {
    let fx = x as f32;
    let fy = y as f32;
    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

    if fx >= 40.0 && fx <= 160.0 && fy >= 70.0 && fy <= 170.0 {
        return Some(DecryptAction::Skip);
    }

    if fx >= 840.0 && fx <= 1180.0 && fy >= 70.0 && fy <= 170.0 {
        return Some(DecryptAction::TryDefaultKey);
    }

    if fx >= 40.0 && fx <= 1180.0 && fy >= 2320.0 && fy <= 2470.0 {
        return Some(DecryptAction::Skip);
    }

    if !show_manual {
        if fx >= 40.0 && fx <= 1180.0 && fy >= 960.0 && fy <= 1100.0 {
            return Some(DecryptAction::InstallCompanion);
        }

        if fx >= 40.0 && fx <= 1180.0 && fy >= 1130.0 && fy <= 1270.0 {
            return Some(DecryptAction::RebootSystem);
        }

        if fx >= 40.0 && fx <= 1180.0 && fy >= 1300.0 && fy <= 1420.0 {
            return Some(DecryptAction::ToggleManualInput);
        }

        return None;
    }

    if fx >= 40.0 && fx <= 1180.0 && fy >= 210.0 && fy <= 310.0 {
        return Some(DecryptAction::ToggleManualInput);
    }

    let pad_start_y = 960.0;
    let btn_size = 190.0;
    let col_gap = 130.0;
    let row_gap = 48.0;

    let center_x = screen_w / 2.0;
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

    let submit_y = 2060.0;
    let submit_w = content_w - 80.0;
    let submit_x = margin + 40.0;
    if fx >= submit_x && fx <= (submit_x + submit_w) && fy >= submit_y && fy <= (submit_y + 140.0) {
        return Some(DecryptAction::Submit);
    }

    None
}

pub fn render_decrypt(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &DecryptState,
    pressed: Option<DecryptAction>,
    is_decrypted: bool,
) {
    canvas.clear(theme.background);

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

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

    fonts.draw_text(&mut canvas.pixmap, "Storage Decryption", back_cx + 64.0, back_cy - 4.0, 38.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, "File-Based Encryption v2 (MT6897)", back_cx + 64.0, back_cy + 34.0, 22.0, theme.on_surface_variant);

    let rescan_w = 260.0;
    let rescan_h = 76.0;
    let rescan_x = screen_w - margin - rescan_w;
    let rescan_y = back_cy - 38.0;
    let rescan_pressed = pressed == Some(DecryptAction::TryDefaultKey);
    let rescan_bg = if rescan_pressed { theme.primary_container } else { theme.surface_container_high };
    canvas.draw_pill(rescan_x, rescan_y, rescan_w, rescan_h, rescan_bg, Some((theme.outline_variant, 1.5)));
    let tw_rescan = fonts.measure_text("Rescan Keys", 24.0);
    fonts.draw_text(&mut canvas.pixmap, "Rescan Keys", rescan_x + (rescan_w - tw_rescan) / 2.0, rescan_y + 47.0, 24.0, theme.primary);

    if state.show_manual_pin {
        let guide_y = 210.0;
        let guide_h = 80.0;
        let guide_pressed = pressed == Some(DecryptAction::ToggleManualInput);
        let guide_bg = if guide_pressed { theme.primary_container } else { theme.surface_container };
        canvas.draw_pill(margin, guide_y, content_w, guide_h, guide_bg, Some((theme.outline_variant, 1.5)));
        let lbl_g = "<- Back to Companion Instructions";
        let w_g = fonts.measure_text(lbl_g, 24.0);
        fonts.draw_text(&mut canvas.pixmap, lbl_g, margin + (content_w - w_g) / 2.0, guide_y + 50.0, 24.0, theme.primary);

        let hero_cy = 380.0;
        let center_x = screen_w / 2.0;
        let lock_color = if is_decrypted || state.is_success {
            Color::from_rgba8(40, 180, 90, 255)
        } else if state.is_error {
            theme.error
        } else {
            theme.primary
        };
        let lock_bg = if is_decrypted || state.is_success {
            Color::from_rgba8(20, 60, 35, 255)
        } else if state.is_error {
            theme.error_container
        } else {
            theme.primary_container
        };
        canvas.draw_circle(center_x, hero_cy, 60.0, lock_bg);
        draw_icon(&mut canvas.pixmap, M3Icon::Lock, center_x, hero_cy, 60.0, lock_color);

        let sub_w = fonts.measure_text(&state.status_msg, 26.0);
        fonts.draw_text(
            &mut canvas.pixmap,
            &state.status_msg,
            (screen_w - sub_w) / 2.0,
            hero_cy + 100.0,
            26.0,
            lock_color,
        );

        let dots_y = hero_cy + 170.0;
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
            let more_str = format!("+{} digits", input_len - max_dots);
            fonts.draw_text(&mut canvas.pixmap, &more_str, center_x + (total_dots_w / 2.0) + 30.0, dots_y + 8.0, 20.0, theme.primary);
        }

        let pad_start_y = 960.0;
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
            [("C", DecryptAction::Clear), ("0", DecryptAction::Digit('0')), ("<", DecryptAction::Backspace)],
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

                if *lbl == "<" {
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

        let submit_y = 2060.0;
        let submit_w = content_w - 80.0;
        let submit_x = margin + 40.0;
        let submit_h = 130.0;
        let submit_pressed = pressed == Some(DecryptAction::Submit);
        let submit_bg = if submit_pressed { theme.secondary } else { theme.primary };
        canvas.draw_pill(submit_x, submit_y, submit_w, submit_h, submit_bg, None);
        let submit_lbl = if state.is_busy {
            "Decrypting Storage..."
        } else {
            "Unlock & Mount Storage"
        };
        let sw = fonts.measure_text(submit_lbl, 30.0);
        fonts.draw_text(&mut canvas.pixmap, submit_lbl, submit_x + (submit_w - sw) / 2.0, submit_y + 76.0, 30.0, theme.on_primary);
    } else {
        let center_x = screen_w / 2.0;
        let hero_cy = 280.0;

        let status_color = if is_decrypted || state.is_success {
            Color::from_rgba8(60, 210, 110, 255)
        } else {
            Color::from_rgba8(245, 140, 50, 255)
        };

        let badge_bg = if is_decrypted || state.is_success {
            Color::from_rgba8(20, 60, 35, 255)
        } else {
            Color::from_rgba8(65, 35, 15, 255)
        };

        canvas.draw_circle(center_x, hero_cy, 60.0, badge_bg);
        if is_decrypted || state.is_success {
            draw_icon(&mut canvas.pixmap, M3Icon::Check, center_x, hero_cy, 56.0, status_color);
        } else {
            draw_icon(&mut canvas.pixmap, M3Icon::Lock, center_x, hero_cy, 56.0, status_color);
        }

        let status_title = if is_decrypted || state.is_success {
            "STORAGE DECRYPTED (ACTIVE)"
        } else {
            "HARDWARE ENCRYPTION LOCKED"
        };
        let tw_title = fonts.measure_text(status_title, 32.0);
        fonts.draw_text(&mut canvas.pixmap, status_title, (screen_w - tw_title) / 2.0, hero_cy + 96.0, 32.0, status_color);

        let sub_text = if is_decrypted || state.is_success {
            "/sdcard is readable and mounted via hardware inlinecrypt."
        } else {
            "MediaTek KeyMint Root-of-Trust (RoT) protects hardware keys."
        };
        let tw_sub = fonts.measure_text(sub_text, 22.0);
        fonts.draw_text(&mut canvas.pixmap, sub_text, (screen_w - tw_sub) / 2.0, hero_cy + 138.0, 22.0, theme.on_surface_variant);

        let card_y = 480.0;
        let card_h = 420.0;
        canvas.draw_rounded_rect(margin, card_y, content_w, card_h, 28.0, Some(theme.surface_container), Some((theme.outline_variant, 1.5)));

        draw_icon(&mut canvas.pixmap, M3Icon::Shield, margin + 44.0, card_y + 44.0, 36.0, theme.primary);
        fonts.draw_text(&mut canvas.pixmap, "Hardware Key Sealing on Xiaomi 14T (MT6897)", margin + 76.0, card_y + 52.0, 26.0, theme.on_surface);

        let info_lines = [
            "MediaTek MT6897 (Dimensity 8300 Ultra) binds FBE v2 metadata keys",
            "to the stock boot image hash inside the hardware MITEE KeyMint.",
            "Booting custom recovery changes the boot hash, so direct lockscreen",
            "PIN decryption is physically refused by the secure processor.",
            "",
            "To decrypt /data, hardware keys must be synchronized once from",
            "the booted Android system using the MrR Companion app or module.",
        ];

        let mut line_y = card_y + 110.0;
        for line in &info_lines {
            if !line.is_empty() {
                fonts.draw_text(&mut canvas.pixmap, line, margin + 36.0, line_y, 21.0, theme.on_surface_variant);
            }
            line_y += 38.0;
        }

        let act1_y = 940.0;
        let act1_h = 140.0;
        let act1_pressed = pressed == Some(DecryptAction::InstallCompanion);
        let act1_bg = if act1_pressed { theme.primary_container } else { theme.surface_container_high };
        canvas.draw_pill(margin, act1_y, content_w, act1_h, act1_bg, Some((theme.outline_variant, 1.5)));
        draw_icon(&mut canvas.pixmap, M3Icon::Install, margin + 60.0, act1_y + 70.0, 52.0, theme.primary);
        fonts.draw_text(&mut canvas.pixmap, "Export MrR Companion (APK & Module)", margin + 110.0, act1_y + 54.0, 28.0, theme.on_surface);
        let act1_sub = if state.companion_installed {
            "Exported to /mnt/vendor/persist/mrr! Boot to system to sync."
        } else {
            "Saves companion APK and Magisk module to persist partition"
        };
        let act1_sub_color = if state.companion_installed { Color::from_rgba8(60, 210, 110, 255) } else { theme.on_surface_variant };
        fonts.draw_text(&mut canvas.pixmap, act1_sub, margin + 110.0, act1_y + 96.0, 21.0, act1_sub_color);

        let act2_y = 1120.0;
        let act2_h = 140.0;
        let act2_pressed = pressed == Some(DecryptAction::RebootSystem);
        let act2_bg = if act2_pressed { theme.primary_container } else { theme.surface_container_high };
        canvas.draw_pill(margin, act2_y, content_w, act2_h, act2_bg, Some((theme.outline_variant, 1.5)));
        draw_icon(&mut canvas.pixmap, M3Icon::Restart, margin + 60.0, act2_y + 70.0, 52.0, theme.primary);
        fonts.draw_text(&mut canvas.pixmap, "Reboot to System", margin + 110.0, act2_y + 54.0, 28.0, theme.on_surface);
        fonts.draw_text(&mut canvas.pixmap, "Boot Android HyperOS to let MrR Companion extract keys", margin + 110.0, act2_y + 96.0, 21.0, theme.on_surface_variant);

        let act3_y = 1300.0;
        let act3_h = 110.0;
        let act3_pressed = pressed == Some(DecryptAction::ToggleManualInput);
        let act3_bg = if act3_pressed { theme.primary_container } else { theme.surface_container };
        canvas.draw_pill(margin, act3_y, content_w, act3_h, act3_bg, Some((theme.outline_variant, 1.5)));
        let lbl_manual = "Manual PIN or 128-Hex Key Input ->";
        let tw_man = fonts.measure_text(lbl_manual, 24.0);
        fonts.draw_text(&mut canvas.pixmap, lbl_manual, margin + (content_w - tw_man) / 2.0, act3_y + 64.0, 24.0, theme.primary);
    }

    let cancel_y = 2320.0;
    let cancel_h = 150.0;
    let cancel_pressed = pressed == Some(DecryptAction::Skip);
    let cancel_bg = if cancel_pressed {
        theme.primary_container
    } else {
        theme.surface_container_highest
    };
    canvas.draw_pill(margin, cancel_y, content_w, cancel_h, cancel_bg, Some((theme.outline_variant, 2.0)));
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, margin + 64.0, cancel_y + 75.0, 48.0, theme.on_surface);

    let cancel_lbl = "Cancel / Skip (Continue to Menu)";
    fonts.draw_text(&mut canvas.pixmap, cancel_lbl, margin + 120.0, cancel_y + 64.0, 32.0, theme.on_surface);

    let cancel_sub = "OTG Storage, Sideload, Fastboot and ROM flashing remain active";
    fonts.draw_text(&mut canvas.pixmap, cancel_sub, margin + 120.0, cancel_y + 108.0, 21.0, theme.on_surface_variant);
}
