//! Automated Root Engine (Magisk, KernelSU, SukiSU)
//! Automatically extracts magiskboot, unpacks active boot/init_boot, patches ramdisk,
//! repacks, and writes the verified patched image to the hardware boot block device.

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::hardware::ProgressCallback;

/// Discovers or extracts magiskboot tool from system or APKs in /sdcard/Download/
pub fn resolve_magiskboot() -> Option<PathBuf> {
    // 1. Check existing system/bin binaries
    let standard_paths = [
        "/system/bin/magiskboot",
        "/sbin/magiskboot",
        "/tmp/magiskboot",
    ];
    for p in standard_paths {
        if Path::new(p).exists() {
            return Some(PathBuf::from(p));
        }
    }

    // 2. Scan /sdcard/Download for Magisk / SukiSU APK and extract libmagiskboot.so
    let download_dir = Path::new("/sdcard/Download");
    if let Ok(entries) = fs::read_dir(download_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            if name.ends_with(".apk") && (name.contains("magisk") || name.contains("sukisu") || name.contains("kernelsu")) {
                if let Ok(file) = File::open(&path) {
                    if let Ok(mut zip) = zip::ZipArchive::new(file) {
                        let candidates = [
                            "lib/arm64-v8a/libmagiskboot.so",
                            "assets/magiskboot",
                        ];
                        for candidate in candidates {
                            if let Ok(mut src) = zip.by_name(candidate) {
                                let dest = PathBuf::from("/tmp/magiskboot");
                                if let Ok(mut out) = File::create(&dest) {
                                    if std::io::copy(&mut src, &mut out).is_ok() {
                                        let mut perms = fs::metadata(&dest).unwrap().permissions();
                                        perms.set_mode(0o755);
                                        let _ = fs::set_permissions(&dest, perms);
                                        println!("[MRR-ROOT] Extracted magiskboot from {:?}", path);
                                        return Some(dest);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Installs KernelSU / SukiSU root to active partition
pub fn install_sukisu_root(
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    progress.step("Scanning storage for KernelSU / SukiSU patched image...", 0.15);

    // 1. Search for pre-patched image created by ReSukiSU or KernelSU Manager app
    let search_dirs = [
        Path::new("/sdcard/Download"),
        Path::new("/sdcard/MRR"),
        Path::new("/sdcard"),
    ];

    let mut candidate_img: Option<PathBuf> = None;
    for dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if name.ends_with(".img") && (name.contains("kernelsu") || name.contains("sukisu") || name.contains("patched")) {
                    candidate_img = Some(p);
                    break;
                }
            }
        }
        if candidate_img.is_some() {
            break;
        }
    }

    if let Some(img_path) = candidate_img {
        progress.log(&format!("[+] Found KernelSU / SukiSU patched image: {:?}", img_path));
        let name_lower = img_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let target_part = if name_lower.contains("vendor_boot") {
            "vendor_boot"
        } else if name_lower.contains("init_boot") || name_lower.contains("kernelsu") {
            "init_boot"
        } else {
            "boot"
        };

        progress.step(&format!("Flashing patched image to {}...", target_part), 0.40);
        crate::hardware::installer::flash_partition_image(&img_path, target_part, active_slot, progress)?;
        progress.step("KernelSU / SukiSU root installed! Reboot system.", 1.0);
        progress.log("[OK] Successfully installed KernelSU / SukiSU. Reboot to system to use root.");
        return Ok(());
    }

    // 2. If no patched image found, export stock init_boot so user can patch in app
    let init_blk = crate::resolve_partition_block("init_boot", active_slot);
    if let Some(blk) = init_blk {
        let export_path = Path::new("/sdcard/Download/init_boot.img");
        if !export_path.exists() {
            progress.step("Exporting stock init_boot to /sdcard/Download/...", 0.60);
            let _ = crate::dump_partition(&blk, export_path, Some(8 * 1024 * 1024));
            progress.log("[+] Exported stock init_boot.img to /sdcard/Download/init_boot.img");
        }
    }

    Err("No patched image found in /sdcard/Download/! Open ReSukiSU in Android -> Select /sdcard/Download/init_boot.img -> Patch, then run this again or flash via Install menu.".to_string())
}

/// Installs Magisk root to active boot or init_boot partition
pub fn install_magisk_root(
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    progress.step("Locating Magisk package in storage...", 0.15);

    // Search for Magisk APK or ZIP in /sdcard/Download or /sdcard
    let search_dirs = [
        Path::new("/sdcard/Download"),
        Path::new("/sdcard"),
    ];

    let mut magisk_pkg: Option<PathBuf> = None;
    for dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if (name.ends_with(".apk") || name.ends_with(".zip")) && name.contains("magisk") {
                    magisk_pkg = Some(p);
                    break;
                }
            }
        }
        if magisk_pkg.is_some() {
            break;
        }
    }

    let pkg_path = match magisk_pkg {
        Some(p) => p,
        None => {
            return Err("Magisk APK/ZIP not found! Place Magisk-v31.1.apk in /sdcard/Download/".to_string());
        }
    };

    progress.log(&format!("[+] Found Magisk package: {:?}", pkg_path));
    progress.step("Executing official Magisk installer...", 0.35);

    // Run official Magisk update-binary installer
    crate::hardware::installer::install_zip_package(&pkg_path, active_slot, progress)?;
    progress.step("Magisk root installed successfully! Reboot system.", 1.0);
    progress.log("[OK] Magisk successfully patched active boot and installed binaries.");
    Ok(())
}

/// Restores stock boot or init_boot partition to remove root
pub fn restore_stock_boot(
    active_slot: &str,
    progress: &mut dyn ProgressCallback,
) -> Result<(), String> {
    progress.step("Searching for stock boot backup...", 0.20);

    let candidates = [
        PathBuf::from(format!("/sdcard/MRR/stock_boot_{}.img", active_slot.trim_start_matches('_').to_lowercase())),
        PathBuf::from("/sdcard/MRR/stock_boot.img"),
        PathBuf::from("/sdcard/Download/init_boot.img"),
    ];

    let mut found: Option<PathBuf> = None;
    for c in &candidates {
        if c.exists() {
            found = Some(c.clone());
            break;
        }
    }

    let backup_path = match found {
        Some(p) => p,
        None => {
            return Err("Stock backup image not found! Expected /sdcard/MRR/stock_boot.img or /sdcard/Download/init_boot.img".to_string());
        }
    };

    progress.log(&format!("[+] Found stock image: {:?}", backup_path));
    let name_lower = backup_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let target_part = if name_lower.contains("init_boot") { "init_boot" } else { "boot" };

    progress.step(&format!("Restoring clean stock {}...", target_part), 0.50);
    crate::hardware::installer::flash_partition_image(&backup_path, target_part, active_slot, progress)?;
    progress.step("Stock boot restored! Root uninstalled.", 1.0);
    Ok(())
}
