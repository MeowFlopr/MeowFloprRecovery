use std::ffi::CString;
use std::fs;
use std::path::Path;

fn raw_mount(source: &str, target: &str, fstype: &str, flags: libc::c_ulong, data: &str) -> Result<(), std::io::Error> {
    let c_source = CString::new(source)?;
    let c_target = CString::new(target)?;
    let c_fstype = CString::new(fstype)?;
    let c_data = if data.is_empty() {
        None
    } else {
        Some(CString::new(data)?)
    };

    let data_ptr = match &c_data {
        Some(d) => d.as_ptr() as *const libc::c_void,
        None => std::ptr::null(),
    };

    let ret = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            flags,
            data_ptr,
        )
    };

    if ret == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EBUSY) {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub fn early_mount_vfs() -> Result<(), std::io::Error> {
    println!("[MRR-INIT] Mounting early virtual filesystems (proc, sysfs, devtmpfs, tmpfs)...");

    let _ = fs::create_dir_all("/proc");
    let _ = fs::create_dir_all("/sys");
    let _ = fs::create_dir_all("/dev");
    let _ = fs::create_dir_all("/tmp");

    if let Err(e) = raw_mount("none", "/proc", "proc", libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC, "") {
        eprintln!("[MRR-INIT] Failed to mount /proc: {}", e);
    } else {
        println!("[MRR-INIT] Mounted /proc");
    }

    if let Err(e) = raw_mount("none", "/sys", "sysfs", libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC, "") {
        eprintln!("[MRR-INIT] Failed to mount /sys: {}", e);
    } else {
        println!("[MRR-INIT] Mounted /sys");
    }

    if let Err(e) = raw_mount("none", "/dev", "devtmpfs", libc::MS_NOSUID, "mode=0755") {
        eprintln!("[MRR-INIT] Failed to mount /dev (devtmpfs): {}", e);
    } else {
        println!("[MRR-INIT] Mounted /dev (devtmpfs)");
    }

    let _ = fs::create_dir_all("/dev/pts");
    let _ = raw_mount("none", "/dev/pts", "devpts", 0, "gid=5,mode=620");

    let _ = raw_mount("none", "/tmp", "tmpfs", 0, "mode=1777");

    let dirs = [
        "/data",
        "/sdcard",
        "/mnt",
        "/mnt/vendor",
        "/mnt/vendor/persist",
        "/system",
        "/vendor",
        "/dev/block",
        "/dev/block/mapper",
        "/dev/block/by-name",
    ];

    for dir in &dirs {
        let _ = fs::create_dir_all(dir);
    }

    populate_by_name_links();

    Ok(())
}

fn populate_by_name_links() {
    let sys_block = Path::new("/sys/class/block");
    if !sys_block.exists() {
        return;
    }

    let entries = match fs::read_dir(sys_block) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let _dev_name = entry.file_name();
        let uevent_path = entry.path().join("uevent");
        if let Ok(content) = fs::read_to_string(&uevent_path) {
            let mut partname = None;
            let mut devname = None;
            for line in content.lines() {
                if let Some(p) = line.strip_prefix("PARTNAME=") {
                    partname = Some(p.trim().to_string());
                } else if let Some(d) = line.strip_prefix("DEVNAME=") {
                    devname = Some(d.trim().to_string());
                }
            }

            if let (Some(part), Some(dev)) = (partname, devname) {
                let dev_in_block = Path::new("/dev/block").join(&dev);
                let dev_in_root = Path::new("/dev").join(&dev);

                let target_dev = if dev_in_block.exists() {
                    dev_in_block
                } else if dev_in_root.exists() {
                    let _ = std::os::unix::fs::symlink(&dev_in_root, &dev_in_block);
                    dev_in_block
                } else {
                    dev_in_block
                };

                let link_path = Path::new("/dev/block/by-name").join(&part);
                if target_dev.exists() && !link_path.exists() {
                    let _ = std::os::unix::fs::symlink(&target_dev, &link_path);
                }
            }
        }
    }
}
