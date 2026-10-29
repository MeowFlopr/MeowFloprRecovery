use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::symlink;
use std::os::unix::io::AsRawFd;
use std::path::Path;

const DM_DEV_NAME: &str = "userdata";
const DM_NAME_LEN: usize = 128;
const DM_UUID_LEN: usize = 129;

const DM_DEV_CREATE: libc::c_int = 0xc138fd03u32 as libc::c_int;
const DM_TABLE_LOAD: libc::c_int = 0xc138fd09u32 as libc::c_int;
const DM_DEV_SUSPEND: libc::c_int = 0xc138fd06u32 as libc::c_int;
const BLKGETSIZE64: libc::c_int = 0x80081272u32 as libc::c_int;

const DM_READONLY_FLAG: u32 = 1 << 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DmIoctl {
    pub version: [u32; 3],
    pub data_size: u32,
    pub data_start: u32,
    pub target_count: u32,
    pub open_count: i32,
    pub flags: u32,
    pub event_nr: u32,
    pub padding: u32,
    pub dev: u64,
    pub name: [u8; DM_NAME_LEN],
    pub uuid: [u8; DM_UUID_LEN],
    pub data: [u8; 7],
}

impl Default for DmIoctl {
    fn default() -> Self {
        Self {
            version: [4, 0, 0],
            data_size: std::mem::size_of::<Self>() as u32,
            data_start: std::mem::size_of::<Self>() as u32,
            target_count: 0,
            open_count: 0,
            flags: 0,
            event_nr: 0,
            padding: 0,
            dev: 0,
            name: [0; DM_NAME_LEN],
            uuid: [0; DM_UUID_LEN],
            data: [0; 7],
        }
    }
}

impl DmIoctl {
    pub fn new(name: &str, flags: u32, total_size: usize) -> Self {
        let mut io = Self::default();
        io.data_size = total_size as u32;
        io.flags = flags;
        let bytes = name.as_bytes();
        let copy_len = bytes.len().min(DM_NAME_LEN - 1);
        io.name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        io
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DmTargetSpec {
    pub sector_start: u64,
    pub length: u64,
    pub status: i32,
    pub next: u32,
    pub target_type: [u8; 16],
}

pub fn get_userdata_sectors() -> u64 {
    let mut sectors = 492158912u64;
    let path = "/dev/block/by-name/userdata";

    if let Ok(file) = File::open(path) {
        let fd = file.as_raw_fd();
        let mut bytes: u64 = 0;
        let ret = unsafe { libc::ioctl(fd, BLKGETSIZE64 as _, &mut bytes) };
        if ret == 0 && bytes > 0 {
            sectors = bytes / 512;
            println!(
                "[MRR-DM] Detected real userdata size: {} bytes ({} sectors)",
                bytes, sectors
            );
        }
    }
    sectors
}

pub fn setup_dm_default_key(key_hex: &str) -> Result<(), std::io::Error> {
    let mapper_dev = format!("/dev/block/mapper/{}", DM_DEV_NAME);
    if Path::new(&mapper_dev).exists() {
        println!("[MRR-DM] Mapper device '{}' already active.", mapper_dev);
        return Ok(());
    }

    if !Path::new("/dev/device-mapper").exists() {
        let _ = fs::create_dir_all("/dev");
        unsafe {
            libc::mknod(
                b"/dev/device-mapper\0".as_ptr() as *const libc::c_char,
                libc::S_IFCHR | 0o660,
                libc::makedev(10, 236),
            );
        }
    }

    let dm_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/device-mapper")?;
    let dm_fd = dm_file.as_raw_fd();

    let mut io_create = DmIoctl::new(DM_DEV_NAME, 0, std::mem::size_of::<DmIoctl>());
    let ret = unsafe { libc::ioctl(dm_fd, DM_DEV_CREATE as _, &mut io_create) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EEXIST) && err.raw_os_error() != Some(libc::EBUSY) {
            eprintln!("[MRR-DM] ioctl DM_DEV_CREATE failed: {}", err);
            return Err(err);
        }
        println!("[MRR-DM] DM device '{}' already registered in kernel.", DM_DEV_NAME);
    } else {
        println!("[MRR-DM] DM_DEV_CREATE '{}' OK (dev=0x{:x})", DM_DEV_NAME, io_create.dev);
    }

    let sectors = get_userdata_sectors();
    let params = format!(
        "aes-xts-plain64 {} 0 /dev/block/by-name/userdata 0 3 allow_discards sector_size:4096 iv_large_sectors\0",
        key_hex
    );
    let params_bytes = params.as_bytes();
    let align_params = (params_bytes.len() + 7) & !7;

    let total_size = std::mem::size_of::<DmIoctl>()
        + std::mem::size_of::<DmTargetSpec>()
        + align_params;

    let mut buf = vec![0u8; total_size];

    let io_load = DmIoctl::new(DM_DEV_NAME, DM_READONLY_FLAG, total_size);
    let mut spec = DmTargetSpec::default();
    spec.sector_start = 0;
    spec.length = sectors;
    spec.status = 0;
    spec.next = (std::mem::size_of::<DmTargetSpec>() + align_params) as u32;

    let target_type = b"default-key\0";
    spec.target_type[..target_type.len()].copy_from_slice(target_type);

    unsafe {
        std::ptr::copy_nonoverlapping(
            &io_load as *const DmIoctl as *const u8,
            buf.as_mut_ptr(),
            std::mem::size_of::<DmIoctl>(),
        );

        let spec_offset = std::mem::size_of::<DmIoctl>();
        std::ptr::copy_nonoverlapping(
            &spec as *const DmTargetSpec as *const u8,
            buf.as_mut_ptr().add(spec_offset),
            std::mem::size_of::<DmTargetSpec>(),
        );

        let params_offset = spec_offset + std::mem::size_of::<DmTargetSpec>();
        std::ptr::copy_nonoverlapping(
            params_bytes.as_ptr(),
            buf.as_mut_ptr().add(params_offset),
            params_bytes.len(),
        );

        let io_ptr = buf.as_mut_ptr() as *mut DmIoctl;
        (*io_ptr).target_count = 1;

        let ret = libc::ioctl(dm_fd, DM_TABLE_LOAD as _, buf.as_mut_ptr());
        if ret < 0 {
            eprintln!("[MRR-DM] ioctl DM_TABLE_LOAD failed: {}", std::io::Error::last_os_error());
        } else {
            println!("[MRR-DM] DM_TABLE_LOAD succeeded (length={} sectors)!", sectors);
        }
    }

    let mut io_resume = DmIoctl::new(DM_DEV_NAME, 0, std::mem::size_of::<DmIoctl>());
    let ret = unsafe { libc::ioctl(dm_fd, DM_DEV_SUSPEND as _, &mut io_resume) };
    if ret < 0 {
        eprintln!("[MRR-DM] ioctl DM_DEV_SUSPEND failed: {}", std::io::Error::last_os_error());
    } else {
        println!("[MRR-DM] DM_DEV_SUSPEND (resume) activated device!");
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    let _ = fs::create_dir_all("/dev/block/mapper");
    if !Path::new(&mapper_dev).exists() {
        let dev_num = if io_resume.dev != 0 {
            io_resume.dev
        } else {
            io_create.dev
        };

        if dev_num != 0 {
            if let Ok(c_path) = std::ffi::CString::new(mapper_dev.clone()) {
                unsafe {
                    libc::mknod(
                        c_path.as_ptr(),
                        libc::S_IFBLK | 0o660,
                        dev_num,
                    );
                }
            }
        } else if Path::new("/dev/block/dm-0").exists() {
            let _ = symlink("/dev/block/dm-0", &mapper_dev);
            println!("[MRR-DM] Created symlink /dev/block/dm-0 -> {}", mapper_dev);
        } else if let Ok(content) = fs::read_to_string("/sys/block/dm-0/dev") {
            let parts: Vec<&str> = content.trim().split(':').collect();
            if parts.len() == 2 {
                if let (Ok(maj), Ok(min)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    unsafe {
                        let c_path = std::ffi::CString::new(mapper_dev.clone()).unwrap();
                        libc::mknod(
                            c_path.as_ptr(),
                            libc::S_IFBLK | 0o660,
                            libc::makedev(maj, min),
                        );
                        println!("[MRR-DM] Created node {} ({}:{})", mapper_dev, maj, min);
                    }
                }
            }
        }
    }

    Ok(())
}
