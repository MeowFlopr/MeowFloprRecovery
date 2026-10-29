use std::ffi::CString;
use std::fs;
use std::path::Path;

use super::fscrypt::is_mounted;

pub fn ensure_persist_mounted() {
    if is_mounted("/mnt/vendor/persist") || is_mounted("/persist") {
        return;
    }
    let _ = fs::create_dir_all("/mnt/vendor/persist");
    let c_dev = match CString::new("/dev/block/by-name/persist") {
        Ok(s) => s,
        Err(_) => return,
    };
    let c_mnt = match CString::new("/mnt/vendor/persist") {
        Ok(s) => s,
        Err(_) => return,
    };
    let c_fs = match CString::new("ext4") {
        Ok(s) => s,
        Err(_) => return,
    };
    let c_opts = match CString::new("commit=1,nodelalloc") {
        Ok(s) => s,
        Err(_) => return,
    };
    let ret = unsafe {
        libc::mount(
            c_dev.as_ptr(),
            c_mnt.as_ptr(),
            c_fs.as_ptr(),
            0,
            c_opts.as_ptr() as *const libc::c_void,
        )
    };
    if ret == 0 {
        println!("[MRR-CRYPTO] Successfully mounted /mnt/vendor/persist (ext4)");
    } else {
        eprintln!("[MRR-CRYPTO] Warning: Failed to mount /mnt/vendor/persist");
    }
}

pub fn parse_hex_key(hex_str: &str) -> Result<[u8; 64], String> {
    let cleaned: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() != 128 {
        return Err(format!("Expected 128 hex characters, got {}", cleaned.len()));
    }

    let mut out = [0u8; 64];
    for i in 0..64 {
        let byte_str = &cleaned[i * 2..i * 2 + 2];
        out[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|e| format!("Invalid hex byte '{}': {}", byte_str, e))?;
    }
    Ok(out)
}

pub fn get_active_keys() -> Option<(String, [u8; 64])> {
    ensure_persist_mounted();

    let dm_key = read_hex_from_file("/mnt/vendor/persist/mrr/userdata.key")
        .or_else(|| read_hex_from_file("/persist/mrr/userdata.key"))?;

    let ce_hex = read_hex_from_file("/mnt/vendor/persist/mrr/ce.key")
        .or_else(|| read_hex_from_file("/persist/mrr/ce.key"))?;

    let ce_key = parse_hex_key(&ce_hex).ok()?;
    Some((dm_key, ce_key))
}

pub fn save_active_keys(dm_key_hex: &str, ce_key_hex: &str) -> Result<(), std::io::Error> {
    ensure_persist_mounted();
    let dir = Path::new("/mnt/vendor/persist/mrr");
    let _ = fs::create_dir_all(dir);
    fs::write(dir.join("userdata.key"), dm_key_hex.trim())?;
    fs::write(dir.join("ce.key"), ce_key_hex.trim())?;
    Ok(())
}

fn read_hex_from_file(path: &str) -> Option<String> {
    if !Path::new(path).exists() {
        return None;
    }
    match fs::read_to_string(path) {
        Ok(s) => {
            let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if cleaned.len() == 128 {
                Some(cleaned)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
