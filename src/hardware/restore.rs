use std::fs;
use std::path::{Path, PathBuf};

use crate::hardware::installer::is_tier0_forbidden_partition;
use crate::hardware::ProgressCallback;

pub fn restore_archive(
    archive_dir: &Path,
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<usize, String> {
    if !archive_dir.exists() {
        return Err(format!("Backup folder does not exist: {:?}", archive_dir));
    }

    progress.step("Scanning backup archive for partition images...", 0.10);
    progress.log(&format!("[+] Reading backup directory: {:?}", archive_dir));

    let read_dir = fs::read_dir(archive_dir)
        .map_err(|e| format!("Failed to read archive directory: {}", e))?;

    let mut images: Vec<PathBuf> = Vec::new();
    for entry in read_dir.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension() {
                if ext == "img" {
                    images.push(p);
                }
            }
        }
    }

    if images.is_empty() {
        return Err(format!("No .img partition backups found in {:?}", archive_dir));
    }

    images.sort();
    progress.log(&format!("[+] Found {} partition images to restore", images.len()));

    let total_images = images.len();
    let mut restored_count = 0;

    for (idx, img_path) in images.iter().enumerate() {
        let file_stem = img_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let base_prog = 0.20 + (idx as f32 / total_images as f32) * 0.75;
        progress.step(
            &format!("Restoring {} ({}/{})...", file_stem, idx + 1, total_images),
            base_prog,
        );

        if is_tier0_forbidden_partition(&file_stem) {
            progress.log(&format!(
                "[-] Skipping Tier 0 protected partition: {} (degas safety guard)",
                file_stem
            ));
            continue;
        }

        let meta = fs::metadata(img_path)
            .map_err(|e| format!("Cannot read metadata for {:?}: {}", img_path, e))?;
        if meta.len() < 4096 {
            progress.log(&format!(
                "[-] Skipping invalid/corrupted small image ({:.1} KB): {:?}",
                meta.len() as f32 / 1024.0,
                img_path
            ));
            continue;
        }

        let dest_blk = crate::resolve_partition_block(&file_stem, active_slot);
        let dest_path = match dest_blk {
            Some(d) => d,
            None => {
                progress.log(&format!(
                    "[-] Warning: No matching block device found for {}, skipping",
                    file_stem
                ));
                continue;
            }
        };

        progress.log(&format!("[+] Flashing {:?} -> {:?}", img_path, dest_path));

        match crate::dump_partition(img_path, &dest_path, None) {
            Ok(bytes) => {
                let mb = bytes as f32 / (1024.0 * 1024.0);
                progress.log(&format!("[OK] Restored {:.1} MB to {:?}", mb, dest_path));
                restored_count += 1;
            }
            Err(e) => {
                progress.log(&format!("[ERR] Failed to restore {}: {}", file_stem, e));
                return Err(format!("Restore failed for {}: {}", file_stem, e));
            }
        }
    }

    progress.step("Partitions restored and synced to storage!", 1.0);
    unsafe { libc::sync() };

    Ok(restored_count)
}
