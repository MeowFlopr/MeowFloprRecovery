//! AOSP Dynamic Partitions (Super / LpMetadata) Parser & dm-linear Device Mapper
//! Parses Android Virtual A/B super partition metadata and creates block device mappers
//! for system, vendor, product, system_ext, etc.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::symlink;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use crate::crypto::dm::{DmIoctl, DmTargetSpec};

const LP_METADATA_GEOMETRY_MAGIC: u32 = 0x616c4430; // "0Dal"
const LP_METADATA_HEADER_MAGIC: u32 = 0x414c5030;   // "0PLA"
const LP_TARGET_TYPE_LINEAR: u32 = 0;
const LP_TARGET_TYPE_ZERO: u32 = 1;

const DM_DEV_CREATE: libc::c_int = 0xc138fd03u32 as libc::c_int;
const DM_TABLE_LOAD: libc::c_int = 0xc138fd09u32 as libc::c_int;
const DM_DEV_SUSPEND: libc::c_int = 0xc138fd06u32 as libc::c_int;
const DM_READONLY_FLAG: u32 = 1 << 0;

#[derive(Debug, Clone)]
pub struct PartitionExtent {
    pub num_sectors: u64,
    pub target_type: u32,
    pub target_data: u64, // sector offset in super
}

#[derive(Debug, Clone)]
pub struct LogicalPartition {
    pub name: String,
    pub extents: Vec<PartitionExtent>,
}

/// Reads and parses logical partitions from /dev/block/by-name/super
pub fn parse_super_metadata(super_path: &Path) -> Result<Vec<LogicalPartition>, String> {
    if !super_path.exists() {
        return Err(format!("Super block device not found at {:?}", super_path));
    }

    let mut file = File::open(super_path).map_err(|e| format!("Failed to open super: {}", e))?;

    // 1. Read Geometry at offset 4096 (0x1000)
    file.seek(SeekFrom::Start(4096))
        .map_err(|e| format!("Seek to geometry failed: {}", e))?;
    let mut geom_buf = [0u8; 4096];
    file.read_exact(&mut geom_buf)
        .map_err(|e| format!("Read geometry failed: {}", e))?;

    let geom_magic = u32::from_le_bytes([geom_buf[0], geom_buf[1], geom_buf[2], geom_buf[3]]);
    if geom_magic != LP_METADATA_GEOMETRY_MAGIC {
        return Err(format!(
            "Invalid LP geometry magic: 0x{:08x} (expected 0x{:08x})",
            geom_magic, LP_METADATA_GEOMETRY_MAGIC
        ));
    }

    let metadata_max_size = u32::from_le_bytes([geom_buf[8], geom_buf[9], geom_buf[10], geom_buf[11]]) as u64;
    let metadata_slot_count = u32::from_le_bytes([geom_buf[12], geom_buf[13], geom_buf[14], geom_buf[15]]) as u64;

    println!(
        "[MRR-SUPER] Found valid LpMetadataGeometry (max_size={}, slots={})",
        metadata_max_size, metadata_slot_count
    );

    // 2. Read Primary Metadata Header (located at 0x1000 + 4096 = 8192)
    let meta_offset = 8192u64;
    file.seek(SeekFrom::Start(meta_offset))
        .map_err(|e| format!("Seek to metadata slot failed: {}", e))?;

    let mut meta_buf = vec![0u8; metadata_max_size as usize];
    file.read_exact(&mut meta_buf)
        .map_err(|e| format!("Read metadata buffer failed: {}", e))?;

    let hdr_magic = u32::from_le_bytes([meta_buf[0], meta_buf[1], meta_buf[2], meta_buf[3]]);
    if hdr_magic != LP_METADATA_HEADER_MAGIC {
        return Err(format!(
            "Invalid LP header magic: 0x{:08x} (expected 0x{:08x})",
            hdr_magic, LP_METADATA_HEADER_MAGIC
        ));
    }

    let header_size = u32::from_le_bytes([meta_buf[8], meta_buf[9], meta_buf[10], meta_buf[11]]) as usize;
    let tables_offset = header_size;

    // Table descriptors in LpMetadataHeader (AOSP v10.0+):
    // Partitions descriptor: offset 80: offset (4 bytes), num_entries (4 bytes), entry_size (4 bytes)
    let part_tbl_offset = u32::from_le_bytes([meta_buf[80], meta_buf[81], meta_buf[82], meta_buf[83]]) as usize;
    let part_tbl_count = u32::from_le_bytes([meta_buf[84], meta_buf[85], meta_buf[86], meta_buf[87]]) as usize;
    let part_entry_size = u32::from_le_bytes([meta_buf[88], meta_buf[89], meta_buf[90], meta_buf[91]]) as usize;

    // Extents descriptor: offset 92: offset (4 bytes), num_entries (4 bytes), entry_size (4 bytes)
    let ext_tbl_offset = u32::from_le_bytes([meta_buf[92], meta_buf[93], meta_buf[94], meta_buf[95]]) as usize;
    let ext_tbl_count = u32::from_le_bytes([meta_buf[96], meta_buf[97], meta_buf[98], meta_buf[99]]) as usize;
    let ext_entry_size = u32::from_le_bytes([meta_buf[100], meta_buf[101], meta_buf[102], meta_buf[103]]) as usize;

    println!(
        "[MRR-SUPER] Partitions table: count={}, entry_size={}, offset={}",
        part_tbl_count, part_entry_size, part_tbl_offset
    );
    println!(
        "[MRR-SUPER] Extents table: count={}, entry_size={}, offset={}",
        ext_tbl_count, ext_entry_size, ext_tbl_offset
    );

    // Parse Extents
    let mut extents = Vec::new();
    let ext_base = tables_offset + ext_tbl_offset;
    for i in 0..ext_tbl_count {
        let off = ext_base + i * ext_entry_size;
        if off + 24 <= meta_buf.len() {
            let num_sectors = u64::from_le_bytes(meta_buf[off..off + 8].try_into().unwrap());
            let target_type = u32::from_le_bytes(meta_buf[off + 8..off + 12].try_into().unwrap());
            let target_data = u64::from_le_bytes(meta_buf[off + 12..off + 20].try_into().unwrap());
            extents.push(PartitionExtent {
                num_sectors,
                target_type,
                target_data,
            });
        }
    }

    // Parse Partitions
    let mut partitions = Vec::new();
    let part_base = tables_offset + part_tbl_offset;
    for i in 0..part_tbl_count {
        let off = part_base + i * part_entry_size;
        if off + 48 <= meta_buf.len() {
            let name_bytes = &meta_buf[off..off + 36];
            let name = std::str::from_utf8(name_bytes)
                .unwrap_or("")
                .trim_matches('\0')
                .trim()
                .to_string();

            let first_extent = u32::from_le_bytes(meta_buf[off + 40..off + 44].try_into().unwrap()) as usize;
            let num_ext = u32::from_le_bytes(meta_buf[off + 44..off + 48].try_into().unwrap()) as usize;

            let mut part_extents = Vec::new();
            for e_idx in 0..num_ext {
                if first_extent + e_idx < extents.len() {
                    part_extents.push(extents[first_extent + e_idx].clone());
                }
            }

            if !name.is_empty() && !part_extents.is_empty() {
                partitions.push(LogicalPartition {
                    name,
                    extents: part_extents,
                });
            }
        }
    }

    Ok(partitions)
}

pub fn map_dm_linear(part_name: &str, super_path: &str, extents: &[PartitionExtent]) -> Result<PathBuf, String> {
    map_dm_linear_mode(part_name, super_path, extents, false)
}

pub fn map_dm_linear_mode(part_name: &str, super_path: &str, extents: &[PartitionExtent], read_only: bool) -> Result<PathBuf, String> {
    let mapper_path = format!("/dev/block/mapper/{}", part_name);
    if Path::new(&mapper_path).exists() {
        return Ok(PathBuf::from(mapper_path));
    }

    if !Path::new("/dev/device-mapper").exists() {
        unsafe {
            libc::mknod(
                b"/dev/device-mapper\0".as_ptr() as *const libc::c_char,
                libc::S_IFCHR | 0o660,
                libc::makedev(10, 236),
            );
        }
    }

    let dm_file = File::options()
        .read(true)
        .write(true)
        .open("/dev/device-mapper")
        .map_err(|e| format!("Failed to open /dev/device-mapper: {}", e))?;
    let dm_fd = dm_file.as_raw_fd();

    let mut io_create = DmIoctl::new(part_name, 0, std::mem::size_of::<DmIoctl>());
    let ret = unsafe { libc::ioctl(dm_fd, DM_DEV_CREATE as _, &mut io_create) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EEXIST) && err.raw_os_error() != Some(libc::EBUSY) {
            return Err(format!("DM_DEV_CREATE '{}' failed: {}", part_name, err));
        }
    }

    let mut table_str = String::new();
    let mut current_sector = 0u64;

    for ext in extents {
        if ext.target_type == LP_TARGET_TYPE_LINEAR {
            table_str.push_str(&format!(
                "{} {} linear {} {}\n",
                current_sector, ext.num_sectors, super_path, ext.target_data
            ));
        } else if ext.target_type == LP_TARGET_TYPE_ZERO {
            table_str.push_str(&format!("{} {} zero\n", current_sector, ext.num_sectors));
        }
        current_sector += ext.num_sectors;
    }

    let mut total_size = std::mem::size_of::<DmIoctl>();
    for ext in extents {
        let param = if ext.target_type == LP_TARGET_TYPE_LINEAR {
            format!("{} {}\0", super_path, ext.target_data)
        } else {
            "\0".to_string()
        };
        let param_align = (param.len() + 7) & !7;
        total_size += std::mem::size_of::<DmTargetSpec>() + param_align;
    }

    let mut buf = vec![0u8; total_size];
    let flags = if read_only { DM_READONLY_FLAG } else { 0 };
    let io_load = DmIoctl::new(part_name, flags, total_size);

    unsafe {
        std::ptr::copy_nonoverlapping(
            &io_load as *const DmIoctl as *const u8,
            buf.as_mut_ptr(),
            std::mem::size_of::<DmIoctl>(),
        );

        let mut offset = std::mem::size_of::<DmIoctl>();
        let mut cur_sec = 0u64;

        for ext in extents {
            let param = if ext.target_type == LP_TARGET_TYPE_LINEAR {
                format!("{} {}\0", super_path, ext.target_data)
            } else {
                "\0".to_string()
            };
            let param_bytes = param.as_bytes();
            let param_align = (param_bytes.len() + 7) & !7;

            let mut spec = DmTargetSpec::default();
            spec.sector_start = cur_sec;
            spec.length = ext.num_sectors;
            spec.next = (std::mem::size_of::<DmTargetSpec>() + param_align) as u32;

            let t_type: &[u8] = if ext.target_type == LP_TARGET_TYPE_LINEAR {
                b"linear\0"
            } else {
                b"zero\0"
            };
            spec.target_type[..t_type.len()].copy_from_slice(t_type);

            std::ptr::copy_nonoverlapping(
                &spec as *const DmTargetSpec as *const u8,
                buf.as_mut_ptr().add(offset),
                std::mem::size_of::<DmTargetSpec>(),
            );
            offset += std::mem::size_of::<DmTargetSpec>();

            std::ptr::copy_nonoverlapping(
                param_bytes.as_ptr(),
                buf.as_mut_ptr().add(offset),
                param_bytes.len(),
            );
            offset += param_align;

            cur_sec += ext.num_sectors;
        }

        let io_ptr = buf.as_mut_ptr() as *mut DmIoctl;
        (*io_ptr).target_count = extents.len() as u32;

        let ret = libc::ioctl(dm_fd, DM_TABLE_LOAD as _, buf.as_mut_ptr());
        if ret < 0 {
            eprintln!(
                "[MRR-SUPER] ioctl DM_TABLE_LOAD for '{}' failed: {}",
                part_name,
                std::io::Error::last_os_error()
            );
        }
    }

    // 3. DM_DEV_SUSPEND (Resume / Activate)
    let mut io_resume = DmIoctl::new(part_name, 0, std::mem::size_of::<DmIoctl>());
    let ret = unsafe { libc::ioctl(dm_fd, DM_DEV_SUSPEND as _, &mut io_resume) };
    if ret < 0 {
        eprintln!(
            "[MRR-SUPER] ioctl DM_DEV_SUSPEND for '{}' failed: {}",
            part_name,
            std::io::Error::last_os_error()
        );
    } else {
        println!("[MRR-SUPER] Activated dynamic partition: {}", part_name);
    }

    // 4. Create block node in /dev/block/mapper/
    let _ = fs::create_dir_all("/dev/block/mapper");
    let out_path = PathBuf::from(mapper_path);

    // Check sysfs for device major:minor
    let sys_dm_path = format!("/sys/block/{}/dev", part_name);
    if let Ok(dev_str) = fs::read_to_string(&sys_dm_path) {
        let parts: Vec<&str> = dev_str.trim().split(':').collect();
        if parts.len() == 2 {
            if let (Ok(maj), Ok(min)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let c_path = std::ffi::CString::new(out_path.to_str().unwrap()).unwrap();
                unsafe {
                    libc::mknod(
                        c_path.as_ptr(),
                        libc::S_IFBLK | 0o660,
                        libc::makedev(maj, min),
                    );
                }
            }
        }
    }

    Ok(out_path)
}

/// Automatically maps all dynamic partitions for the active slot (e.g. system_a -> system)
pub fn setup_all_dynamic_partitions(active_slot: &str) {
    let super_path = Path::new("/dev/block/by-name/super");
    if !super_path.exists() {
        return;
    }

    println!("[MRR-SUPER] Initializing Xiaomi 14T dynamic partitions from /dev/block/by-name/super...");
    let partitions = match parse_super_metadata(super_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[MRR-SUPER] Super metadata parse warning: {}", e);
            return;
        }
    };

    let s_lower = format!("_{}", active_slot.to_lowercase());

    for part in &partitions {
        // Match active slot partitions (system_a, vendor_a, product_a, etc.)
        let is_active_slot = part.name.ends_with(&s_lower);
        let base_name = if is_active_slot {
            part.name.strip_suffix(&s_lower).unwrap_or(&part.name)
        } else {
            &part.name
        };

        if let Ok(mapped_dev) = map_dm_linear(&part.name, "/dev/block/by-name/super", &part.extents) {
            // Create symlink without slot suffix: /dev/block/mapper/system -> /dev/block/mapper/system_a
            let alias_path = format!("/dev/block/mapper/{}", base_name);
            if is_active_slot && !Path::new(&alias_path).exists() {
                let _ = symlink(&mapped_dev, Path::new(&alias_path));
                println!("[MRR-SUPER] Created symlink: {} -> {:?}", alias_path, mapped_dev);
            }
        }
    }
}
