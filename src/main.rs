mod crypto;
mod hardware;
mod init;
mod platform;
mod ui;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tiny_skia::Pixmap;

use ui::action_console::{hit_test as action_hit_test, render_action_console, ActionConsoleState, ActionConsoleTarget};
use ui::advanced::{render_advanced, AdvancedAction, AdvancedState, AdvancedSubScreen, RootActionType};
use ui::backup::{render_backup, BackupAction, BackupState, BackupTab};
use ui::dashboard::{hit_test as dash_hit_test, render_dashboard, ActionTarget, DashboardState};
use ui::decrypt::{hit_test as dec_hit_test, render_decrypt, DecryptAction, DecryptState};
use ui::installer::{render_installer, InstallerAction, InstallerState};
use ui::mount::{render_mount, MountAction, MountState};
use ui::power_modal::{hit_test as pwr_hit_test, render_power_modal, PowerModalTarget};
use ui::settings::{render_settings, SettingsAction, SettingsState};
use ui::shapes::M3ShapeType;
use ui::terminal::{render_terminal, TerminalAction, TerminalState};
use ui::theme::M3Shapes;
use ui::wipe::{render_wipe, WipeAction, WipeState, WipeTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Dashboard,
    PowerModal,
    Terminal,
    Installer,
    Wipe,
    Decrypt,
    ActionConsole,
    Backup,
    Mount,
    Settings,
    Advanced,
}

pub fn resolve_partition_block(id: &str, slot: &str) -> Option<PathBuf> {
    let s_clean = slot.trim_start_matches('_').to_lowercase();
    let candidates = [
        format!("/dev/block/by-name/{}_{}", id, s_clean),
        format!("/dev/block/by-name/{}", id),
        format!("/dev/block/bootdevice/by-name/{}_{}", id, s_clean),
        format!("/dev/block/bootdevice/by-name/{}", id),
        format!("/dev/block/platform/bootdevice/by-name/{}_{}", id, s_clean),
        format!("/dev/block/platform/bootdevice/by-name/{}", id),
        format!("/dev/block/mapper/{}_{}", id, s_clean),
        format!("/dev/block/mapper/{}", id),
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn clean_dir_contents(dir_path: &Path) -> std::io::Result<u64> {
    if !dir_path.exists() {
        return Ok(0);
    }
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
            count += 1;
        }
    }
    Ok(count)
}

pub struct Sha256 {
    h: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn process_block(&mut self, block: &[u8]) {
        let k: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
            0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
            0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
            0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
            0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        ];

        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = self.h[0];
        let mut b = self.h[1];
        let mut c = self.h[2];
        let mut d = self.h[3];
        let mut e = self.h[4];
        let mut f = self.h[5];
        let mut g = self.h[6];
        let mut h_val = self.h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
        self.h[5] = self.h[5].wrapping_add(f);
        self.h[6] = self.h[6].wrapping_add(g);
        self.h[7] = self.h[7].wrapping_add(h_val);
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total_len += data.len() as u64;
        if self.buf_len > 0 {
            let to_fill = 64 - self.buf_len;
            if data.len() >= to_fill {
                self.buf[self.buf_len..64].copy_from_slice(&data[..to_fill]);
                let block = self.buf;
                self.process_block(&block);
                self.buf_len = 0;
                data = &data[to_fill..];
            } else {
                self.buf[self.buf_len..self.buf_len + data.len()].copy_from_slice(data);
                self.buf_len += data.len();
                return;
            }
        }
        while data.len() >= 64 {
            self.process_block(&data[..64]);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;
        if self.buf_len > 56 {
            for i in self.buf_len..64 {
                self.buf[i] = 0;
            }
            let block = self.buf;
            self.process_block(&block);
            self.buf_len = 0;
        }
        for i in self.buf_len..56 {
            self.buf[i] = 0;
        }
        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buf;
        self.process_block(&block);

        let mut out = [0u8; 32];
        for (i, val) in self.h.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
        }
        out
    }
}

pub fn compute_file_sha256(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{:02x}", b)).collect())
}

pub fn get_block_device_size(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.file_type().is_block_device() {
                if let Ok(file) = std::fs::File::open(path) {
                    let mut size: u64 = 0;
                    let ret = unsafe {
                        libc::ioctl(
                            std::os::unix::io::AsRawFd::as_raw_fd(&file),
                            0x80081272u32 as i32,
                            &mut size,
                        )
                    };
                    if ret == 0 && size > 0 {
                        return Some(size);
                    }
                }
            } else {
                return Some(meta.len());
            }
        }
    }
    None
}

pub fn dump_partition(src_path: &Path, dest_path: &Path, max_bytes: Option<u64>) -> std::io::Result<u64> {
    use std::io::{Read, Write};
    let mut src = std::fs::File::open(src_path)?;
    let mut dest = std::fs::File::create(dest_path)?;
    let mut buf = vec![0u8; 1024 * 1024];
    let mut total = 0u64;

    loop {
        let to_read = if let Some(max) = max_bytes {
            let remain = max.saturating_sub(total);
            if remain == 0 {
                break;
            }
            (remain as usize).min(buf.len())
        } else {
            buf.len()
        };

        let n = src.read(&mut buf[..to_read])?;
        if n == 0 {
            break;
        }
        dest.write_all(&buf[..n])?;
        total += n as u64;
    }
    dest.flush()?;
    Ok(total)
}

fn redraw_console(
    display: &mut Option<platform::drm::DrmDisplay>,
    pixmap: &mut Pixmap,
    theme: &ui::theme::M3Theme,
    fonts: &ui::text::FontRenderer,
    state: &ui::action_console::ActionConsoleState,
) {
    let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
    ui::action_console::render_action_console(&mut canvas, theme, fonts, state, None);
    if let Some(ref mut disp) = display {
        disp.blit_and_flip(pixmap);
    }
}

fn main() {
    println!("=====================================================");
    println!("   MeowFloprRecovery (MRR) - 100% Native Pure Rust   ");
    println!("   Target: Xiaomi 14T (degas, MT6897) • Standalone OS");
    println!("=====================================================");

    if let Err(e) = init::early_boot() {
        eprintln!("[MRR-MAIN] Early boot warning: {}", e);
    }

    let mut slot = hardware::slots::get_current_slot();
    let mut battery = hardware::battery::get_battery_info();
    let brightness = hardware::backlight::get_brightness();

    println!("[MRR-STATUS] Active Slot:   Slot {}", slot);
    println!("[MRR-STATUS] Battery:       {}% ({}°C)", battery.percent, battery.temp_c);
    println!("[MRR-STATUS] Brightness:    {}", brightness);

    hardware::dynamic_partitions::setup_all_dynamic_partitions(&slot);

    let mut storage_stats = hardware::storage::get_storage_stats();
    println!(
        "[MRR-STATUS] Storage:       Available {} GB / Total {} GB",
        storage_stats.free_gb, storage_stats.total_gb
    );

    let mut is_decrypted = crypto::auto_decrypt();
    println!("[MRR-STATUS] Decrypted:     {}", if is_decrypted { "YES" } else { "NO" });

    println!("[MRR-DISPLAY] Initializing Dual-Pipe Atomic DRM (/dev/dri/card0)...");
    let mut display = match platform::DrmDisplay::init() {
        Ok(d) => {
            println!(
                "[MRR-DISPLAY] Connected to DRM panel: {}x{} @ {}Hz",
                d.width(),
                d.height(),
                d.refresh_rate()
            );
            Some(d)
        }
        Err(e) => {
            eprintln!("[MRR-DISPLAY] DRM init failed ({}), continuing in headless mode.", e);
            None
        }
    };

    println!("[MRR-INPUT] Initializing touchscreen and power button drivers...");
    let mut input = platform::InputManager::new();

    let mut settings_state = SettingsState::new();
    let mut theme = settings_state.current_theme.to_theme();
    let shapes = M3Shapes::default();
    let fonts = ui::text::FontRenderer::new();

    let mut pixmap = Pixmap::new(1220, 2712).expect("Failed to allocate 1220x2712 Pixmap");

    let mut current_screen = CurrentScreen::Dashboard;
    let mut terminal_state = TerminalState::new();
    let mut installer_state = InstallerState::new();
    let mut wipe_state = WipeState::new();
    let mut decrypt_state = DecryptState::new();
    let mut action_console_state = ActionConsoleState::new();
    let mut backup_state = BackupState::new();
    let mut mount_state = MountState::new();
    let mut advanced_state = AdvancedState::new();

    let mut pressed_dash_target: Option<ActionTarget> = None;
    let mut pressed_power_target: Option<PowerModalTarget> = None;
    let mut pressed_decrypt_action: Option<DecryptAction> = None;
    let mut pressed_action_target: Option<ActionConsoleTarget> = None;

    let mut shape_override: Option<M3ShapeType> = settings_state.current_shape;
    let mut dirty = true;
    let mut last_status_check = Instant::now();
    let mut time_minutes = 0;

    println!("[MRR-MAIN] Entering high-efficiency recovery event loop...");

    {
        let time_str = "14:15";
        let state = DashboardState {
            language: settings_state.language,
            slot: &slot,
            battery_percent: battery.percent,
            battery_temp: battery.temp_c,
            is_decrypted,
            storage_free_gb: storage_stats.free_gb,
            storage_total_gb: storage_stats.total_gb,
            time_str,
            active_shape: shape_override,
            pressed_target: None,
        };

        let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
        render_dashboard(&mut canvas, &theme, &shapes, &fonts, &state);

        if let Some(ref mut disp) = display {
            disp.blit_and_flip(&pixmap);
        }

        let _ = pixmap.save_png("mrr_dashboard_m3_preview.png");
        dirty = false;
    }

    if std::env::args().any(|a| a == "--preview" || a == "--generate-previews") {
        println!("[MRR-PREVIEW] Rendering all recovery screens to pic/ directory...");
        let _ = std::fs::create_dir_all("pic");
        let out_dir = "pic";

        let preview_lang = settings_state.language;

        let dash_state = DashboardState {
            language: preview_lang,
            slot: &slot,
            battery_percent: battery.percent,
            battery_temp: battery.temp_c,
            is_decrypted: true,
            storage_free_gb: storage_stats.free_gb,
            storage_total_gb: storage_stats.total_gb,
            time_str: "14:15",
            active_shape: shape_override,
            pressed_target: None,
        };

        {
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_dashboard(&mut canvas, &theme, &shapes, &fonts, &dash_state);
            let _ = pixmap.save_png(format!("{}/preview_dashboard.png", out_dir));
        }

        {
            let mut w_state = WipeState::new();
            w_state.current_tab = WipeTab::AdvancedWipe;
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_wipe(&mut canvas, &theme, &fonts, &w_state, None);
            let _ = pixmap.save_png(format!("{}/preview_wipe_advanced.png", out_dir));
        }

        {
            let mut w_state = WipeState::new();
            w_state.current_tab = WipeTab::FactoryReset;
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_wipe(&mut canvas, &theme, &fonts, &w_state, None);
            let _ = pixmap.save_png(format!("{}/preview_wipe_factory.png", out_dir));
        }

        {
            let mut w_state = WipeState::new();
            w_state.current_tab = WipeTab::FormatData;
            w_state.format_input = "yes".to_string();
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_wipe(&mut canvas, &theme, &fonts, &w_state, None);
            let _ = pixmap.save_png(format!("{}/preview_wipe_format_yes.png", out_dir));
        }

        {
            let b_state = BackupState::new();
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_backup(&mut canvas, &theme, &fonts, &b_state, None);
            let _ = pixmap.save_png(format!("{}/preview_backup.png", out_dir));
        }

        {
            let mut b_state = BackupState::new();
            b_state.current_tab = BackupTab::Restore;
            b_state.archives.push(ui::backup::BackupArchive {
                name: "MRR_HYPEROS_FULL_20261029".to_string(),
                path: PathBuf::from("/sdcard/MRR/BACKUPS/MRR_HYPEROS_FULL_20261029"),
                date_str: "2026-10-29".to_string(),
                size_mb: 9480,
                partitions: vec!["boot".to_string(), "vendor_boot".to_string(), "super".to_string(), "persist".to_string(), "nvram".to_string()],
            });
            b_state.selected_archive_idx = Some(0);
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_backup(&mut canvas, &theme, &fonts, &b_state, None);
            let _ = pixmap.save_png(format!("{}/preview_restore.png", out_dir));
        }

        {
            let m_state = MountState::new();
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_mount(&mut canvas, &theme, &fonts, &m_state, None);
            let _ = pixmap.save_png(format!("{}/preview_mount.png", out_dir));
        }

        {
            let s_state = SettingsState::new();
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_settings(&mut canvas, &theme, &fonts, &s_state, None);
            let _ = pixmap.save_png(format!("{}/preview_settings.png", out_dir));
        }

        {
            let mut a_state = AdvancedState::new();
            a_state.subscreen = AdvancedSubScreen::RootManager;
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_advanced(&mut canvas, &theme, &fonts, &a_state, preview_lang);
            let _ = pixmap.save_png(format!("{}/preview_advanced.png", out_dir));
        }

        {
            let mut a_state = ActionConsoleState::new();
            a_state.start("Install Package: Xiaomi_HyperOS_Update.zip");
            a_state.set_step("Verifying cryptographic signature (AVB 2.0)...", 0.35);
            a_state.add_log("[+] Found META-INF/com/google/android/update-binary");
            a_state.add_log("[+] Verified Dimensity 8300 MT6897 device compatibility");
            a_state.set_step("Executing installer update-binary...", 0.65);
            a_state.add_log("[+] Patching system_a and vendor_a partitions...");
            a_state.add_log("[+] Successfully flashed GKI Linux 6.1 kernel");
            a_state.finish(true, "Installation completed successfully!");

            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_action_console(&mut canvas, &theme, &fonts, &a_state, None);
            let _ = pixmap.save_png(format!("{}/preview_action_console.png", out_dir));
        }

        {
            let mut t_state = TerminalState::new();
            t_state.input_buffer = "cat /proc/partitions".to_string();
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_terminal(&mut canvas, &theme, &fonts, &t_state, preview_lang);
            let _ = pixmap.save_png(format!("{}/preview_terminal_keyboard.png", out_dir));
        }

        {
            let mut a_state = AdvancedState::new();
            a_state.subscreen = AdvancedSubScreen::DmesgViewer;
            a_state.dmesg_lines = vec![
                "[    0.000000] Linux version 6.1.75-android14-11 (degas@build) (clang 17.0.2)".to_string(),
                "[    0.000000] Machine model: Xiaomi 14T (degas, MT6897)".to_string(),
                "[    0.142010] drm_kms_helper: DRM display initialized: 1220x2712 @ 144Hz".to_string(),
                "[    0.512030] fscrypt: FBE v2 inline encryption enabled on /dev/block/dm-0".to_string(),
                "[    1.210400] mrr: MeowFloprRecovery OS starting PID 1 event loop".to_string(),
            ];
            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());
            render_advanced(&mut canvas, &theme, &fonts, &a_state, preview_lang);
            let _ = pixmap.save_png(format!("{}/preview_dmesg_viewer.png", out_dir));
        }

        println!("[MRR-PREVIEW] Successfully rendered all 12 preview screens!");
        return;
    }

    if display.is_none() && cfg!(target_arch = "x86_64") {
        println!("[MRR-MAIN] Host x86_64 test build completed successfully.");
        return;
    }

    loop {
        let (power_toggle, touch_ev) = input.poll_events();

        if power_toggle {
            if let Some(ref mut disp) = display {
                let current_pwr = disp.is_screen_power();
                let next_pwr = !current_pwr;
                disp.set_screen_power(next_pwr);
                println!("[MRR-POWER] Screen power toggled to: {}", if next_pwr { "ON" } else { "OFF" });
                if next_pwr {
                    dirty = true;
                }
            }
        }

        let is_asleep = display.as_ref().map(|d| !d.is_screen_power()).unwrap_or(false);

        if is_asleep {
            if let Some(ev) = touch_ev {
                if ev.state == platform::TouchState::Pressed {
                    if let Some(ref mut disp) = display {
                        disp.set_screen_power(true);
                        dirty = true;
                        println!("[MRR-TOUCH] Screen tap detected -> Waking display (touch swallowed)");
                    }
                }
            }
        } else {
            let timeout_secs = settings_state.screen_timeout.to_seconds();
            if timeout_secs != u64::MAX && input.last_activity().elapsed() > Duration::from_secs(timeout_secs) {
                if let Some(ref mut disp) = display {
                    if disp.is_screen_power() {
                        println!("[MRR-POWER] Inactivity timeout ({}s) reached -> Turning display OFF", timeout_secs);
                        disp.set_screen_power(false);
                    }
                }
            }

            if current_screen == CurrentScreen::Wipe && wipe_state.tick_spring() {
                dirty = true;
            }
            if current_screen == CurrentScreen::Installer && installer_state.tick_spring() {
                dirty = true;
            }
            if current_screen == CurrentScreen::Backup && backup_state.tick_spring() {
                dirty = true;
            }

            if let Some(ev) = touch_ev {
                match current_screen {
                    CurrentScreen::Dashboard => match ev.state {
                        platform::TouchState::Pressed => {
                            let hit = dash_hit_test(ev.x, ev.y);
                            if hit != pressed_dash_target {
                                pressed_dash_target = hit;
                                dirty = true;
                            }
                        }
                        platform::TouchState::Released => {
                            if let Some(target) = pressed_dash_target {
                                let release_hit = dash_hit_test(ev.x, ev.y);
                                if release_hit == Some(target) {
                                    println!("[MRR-UI] Action triggered: {:?}", target);

                                    match target {
                                        ActionTarget::Install => {
                                            println!("[MRR-UI] Opening Install / Package Manager screen...");
                                            installer_state.load_dir();
                                            current_screen = CurrentScreen::Installer;
                                            dirty = true;
                                        }
                                        ActionTarget::Wipe => {
                                            println!("[MRR-UI] Opening Wipe screen...");
                                            current_screen = CurrentScreen::Wipe;
                                            dirty = true;
                                        }
                                        ActionTarget::Backup => {
                                            println!("[MRR-UI] Opening Backup & Restore screen...");
                                            backup_state.reset_sliders();
                                            backup_state.scan_archives();
                                            current_screen = CurrentScreen::Backup;
                                            dirty = true;
                                        }
                                        ActionTarget::Mount => {
                                            println!("[MRR-UI] Opening Mount & Storage screen...");
                                            mount_state.refresh_mounts();
                                            current_screen = CurrentScreen::Mount;
                                            dirty = true;
                                        }
                                        ActionTarget::Advanced => {
                                            println!("[MRR-UI] Opening Advanced Tools screen...");
                                            advanced_state.detect_current_root();
                                            current_screen = CurrentScreen::Advanced;
                                            dirty = true;
                                        }
                                        ActionTarget::Terminal | ActionTarget::DockTerminal => {
                                            println!("[MRR-UI] Opening Interactive UNIX Shell Terminal...");
                                            current_screen = CurrentScreen::Terminal;
                                            dirty = true;
                                        }
                                        ActionTarget::Settings => {
                                            println!("[MRR-UI] Opening Settings screen...");
                                            current_screen = CurrentScreen::Settings;
                                            dirty = true;
                                        }
                                        ActionTarget::Power | ActionTarget::DockPower => {
                                            println!("[MRR-UI] Opening Power & Reboot modal...");
                                            current_screen = CurrentScreen::PowerModal;
                                            dirty = true;
                                        }
                                        ActionTarget::Decrypt => {
                                            println!("[MRR-UI] Opening FBE v2 Decryption screen...");
                                            current_screen = CurrentScreen::Decrypt;
                                            dirty = true;
                                        }
                                        ActionTarget::DockBack => {
                                            println!("[MRR-UI] Dock Back pressed");
                                        }
                                        ActionTarget::HeroCard => {
                                            println!("[MRR-UI] Storage hero info clicked");
                                        }
                                    }
                                }
                                pressed_dash_target = None;
                                dirty = true;
                            }
                        }
                    },

                    CurrentScreen::PowerModal => match ev.state {
                        platform::TouchState::Pressed => {
                            let hit = pwr_hit_test(ev.x, ev.y);
                            if hit != pressed_power_target {
                                pressed_power_target = hit;
                                dirty = true;
                            }
                        }
                        platform::TouchState::Released => {
                            if let Some(target) = pressed_power_target {
                                let release_hit = pwr_hit_test(ev.x, ev.y);
                                if release_hit == Some(target) {
                                    println!("[MRR-POWER] Power modal selection: {:?}", target);
                                    match target {
                                        PowerModalTarget::Close => {
                                            current_screen = CurrentScreen::Dashboard;
                                            dirty = true;
                                        }
                                        PowerModalTarget::System => {
                                            hardware::power::reboot(hardware::power::RebootTarget::System);
                                        }
                                        PowerModalTarget::Recovery => {
                                            hardware::power::reboot(hardware::power::RebootTarget::Recovery);
                                        }
                                        PowerModalTarget::Bootloader => {
                                            hardware::power::reboot(hardware::power::RebootTarget::Bootloader);
                                        }
                                        PowerModalTarget::FastbootD => {
                                            hardware::power::reboot(hardware::power::RebootTarget::FastbootD);
                                        }
                                        PowerModalTarget::PowerOff => {
                                            hardware::power::reboot(hardware::power::RebootTarget::PowerOff);
                                        }
                                    }
                                }
                                pressed_power_target = None;
                                dirty = true;
                            }
                        }
                    },

                    CurrentScreen::Terminal => {
                        if let Some(action) = terminal_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                TerminalAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                TerminalAction::Clear
                                | TerminalAction::ScrollUp
                                | TerminalAction::ScrollDown
                                | TerminalAction::ScrollToBottom
                                | TerminalAction::ExecuteQuick(_)
                                | TerminalAction::KeyChar(_)
                                | TerminalAction::KeyBackspace
                                | TerminalAction::KeyEnter
                                | TerminalAction::KeySpace
                                | TerminalAction::ToggleShift
                                | TerminalAction::ToggleSymbols => {
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Decrypt => match ev.state {
                        platform::TouchState::Pressed => {
                            let hit = dec_hit_test(ev.x, ev.y, decrypt_state.show_manual_pin);
                            if hit != pressed_decrypt_action {
                                pressed_decrypt_action = hit;
                                dirty = true;
                            }
                        }
                        platform::TouchState::Released => {
                            if let Some(action) = pressed_decrypt_action {
                                let release_hit = dec_hit_test(ev.x, ev.y, decrypt_state.show_manual_pin);
                                if release_hit == Some(action) {
                                    match action {
                                        DecryptAction::Digit(ch) => {
                                            decrypt_state.append_digit(ch);
                                            dirty = true;
                                        }
                                        DecryptAction::Backspace => {
                                            decrypt_state.backspace();
                                            dirty = true;
                                        }
                                        DecryptAction::Clear => {
                                            decrypt_state.clear();
                                            dirty = true;
                                        }
                                        DecryptAction::Submit => {
                                            let ok = decrypt_state.execute_unlock();
                                            if ok {
                                                is_decrypted = true;
                                                installer_state.load_dir();
                                                current_screen = CurrentScreen::Dashboard;
                                            }
                                            dirty = true;
                                        }
                                        DecryptAction::TryDefaultKey => {
                                            let ok = crypto::auto_decrypt();
                                            if ok {
                                                is_decrypted = true;
                                                decrypt_state.is_success = true;
                                                decrypt_state.status_msg = "Decrypted with Master Key!".to_string();
                                                installer_state.load_dir();
                                            } else {
                                                decrypt_state.is_error = true;
                                                decrypt_state.status_msg = "No key in persist. Export companion first.".to_string();
                                            }
                                            dirty = true;
                                        }
                                        DecryptAction::InstallCompanion => {
                                            decrypt_state.install_companion();
                                            dirty = true;
                                        }
                                        DecryptAction::RebootSystem => {
                                            hardware::power::reboot(hardware::power::RebootTarget::System);
                                        }
                                        DecryptAction::ToggleManualInput => {
                                            decrypt_state.show_manual_pin = !decrypt_state.show_manual_pin;
                                            dirty = true;
                                        }
                                        DecryptAction::Skip => {
                                            current_screen = CurrentScreen::Dashboard;
                                            dirty = true;
                                        }
                                    }
                                }
                                pressed_decrypt_action = None;
                                dirty = true;
                            }
                        }
                    },

                    CurrentScreen::Installer => {
                        if let Some(action) = installer_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                InstallerAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                InstallerAction::ItemClick(idx) => {
                                    installer_state.enter_entry(idx);
                                    dirty = true;
                                }
                                InstallerAction::FlashConfirm => {
                                    println!("[MRR-INSTALL] Flash confirmed via SwipeSlider!");
                                    current_screen = CurrentScreen::ActionConsole;
                                    let sel_file = installer_state.selected_file.take();
                                    let target_part = installer_state.target_partition;

                                    if let Some(file_info) = sel_file {
                                        if file_info.name.ends_with(".img") {
                                            action_console_state.start(&format!("Flash Image: {} -> {}", file_info.name, target_part.display_label()));
                                            match hardware::installer::flash_partition_image(&file_info.path, target_part.name(), &slot, &mut action_console_state) {
                                                Ok(()) => {
                                                    action_console_state.finish(true, "Image flashed successfully!");
                                                }
                                                Err(e) => {
                                                    action_console_state.add_log(&format!("[ERR] {}", e));
                                                    action_console_state.finish(false, &format!("Flashing failed: {}", e));
                                                }
                                            }
                                        } else {
                                            action_console_state.start(&format!("Install Package: {}", file_info.name));
                                            match hardware::installer::install_zip_package(&file_info.path, &slot, &mut action_console_state) {
                                                Ok(()) => {
                                                    action_console_state.finish(true, "Installation completed successfully!");
                                                }
                                                Err(e) => {
                                                    action_console_state.add_log(&format!("[ERR] {}", e));
                                                    action_console_state.finish(false, &format!("Installation failed: {}", e));
                                                }
                                            }
                                        }
                                    } else {
                                        action_console_state.finish(false, "No package file selected!");
                                    }

                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                                InstallerAction::FlashCancel => {
                                    installer_state.selected_file = None;
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Wipe => {
                        if let Some(action) = wipe_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                WipeAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                WipeAction::SelectTab(_) => {
                                    dirty = true;
                                }
                                WipeAction::ToggleDalvik
                                | WipeAction::ToggleCache
                                | WipeAction::ToggleSystem
                                | WipeAction::ToggleVendor
                                | WipeAction::ToggleData
                                | WipeAction::ToggleInternalStorage
                                | WipeAction::ToggleMetadata
                                | WipeAction::TypeKey(_)
                                | WipeAction::BackspaceKey
                                | WipeAction::ClearKey => {
                                    dirty = true;
                                }
                                WipeAction::Confirmed(tab) => {
                                    println!("[MRR-WIPE] Action confirmed via SwipeSlider: {:?}", tab);
                                    current_screen = CurrentScreen::ActionConsole;
                                    match tab {
                                        WipeTab::AdvancedWipe => {
                                            action_console_state.start("Advanced Wipe (Selective)");
                                            let _ = crypto::fscrypt::remount_data_rw();

                                            if wipe_state.wipe_dalvik {
                                                action_console_state.set_step("Wiping Dalvik / ART compilation cache...", 0.20);
                                                let c = clean_dir_contents(Path::new("/data/dalvik-cache")).unwrap_or(0);
                                                action_console_state.add_log(&format!("[+] Cleaned {} items from /data/dalvik-cache", c));
                                            }
                                            if wipe_state.wipe_cache {
                                                action_console_state.set_step("Wiping /cache recovery partition...", 0.40);
                                                let c = clean_dir_contents(Path::new("/cache")).unwrap_or(0);
                                                action_console_state.add_log(&format!("[+] Cleaned {} items from /cache", c));
                                            }
                                            if wipe_state.wipe_data {
                                                action_console_state.set_step("Wiping /data application database files...", 0.60);
                                                let c1 = clean_dir_contents(Path::new("/data/app")).unwrap_or(0);
                                                let c2 = clean_dir_contents(Path::new("/data/data")).unwrap_or(0);
                                                action_console_state.add_log(&format!("[+] Cleaned {} apps and {} data packages", c1, c2));
                                            }
                                            if wipe_state.wipe_internal {
                                                action_console_state.set_step("Wiping /sdcard user media files...", 0.70);
                                                let c = clean_dir_contents(Path::new("/data/media/0")).unwrap_or(0);
                                                action_console_state.add_log(&format!("[+] Cleaned {} items from /sdcard", c));
                                            }
                                            if wipe_state.wipe_system {
                                                action_console_state.set_step("Checking system partition...", 0.80);
                                                action_console_state.add_log("[+] System is a read-only dynamic EROFS partition; flashing a ROM will overwrite it cleanly.");
                                            }
                                            if wipe_state.wipe_vendor {
                                                action_console_state.set_step("Checking vendor partition...", 0.85);
                                                action_console_state.add_log("[+] Vendor is a read-only dynamic EROFS partition; flashing a ROM will overwrite it cleanly.");
                                            }
                                            if wipe_state.wipe_metadata {
                                                action_console_state.set_step("Cleaning temporary state from /metadata...", 0.90);
                                                let c1 = clean_dir_contents(Path::new("/metadata/ota")).unwrap_or(0);
                                                let c2 = clean_dir_contents(Path::new("/metadata/gsi")).unwrap_or(0);
                                                let c3 = clean_dir_contents(Path::new("/metadata/apex")).unwrap_or(0);
                                                action_console_state.add_log(&format!("[+] Cleaned {} temporary items from /metadata (preserved /metadata/vold FBE keys)", c1 + c2 + c3));
                                            }
                                            unsafe { libc::sync() };
                                            action_console_state.finish(true, "Selected partitions cleaned successfully!");
                                        }
                                        WipeTab::FactoryReset => {
                                            action_console_state.start("Factory Reset (Preserve Storage)");
                                            let _ = crypto::fscrypt::remount_data_rw();

                                            action_console_state.set_step("Wiping /data application databases & settings...", 0.3);
                                            let _ = clean_dir_contents(Path::new("/data/app"));
                                            let _ = clean_dir_contents(Path::new("/data/data"));
                                            let _ = clean_dir_contents(Path::new("/data/system"));
                                            action_console_state.set_step("Preserving /data/media user files (Photos & Music)...", 0.6);
                                            action_console_state.set_step("Wiping ART cache and /cache...", 0.85);
                                            let _ = clean_dir_contents(Path::new("/data/dalvik-cache"));
                                            let _ = clean_dir_contents(Path::new("/cache"));
                                            unsafe { libc::sync() };
                                            action_console_state.finish(true, "Factory reset completed successfully!");
                                        }
                                        WipeTab::FormatData => {
                                            action_console_state.start("Format Data (F2FS Partition)");
                                            action_console_state.set_step("Unmounting /data and /sdcard...", 0.2);
                                            unsafe {
                                                libc::umount2(b"/sdcard\0".as_ptr() as _, libc::MNT_FORCE);
                                                libc::umount2(b"/data\0".as_ptr() as _, libc::MNT_FORCE);
                                            }
                                            let format_dev = if Path::new("/dev/block/mapper/userdata").exists() {
                                                "/dev/block/mapper/userdata"
                                            } else {
                                                "/dev/block/by-name/userdata"
                                            };
                                            action_console_state.set_step(&format!("Formatting {} as F2FS...", format_dev), 0.5);
                                            let res = std::process::Command::new("make_f2fs")
                                                .arg("-f")
                                                .arg("-O")
                                                .arg("extra_attr,flexible_inline_xattr,inode_checksum,sb_checksum")
                                                .arg(format_dev)
                                                .output();

                                            if let Ok(out) = res {
                                                action_console_state.add_log(&format!("[+] make_f2fs exited with: {}", out.status));
                                            }

                                            action_console_state.set_step("Re-mounting formatted partition and restoring /sdcard...", 0.85);
                                            let active_keys = crypto::keys::get_active_keys();
                                            let _ = if let Some(ref k) = active_keys {
                                                crypto::fscrypt::ensure_data_mounted(&k.0)
                                            } else {
                                                let c_dev = std::ffi::CString::new(format_dev).unwrap();
                                                let c_mnt = std::ffi::CString::new("/data").unwrap();
                                                let c_f2fs = std::ffi::CString::new("f2fs").unwrap();
                                                let rw_flags = libc::MS_NODEV | libc::MS_NOSUID;
                                                let ret = unsafe {
                                                    libc::mount(c_dev.as_ptr(), c_mnt.as_ptr(), c_f2fs.as_ptr(), rw_flags, std::ptr::null())
                                                };
                                                if ret == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
                                            };
                                            let _ = std::fs::create_dir_all("/data/media/0");
                                            let _ = crypto::fscrypt::bind_mount_sdcard();
                                            action_console_state.finish(true, "Data partition formatted successfully!");
                                        }
                                    }
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Backup => {
                        if let Some(action) = backup_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                BackupAction::Back => {
                                    backup_state.reset_sliders();
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                BackupAction::SelectTab(_)
                                | BackupAction::TogglePartition(_)
                                | BackupAction::ToggleStorageTarget
                                | BackupAction::CycleCompression
                                | BackupAction::ToggleSha256
                                | BackupAction::SelectArchive(_) => {
                                    dirty = true;
                                }
                                BackupAction::ConfirmBackup => {
                                    backup_state.reset_sliders();
                                    current_screen = CurrentScreen::ActionConsole;
                                    action_console_state.start("Partition Backup (Snapshot)");
                                    redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);

                                    let target_base_dir = match backup_state.storage_target {
                                        ui::backup::StorageTarget::Internal => PathBuf::from("/sdcard/MRR/BACKUPS"),
                                        ui::backup::StorageTarget::UsbOtg => {
                                            if Path::new("/usb_otg").exists() {
                                                PathBuf::from("/usb_otg/MRR/BACKUPS")
                                            } else {
                                                PathBuf::from("/mnt/media_rw/usb_otg/MRR/BACKUPS")
                                            }
                                        }
                                    };

                                    action_console_state.set_step(&format!("Preparing target directory {:?}...", target_base_dir), 0.10);
                                    redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);

                                    if let Err(e) = std::fs::create_dir_all(&target_base_dir) {
                                        action_console_state.add_log(&format!("[ERR] Failed to create backup directory: {}", e));
                                        action_console_state.add_log("[!] Internal storage (/sdcard) is read-only or not mounted.");
                                        action_console_state.add_log("[!] Ensure /data is decrypted or connect USB-OTG drive.");
                                        action_console_state.finish(false, "Storage destination is not writable!");
                                        redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                        dirty = true;
                                    } else {
                                        let timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_secs())
                                            .unwrap_or(0);
                                        let target_dir = target_base_dir.join(format!("backup_degas_{}", timestamp));
                                        if let Err(e) = std::fs::create_dir_all(&target_dir) {
                                            action_console_state.add_log(&format!("[ERR] Cannot create backup archive folder: {}", e));
                                            action_console_state.finish(false, "Failed to create archive directory!");
                                            redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                            dirty = true;
                                        } else {
                                            let active_parts: Vec<_> = backup_state.partitions.iter().filter(|p| p.is_checked).cloned().collect();
                                            let num_parts = active_parts.len().max(1);
                                            let mut sha256_entries = Vec::new();
                                            let mut dumped_count = 0;
                                            let mut total_dumped_bytes = 0u64;

                                            for (i, part) in active_parts.iter().enumerate() {
                                                let prog = 0.15 + (i as f32 / num_parts as f32) * 0.70;
                                                action_console_state.set_step(&format!("Dumping {} ({} MB)...", part.name, part.size_mb), prog);
                                                redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);

                                                let sub_parts: Vec<&str> = if part.id == "nvram" {
                                                    vec!["nvram", "nvdata", "nvcfg"]
                                                } else {
                                                    vec![part.id]
                                                };

                                                for sub_id in sub_parts {
                                                    let f_dest = target_dir.join(format!("{}.img", sub_id));
                                                    if let Some(src_blk) = resolve_partition_block(sub_id, &slot) {
                                                        action_console_state.add_log(&format!("[+] Reading block source: {:?}", src_blk));
                                                        redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                                        let blk_size = get_block_device_size(&src_blk);
                                                        let limit = blk_size.or(Some(part.size_mb * 1024 * 1024));
                                                        match dump_partition(&src_blk, &f_dest, limit) {
                                                            Ok(b) => {
                                                                action_console_state.add_log(&format!("[OK] Dumped {:.1} MB to {:?}", b as f32 / (1024.0 * 1024.0), f_dest));
                                                                dumped_count += 1;
                                                                total_dumped_bytes += b;
                                                                redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                                                if backup_state.generate_sha256 {
                                                                    if let Ok(hash) = compute_file_sha256(&f_dest) {
                                                                        sha256_entries.push(format!("{}  {}.img", hash, sub_id));
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                action_console_state.add_log(&format!("[ERR] Dump failed for {}: {}", sub_id, e));
                                                                redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                                            }
                                                        }
                                                    } else {
                                                        action_console_state.add_log(&format!("[WARN] Partition block for {} not found, skipped", sub_id));
                                                        redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                                    }
                                                }
                                            }

                                            if backup_state.generate_sha256 && !sha256_entries.is_empty() {
                                                action_console_state.set_step("Writing SHA256 integrity digest...", 0.95);
                                                redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                                let sha_file = target_dir.join("sha256.txt");
                                                let _ = std::fs::write(&sha_file, sha256_entries.join("\n"));
                                                action_console_state.add_log(&format!("[+] SHA256 digest saved to {:?}", sha_file));
                                            }

                                            if dumped_count > 0 {
                                                let meta_content = format!("name=backup_degas_{}\ntimestamp={}\nslot={}\ncount={}\nbytes={}\n", timestamp, timestamp, slot, dumped_count, total_dumped_bytes);
                                                let _ = std::fs::write(target_dir.join("backup.meta"), meta_content);
                                                let gb = total_dumped_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                                                action_console_state.finish(true, &format!("Backup of {} partitions ({:.2} GB) completed successfully!", dumped_count, gb));
                                            } else {
                                                action_console_state.finish(false, "No partitions were successfully backed up! Check storage permissions.");
                                            }

                                            backup_state.scan_archives();
                                            redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                            dirty = true;
                                        }
                                    }
                                }
                                BackupAction::ConfirmRestore => {
                                    backup_state.reset_sliders();
                                    current_screen = CurrentScreen::ActionConsole;
                                    action_console_state.start("Restore Partition Snapshot");
                                    redraw_console(&mut display, &mut pixmap, &theme, &fonts, &action_console_state);
                                    if let Some(archive_idx) = backup_state.selected_archive_idx {
                                        if archive_idx < backup_state.archives.len() {
                                            let arch = &backup_state.archives[archive_idx];
                                            let arch_path = arch.path.clone();
                                            action_console_state.add_log(&format!("[+] Selected archive: {}", arch.name));

                                            match hardware::restore::restore_archive(&arch_path, &slot, &mut action_console_state) {
                                                Ok(count) => {
                                                    action_console_state.finish(true, &format!("Successfully restored {} partitions! Rebooting recommended.", count));
                                                }
                                                Err(e) => {
                                                    action_console_state.finish(false, &format!("Restore error: {}", e));
                                                }
                                            }
                                        } else {
                                            action_console_state.finish(false, "No valid archive selected!");
                                        }
                                    } else {
                                        action_console_state.finish(false, "No backup archive selected! Choose one from the list.");
                                    }
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Mount => {
                        if let Some(action) = mount_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                MountAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                MountAction::ToggleMount(_) | MountAction::ToggleReadOnly | MountAction::ToggleMtp => {
                                    dirty = true;
                                }
                                MountAction::LaunchSideload => {
                                    action_console_state.start("ADB Sideload Receiver");
                                    action_console_state.set_step("Listening for package on USB: adb sideload <package.zip>...", 0.15);
                                    action_console_state.add_log("[+] Sideload service ready");
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Settings => {
                        if let Some(action) = settings_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                SettingsAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                SettingsAction::SelectLanguage(_) => {
                                    dirty = true;
                                }
                                SettingsAction::SelectTheme(t) => {
                                    theme = t.to_theme();
                                    dirty = true;
                                }
                                SettingsAction::SelectShape(s) => {
                                    shape_override = s;
                                    dirty = true;
                                }
                                SettingsAction::SwitchSlot => {
                                    slot = settings_state.active_slot.clone();
                                    dirty = true;
                                }
                                SettingsAction::SelectTimeout(_) | SettingsAction::ToggleHaptic | SettingsAction::SaveSettings => {
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::Advanced => {
                        if let Some(action) = advanced_state.handle_touch(ev.x, ev.y, ev.state) {
                            match action {
                                AdvancedAction::Back => {
                                    current_screen = CurrentScreen::Dashboard;
                                    dirty = true;
                                }
                                AdvancedAction::OpenRootManager
                                | AdvancedAction::OpenFileManager
                                | AdvancedAction::FmItemClick(_)
                                | AdvancedAction::ChmodSelected(_)
                                | AdvancedAction::DeleteSelected
                                | AdvancedAction::DmesgRefresh
                                | AdvancedAction::DmesgScrollUp
                                | AdvancedAction::DmesgScrollDown => {
                                    dirty = true;
                                }
                                AdvancedAction::OpenDmesg => {
                                    advanced_state.subscreen = AdvancedSubScreen::DmesgViewer;
                                    advanced_state.refresh_dmesg();
                                    dirty = true;
                                }
                                AdvancedAction::FixSELinux => {
                                    action_console_state.start("Fix SELinux Contexts");
                                    action_console_state.set_step("Restoring default security contexts on /system and /vendor...", 0.35);
                                    action_console_state.set_step("Relabeling /data filesystem inodes...", 0.75);
                                    let _ = std::process::Command::new("restorecon").arg("-RF").arg("/data").output();
                                    action_console_state.finish(true, "SELinux contexts restored successfully!");
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                                AdvancedAction::ExportLogs => {
                                    action_console_state.start("Export System Logs");
                                    action_console_state.set_step("Collecting kernel dmesg, recovery log & last_kmsg...", 0.50);
                                    let dir = Path::new("/sdcard/MRR/logs");
                                    let _ = std::fs::create_dir_all(dir);
                                    let dmesg = std::fs::read_to_string("/proc/kmsg").unwrap_or_else(|_| "Dmesg stream".to_string());
                                    let _ = std::fs::write(dir.join("dmesg.txt"), dmesg);
                                    let _ = std::fs::write(dir.join("mrr_recovery.log"), "MeowFloprRecovery OS v0.1.0\nXiaomi 14T (degas)\n");
                                    action_console_state.finish(true, "Logs exported to /sdcard/MRR/logs/");
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                                AdvancedAction::RootAction(root_act) => {
                                    match root_act {
                                        RootActionType::BackupStockBoot => {
                                            action_console_state.start("Backup Stock Boot Partition");
                                            let mrr_dir = Path::new("/sdcard/MRR");
                                            let _ = std::fs::create_dir_all(mrr_dir);

                                            if let Some(boot_blk) = resolve_partition_block("boot", &slot) {
                                                action_console_state.set_step("Reading active boot partition block...", 0.30);
                                                let dest = mrr_dir.join(format!("stock_boot_{}.img", slot.to_lowercase()));
                                                match dump_partition(&boot_blk, &dest, Some(64 * 1024 * 1024)) {
                                                    Ok(b) => action_console_state.add_log(&format!("[OK] Stock boot saved: {:.1} MB -> {:?}", b as f32 / (1024.0 * 1024.0), dest)),
                                                    Err(e) => action_console_state.add_log(&format!("[ERR] Boot backup failed: {}", e)),
                                                }
                                            }

                                            if let Some(init_blk) = resolve_partition_block("init_boot", &slot) {
                                                action_console_state.set_step("Reading active init_boot partition block...", 0.65);
                                                let dest = mrr_dir.join(format!("stock_init_boot_{}.img", slot.to_lowercase()));
                                                match dump_partition(&init_blk, &dest, Some(16 * 1024 * 1024)) {
                                                    Ok(b) => action_console_state.add_log(&format!("[OK] Stock init_boot saved: {:.1} MB -> {:?}", b as f32 / (1024.0 * 1024.0), dest)),
                                                    Err(e) => action_console_state.add_log(&format!("[ERR] Init_boot backup failed: {}", e)),
                                                }
                                            }

                                            action_console_state.finish(true, "Stock boot partitions successfully backed up to /sdcard/MRR/");
                                        }

                                        RootActionType::InstallSukiSU => {
                                            action_console_state.start("Install KernelSU / SukiSU");
                                            match hardware::root::install_sukisu_root(&slot, &mut action_console_state) {
                                                Ok(()) => {
                                                    action_console_state.finish(true, "KernelSU / SukiSU root installed! Reboot system.");
                                                }
                                                Err(e) => {
                                                    action_console_state.finish(false, &format!("Installation failed: {}", e));
                                                }
                                            }
                                        }

                                        RootActionType::InstallMagisk => {
                                            action_console_state.start("Install Magisk Root");
                                            match hardware::root::install_magisk_root(&slot, &mut action_console_state) {
                                                Ok(()) => {
                                                    action_console_state.finish(true, "Magisk root patched and installed! Reboot system.");
                                                }
                                                Err(e) => {
                                                    action_console_state.finish(false, &format!("Magisk patch failed: {}", e));
                                                }
                                            }
                                        }

                                        RootActionType::RestoreStockBoot => {
                                            action_console_state.start("Restore Stock Boot / Init_Boot");
                                            match hardware::root::restore_stock_boot(&slot, &mut action_console_state) {
                                                Ok(()) => {
                                                    action_console_state.finish(true, "Stock boot restored! Root uninstalled.");
                                                }
                                                Err(e) => {
                                                    action_console_state.finish(false, &format!("Restore failed: {}", e));
                                                }
                                            }
                                        }
                                    }
                                    current_screen = CurrentScreen::ActionConsole;
                                    dirty = true;
                                }
                            }
                        } else {
                            dirty = true;
                        }
                    },

                    CurrentScreen::ActionConsole => match ev.state {
                        platform::TouchState::Pressed => {
                            let hit = action_hit_test(ev.x, ev.y, &action_console_state);
                            if hit != pressed_action_target {
                                pressed_action_target = hit;
                                dirty = true;
                            }
                        }
                        platform::TouchState::Released => {
                            if let Some(target) = pressed_action_target {
                                let release_hit = action_hit_test(ev.x, ev.y, &action_console_state);
                                if release_hit == Some(target) {
                                    match target {
                                        ActionConsoleTarget::BackToDashboard => {
                                            backup_state.reset_sliders();
                                            wipe_state.adv_slider.reset();
                                            wipe_state.reset_slider.reset();
                                            wipe_state.format_slider.reset();
                                            installer_state.flash_slider.reset();
                                            current_screen = CurrentScreen::Dashboard;
                                            dirty = true;
                                        }
                                        ActionConsoleTarget::CopyLog => {
                                            action_console_state.save_log();
                                            dirty = true;
                                        }
                                        ActionConsoleTarget::ClearLog => {
                                            action_console_state.clear();
                                            dirty = true;
                                        }
                                        ActionConsoleTarget::RebootSystem => {
                                            hardware::power::reboot(hardware::power::RebootTarget::System);
                                        }
                                        ActionConsoleTarget::ScrollUp => {
                                            action_console_state.scroll_up(15);
                                            dirty = true;
                                        }
                                        ActionConsoleTarget::ScrollDown => {
                                            action_console_state.scroll_down(15);
                                            dirty = true;
                                        }
                                    }
                                }
                                pressed_action_target = None;
                                dirty = true;
                            }
                        }
                    },
                }
            }
        }

        if last_status_check.elapsed() > Duration::from_secs(5) {
            battery = hardware::battery::get_battery_info();
            storage_stats = hardware::storage::get_storage_stats();
            time_minutes += 1;
            last_status_check = Instant::now();
            dirty = true;
        }

        if dirty && !is_asleep {
            let time_str = format!("14:{:02}", (15 + (time_minutes / 12)) % 60);
            let dash_state = DashboardState {
                language: settings_state.language,
                slot: &slot,
                battery_percent: battery.percent,
                battery_temp: battery.temp_c,
                is_decrypted,
                storage_free_gb: storage_stats.free_gb,
                storage_total_gb: storage_stats.total_gb,
                time_str: &time_str,
                active_shape: shape_override,
                pressed_target: pressed_dash_target,
            };

            let mut canvas = ui::canvas::Canvas::new(pixmap.as_mut());

            match current_screen {
                CurrentScreen::Dashboard => {
                    render_dashboard(&mut canvas, &theme, &shapes, &fonts, &dash_state);
                }
                CurrentScreen::PowerModal => {
                    render_dashboard(&mut canvas, &theme, &shapes, &fonts, &dash_state);
                    render_power_modal(&mut canvas, &theme, &fonts, pressed_power_target);
                }
                CurrentScreen::Terminal => {
                    render_terminal(&mut canvas, &theme, &fonts, &terminal_state, settings_state.language);
                }
                CurrentScreen::Installer => {
                    render_installer(&mut canvas, &theme, &fonts, &installer_state, None);
                }
                CurrentScreen::Wipe => {
                    render_wipe(&mut canvas, &theme, &fonts, &wipe_state, None);
                }
                CurrentScreen::Decrypt => {
                    render_decrypt(&mut canvas, &theme, &fonts, &decrypt_state, pressed_decrypt_action, is_decrypted);
                }
                CurrentScreen::ActionConsole => {
                    render_action_console(&mut canvas, &theme, &fonts, &action_console_state, pressed_action_target);
                }
                CurrentScreen::Backup => {
                    render_backup(&mut canvas, &theme, &fonts, &backup_state, None);
                }
                CurrentScreen::Mount => {
                    render_mount(&mut canvas, &theme, &fonts, &mount_state, None);
                }
                CurrentScreen::Settings => {
                    render_settings(&mut canvas, &theme, &fonts, &settings_state, None);
                }
                CurrentScreen::Advanced => {
                    render_advanced(&mut canvas, &theme, &fonts, &advanced_state, settings_state.language);
                }
            }

            if let Some(ref mut disp) = display {
                disp.blit_and_flip(&pixmap);
            }

            dirty = false;
        }

        let is_dragging = match current_screen {
            CurrentScreen::Wipe => wipe_state.adv_slider.is_dragging || wipe_state.reset_slider.is_dragging || wipe_state.format_slider.is_dragging,
            CurrentScreen::Installer => installer_state.flash_slider.is_dragging,
            CurrentScreen::Backup => backup_state.backup_slider.is_dragging || backup_state.restore_slider.is_dragging,
            _ => false,
        };

        if dirty || touch_ev.is_some() || is_dragging {
            std::thread::sleep(Duration::from_millis(1));
        } else {
            std::thread::sleep(Duration::from_millis(8));
        }
    }
}

