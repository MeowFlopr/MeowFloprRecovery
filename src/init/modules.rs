use std::fs::{self, File};
use std::os::unix::io::AsRawFd;
use std::path::Path;

pub fn load_kernel_module(path: &str) -> Result<(), std::io::Error> {
    if !Path::new(path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Module {} not found", path),
        ));
    }

    let file = File::open(path)?;
    let fd = file.as_raw_fd();
    let empty_params = b"\0";

    println!("[MRR-INIT] Loading kernel module: {}...", path);
    let ret = unsafe {
        libc::syscall(
            libc::SYS_finit_module,
            fd,
            empty_params.as_ptr() as *const libc::c_char,
            0 as libc::c_int,
        )
    };

    if ret == 0 {
        println!("[MRR-INIT] Module {} successfully loaded into kernel.", path);
        Ok(())
    } else {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EEXIST) {
            println!("[MRR-INIT] Module {} already inserted.", path);
            Ok(())
        } else {
            eprintln!("[MRR-INIT] finit_module {} failed: {}", path, err);
            Err(err)
        }
    }
}

pub fn init_touch_hardware() {
    println!("[MRR-INIT] Initializing Xiaomi 14T touch subsystem hardware...");

    // 1. Try loading patched touch module if present
    let module_candidates = [
        "/lib/modules/xiaomi_touch_common.ko",
        "/vendor/lib/modules/xiaomi_touch_common.ko",
        "/system/lib/modules/xiaomi_touch_common.ko",
        "/xiaomi_touch_common_patched.ko",
    ];

    for mod_path in &module_candidates {
        if Path::new(mod_path).exists() {
            let _ = load_kernel_module(mod_path);
            break;
        }
    }

    // 2. Setup sysfs controls for Xiaomi 14T AMOLED touch panel
    if Path::new("/sys/class/touch/touch_dev/enable_touch_raw").exists() {
        let _ = fs::write("/sys/class/touch/touch_dev/enable_touch_raw", "0");
        println!("[MRR-INIT] Configured enable_touch_raw=0");
    }

    if Path::new("/sys/class/touch/touch_dev/panel_display").exists() {
        let _ = fs::write("/sys/class/touch/touch_dev/panel_display", "U");
        println!("[MRR-INIT] Configured panel_display=U");
    }
}
