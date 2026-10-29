use std::ffi::CString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn set_bcb_fastbootd() {
    println!("[MRR-POWER] Writing boot-fastboot command to misc BCB...");
    if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open("/dev/block/by-name/misc") {
        use std::io::Write;
        let mut msg = [0u8; 2048];
        let cmd = b"boot-fastboot\0";
        msg[..cmd.len()].copy_from_slice(cmd);
        let recovery_str = b"recovery\n--fastboot\n\0";
        msg[64..64 + recovery_str.len()].copy_from_slice(recovery_str);
        let _ = f.write_all(&msg);
        let _ = f.flush();
    }
}

pub fn reboot(target: RebootTarget) -> ! {
    println!("[MRR-POWER] Rebooting device into {:?}...", target);

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
            clear_bcb();
            reboot_with_cmd("recovery");
        }
        RebootTarget::Bootloader => {
            clear_bcb();
            reboot_with_cmd("bootloader");
        }
        RebootTarget::FastbootD => {
            set_bcb_fastbootd();
            reboot_with_cmd("recovery");
        }
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn reboot_with_cmd(cmd: &str) {
    if let Ok(c_cmd) = CString::new(cmd) {
        unsafe {
            libc::syscall(
                libc::SYS_reboot,
                libc::LINUX_REBOOT_MAGIC1,
                libc::LINUX_REBOOT_MAGIC2,
                0xA1B2C3D4u32,
                c_cmd.as_ptr(),
            );
        }
    }
}
