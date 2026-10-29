use std::fs::{self, OpenOptions};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

const DM_NAME_LEN: usize = 128;
const DM_UUID_LEN: usize = 129;
const DM_TABLE_STATUS: libc::c_int = 0xc138fd0cu32 as libc::c_int;
const DM_STATUS_TABLE_FLAG: u32 = 1 << 4;

#[repr(C)]
struct DmIoctl {
    version: [u32; 3],
    data_size: u32,
    data_start: u32,
    target_count: u32,
    open_count: i32,
    flags: u32,
    event_nr: u32,
    padding: u32,
    dev: u64,
    name: [u8; DM_NAME_LEN],
    uuid: [u8; DM_UUID_LEN],
    data: [u8; 7],
}

#[repr(C)]
struct DmTargetSpec {
    sector_start: u64,
    length: u64,
    status: i32,
    next: u32,
    target_type: [u8; 16],
}

fn query_dm_table(dev_name: &str) -> Option<String> {
    let dm_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/device-mapper")
        .ok()?;
    let fd = dm_file.as_raw_fd();

    let total_size = 16384usize;
    let mut buf = vec![0u8; total_size];

    let io = unsafe { &mut *(buf.as_mut_ptr() as *mut DmIoctl) };
    io.version = [4, 0, 0];
    io.data_size = total_size as u32;
    io.data_start = std::mem::size_of::<DmIoctl>() as u32;
    io.flags = DM_STATUS_TABLE_FLAG;

    let b = dev_name.as_bytes();
    let clen = b.len().min(DM_NAME_LEN - 1);
    io.name[..clen].copy_from_slice(&b[..clen]);

    let ret = unsafe { libc::ioctl(fd, DM_TABLE_STATUS, buf.as_mut_ptr()) };
    if ret < 0 {
        return None;
    }

    let out_io = unsafe { &*(buf.as_ptr() as *const DmIoctl) };
    let start = out_io.data_start as usize;
    if start >= total_size {
        return None;
    }

    if out_io.target_count == 0 {
        return None;
    }

    let spec = unsafe { &*(buf[start..].as_ptr() as *const DmTargetSpec) };
    let spec_size = std::mem::size_of::<DmTargetSpec>();
    let param_start = start + spec_size;

    let end = if spec.next > 0 && (start + spec.next as usize) <= total_size {
        start + spec.next as usize
    } else {
        total_size
    };

    let params = &buf[param_start..end];
    let s = String::from_utf8_lossy(params).to_string();
    Some(s)
}

fn extract_hex_128(text: &str) -> Option<String> {
    for part in text.split_whitespace() {
        let cleaned: String = part.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if cleaned.len() == 128 {
            return Some(cleaned);
        }
    }
    None
}

fn main() {
    println!("[MRR-SYNC] Starting direct kernel device-mapper key extraction...");

    let _ = fs::create_dir_all("/mnt/vendor/persist/mrr");

    if let Some(params) = query_dm_table("userdata") {
        println!("[MRR-SYNC] Userdata target params: {}", params);
        if let Some(k) = extract_hex_128(&params) {
            println!("[MRR-SYNC] Found 128-hex key: {}", k);
            let persist_path = Path::new("/mnt/vendor/persist/mrr/userdata.key");
            let _ = fs::write(persist_path, k.trim());
            let _ = fs::set_permissions(persist_path, fs::Permissions::from_mode(0o600));
            println!("[MRR-SYNC] SUCCESS: Synchronized key to persist storage!");
            return;
        }
    }

    eprintln!("[MRR-SYNC] Could not parse 128-hex key from userdata params.");
}
