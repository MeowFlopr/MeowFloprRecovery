use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const MISC_DEV: &str = "/dev/block/by-name/misc";
const BC_OFFSET: u64 = 2048;
const BC_SIZE: usize = 32;
const BC_MAGIC: u32 = 0x42414342; // "BCAB"

// Standard CRC32 table for IEEE 802.3
fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1) != 0;
            crc >>= 1;
            if mask {
                crc ^= 0xedb8_8320;
            }
        }
    }
    !crc
}

pub fn get_current_slot() -> String {
    // 1. Try /proc/cmdline
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        for token in cmdline.split_whitespace() {
            if let Some(suffix) = token.strip_prefix("androidboot.slot_suffix=") {
                let clean = suffix.trim_start_matches('_');
                return clean.to_uppercase();
            }
        }
    }

    // 2. Read directly from misc partition bootloader_control
    if let Ok(data) = read_bc_struct() {
        let magic = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if magic == BC_MAGIC {
            if data[0] == b'_' && data[1] == b'b' {
                return "B".to_string();
            } else {
                return "A".to_string();
            }
        }
    }

    "A".to_string()
}

fn read_bc_struct() -> Result<[u8; BC_SIZE], std::io::Error> {
    if !Path::new(MISC_DEV).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not found", MISC_DEV),
        ));
    }

    let mut file = File::open(MISC_DEV)?;
    file.seek(SeekFrom::Start(BC_OFFSET))?;
    let mut buf = [0u8; BC_SIZE];
    file.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn set_active_slot(slot_name: &str) -> Result<(), std::io::Error> {
    let is_slot_b = match slot_name.to_lowercase().as_str() {
        "b" | "_b" => true,
        "a" | "_a" => false,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Slot must be 'A' or 'B'",
            ))
        }
    };

    println!("[MRR-SLOTS] Switching active boot slot to {}...", if is_slot_b { "B" } else { "A" });

    let mut buf = read_bc_struct()?;
    let magic = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if magic != BC_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid bootloader_control magic 0x{:08x}", magic),
        ));
    }

    // Update slot suffix
    if is_slot_b {
        buf[0..4].copy_from_slice(b"_b\0\0");
        buf[12] = 14 | (7 << 4); // slot A priority=14, tries=7
        buf[14] = 15 | (7 << 4); // slot B priority=15, tries=7
    } else {
        buf[0..4].copy_from_slice(b"_a\0\0");
        buf[12] = 15 | (7 << 4); // slot A priority=15, tries=7
        buf[14] = 14 | (7 << 4); // slot B priority=14, tries=7
    }

    // Recompute CRC32 over the first 28 bytes
    let crc = crc32_ieee(&buf[0..28]);
    buf[28..32].copy_from_slice(&crc.to_le_bytes());

    // Write back to misc partition at offset 2048 without truncating
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(MISC_DEV)?;
    file.seek(SeekFrom::Start(BC_OFFSET))?;
    file.write_all(&buf)?;
    file.sync_all()?;

    println!("[MRR-SLOTS] Successfully switched active slot to {} (crc=0x{:08x})!",
             if is_slot_b { "B" } else { "A" }, crc);
    Ok(())
}
