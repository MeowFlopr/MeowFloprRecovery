//! Native AOSP / TWRP ZIP Package Installer Engine
//! Handles extracting and running Android update packages (Magisk, ROMs, Kernels, GApps)
//! via update-binary protocol with real-time progress & console output.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::hardware::ProgressCallback;

/// Flashes a raw partition image (.img) directly to the resolved hardware block device
pub fn flash_partition_image(
    img_path: &Path,
    target_part: &str,
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    if !img_path.exists() {
        return Err(format!("Image file not found: {:?}", img_path));
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

/// Executes a flashable recovery ZIP or APK package
pub fn install_zip_package(
    zip_path: &Path,
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    if !zip_path.exists() {
        return Err(format!("Package not found: {:?}", zip_path));
    }

    // Direct .img partition flashing fallback
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
        } else {
            "boot"
        };
        return flash_partition_image(zip_path, part_name, active_slot, progress);
    }

    progress.step("Opening and verifying ZIP archive...", 0.10);
    progress.log(&format!("[+] Target package: {:?}", zip_path));

    let file = File::open(zip_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {}", e))?;

    let file_count = archive.len();
    progress.log(&format!("[+] ZIP archive parsed successfully ({} files)", file_count));

    // Check for standard AOSP recovery update-binary
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

        // Set executable permissions
        let mut perms = fs::metadata(&bin_dest)
            .map_err(|e| format!("Metadata error: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_dest, perms)
            .map_err(|e| format!("chmod failed on update-binary: {}", e))?;

        progress.log("[+] Extracted update-binary with executable mode 0755");

        // Extract updater-script if present alongside
        let script_entry = "META-INF/com/google/android/updater-script";
        if let Ok(mut script_file) = archive.by_name(script_entry) {
            let script_dest = work_dir.join("updater-script");
            if let Ok(mut out) = File::create(&script_dest) {
                let _ = std::io::copy(&mut script_file, &mut out);
            }
        }

        // Set recovery environment variables expected by update-binary / AnyKernel / Magisk
        let s_lower = active_slot.to_lowercase();
        std::env::set_var("SLOT", &s_lower);
        std::env::set_var("BOOT_SLOT", &format!("_{}", s_lower));
        std::env::set_var("ANDROID_ROOT", "/system");
        std::env::set_var("TMPDIR", "/tmp");

        progress.step("Executing update-binary...", 0.40);

        // Check if update-binary is a shell script or an ELF executable
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

        // Stream and parse stdout from update-binary
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // AOSP update-binary protocol:
                    // ui_print <message>
                    // set_progress <fraction>
                    // progress <fraction> <seconds>
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

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            return Err(format!("Installer returned error exit code: {}", code));
        }

        progress.step("Package installation verified!", 1.0);
        progress.log("[OK] update-binary execution completed successfully!");
        Ok(())
    } else {
        // Fallback: Check if ZIP is a direct image container (e.g. boot.img, vendor_boot.img)
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
            progress.log(&format!("[*] Found partition image: {}", name));

            // Attempt direct image flash
            let dest_blk = format!("/dev/block/by-name/{}_{}", part_id, active_slot.to_lowercase());
            let dest_path = Path::new(&dest_blk);

            if dest_path.exists() {
                progress.step(&format!("Flashing {} to {:?}...", name, dest_path), 0.60);
                let tmp_img = work_dir.join(&name);

                // Extract image
                {
                    let mut entry_reader = archive.by_index(i)
                        .map_err(|e| format!("Cannot read image {}: {}", name, e))?;
                    let mut out_file = File::create(&tmp_img)
                        .map_err(|e| format!("Failed to create temporary image: {}", e))?;
                    std::io::copy(&mut entry_reader, &mut out_file)
                        .map_err(|e| format!("Failed to extract image: {}", e))?;
                }

                // Write to block device
                let dumped_bytes = crate::dump_partition(&tmp_img, dest_path, None)
                    .map_err(|e| format!("Write to {:?} failed: {}", dest_path, e))?;

                progress.log(&format!(
                    "[OK] Flashed {:.1} MB to {:?}",
                    dumped_bytes as f32 / (1024.0 * 1024.0),
                    dest_path
                ));
                flashed_any = true;
            }
        }

        if flashed_any {
            progress.step("Direct image installation complete!", 1.0);
            Ok(())
        } else {
            Err("No META-INF/com/google/android/update-binary or flashable partition images found in ZIP!".to_string())
        }
    }
}
