use std::time::Instant;
use tiny_skia::Color;

use super::canvas::Canvas;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionConsoleTarget {
    BackToDashboard,
    RebootSystem,
    CopyLog,
    ClearLog,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone)]
pub struct ActionConsoleState {
    pub title: String,
    pub current_step: String,
    pub progress: f32,
    pub logs: Vec<String>,
    pub is_running: bool,
    pub is_finished: bool,
    pub is_success: bool,
    pub scroll_offset: usize,
    pub started_at: Instant,
}

impl Default for ActionConsoleState {
    fn default() -> Self {
        Self {
            title: "Recovery Operation".to_string(),
            current_step: "Ready".to_string(),
            progress: 0.0,
            logs: Vec::new(),
            is_running: false,
            is_finished: false,
            is_success: true,
            scroll_offset: 0,
            started_at: Instant::now(),
        }
    }
}

impl ActionConsoleState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, title: &str) {
        self.title = title.to_string();
        self.current_step = "Initializing operation...".to_string();
        self.progress = 0.05;
        self.logs.clear();
        self.logs.push(format!("[MRR-ACTION] Started: {}", title));
        self.is_running = true;
        self.is_finished = false;
        self.is_success = true;
        self.scroll_offset = 0;
        self.started_at = Instant::now();
    }

    pub fn add_log(&mut self, line: &str) {
        self.logs.push(line.to_string());
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    pub fn set_step(&mut self, step: &str, progress: f32) {
        self.current_step = step.to_string();
        self.progress = progress.clamp(0.0, 1.0);
        self.add_log(step);
    }

    pub fn finish(&mut self, success: bool, final_msg: &str) {
        self.is_running = false;
        self.is_finished = true;
        self.is_success = success;
        self.progress = 1.0;
        self.current_step = final_msg.to_string();
        let elapsed = self.started_at.elapsed().as_secs_f32();
        if success {
            self.add_log(&format!("[MRR-ACTION] SUCCESS in {:.2}s: {}", elapsed, final_msg));
        } else {
            self.add_log(&format!("[MRR-ACTION] FAILED in {:.2}s: {}", elapsed, final_msg));
        }
    }

    pub fn clear(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
        self.current_step = "Console cleared".to_string();
        self.add_log("[MRR-ACTION] Console cleared by user");
    }
}

impl crate::hardware::ProgressCallback for ActionConsoleState {
    fn step(&mut self, desc: &str, pct: f32) {
        self.set_step(desc, pct);
    }
    fn log(&mut self, line: &str) {
        self.add_log(line);
    }
}

impl ActionConsoleState {

    pub fn save_log(&mut self) {
        let dir = std::path::Path::new("/sdcard/MRR/logs");
        let _ = std::fs::create_dir_all(dir);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("recovery_action_{}.log", timestamp));
        let content = self.logs.join("\n");
        match std::fs::write(&path, content) {
            Ok(_) => {
                self.add_log(&format!("[+] Log exported to: {}", path.display()));
            }
            Err(e) => {
                let fb = format!("/tmp/recovery_action_{}.log", timestamp);
                let _ = std::fs::write(&fb, self.logs.join("\n"));
                self.add_log(&format!("[!] /sdcard write failed ({}), saved to {}", e, fb));
            }
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        if self.scroll_offset + amount < self.logs.len() {
            self.scroll_offset += amount;
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        if self.scroll_offset >= amount {
            self.scroll_offset -= amount;
        } else {
            self.scroll_offset = 0;
        }
    }
}

pub fn hit_test(x: i32, y: i32, state: &ActionConsoleState) -> Option<ActionConsoleTarget> {
    let fx = x as f32;
    let fy = y as f32;

    if fy >= 2460.0 && fy <= 2580.0 {
        if fx >= 40.0 && fx <= 340.0 {
            return Some(ActionConsoleTarget::BackToDashboard);
        }

        if fx >= 360.0 && fx <= 590.0 {
            return Some(ActionConsoleTarget::CopyLog);
        }

        if fx >= 610.0 && fx <= 770.0 {
            return Some(ActionConsoleTarget::ClearLog);
        }

        if fx >= 790.0 && fx <= 1180.0 && state.is_finished {
            return Some(ActionConsoleTarget::RebootSystem);
        }
    }

    if fy >= 380.0 && fy <= 1400.0 && fx >= 40.0 && fx <= 1180.0 {
        return Some(ActionConsoleTarget::ScrollUp);
    }
    if fy >= 1400.0 && fy <= 2420.0 && fx >= 40.0 && fx <= 1180.0 {
        return Some(ActionConsoleTarget::ScrollDown);
    }

    None
}

pub fn render_action_console(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &ActionConsoleState,
    pressed: Option<ActionConsoleTarget>,
) {
    canvas.clear(Color::from_rgba8(8, 9, 12, 255));

    let screen_w = 1220.0;
    let margin = 40.0;
    let content_w = screen_w - (margin * 2.0);

    let header_y = 70.0;
    let icon_cx = margin + 44.0;
    let icon_cy = header_y + 45.0;

    let (icon_color, header_bg) = if state.is_running {
        (theme.primary, theme.primary_container)
    } else if state.is_success {
        (Color::from_rgba8(120, 240, 160, 255), Color::from_rgba8(20, 50, 30, 255))
    } else {
        (theme.error, theme.error_container)
    };

    canvas.draw_circle(icon_cx, icon_cy, 44.0, header_bg);
    draw_icon(&mut canvas.pixmap, M3Icon::Terminal, icon_cx, icon_cy, 44.0, icon_color);

    fonts.draw_text(&mut canvas.pixmap, &state.title, icon_cx + 64.0, icon_cy - 4.0, 36.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, &state.current_step, icon_cx + 64.0, icon_cy + 34.0, 22.0, theme.on_surface_variant);

    let pbar_y = 200.0;
    let pbar_h = 28.0;
    let pbar_color = if state.is_success { theme.primary } else { theme.error };

    canvas.draw_progress_bar(
        margin,
        pbar_y,
        content_w,
        pbar_h,
        state.progress,
        theme.surface_container_highest,
        pbar_color,
    );

    let pill_y = 250.0;
    let (pill_bg, pill_fg, pill_txt) = if state.is_running {
        (theme.surface_container_high, theme.primary, format!("EXECUTING... {:.0}%", state.progress * 100.0))
    } else if state.is_success {
        (Color::from_rgba8(20, 60, 35, 255), Color::from_rgba8(120, 240, 160, 255), "COMPLETED SUCCESSFULLY".to_string())
    } else {
        (theme.error_container, theme.error, "OPERATION FAILED".to_string())
    };

    canvas.draw_pill(margin, pill_y, 420.0, 60.0, pill_bg, Some((theme.outline_variant, 1.5)));
    fonts.draw_text(&mut canvas.pixmap, &pill_txt, margin + 30.0, pill_y + 40.0, 22.0, pill_fg);

    let count_str = format!("{} log entries", state.logs.len());
    let cw = fonts.measure_text(&count_str, 20.0);
    fonts.draw_text(&mut canvas.pixmap, &count_str, screen_w - margin - cw, pill_y + 40.0, 20.0, theme.on_surface_variant);

    let log_y = 340.0;
    let log_h = 2080.0;

    canvas.draw_rounded_rect(
        margin,
        log_y,
        content_w,
        log_h,
        28.0,
        Some(Color::from_rgba8(14, 15, 20, 255)),
        Some((theme.outline_variant, 1.0)),
    );

    let line_height = 36.0;
    let font_size = 21.0;
    let max_visible_lines = ((log_h - 60.0) / line_height) as usize;

    let total_lines = state.logs.len();
    let display_start = if total_lines <= max_visible_lines {
        0
    } else {
        let base_start = total_lines - max_visible_lines;
        if state.scroll_offset >= base_start {
            0
        } else {
            base_start - state.scroll_offset
        }
    };

    let visible_slice = &state.logs[display_start..total_lines.min(display_start + max_visible_lines)];

    for (idx, line) in visible_slice.iter().enumerate() {
        let ly = log_y + 50.0 + (idx as f32) * line_height;

        let line_color = if line.contains("SUCCESS") || line.contains("Done") || line.contains("[+]") {
            Color::from_rgba8(140, 240, 160, 255)
        } else if line.contains("ERROR") || line.contains("Failed") || line.contains("[-]") {
            theme.error
        } else if line.contains("WARN") || line.contains("[*]") {
            theme.tertiary
        } else {
            Color::from_rgba8(200, 205, 220, 255)
        };

        let trimmed = if line.len() > 70 {
            format!("{}...", &line[..67])
        } else {
            line.clone()
        };

        fonts.draw_text(&mut canvas.pixmap, &trimmed, margin + 28.0, ly, font_size, line_color);
    }

    let dock_y = 2460.0;
    let dock_h = 100.0;

    let back_w = 300.0;
    let is_back_pressed = pressed == Some(ActionConsoleTarget::BackToDashboard);
    canvas.draw_pill(
        margin,
        dock_y,
        back_w,
        dock_h,
        if is_back_pressed { theme.primary_container } else { theme.surface_container_high },
        Some((theme.outline_variant, 1.5)),
    );
    let back_lbl = "<- Dashboard";
    let bw = fonts.measure_text(back_lbl, 24.0);
    fonts.draw_text(&mut canvas.pixmap, back_lbl, margin + (back_w - bw) / 2.0, dock_y + 58.0, 24.0, theme.on_surface);

    let copy_x = margin + back_w + 16.0;
    let copy_w = 230.0;
    let is_copy_pressed = pressed == Some(ActionConsoleTarget::CopyLog);
    canvas.draw_pill(
        copy_x,
        dock_y,
        copy_w,
        dock_h,
        if is_copy_pressed { theme.tertiary_container } else { theme.surface_container_high },
        Some((theme.outline_variant, 1.5)),
    );
    let copy_lbl = "Копировать";
    let cw = fonts.measure_text(copy_lbl, 22.0);
    fonts.draw_text(&mut canvas.pixmap, copy_lbl, copy_x + (copy_w - cw) / 2.0, dock_y + 58.0, 22.0, theme.on_surface);

    let clear_x = copy_x + copy_w + 16.0;
    let clear_w = 170.0;
    let is_clear_pressed = pressed == Some(ActionConsoleTarget::ClearLog);
    canvas.draw_pill(
        clear_x,
        dock_y,
        clear_w,
        dock_h,
        if is_clear_pressed { theme.error_container } else { theme.surface_container_high },
        Some((theme.outline_variant, 1.5)),
    );
    let clr_lbl = "Очистить";
    let clrw = fonts.measure_text(clr_lbl, 22.0);
    fonts.draw_text(&mut canvas.pixmap, clr_lbl, clear_x + (clear_w - clrw) / 2.0, dock_y + 58.0, 22.0, theme.on_surface);

    if state.is_finished {
        let reboot_x = clear_x + clear_w + 16.0;
        let reboot_w = screen_w - margin - reboot_x;
        let is_reboot_pressed = pressed == Some(ActionConsoleTarget::RebootSystem);
        let reboot_bg = if is_reboot_pressed {
            Color::from_rgba8(40, 140, 80, 255)
        } else {
            Color::from_rgba8(30, 90, 50, 255)
        };
        canvas.draw_pill(
            reboot_x,
            dock_y,
            reboot_w,
            dock_h,
            reboot_bg,
            Some((theme.outline_variant, 1.5)),
        );
        let reb_lbl = "Reboot System ->";
        let rw = fonts.measure_text(reb_lbl, 24.0);
        fonts.draw_text(&mut canvas.pixmap, reb_lbl, reboot_x + (reboot_w - rw) / 2.0, dock_y + 58.0, 24.0, Color::from_rgba8(230, 255, 230, 255));
    }
}
