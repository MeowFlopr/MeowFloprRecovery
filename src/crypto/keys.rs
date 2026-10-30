use std::ffi::CString;
use std::fs;
use std::path::Path;

use super::fscrypt::is_mounted;

pub const DEFAULT_DM_KEY_HEX: &str =
    "6c85931629f3858cf3a5e17d63206b2ae3954cddf5b8163bd75bae972548fab6\
     d8e157345140c659860fdf9f0358b4fe62e95d86939e36486c322ce00db6975b";

pub const DEFAULT_MASTER_KEY_HEX: &str =
    "8be3408ccdd7fe63daaaac7c5671ef786b16fd4822cebaf625af802268a7d3e5\
     8d62ffb25ea2a8552c48385535573392d49ea14ea4b86e55bbb61e78817f9d9d";

pub fn ensure_persist_mounted() {
    if is_mounted("/mnt/vendor/persist") || is_mounted("/persist") {
        return;
    }
    let _ = fs::create_dir_all("/mnt/vendor/persist");
    let c_dev = CString::new("/dev/block/by-name/persist").unwrap();
    let c_mnt = CString::new("/mnt/vendor/persist").unwrap();
    let c_fs = CString::new("ext4").unwrap();
    let c_opts = CString::new("commit=1,nodelalloc").unwrap();
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

pub fn get_active_keys() -> (String, [u8; 64]) {
    ensure_persist_mounted();

    // 1. Userdata dm-default-key (hex)
    let dm_key = read_hex_from_file("/mnt/vendor/persist/mrr/userdata.key")
        .or_else(|| read_hex_from_file("/persist/mrr/userdata.key"))
        .unwrap_or_else(|| {
            println!("[MRR-CRYPTO] Using embedded verified userdata default key");
            DEFAULT_DM_KEY_HEX.to_string()
        });

    // 2. CE Master Key (raw 64 bytes)
    let ce_hex = read_hex_from_file("/mnt/vendor/persist/mrr/ce.key")
        .or_else(|| read_hex_from_file("/persist/mrr/ce.key"))
        .unwrap_or_else(|| {
            println!("[MRR-CRYPTO] Using embedded verified CE master key");
            DEFAULT_MASTER_KEY_HEX.to_string()
        });

    let ce_key = parse_hex_key(&ce_hex).unwrap_or_else(|e| {
        eprintln!("[MRR-CRYPTO] Error parsing CE key hex: {}, falling back", e);
        parse_hex_key(DEFAULT_MASTER_KEY_HEX).unwrap()
    });

    (dm_key, ce_key)
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
