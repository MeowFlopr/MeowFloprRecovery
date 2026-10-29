use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const MISC_DEV: &str = "/dev/block/by-name/misc";
const BC_OFFSET: u64 = 2048;
const BC_SIZE: usize = 32;
const BC_MAGIC: u32 = 0x42414342;

pub fn crc32_ieee(data: &[u8]) -> u32 {
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
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        for token in cmdline.split_whitespace() {
            if let Some(suffix) = token.strip_prefix("androidboot.slot_suffix=") {
                let clean = suffix.trim_start_matches('_');
                return clean.to_uppercase();
            }
            if let Some(suffix) = token.strip_prefix("androidboot.slot=") {
                let clean = suffix.trim_start_matches('_');
                return clean.to_uppercase();
            }
        }
    }

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

pub fn read_bc_struct() -> Result<[u8; BC_SIZE], std::io::Error> {
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

pub fn set_active_slot_in_buf(buf: &mut [u8; BC_SIZE], is_slot_b: bool) -> Result<u32, std::io::Error> {
    let magic = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if magic != BC_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid bootloader_control magic 0x{:08x}", magic),
        ));
    }

    if is_slot_b {
        buf[0..4].copy_from_slice(b"_b\0\0");
        buf[10] = 14 | (buf[10] & 0x70);
        buf[11] = 15 | (7 << 4);
    } else {
        buf[0..4].copy_from_slice(b"_a\0\0");
        buf[10] = 15 | (7 << 4);
        buf[11] = 14 | (buf[11] & 0x70);
    }

    let crc = crc32_ieee(&buf[0..28]);
    buf[28..32].copy_from_slice(&crc.to_le_bytes());
    Ok(crc)
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
    let crc = set_active_slot_in_buf(&mut buf, is_slot_b)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_ieee() {
        let test_data = b"123456789";
        let crc = crc32_ieee(test_data);
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_slot_control_buffer_switch() {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(b"_a\0\0");
        buf[4..8].copy_from_slice(&BC_MAGIC.to_le_bytes());
        buf[8] = 1;
        buf[9] = 0;
        buf[10] = 15 | (7 << 4);
        buf[11] = 14;

        let crc_b = set_active_slot_in_buf(&mut buf, true).unwrap();
        assert_eq!(&buf[0..4], b"_b\0\0");
        assert_eq!(buf[10] & 0x0f, 14);
        assert_eq!(buf[11] & 0x0f, 15);
        assert_eq!(crc32_ieee(&buf[0..28]), crc_b);

        let crc_a = set_active_slot_in_buf(&mut buf, false).unwrap();
        assert_eq!(&buf[0..4], b"_a\0\0");
        assert_eq!(buf[10] & 0x0f, 15);
        assert_eq!(buf[11] & 0x0f, 14);
        assert_eq!(crc32_ieee(&buf[0..28]), crc_a);
    }
}
