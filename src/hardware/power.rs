use std::ffi::CString;

#[derive(Debug, Clone, Copy)]
pub enum RebootTarget {
    System,
    Recovery,
    Bootloader,
    FastbootD,
    PowerOff,
}

pub fn clear_bcb() {
    println!("[MRR-POWER] Clearing Bootloader Control Block (BCB) on /dev/block/by-name/misc...");
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open("/dev/block/by-name/misc") {
        use std::io::Write;
        let zeroes = [0u8; 2048];
        let _ = f.write_all(&zeroes);
        let _ = f.flush();
    }
}

pub fn reboot(target: RebootTarget) -> ! {
    println!("[MRR-POWER] Rebooting device into {:?}...", target);

    // Sync filesystems
    unsafe { libc::sync() };

    match target {
        RebootTarget::System => {
            clear_bcb();
            unsafe {
                libc::reboot(libc::RB_AUTOBOOT);
            }
        }
        RebootTarget::PowerOff => {
            clear_bcb();
            unsafe {
                libc::reboot(libc::RB_POWER_OFF);
            }
        }
        RebootTarget::Recovery => {
            reboot_with_cmd("recovery");
        }
        RebootTarget::Bootloader => {
            clear_bcb();
            reboot_with_cmd("bootloader");
        }
        RebootTarget::FastbootD => {
            clear_bcb();
            reboot_with_cmd("fastboot");
        }
    }

    // Fallback infinite loop if reboot syscall returns
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn reboot_with_cmd(cmd: &str) {
    let c_cmd = CString::new(cmd).unwrap();
    unsafe {
        // LINUX_REBOOT_CMD_RESTART2 = 0xA1B2C3D4
        libc::syscall(
            libc::SYS_reboot,
            libc::LINUX_REBOOT_MAGIC1,
            libc::LINUX_REBOOT_MAGIC2,
            0xA1B2C3D4u32,
            c_cmd.as_ptr(),
        );
    }
}
