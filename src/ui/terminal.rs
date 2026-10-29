use std::process::Command;
use tiny_skia::Color;

use super::canvas::Canvas;
use super::i18n::Language;
use super::icons::{draw_icon, M3Icon};
use super::text::FontRenderer;
use super::theme::M3Theme;
use crate::platform::TouchState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalAction {
    Back,
    Clear,
    ScrollUp,
    ScrollDown,
    ScrollToBottom,
    ExecuteQuick(&'static str),
    KeyChar(char),
    KeyBackspace,
    KeyEnter,
    KeySpace,
    ToggleShift,
    ToggleSymbols,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardMode {
    AlphaLower,
    AlphaUpper,
    Symbols,
}

pub struct TerminalState {
    pub lines: Vec<String>,
    pub input_buffer: String,
    pub scroll_offset: usize,
    pub max_lines: usize,
    pub kb_mode: KeyboardMode,
    pub last_touch_y: Option<f32>,
    pub is_dragging_scroll: bool,
}

const DEFAULT_BANNER: &str = "\
=====================================================
 MeowFloprRecovery Interactive UNIX Shell (degas)
 Linux 6.1 aarch64 • Root UID 0 (MT6897)
 Type commands or use quick chips below:
=====================================================";

impl Default for TerminalState {
    fn default() -> Self {
        let mut lines = Vec::new();
        for l in DEFAULT_BANNER.lines() {
            lines.push(l.to_string());
        }

        Self {
            lines,
            input_buffer: String::new(),
            scroll_offset: 0,
            max_lines: 500,
            kb_mode: KeyboardMode::AlphaLower,
            last_touch_y: None,
            is_dragging_scroll: false,
        }
    }
}

pub const TERM_MARGIN: f32 = 30.0;
pub const TERM_WIDTH: f32 = 1220.0;
pub const CONTENT_W: f32 = TERM_WIDTH - (TERM_MARGIN * 2.0);

pub const CONSOLE_TOP: f32 = 170.0;
pub const CONSOLE_BOTTOM: f32 = 1560.0;
pub const CONSOLE_H: f32 = CONSOLE_BOTTOM - CONSOLE_TOP;

pub const CHIPS_Y: f32 = 1575.0;
pub const CHIPS_H: f32 = 70.0;

pub const INPUT_Y: f32 = 1655.0;
pub const INPUT_H: f32 = 80.0;

pub const KB_Y: f32 = 1750.0;
pub const KB_ROW_H: f32 = 88.0;
pub const KB_GAP_Y: f32 = 10.0;

pub const TERM_DOCK_Y: f32 = 2540.0;
pub const TERM_DOCK_H: f32 = 110.0;

impl TerminalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(&mut self, cmd: &str) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        self.lines.push(format!("degas:/ # {}", trimmed));

        if trimmed == "clear" {
            self.lines.clear();
            self.scroll_offset = 0;
            self.input_buffer.clear();
            return;
        }

        let shell = if std::path::Path::new("/system/bin/sh").exists() {
            "/system/bin/sh"
        } else if std::path::Path::new("/bin/sh").exists() {
            "/bin/sh"
        } else {
            "sh"
        };

        let mut child = match Command::new(shell)
            .arg("-c")
            .arg(trimmed)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                self.lines.push(format!("[EXEC ERROR] {}", e));
                self.input_buffer.clear();
                return;
            }
        };

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);
        let mut finished = false;

        while start.elapsed() < timeout {
            match child.try_wait() {
                Ok(Some(status)) => {
                    finished = true;
                    if let Some(mut stdout) = child.stdout.take() {
                        let mut buf = String::new();
                        use std::io::Read;
                        let _ = stdout.read_to_string(&mut buf);
                        for line in buf.lines() {
                            self.lines.push(line.to_string());
                        }
                    }
                    if let Some(mut stderr) = child.stderr.take() {
                        let mut buf = String::new();
                        use std::io::Read;
                        let _ = stderr.read_to_string(&mut buf);
                        for line in buf.lines() {
                            self.lines.push(format!("[STDERR] {}", line));
                        }
                    }
                    if !status.success() {
                        if let Some(code) = status.code() {
                            self.lines.push(format!("[Exit code: {}]", code));
                        }
                    }
                    break;
                }
                Ok(None) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    self.lines.push(format!("[WAIT ERROR] {}", e));
                    break;
                }
            }
        }

        if !finished {
            let _ = child.kill();
            let _ = child.wait();
            self.lines.push("[TIMEOUT] Command execution timed out (5s limit)".to_string());
        }

        if self.lines.len() > self.max_lines {
            let start = self.lines.len() - self.max_lines;
            self.lines = self.lines[start..].to_vec();
        }

        self.scroll_offset = 0;
        self.input_buffer.clear();
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let total = self.lines.len();
        if self.scroll_offset + amount < total {
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

    pub fn handle_touch(&mut self, tx: i32, ty: i32, state: TouchState) -> Option<TerminalAction> {
        let fx = tx as f32;
        let fy = ty as f32;

        if state == TouchState::Released {
            if fx >= 30.0 && fx <= 150.0 && fy >= 50.0 && fy <= 150.0 {
                return Some(TerminalAction::Back);
            }
            if fx >= 1040.0 && fx <= 1190.0 && fy >= 50.0 && fy <= 150.0 {
                self.lines.clear();
                self.scroll_offset = 0;
                return Some(TerminalAction::Clear);
            }
        }

        if fy >= CONSOLE_TOP && fy <= CONSOLE_BOTTOM && fx >= TERM_MARGIN && fx <= TERM_MARGIN + CONTENT_W {
            match state {
                TouchState::Pressed => {
                    if let Some(last_y) = self.last_touch_y {
                        let delta = fy - last_y;
                        if delta.abs() > 15.0 {
                            let lines_delta = (delta.abs() / 24.0).round() as usize;
                            if delta > 0.0 {
                                self.scroll_up(lines_delta.max(1));
                            } else {
                                self.scroll_down(lines_delta.max(1));
                            }
                            self.last_touch_y = Some(fy);
                            return None;
                        }
                    } else {
                        self.last_touch_y = Some(fy);
                        self.is_dragging_scroll = true;
                        return None;
                    }
                }
                TouchState::Released => {
                    self.is_dragging_scroll = false;
                    self.last_touch_y = None;
                    return None;
                }
            }
        } else if state == TouchState::Released {
            self.is_dragging_scroll = false;
            self.last_touch_y = None;
        }

        if state != TouchState::Released {
            return None;
        }

        if fy >= CHIPS_Y && fy <= CHIPS_Y + CHIPS_H {
            let chips = [
                ("ls -la", "ls -la"),
                ("df -h", "df -h"),
                ("mount", "mount"),
                ("ps -ef", "ps -ef"),
                ("dmesg", "dmesg | tail -n 25"),
                ("reboot", "reboot"),
            ];

            let chip_gap = 10.0;
            let chip_w = (CONTENT_W - (chip_gap * (chips.len() as f32 - 1.0))) / chips.len() as f32;

            for (i, (_, cmd)) in chips.iter().enumerate() {
                let cx = TERM_MARGIN + (i as f32) * (chip_w + chip_gap);
                if fx >= cx && fx <= cx + chip_w {
                    self.execute(cmd);
                    return Some(TerminalAction::ExecuteQuick(cmd));
                }
            }
        }

        if fy >= INPUT_Y && fy <= INPUT_Y + INPUT_H {
            if fx >= TERM_WIDTH - TERM_MARGIN - 140.0 && fx <= TERM_WIDTH - TERM_MARGIN {
                let cmd = self.input_buffer.clone();
                self.execute(&cmd);
                return Some(TerminalAction::KeyEnter);
            }
        }

        if fy >= KB_Y && fy <= KB_Y + (KB_ROW_H + KB_GAP_Y) * 5.0 {
            let row_idx = ((fy - KB_Y) / (KB_ROW_H + KB_GAP_Y)) as usize;
            let rel_y = (fy - KB_Y) - (row_idx as f32 * (KB_ROW_H + KB_GAP_Y));

            if rel_y <= KB_ROW_H && row_idx < 5 {
                match self.kb_mode {
                    KeyboardMode::AlphaLower | KeyboardMode::AlphaUpper => {
                        let is_upper = self.kb_mode == KeyboardMode::AlphaUpper;
                        match row_idx {
                            0 => {
                                let keys = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
                                let key_gap = 8.0;
                                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                                for (i, ch) in keys.iter().enumerate() {
                                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            1 => {
                                let keys_l = ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'];
                                let keys_u = ['Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'];
                                let keys = if is_upper { keys_u } else { keys_l };
                                let key_gap = 8.0;
                                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                                for (i, ch) in keys.iter().enumerate() {
                                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            2 => {
                                let keys_l = ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];
                                let keys_u = ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L'];
                                let keys = if is_upper { keys_u } else { keys_l };
                                let key_gap = 8.0;
                                let row_w = CONTENT_W * 0.92;
                                let key_w = (row_w - (key_gap * 8.0)) / 9.0;
                                let start_x = TERM_MARGIN + (CONTENT_W - row_w) / 2.0;

                                for (i, ch) in keys.iter().enumerate() {
                                    let kx = start_x + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            3 => {
                                let key_gap = 8.0;
                                let unit_w = (CONTENT_W - (key_gap * 8.0)) / 10.0;
                                let shift_w = unit_w * 1.5;
                                let bksp_w = unit_w * 1.5;
                                let mid_w = (CONTENT_W - shift_w - bksp_w - (key_gap * 8.0)) / 7.0;

                                if fx >= TERM_MARGIN && fx <= TERM_MARGIN + shift_w {
                                    self.kb_mode = if is_upper {
                                        KeyboardMode::AlphaLower
                                    } else {
                                        KeyboardMode::AlphaUpper
                                    };
                                    return Some(TerminalAction::ToggleShift);
                                }

                                let keys_l = ['z', 'x', 'c', 'v', 'b', 'n', 'm'];
                                let keys_u = ['Z', 'X', 'C', 'V', 'B', 'N', 'M'];
                                let keys = if is_upper { keys_u } else { keys_l };

                                for (i, ch) in keys.iter().enumerate() {
                                    let kx = TERM_MARGIN + shift_w + key_gap + (i as f32) * (mid_w + key_gap);
                                    if fx >= kx && fx <= kx + mid_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }

                                let bk_x = TERM_WIDTH - TERM_MARGIN - bksp_w;
                                if fx >= bk_x && fx <= bk_x + bksp_w {
                                    self.input_buffer.pop();
                                    return Some(TerminalAction::KeyBackspace);
                                }
                            }
                            4 => {
                                let key_gap = 8.0;
                                let sym_w = 140.0;
                                let enter_w = 190.0;
                                let slash_w = 90.0;
                                let minus_w = 90.0;
                                let dot_w = 90.0;
                                let space_w = CONTENT_W - sym_w - enter_w - slash_w - minus_w - dot_w - (key_gap * 5.0);

                                let mut cur_x = TERM_MARGIN;

                                if fx >= cur_x && fx <= cur_x + sym_w {
                                    self.kb_mode = KeyboardMode::Symbols;
                                    return Some(TerminalAction::ToggleSymbols);
                                }
                                cur_x += sym_w + key_gap;

                                if fx >= cur_x && fx <= cur_x + slash_w {
                                    self.input_buffer.push('/');
                                    return Some(TerminalAction::KeyChar('/'));
                                }
                                cur_x += slash_w + key_gap;

                                if fx >= cur_x && fx <= cur_x + space_w {
                                    self.input_buffer.push(' ');
                                    return Some(TerminalAction::KeySpace);
                                }
                                cur_x += space_w + key_gap;

                                if fx >= cur_x && fx <= cur_x + minus_w {
                                    self.input_buffer.push('-');
                                    return Some(TerminalAction::KeyChar('-'));
                                }
                                cur_x += minus_w + key_gap;

                                if fx >= cur_x && fx <= cur_x + dot_w {
                                    self.input_buffer.push('.');
                                    return Some(TerminalAction::KeyChar('.'));
                                }
                                cur_x += dot_w + key_gap;

                                if fx >= cur_x && fx <= cur_x + enter_w {
                                    let cmd = self.input_buffer.clone();
                                    self.execute(&cmd);
                                    return Some(TerminalAction::KeyEnter);
                                }
                            }
                            _ => {}
                        }
                    }
                    KeyboardMode::Symbols => {
                        match row_idx {
                            0 => {
                                let syms = ['`', '~', '!', '@', '#', '$', '%', '^', '&', '*'];
                                let key_gap = 8.0;
                                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                                for (i, ch) in syms.iter().enumerate() {
                                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            1 => {
                                let syms = ['(', ')', '-', '_', '=', '+', '[', ']', '{', '}'];
                                let key_gap = 8.0;
                                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                                for (i, ch) in syms.iter().enumerate() {
                                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            2 => {
                                let syms = ['|', '\\', ':', ';', '"', '\'', '<', '>', ',', '.'];
                                let key_gap = 8.0;
                                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                                for (i, ch) in syms.iter().enumerate() {
                                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                                    if fx >= kx && fx <= kx + key_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }
                            }
                            3 => {
                                let key_gap = 8.0;
                                let unit_w = (CONTENT_W - (key_gap * 8.0)) / 10.0;
                                let abc_w = unit_w * 1.5;
                                let bksp_w = unit_w * 1.5;
                                let mid_w = (CONTENT_W - abc_w - bksp_w - (key_gap * 8.0)) / 7.0;

                                if fx >= TERM_MARGIN && fx <= TERM_MARGIN + abc_w {
                                    self.kb_mode = KeyboardMode::AlphaLower;
                                    return Some(TerminalAction::ToggleSymbols);
                                }

                                let syms = ['?', '/', '*', '=', '&', '$', '%'];
                                for (i, ch) in syms.iter().enumerate() {
                                    let kx = TERM_MARGIN + abc_w + key_gap + (i as f32) * (mid_w + key_gap);
                                    if fx >= kx && fx <= kx + mid_w {
                                        self.input_buffer.push(*ch);
                                        return Some(TerminalAction::KeyChar(*ch));
                                    }
                                }

                                let bk_x = TERM_WIDTH - TERM_MARGIN - bksp_w;
                                if fx >= bk_x && fx <= bk_x + bksp_w {
                                    self.input_buffer.pop();
                                    return Some(TerminalAction::KeyBackspace);
                                }
                            }
                            4 => {
                                let key_gap = 8.0;
                                let abc_w = 140.0;
                                let enter_w = 190.0;
                                let space_w = CONTENT_W - abc_w - enter_w - (key_gap * 2.0);

                                if fx >= TERM_MARGIN && fx <= TERM_MARGIN + abc_w {
                                    self.kb_mode = KeyboardMode::AlphaLower;
                                    return Some(TerminalAction::ToggleSymbols);
                                }
                                if fx >= TERM_MARGIN + abc_w + key_gap && fx <= TERM_MARGIN + abc_w + key_gap + space_w {
                                    self.input_buffer.push(' ');
                                    return Some(TerminalAction::KeySpace);
                                }
                                if fx >= TERM_WIDTH - TERM_MARGIN - enter_w && fx <= TERM_WIDTH - TERM_MARGIN {
                                    let cmd = self.input_buffer.clone();
                                    self.execute(&cmd);
                                    return Some(TerminalAction::KeyEnter);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if fy >= TERM_DOCK_Y && fy <= TERM_DOCK_Y + TERM_DOCK_H {
            let dock_gap = 12.0;
            let dock_w = (CONTENT_W - (dock_gap * 3.0)) / 4.0;

            let b_x = TERM_MARGIN;
            if fx >= b_x && fx <= b_x + dock_w {
                return Some(TerminalAction::Back);
            }
            let su_x = b_x + dock_w + dock_gap;
            if fx >= su_x && fx <= su_x + dock_w {
                self.scroll_up(20);
                return Some(TerminalAction::ScrollUp);
            }
            let sd_x = su_x + dock_w + dock_gap;
            if fx >= sd_x && fx <= sd_x + dock_w {
                self.scroll_down(20);
                return Some(TerminalAction::ScrollDown);
            }
            let bt_x = sd_x + dock_w + dock_gap;
            if fx >= bt_x && fx <= bt_x + dock_w {
                self.scroll_offset = 0;
                return Some(TerminalAction::ScrollToBottom);
            }
        }

        None
    }
}

pub fn render_terminal(
    canvas: &mut Canvas,
    theme: &M3Theme,
    fonts: &FontRenderer,
    state: &TerminalState,
    lang: Language,
) {
    canvas.clear(Color::from_rgba8(8, 9, 12, 255));

    let back_cx = TERM_MARGIN + 44.0;
    let back_cy = 110.0;
    canvas.draw_circle(back_cx, back_cy, 44.0, theme.surface_container_high);
    draw_icon(&mut canvas.pixmap, M3Icon::ArrowBack, back_cx, back_cy, 44.0, theme.on_surface);

    let title = match lang {
        Language::English => "UNIX Recovery Shell",
        Language::Russian => "Командная строка рекавери",
    };
    let subtitle = "degas (MT6897) • /system/bin/sh • root uid 0";
    fonts.draw_text(&mut canvas.pixmap, title, back_cx + 64.0, back_cy - 4.0, 36.0, theme.on_surface);
    fonts.draw_text(&mut canvas.pixmap, subtitle, back_cx + 64.0, back_cy + 34.0, 20.0, theme.primary);

    let clr_w = 150.0;
    let clr_x = TERM_WIDTH - TERM_MARGIN - clr_w;
    canvas.draw_pill(clr_x, 70.0, clr_w, 75.0, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
    let clr_lbl = match lang {
        Language::English => "Clear",
        Language::Russian => "Очистить",
    };
    let cw = fonts.measure_text(clr_lbl, 22.0);
    fonts.draw_text(&mut canvas.pixmap, clr_lbl, clr_x + (clr_w - cw) / 2.0, 116.0, 22.0, theme.on_surface);

    canvas.draw_rounded_rect(
        TERM_MARGIN,
        CONSOLE_TOP,
        CONTENT_W,
        CONSOLE_H,
        20.0,
        Some(Color::from_rgba8(14, 16, 22, 255)),
        Some((theme.outline_variant, 1.5)),
    );

    let line_h = 32.0;
    let visible_lines_count = ((CONSOLE_H - 40.0) / line_h) as usize;
    let total_lines = state.lines.len();

    let end_idx = total_lines.saturating_sub(state.scroll_offset);
    let start_idx = end_idx.saturating_sub(visible_lines_count);

    let mut cur_y = CONSOLE_TOP + 36.0;
    for i in start_idx..end_idx {
        if let Some(line) = state.lines.get(i) {
            let line_color = if line.starts_with("degas:/ #") {
                theme.primary
            } else if line.starts_with("[STDERR]") || line.starts_with("[EXEC ERROR]") {
                theme.error
            } else if line.starts_with("===") || line.starts_with(" MeowFloprRecovery") {
                theme.secondary
            } else if line.starts_with("[+]") || line.starts_with("[OK]") {
                Color::from_rgba8(120, 230, 140, 255)
            } else {
                Color::from_rgba8(220, 224, 230, 255)
            };

            let display_txt = if line.len() > 72 {
                format!("{}...", &line[..69])
            } else {
                line.clone()
            };

            fonts.draw_text(&mut canvas.pixmap, &display_txt, TERM_MARGIN + 24.0, cur_y, 20.0, line_color);
            cur_y += line_h;
        }
    }

    if total_lines > visible_lines_count {
        let scroll_track_h = CONSOLE_H - 40.0;
        let thumb_h = (visible_lines_count as f32 / total_lines as f32 * scroll_track_h).clamp(40.0, scroll_track_h);
        let max_offset = total_lines - visible_lines_count;
        let offset_ratio = 1.0 - (state.scroll_offset as f32 / max_offset as f32).clamp(0.0, 1.0);
        let thumb_y = CONSOLE_TOP + 20.0 + (scroll_track_h - thumb_h) * offset_ratio;

        canvas.draw_pill(TERM_WIDTH - TERM_MARGIN - 14.0, thumb_y, 8.0, thumb_h, theme.primary, None);
    }

    let chips = ["ls -la", "df -h", "mount", "ps -ef", "dmesg", "reboot"];
    let chip_gap = 10.0;
    let chip_w = (CONTENT_W - (chip_gap * (chips.len() as f32 - 1.0))) / chips.len() as f32;

    for (i, label) in chips.iter().enumerate() {
        let cx = TERM_MARGIN + (i as f32) * (chip_w + chip_gap);
        canvas.draw_pill(cx, CHIPS_Y, chip_w, CHIPS_H, theme.surface_container_high, Some((theme.outline_variant, 1.0)));
        let nw = fonts.measure_text(label, 20.0);
        fonts.draw_text(&mut canvas.pixmap, label, cx + (chip_w - nw) / 2.0, CHIPS_Y + 43.0, 20.0, theme.primary);
    }

    canvas.draw_rounded_rect(
        TERM_MARGIN,
        INPUT_Y,
        CONTENT_W,
        INPUT_H,
        22.0,
        Some(theme.surface_container_highest),
        Some((theme.primary, 2.0)),
    );

    let prompt_txt = "degas:/ # ";
    fonts.draw_text(&mut canvas.pixmap, prompt_txt, TERM_MARGIN + 24.0, INPUT_Y + 49.0, 22.0, theme.primary);
    let pw = fonts.measure_text(prompt_txt, 22.0);

    let input_display = format!("{}_", state.input_buffer);
    fonts.draw_text(&mut canvas.pixmap, &input_display, TERM_MARGIN + 24.0 + pw, INPUT_Y + 49.0, 22.0, theme.on_surface);

    let run_w = 120.0;
    let run_x = TERM_WIDTH - TERM_MARGIN - run_w - 10.0;
    canvas.draw_pill(run_x, INPUT_Y + 8.0, run_w, INPUT_H - 16.0, theme.primary, None);
    let run_lbl = "RUN";
    let rw = fonts.measure_text(run_lbl, 21.0);
    fonts.draw_text(&mut canvas.pixmap, run_lbl, run_x + (run_w - rw) / 2.0, INPUT_Y + 49.0, 21.0, theme.on_primary);

    match state.kb_mode {
        KeyboardMode::AlphaLower | KeyboardMode::AlphaUpper => {
            let is_upper = state.kb_mode == KeyboardMode::AlphaUpper;

            {
                let keys = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
                let key_gap = 8.0;
                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                let y = KB_Y;
                for (i, ch) in keys.iter().enumerate() {
                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, key_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (key_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }
            }

            {
                let keys_l = ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'];
                let keys_u = ['Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P'];
                let keys = if is_upper { keys_u } else { keys_l };
                let key_gap = 8.0;
                let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y);
                for (i, ch) in keys.iter().enumerate() {
                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, key_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (key_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }
            }

            {
                let keys_l = ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];
                let keys_u = ['A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L'];
                let keys = if is_upper { keys_u } else { keys_l };
                let key_gap = 8.0;
                let row_w = CONTENT_W * 0.92;
                let key_w = (row_w - (key_gap * 8.0)) / 9.0;
                let start_x = TERM_MARGIN + (CONTENT_W - row_w) / 2.0;
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * 2.0;

                for (i, ch) in keys.iter().enumerate() {
                    let kx = start_x + (i as f32) * (key_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, key_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (key_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }
            }

            {
                let key_gap = 8.0;
                let unit_w = (CONTENT_W - (key_gap * 8.0)) / 10.0;
                let shift_w = unit_w * 1.5;
                let bksp_w = unit_w * 1.5;
                let mid_w = (CONTENT_W - shift_w - bksp_w - (key_gap * 8.0)) / 7.0;
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * 3.0;

                let shift_bg = if is_upper { theme.primary } else { theme.surface_container_highest };
                let shift_fg = if is_upper { theme.on_primary } else { theme.on_surface };
                canvas.draw_rounded_rect(TERM_MARGIN, y, shift_w, KB_ROW_H, 16.0, Some(shift_bg), Some((theme.outline_variant, 1.0)));
                let sh_lbl = "SHIFT";
                let sw = fonts.measure_text(sh_lbl, 20.0);
                fonts.draw_text(&mut canvas.pixmap, sh_lbl, TERM_MARGIN + (shift_w - sw) / 2.0, y + 53.0, 20.0, shift_fg);

                let keys_l = ['z', 'x', 'c', 'v', 'b', 'n', 'm'];
                let keys_u = ['Z', 'X', 'C', 'V', 'B', 'N', 'M'];
                let keys = if is_upper { keys_u } else { keys_l };

                for (i, ch) in keys.iter().enumerate() {
                    let kx = TERM_MARGIN + shift_w + key_gap + (i as f32) * (mid_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, mid_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (mid_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }

                let bk_x = TERM_WIDTH - TERM_MARGIN - bksp_w;
                canvas.draw_rounded_rect(bk_x, y, bksp_w, KB_ROW_H, 16.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.0)));
                let bk_lbl = "DEL";
                let bw = fonts.measure_text(bk_lbl, 21.0);
                fonts.draw_text(&mut canvas.pixmap, bk_lbl, bk_x + (bksp_w - bw) / 2.0, y + 53.0, 21.0, theme.on_surface);
            }

            {
                let key_gap = 8.0;
                let sym_w = 140.0;
                let enter_w = 190.0;
                let slash_w = 90.0;
                let minus_w = 90.0;
                let dot_w = 90.0;
                let space_w = CONTENT_W - sym_w - enter_w - slash_w - minus_w - dot_w - (key_gap * 5.0);
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * 4.0;

                let mut cur_x = TERM_MARGIN;

                canvas.draw_rounded_rect(cur_x, y, sym_w, KB_ROW_H, 16.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.0)));
                let s_lbl = "?123";
                let sw = fonts.measure_text(s_lbl, 22.0);
                fonts.draw_text(&mut canvas.pixmap, s_lbl, cur_x + (sym_w - sw) / 2.0, y + 53.0, 22.0, theme.on_surface);
                cur_x += sym_w + key_gap;

                canvas.draw_rounded_rect(cur_x, y, slash_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, "/", cur_x + 36.0, y + 54.0, 24.0, theme.on_surface);
                cur_x += slash_w + key_gap;

                canvas.draw_rounded_rect(cur_x, y, space_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                let sp_lbl = "SPACE";
                let sp_w = fonts.measure_text(sp_lbl, 20.0);
                fonts.draw_text(&mut canvas.pixmap, sp_lbl, cur_x + (space_w - sp_w) / 2.0, y + 53.0, 20.0, theme.on_surface_variant);
                cur_x += space_w + key_gap;

                canvas.draw_rounded_rect(cur_x, y, minus_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, "-", cur_x + 38.0, y + 54.0, 24.0, theme.on_surface);
                cur_x += minus_w + key_gap;

                canvas.draw_rounded_rect(cur_x, y, dot_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, ".", cur_x + 38.0, y + 54.0, 24.0, theme.on_surface);
                cur_x += dot_w + key_gap;

                canvas.draw_rounded_rect(cur_x, y, enter_w, KB_ROW_H, 16.0, Some(theme.primary), None);
                let ent_lbl = "ENTER";
                let ew = fonts.measure_text(ent_lbl, 21.0);
                fonts.draw_text(&mut canvas.pixmap, ent_lbl, cur_x + (enter_w - ew) / 2.0, y + 53.0, 21.0, theme.on_primary);
            }
        }
        KeyboardMode::Symbols => {
            let sym_rows = [
                vec!['`', '~', '!', '@', '#', '$', '%', '^', '&', '*'],
                vec!['(', ')', '-', '_', '=', '+', '[', ']', '{', '}'],
                vec!['|', '\\', ':', ';', '"', '\'', '<', '>', ',', '.'],
            ];

            let key_gap = 8.0;
            let key_w = (CONTENT_W - (key_gap * 9.0)) / 10.0;

            for (r_idx, row) in sym_rows.iter().enumerate() {
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * r_idx as f32;
                for (i, ch) in row.iter().enumerate() {
                    let kx = TERM_MARGIN + (i as f32) * (key_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, key_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (key_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }
            }

            {
                let unit_w = (CONTENT_W - (key_gap * 8.0)) / 10.0;
                let abc_w = unit_w * 1.5;
                let bksp_w = unit_w * 1.5;
                let mid_w = (CONTENT_W - abc_w - bksp_w - (key_gap * 8.0)) / 7.0;
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * 3.0;

                canvas.draw_rounded_rect(TERM_MARGIN, y, abc_w, KB_ROW_H, 16.0, Some(theme.primary), None);
                fonts.draw_text(&mut canvas.pixmap, "ABC", TERM_MARGIN + 32.0, y + 53.0, 22.0, theme.on_primary);

                let syms = ['?', '/', '*', '=', '&', '$', '%'];
                for (i, ch) in syms.iter().enumerate() {
                    let kx = TERM_MARGIN + abc_w + key_gap + (i as f32) * (mid_w + key_gap);
                    canvas.draw_rounded_rect(kx, y, mid_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                    let ch_s = ch.to_string();
                    let cw = fonts.measure_text(&ch_s, 24.0);
                    fonts.draw_text(&mut canvas.pixmap, &ch_s, kx + (mid_w - cw) / 2.0, y + 54.0, 24.0, theme.on_surface);
                }

                let bk_x = TERM_WIDTH - TERM_MARGIN - bksp_w;
                canvas.draw_rounded_rect(bk_x, y, bksp_w, KB_ROW_H, 16.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, "DEL", bk_x + 32.0, y + 53.0, 21.0, theme.on_surface);
            }

            {
                let abc_w = 140.0;
                let enter_w = 190.0;
                let space_w = CONTENT_W - abc_w - enter_w - (key_gap * 2.0);
                let y = KB_Y + (KB_ROW_H + KB_GAP_Y) * 4.0;

                canvas.draw_rounded_rect(TERM_MARGIN, y, abc_w, KB_ROW_H, 16.0, Some(theme.surface_container_highest), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, "ABC", TERM_MARGIN + 44.0, y + 53.0, 22.0, theme.on_surface);

                let sp_x = TERM_MARGIN + abc_w + key_gap;
                canvas.draw_rounded_rect(sp_x, y, space_w, KB_ROW_H, 16.0, Some(theme.surface_container_high), Some((theme.outline_variant, 1.0)));
                fonts.draw_text(&mut canvas.pixmap, "SPACE", sp_x + (space_w - 60.0) / 2.0, y + 53.0, 20.0, theme.on_surface_variant);

                let ent_x = TERM_WIDTH - TERM_MARGIN - enter_w;
                canvas.draw_rounded_rect(ent_x, y, enter_w, KB_ROW_H, 16.0, Some(theme.primary), None);
                fonts.draw_text(&mut canvas.pixmap, "ENTER", ent_x + 55.0, y + 53.0, 21.0, theme.on_primary);
            }
        }
    }

    let dock_gap = 12.0;
    let dock_w = (CONTENT_W - (dock_gap * 3.0)) / 4.0;

    let dock_items = [
        ("BACK", M3Icon::ArrowBack),
        ("PG UP", M3Icon::Settings),
        ("PG DN", M3Icon::Settings),
        ("BOTTOM", M3Icon::Restart),
    ];

    for (i, (lbl, _icon)) in dock_items.iter().enumerate() {
        let dx = TERM_MARGIN + (i as f32) * (dock_w + dock_gap);
        let bg = if i == 0 { theme.surface_container_highest } else { theme.surface_container_high };
        canvas.draw_pill(dx, TERM_DOCK_Y, dock_w, TERM_DOCK_H, bg, Some((theme.outline_variant, 1.5)));

        let tw = fonts.measure_text(lbl, 21.0);
        fonts.draw_text(&mut canvas.pixmap, lbl, dx + (dock_w - tw) / 2.0, TERM_DOCK_Y + 66.0, 21.0, theme.on_surface);
    }
}

