use std::ffi::CString;
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;

const BLKGETSIZE64: libc::c_int = 0x80081272u32 as libc::c_int;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageStats {
    pub total_gb: u32,
    pub free_gb: u32,
    pub used_gb: u32,
}

pub fn get_storage_stats() -> StorageStats {
    let mount_candidates = ["/data", "/sdcard", "/"];

    for path in mount_candidates {
        if let Ok(c_path) = CString::new(path) {
            let mut stat = MaybeUninit::<libc::statvfs>::uninit();
            let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
            if ret == 0 {
                let s = unsafe { stat.assume_init() };
                let frsize = if s.f_frsize > 0 { s.f_frsize as u64 } else { s.f_bsize as u64 };
                let total_bytes = s.f_blocks as u64 * frsize;
                let free_bytes = s.f_bavail as u64 * frsize;

                let total_gb = (total_bytes / (1024 * 1024 * 1024)) as u32;
                let free_gb = (free_bytes / (1024 * 1024 * 1024)) as u32;
                let used_gb = total_gb.saturating_sub(free_gb);

                if total_gb >= 2 {
                    return StorageStats {
                        total_gb,
                        free_gb,
                        used_gb,
                    };
                }
            }
        }
    }

    if let Ok(file) = File::open("/dev/block/by-name/userdata") {
        let fd = file.as_raw_fd();
        let mut bytes: u64 = 0;
        let ret = unsafe { libc::ioctl(fd, BLKGETSIZE64 as _, &mut bytes) };
        if ret == 0 && bytes > 0 {
            let total_gb = (bytes / (1024 * 1024 * 1024)) as u32;
            return StorageStats {
                total_gb,
                free_gb: 0,
                used_gb: total_gb,
            };
        }
    }

    StorageStats {
        total_gb: 0,
        free_gb: 0,
        used_gb: 0,
    }
}
