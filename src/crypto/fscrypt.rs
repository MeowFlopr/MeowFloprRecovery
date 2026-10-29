use std::ffi::CString;
use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use super::dm::setup_dm_default_key;

const FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER: u32 = 2;
const FS_IOC_ADD_ENCRYPTION_KEY: libc::c_int = 0xc0506617u32 as libc::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FscryptKeySpecifier {
    pub key_type: u32,
    pub reserved: u32,
    pub u: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FscryptAddKeyArg {
    pub key_spec: FscryptKeySpecifier,
    pub raw_size: u32,
    pub key_id: u32,
    pub reserved: [u32; 8],
    pub raw: [u8; 64],
}

pub fn is_mounted(target_path: &str) -> bool {
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == target_path {
                return true;
            }
        }
    }
    false
}

pub fn remount_data_rw() -> Result<(), std::io::Error> {
    if !is_mounted("/data") {
        return Ok(());
    }
    println!("[MRR-CRYPTO] Remounting /data as read-write (RW)...");
    let c_mnt = CString::new("/data").unwrap();
    let ret = unsafe {
        libc::mount(
            std::ptr::null(),
            c_mnt.as_ptr(),
            std::ptr::null(),
            libc::MS_REMOUNT | libc::MS_NODEV | libc::MS_NOSUID,
            std::ptr::null(),
        )
    };
    if ret == 0 {
        println!("[MRR-CRYPTO] /data successfully remounted as RW!");
        Ok(())
    } else {
        let err = std::io::Error::last_os_error();
        eprintln!("[MRR-CRYPTO] Warning: Failed to remount /data as RW: {}", err);
        Err(err)
    }
}

pub fn ensure_data_mounted(dm_key_hex: &str) -> Result<(), std::io::Error> {
    if is_mounted("/data") {
        println!("[MRR-CRYPTO] /data is already mounted.");
        let _ = remount_data_rw();
        return Ok(());
    }

    setup_dm_default_key(dm_key_hex)?;

    let mapper_dev = "/dev/block/mapper/userdata";
    let _ = fs::create_dir_all("/data");

    println!("[MRR-CRYPTO] Mounting {} -> /data (f2fs rw,inlinecrypt)...", mapper_dev);

    let c_dev = CString::new(mapper_dev).unwrap();
    let c_mnt = CString::new("/data").unwrap();
    let c_f2fs = CString::new("f2fs").unwrap();
    let c_opts = CString::new("inlinecrypt").unwrap();

    let rw_flags = libc::MS_NODEV | libc::MS_NOSUID;
    let ro_flags = libc::MS_RDONLY | libc::MS_NODEV | libc::MS_NOSUID;

    let ret = unsafe {
        libc::mount(
            c_dev.as_ptr(),
            c_mnt.as_ptr(),
            c_f2fs.as_ptr(),
            rw_flags,
            c_opts.as_ptr() as *const libc::c_void,
        )
    };

    if ret == 0 {
        println!("[MRR-CRYPTO] Successfully mounted /data (f2fs rw,inlinecrypt)!");
        return Ok(());
    }

    let ret_ro = unsafe {
        libc::mount(
            c_dev.as_ptr(),
            c_mnt.as_ptr(),
            c_f2fs.as_ptr(),
            ro_flags,
            c_opts.as_ptr() as *const libc::c_void,
        )
    };

    if ret_ro == 0 {
        println!("[MRR-CRYPTO] Successfully mounted /data (f2fs ro,inlinecrypt)!");
        return Ok(());
    }

    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EBUSY) {
        println!("[MRR-CRYPTO] /data is already mounted (EBUSY).");
        return Ok(());
    }

    let ret_no_opt = unsafe {
        libc::mount(
            c_dev.as_ptr(),
            c_mnt.as_ptr(),
            c_f2fs.as_ptr(),
            ro_flags,
            std::ptr::null(),
        )
    };

    if ret_no_opt == 0 {
        println!("[MRR-CRYPTO] Successfully mounted /data (f2fs ro)!");
        return Ok(());
    }

    let c_ext4 = CString::new("ext4").unwrap();
    let ret_ext4 = unsafe {
        libc::mount(
            c_dev.as_ptr(),
            c_mnt.as_ptr(),
            c_ext4.as_ptr(),
            ro_flags,
            std::ptr::null(),
        )
    };

    if ret_ext4 == 0 {
        println!("[MRR-CRYPTO] Successfully mounted /data (ext4 ro)!");
        return Ok(());
    }

    eprintln!("[MRR-CRYPTO] Failed to mount /data: {}", err);
    Err(err)
}

pub fn inject_master_key_to_kernel(raw_key: &[u8; 64]) -> Result<[u8; 16], std::io::Error> {
    println!("[MRR-CRYPTO] Injecting 64-byte CE Master Key into Linux kernel fscrypt keyring...");

    let data_dir = File::open("/data")?;
    let fd = data_dir.as_raw_fd();

    let mut arg = FscryptAddKeyArg {
        key_spec: FscryptKeySpecifier {
            key_type: FSCRYPT_KEY_SPEC_TYPE_IDENTIFIER,
            reserved: 0,
            u: [0; 32],
        },
        raw_size: 64,
        key_id: 0,
        reserved: [0; 8],
        raw: *raw_key,
    };

    let ret = unsafe { libc::ioctl(fd, FS_IOC_ADD_ENCRYPTION_KEY as _, &mut arg) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EEXIST) || err.raw_os_error() == Some(libc::EBUSY) {
            println!("[MRR-CRYPTO] Key is already present in kernel keyring.");
        } else {
            eprintln!("[MRR-CRYPTO] FS_IOC_ADD_ENCRYPTION_KEY failed: {}", err);
            return Err(err);
        }
    }

    let mut identifier = [0u8; 16];
    identifier.copy_from_slice(&arg.key_spec.u[..16]);

    let id_hex: String = identifier.iter().map(|b| format!("{:02x}", b)).collect();
    println!("[MRR-CRYPTO] Master Key installed! Key Identifier: {}", id_hex);

    Ok(identifier)
}

pub fn bind_mount_sdcard() -> Result<(), std::io::Error> {
    if is_mounted("/sdcard") {
        return Ok(());
    }

    let _ = fs::create_dir_all("/sdcard");
    let c_src = CString::new("/data/media/0").unwrap();
    let c_dst = CString::new("/sdcard").unwrap();

    let ret = unsafe {
        libc::mount(
            c_src.as_ptr(),
            c_dst.as_ptr(),
            std::ptr::null(),
            libc::MS_BIND,
            std::ptr::null(),
        )
    };

    if ret == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EBUSY) {
        println!("[MRR-CRYPTO] Successfully bind-mounted /data/media/0 -> /sdcard!");
        Ok(())
    } else {
        if !Path::new("/sdcard").exists() {
            let _ = symlink("/data/media/0", "/sdcard");
            println!("[MRR-CRYPTO] Symlinked /data/media/0 -> /sdcard");
        }
        Ok(())
    }
}

pub fn verify_decryption() -> bool {
    let test_dirs = [
        "/data/media/0/Download",
        "/data/media/0/DCIM",
        "/data/media/0/Documents",
        "/data/media/0/Pictures",
        "/data/media/0/Music",
    ];

    for path in &test_dirs {
        if Path::new(path).exists() {
            println!("[MRR-CRYPTO] Decrypted folder accessible: {}", path);
            let _ = bind_mount_sdcard();
            return true;
        }
    }

    false
}
