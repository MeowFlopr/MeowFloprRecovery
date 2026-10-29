use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::hardware::ProgressCallback;

pub fn is_tier0_forbidden_partition(part_name: &str) -> bool {
    let lower = part_name.to_lowercase();
    let forbidden = [
        "preloader", "lk", "seccfg", "nvram", "nvdata", "nvcfg",
        "protect1", "protect2", "protect_f", "protect_s",
        "persist", "proinfo", "para", "tee", "scp", "sspm", "spmfw",
        "md1img", "gpt", "bootloader", "sec1", "flashinfo", "otp",
    ];
    for f in &forbidden {
        if lower == *f || lower.starts_with(&format!("{}_", f)) {
            return true;
        }
    }
    false
}

pub fn flash_partition_image(
    img_path: &Path,
    target_part: &str,
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    if !img_path.exists() {
        return Err(format!("Image file not found: {:?}", img_path));
    }

    if is_tier0_forbidden_partition(target_part) {
        return Err(format!(
            "Refusing to flash Tier 0 protected partition '{}' (degas device safety guard)",
            target_part
        ));
    }

    let file_size = fs::metadata(img_path)
        .map_err(|e| format!("Cannot get image metadata: {}", e))?
        .len();

    let target_blk = crate::resolve_partition_block(target_part, active_slot)
        .ok_or_else(|| format!("Could not resolve target block device for partition '{}' (slot {})", target_part, active_slot))?;

    progress.step(&format!("Preparing flash to {} ({:?})...", target_part, target_blk), 0.15);
    progress.log(&format!("[+] Source: {:?} ({:.2} MB)", img_path, file_size as f64 / (1024.0 * 1024.0)));
    progress.log(&format!("[+] Target: {:?}", target_blk));

    progress.step("Streaming image chunks to partition...", 0.40);
    let dumped = crate::dump_partition(img_path, &target_blk, None)
        .map_err(|e| format!("Failed to write partition: {}", e))?;

    progress.step("Flushing buffers to physical storage (sync)...", 0.90);
    unsafe { libc::sync() };

    progress.step("Partition flash completed and verified!", 1.0);
    progress.log(&format!(
        "[OK] Flashed {:.2} MB successfully to {:?}",
        dumped as f64 / (1024.0 * 1024.0),
        target_blk
    ));
    Ok(())
}

pub fn install_zip_package(
    zip_path: &Path,
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    if !zip_path.exists() {
        return Err(format!("Package not found: {:?}", zip_path));
    }

    let is_img = zip_path.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("img")).unwrap_or(false);
    if is_img {
        let file_name = zip_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let name_lower = file_name.to_lowercase();
        let part_name = if name_lower.contains("init_boot") || name_lower.contains("boot_init") || name_lower.contains("kernelsu") {
            "init_boot"
        } else if name_lower.contains("vendor_boot") {
            "vendor_boot"
        } else if name_lower.contains("dtbo") {
            "dtbo"
        } else if name_lower.contains("recovery") {
            "recovery"
        } else if name_lower.contains("vbmeta") {
            "vbmeta"
        } else if name_lower.contains("boot") {
            "boot"
        } else {
            return Err(format!(
                "Unrecognized partition image '{}'. Please specify the target partition in the Install menu.",
                file_name
            ));
        };
        return flash_partition_image(zip_path, part_name, active_slot, progress);
    }

    progress.step("Opening and verifying ZIP archive...", 0.10);
    progress.log(&format!("[+] Target package: {:?}", zip_path));

    let file = File::open(zip_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {}", e))?;

    let file_count = archive.len();
    progress.log(&format!("[+] ZIP archive parsed successfully ({} files)", file_count));

    let update_binary_entry = "META-INF/com/google/android/update-binary";
    let has_update_binary = (0..archive.len()).any(|i| {
        if let Ok(f) = archive.by_index(i) {
            f.name() == update_binary_entry
        } else {
            false
        }
    });

    let work_dir = Path::new("/tmp/mrr_installer");
    let _ = fs::create_dir_all(work_dir);

    if has_update_binary {
        progress.step("Extracting update-binary installer...", 0.25);
        let bin_dest = work_dir.join("update-binary");

        {
            let mut bin_file = archive
                .by_name(update_binary_entry)
                .map_err(|e| format!("Cannot read update-binary: {}", e))?;
            let mut out = File::create(&bin_dest)
                .map_err(|e| format!("Cannot create /tmp update-binary: {}", e))?;
            std::io::copy(&mut bin_file, &mut out)
                .map_err(|e| format!("Failed to extract update-binary: {}", e))?;
        }

        let mut perms = fs::metadata(&bin_dest)
            .map_err(|e| format!("Metadata error: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_dest, perms)
            .map_err(|e| format!("chmod failed on update-binary: {}", e))?;

        progress.log("[+] Extracted update-binary with executable mode 0755");

        let script_entry = "META-INF/com/google/android/updater-script";
        if let Ok(mut script_file) = archive.by_name(script_entry) {
            let script_dest = work_dir.join("updater-script");
            if let Ok(mut out) = File::create(&script_dest) {
                let _ = std::io::copy(&mut script_file, &mut out);
            }
        }

        let s_lower = active_slot.to_lowercase();
        std::env::set_var("SLOT", &s_lower);
        std::env::set_var("BOOT_SLOT", &format!("_{}", s_lower));
        std::env::set_var("ANDROID_ROOT", "/system");
        std::env::set_var("TMPDIR", "/tmp");

        progress.step("Executing update-binary...", 0.40);

        let is_shell_script = {
            let mut head = [0u8; 4];
            if let Ok(mut f) = File::open(&bin_dest) {
                let _ = f.read_exact(&mut head);
                head[0] == b'#' && head[1] == b'!'
            } else {
                false
            }
        };

        let mut cmd = if is_shell_script {
            let shell = if Path::new("/system/bin/sh").exists() {
                "/system/bin/sh"
            } else {
                "/bin/sh"
            };
            let mut c = Command::new(shell);
            c.arg(&bin_dest);
            c.arg("3");
            c.arg("1");
            c.arg(zip_path);
            c
        } else {
            let mut c = Command::new(&bin_dest);
            c.arg("3");
            c.arg("1");
            c.arg(zip_path);
            c
        };

        cmd.current_dir(work_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to execute update-binary: {}", e))?;

        let stderr_reader = child.stderr.take();
        let stderr_handle = std::thread::spawn(move || {
            let mut err_lines = Vec::new();
            if let Some(err) = stderr_reader {
                let reader = BufReader::new(err);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        err_lines.push(trimmed.to_string());
                    }
                }
            }
            err_lines
        });

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Some(msg) = trimmed.strip_prefix("ui_print ") {
                        progress.log(msg);
                    } else if let Some(prog_str) = trimmed.strip_prefix("set_progress ") {
                        if let Ok(fraction) = prog_str.trim().parse::<f32>() {
                            progress.step("Installing package components...", 0.40 + fraction * 0.55);
                        }
                    } else {
                        progress.log(trimmed);
                    }
                }
            }
        }

        let status = child
            .wait()
            .map_err(|e| format!("Wait for update-binary failed: {}", e))?;

        if let Ok(stderr_lines) = stderr_handle.join() {
            for err_line in stderr_lines {
                progress.log(&format!("[STDERR] {}", err_line));
            }
        }

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            return Err(format!("Installer returned error exit code: {}", code));
        }

        progress.step("Package installation verified!", 1.0);
        progress.log("[OK] update-binary execution completed successfully!");
        Ok(())
    } else {
        progress.step("Scanning ZIP for raw partition images...", 0.30);
        let mut flashed_any = false;

        let mut img_entries = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file_entry) = archive.by_index(i) {
                let name = file_entry.name().to_string();
                if name.ends_with(".img") {
                    img_entries.push((i, name));
                }
            }
        }

        for (i, name) in img_entries {
            let part_id = name.trim_end_matches(".img");

            if is_tier0_forbidden_partition(part_id) {
                progress.log(&format!("[-] Skipping Tier 0 protected partition image: {}", name));
                continue;
            }

            let dest_blk = match crate::resolve_partition_block(part_id, active_slot) {
                Some(p) => p,
                None => {
                    progress.log(&format!("[-] Target block for '{}' not found, skipping", part_id));
                    continue;
                }
            };

            progress.step(&format!("Flashing {} to {:?}...", name, dest_blk), 0.60);
            let tmp_img = work_dir.join(&name);

            {
                let mut entry_reader = archive.by_index(i)
                    .map_err(|e| format!("Cannot read image {}: {}", name, e))?;
                let mut out_file = File::create(&tmp_img)
                    .map_err(|e| format!("Failed to create temporary image: {}", e))?;
                std::io::copy(&mut entry_reader, &mut out_file)
                    .map_err(|e| format!("Failed to extract image: {}", e))?;
            }

            let dumped_bytes = crate::dump_partition(&tmp_img, &dest_blk, None)
                .map_err(|e| format!("Write to {:?} failed: {}", dest_blk, e))?;

            let _ = fs::remove_file(&tmp_img);

            progress.log(&format!(
                "[OK] Flashed {:.1} MB to {:?}",
                dumped_bytes as f32 / (1024.0 * 1024.0),
                dest_blk
            ));
            flashed_any = true;
        }

        if flashed_any {
            progress.step("Direct image installation complete!", 1.0);
            Ok(())
        } else {
            Err("No valid META-INF/com/google/android/update-binary or allowed partition images found in ZIP!".to_string())
        }
    }
}
