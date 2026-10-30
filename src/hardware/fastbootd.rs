use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hardware::slots;

pub const FASTBOOT_MAX_DOWNLOAD: usize = 512 * 1024 * 1024;
const SPARSE_HEADER_MAGIC: u32 = 0xed26ff3a;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct SparseHeader {
    magic: u32,
    major_version: u16,
    minor_version: u16,
    file_hdr_sz: u16,
    chunk_hdr_sz: u16,
    blk_sz: u32,
    total_blks: u32,
    total_chunks: u32,
    image_checksum: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ChunkHeader {
    chunk_type: u16,
    reserved1: u16,
    chunk_sz: u32,
    total_sz: u32,
}

#[derive(Clone)]
pub struct FastbootStatus {
    pub is_connected: bool,
    pub status_text: String,
    pub last_command: String,
    pub logs: Vec<String>,
    pub progress: f32,
    pub current_slot: String,
    pub download_size: usize,
}

impl Default for FastbootStatus {
    fn default() -> Self {
        Self {
            is_connected: false,
            status_text: "FastbootD service active. Waiting for host...".to_string(),
            last_command: String::new(),
            logs: vec!["FastbootD service initialized.".to_string()],
            progress: 0.0,
            current_slot: slots::get_current_slot(),
            download_size: 0,
        }
    }
}

pub struct FastbootServer {
    pub status: Arc<Mutex<FastbootStatus>>,
    stop_signal: Arc<AtomicBool>,
    worker_thread: Option<thread::JoinHandle<()>>,
}

impl FastbootServer {
    pub fn start() -> Self {
        let status = Arc::new(Mutex::new(FastbootStatus::default()));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let s_clone = Arc::clone(&status);
        let stop_clone = Arc::clone(&stop_signal);

        let worker_thread = thread::spawn(move || {
            Self::worker_loop(s_clone, stop_clone);
        });

        Self {
            status,
            stop_signal,
            worker_thread: Some(worker_thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        Self::cleanup_usb_gadget();
        if let Some(h) = self.worker_thread.take() {
            let _ = h.join();
        }
    }

    pub fn get_status(&self) -> FastbootStatus {
        self.status.lock().unwrap().clone()
    }

    fn setup_usb_gadget() {
        if Path::new("/system/bin/setprop").exists() {
            let _ = Command::new("/system/bin/setprop")
                .arg("sys.usb.config")
                .arg("none")
                .status();
        }

        let _ = fs::write("/config/usb_gadget/g1/UDC", "none\n");
        let _ = fs::remove_file("/config/usb_gadget/g1/configs/b.1/f1");
        thread::sleep(Duration::from_millis(100));

        let _ = fs::create_dir_all("/config/usb_gadget/g1/functions/ffs.fastboot");
        let _ = fs::create_dir_all("/dev/usb-ffs/fastboot");

        let c_src = std::ffi::CString::new("fastboot").unwrap();
        let c_tgt = std::ffi::CString::new("/dev/usb-ffs/fastboot").unwrap();
        let c_fst = std::ffi::CString::new("functionfs").unwrap();
        unsafe {
            libc::mount(
                c_src.as_ptr(),
                c_tgt.as_ptr(),
                c_fst.as_ptr(),
                0,
                std::ptr::null(),
            );
        }

        let _ = fs::write("/config/usb_gadget/g1/idVendor", "0x18D1\n");
        let _ = fs::write("/config/usb_gadget/g1/idProduct", "0xD00D\n");
        let _ = fs::write(
            "/config/usb_gadget/g1/configs/b.1/strings/0x409/configuration",
            "fastboot\n",
        );

        let link_src = "/config/usb_gadget/g1/functions/ffs.fastboot";
        let link_dst = "/config/usb_gadget/g1/configs/b.1/f1";
        let _ = std::os::unix::fs::symlink(link_src, link_dst);

        if let Ok(entries) = fs::read_dir("/sys/class/udc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let _ = fs::write("/config/usb_gadget/g1/UDC", format!("{}\n", name.trim()));
                    break;
                }
            }
        }

        if Path::new("/system/bin/setprop").exists() {
            let _ = Command::new("/system/bin/setprop")
                .arg("sys.usb.config")
                .arg("fastboot")
                .status();
        }
    }

    fn cleanup_usb_gadget() {
        let _ = fs::write("/config/usb_gadget/g1/UDC", "none\n");
        let _ = fs::remove_file("/config/usb_gadget/g1/configs/b.1/f1");

        let _ = fs::write("/config/usb_gadget/g1/idVendor", "0x18D1\n");
        let _ = fs::write("/config/usb_gadget/g1/idProduct", "0xD001\n");
        let _ = fs::write(
            "/config/usb_gadget/g1/configs/b.1/strings/0x409/configuration",
            "recovery\n",
        );

        let link_src = "/config/usb_gadget/g1/functions/ffs.adb";
        let link_dst = "/config/usb_gadget/g1/configs/b.1/f1";
        let _ = std::os::unix::fs::symlink(link_src, link_dst);

        if let Ok(entries) = fs::read_dir("/sys/class/udc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let _ = fs::write("/config/usb_gadget/g1/UDC", format!("{}\n", name.trim()));
                    break;
                }
            }
        }

        if Path::new("/system/bin/setprop").exists() {
            let _ = Command::new("/system/bin/setprop")
                .arg("sys.usb.config")
                .arg("adb")
                .status();
        }
    }

    fn worker_loop(status: Arc<Mutex<FastbootStatus>>, stop_signal: Arc<AtomicBool>) {
        Self::setup_usb_gadget();

        let is_already_running = fs::read_dir("/proc")
            .map(|entries| {
                entries.flatten().any(|e| {
                    let comm_path = e.path().join("comm");
                    fs::read_to_string(comm_path)
                        .map(|s| s.trim() == "fastbootd")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        let mut child_proc: Option<Child> = None;
        if !is_already_running && Path::new("/system/bin/fastbootd").exists() {
            if let Ok(child) = Command::new("/system/bin/fastbootd")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                child_proc = Some(child);
                if let Ok(mut st) = status.lock() {
                    st.logs.push("AOSP fastbootd service spawned.".to_string());
                    st.status_text = "FastbootD AOSP engine running.".to_string();
                }
            }
        } else if is_already_running {
            if let Ok(mut st) = status.lock() {
                st.logs.push("Init fastbootd service detected.".to_string());
                st.status_text = "FastbootD init service active.".to_string();
            }
        }

        let mut ep_in_opt: Option<File> = None;
        let mut ep_out_opt: Option<File> = None;

        if child_proc.is_none() {
            if let Ok(ep_in) = OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/usb-ffs/fastboot/ep1")
            {
                ep_in_opt = Some(ep_in);
            }
            if let Ok(ep_out) = OpenOptions::new()
                .read(true)
                .open("/dev/usb-ffs/fastboot/ep2")
            {
                ep_out_opt = Some(ep_out);
            }
        }

        let mut download_buffer: Vec<u8> = Vec::new();

        while !stop_signal.load(Ordering::Relaxed) {
            if let Some(ref mut child) = child_proc {
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        if let Ok(mut st) = status.lock() {
                            st.logs.push(format!("fastbootd daemon exited: {}", exit_status));
                            st.status_text = "Daemon stopped. Listening for native connections.".to_string();
                        }
                        child_proc = None;
                    }
                    Ok(None) => {
                        if let Ok(mut st) = status.lock() {
                            st.is_connected = true;
                        }
                    }
                    Err(_) => {
                        child_proc = None;
                    }
                }
            } else if let (Some(ref mut ep_in), Some(ref mut ep_out)) = (&mut ep_in_opt, &mut ep_out_opt) {
                let mut cmd_buf = [0u8; 128];
                let fd = ep_out.as_raw_fd();
                let mut pfd = libc::pollfd {
                    fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pfd, 1, 150) };
                if ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
                    if let Ok(n) = ep_out.read(&mut cmd_buf) {
                        if n > 0 {
                            let cmd = String::from_utf8_lossy(&cmd_buf[..n]).trim().to_string();
                            if !cmd.is_empty() {
                                Self::handle_native_command(
                                    &cmd,
                                    ep_in,
                                    ep_out,
                                    &mut download_buffer,
                                    &status,
                                );
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));
        }

        if let Some(mut child) = child_proc {
            let _ = child.kill();
        }
    }

    fn handle_native_command(
        cmd: &str,
        ep_in: &mut File,
        ep_out: &mut File,
        download_buffer: &mut Vec<u8>,
        status: &Arc<Mutex<FastbootStatus>>,
    ) {
        let cur_slot = {
            let mut st = status.lock().unwrap();
            st.is_connected = true;
            st.last_command = cmd.to_string();
            st.logs.push(format!("> {}", cmd));
            if st.logs.len() > 60 {
                st.logs.remove(0);
            }
            st.current_slot.clone()
        };

        if let Some(var) = cmd.strip_prefix("getvar:") {
            Self::handle_getvar(var, ep_in, &cur_slot);
        } else if let Some(hex_sz) = cmd.strip_prefix("download:") {
            Self::handle_download(hex_sz, ep_in, ep_out, download_buffer, status);
        } else if let Some(part) = cmd.strip_prefix("flash:") {
            Self::handle_flash(part, ep_in, download_buffer, &cur_slot, status);
        } else if let Some(part) = cmd.strip_prefix("erase:") {
            Self::handle_erase(part, ep_in, &cur_slot);
        } else if let Some(s) = cmd.strip_prefix("set_active:") {
            let slot_clean = s.trim();
            if slots::set_active_slot(slot_clean).is_ok() {
                if let Ok(mut st) = status.lock() {
                    st.current_slot = slot_clean.to_uppercase();
                }
                let _ = Self::send_response(ep_in, "OKAY", "");
            } else {
                let _ = Self::send_response(ep_in, "FAIL", "slot switch failed");
            }
        } else if cmd == "reboot" {
            let _ = Self::send_response(ep_in, "OKAY", "");
            crate::hardware::power::reboot(crate::hardware::power::RebootTarget::System);
        } else if cmd == "reboot-recovery" {
            let _ = Self::send_response(ep_in, "OKAY", "");
            crate::hardware::power::reboot(crate::hardware::power::RebootTarget::Recovery);
        } else if cmd == "reboot-bootloader" {
            let _ = Self::send_response(ep_in, "OKAY", "");
            crate::hardware::power::reboot(crate::hardware::power::RebootTarget::Bootloader);
        } else if cmd == "reboot-fastboot" {
            let _ = Self::send_response(ep_in, "OKAY", "");
        } else {
            let _ = Self::send_response(ep_in, "FAIL", "unknown command");
        }
    }

    fn send_response(ep_in: &mut File, prefix: &str, msg: &str) -> std::io::Result<()> {
        let payload = format!("{}{}", prefix, msg);
        ep_in.write_all(payload.as_bytes())?;
        ep_in.flush()
    }

    fn handle_getvar(var: &str, ep_in: &mut File, current_slot: &str) {
        match var {
            "is-userspace" => {
                let _ = Self::send_response(ep_in, "OKAY", "yes");
            }
            "max-download-size" => {
                let _ = Self::send_response(ep_in, "OKAY", "0x20000000");
            }
            "slot-count" => {
                let _ = Self::send_response(ep_in, "OKAY", "2");
            }
            "current-slot" => {
                let _ = Self::send_response(ep_in, "OKAY", &current_slot.to_lowercase());
            }
            "product" => {
                let _ = Self::send_response(ep_in, "OKAY", "degas");
            }
            "secure" => {
                let _ = Self::send_response(ep_in, "OKAY", "yes");
            }
            "unlocked" => {
                let _ = Self::send_response(ep_in, "OKAY", "yes");
            }
            "version" => {
                let _ = Self::send_response(ep_in, "OKAY", "0.4");
            }
            "version-bootloader" => {
                let _ = Self::send_response(ep_in, "OKAY", "MRR-Rust-1.0");
            }
            _ if var.starts_with("has-slot:") => {
                let part = var.strip_prefix("has-slot:").unwrap_or("");
                let is_slotted = matches!(
                    part,
                    "boot"
                        | "vendor_boot"
                        | "init_boot"
                        | "dtbo"
                        | "system"
                        | "vendor"
                        | "product"
                        | "system_ext"
                        | "odm"
                );
                let _ = Self::send_response(ep_in, "OKAY", if is_slotted { "yes" } else { "no" });
            }
            _ if var.starts_with("is-logical:") => {
                let part = var.strip_prefix("is-logical:").unwrap_or("");
                let clean = part.trim_end_matches("_a").trim_end_matches("_b");
                let is_log = matches!(
                    clean,
                    "system" | "vendor" | "product" | "system_ext" | "mi_ext" | "odm"
                );
                let _ = Self::send_response(ep_in, "OKAY", if is_log { "yes" } else { "no" });
            }
            _ if var.starts_with("partition-size:") => {
                let part = var.strip_prefix("partition-size:").unwrap_or("");
                if let Some(p) = crate::resolve_partition_block(part, current_slot) {
                    if let Ok(meta) = fs::metadata(p) {
                        let _ = Self::send_response(ep_in, "OKAY", &format!("0x{:x}", meta.len()));
                        return;
                    }
                }
                let _ = Self::send_response(ep_in, "FAIL", "partition not found");
            }
            _ => {
                let _ = Self::send_response(ep_in, "FAIL", "unknown variable");
            }
        }
    }

    fn handle_download(
        hex_size: &str,
        ep_in: &mut File,
        ep_out: &mut File,
        download_buffer: &mut Vec<u8>,
        status: &Arc<Mutex<FastbootStatus>>,
    ) {
        let sz = match usize::from_str_radix(hex_size.trim(), 16) {
            Ok(s) => s,
            Err(_) => {
                let _ = Self::send_response(ep_in, "FAIL", "invalid size");
                return;
            }
        };

        if sz > FASTBOOT_MAX_DOWNLOAD {
            let _ = Self::send_response(ep_in, "FAIL", "data exceeds max size");
            return;
        }

        if Self::send_response(ep_in, "DATA", &format!("{:08x}", sz)).is_err() {
            return;
        }

        download_buffer.resize(sz, 0);
        let mut read_bytes = 0;
        let mut chunk_buf = vec![0u8; 1024 * 1024];

        while read_bytes < sz {
            let to_read = (sz - read_bytes).min(chunk_buf.len());
            match ep_out.read(&mut chunk_buf[..to_read]) {
                Ok(0) => {
                    let _ = Self::send_response(ep_in, "FAIL", "unexpected EOF during download");
                    return;
                }
                Ok(n) => {
                    download_buffer[read_bytes..read_bytes + n].copy_from_slice(&chunk_buf[..n]);
                    read_bytes += n;
                    if let Ok(mut st) = status.lock() {
                        st.progress = (read_bytes as f32) / (sz as f32);
                        st.download_size = read_bytes;
                    }
                }
                Err(_) => {
                    let _ = Self::send_response(ep_in, "FAIL", "read error during download");
                    return;
                }
            }
        }

        if let Ok(mut st) = status.lock() {
            st.status_text = format!("Downloaded {} MB successfully.", sz / (1024 * 1024));
        }

        let _ = Self::send_response(ep_in, "OKAY", "");
    }

    fn handle_flash(
        partition: &str,
        ep_in: &mut File,
        download_buffer: &mut Vec<u8>,
        current_slot: &str,
        status: &Arc<Mutex<FastbootStatus>>,
    ) {
        if download_buffer.is_empty() {
            let _ = Self::send_response(ep_in, "FAIL", "no data in buffer to flash");
            return;
        }

        let target_block = match crate::resolve_partition_block(partition, current_slot) {
            Some(b) => b,
            None => {
                let mapper_cand = PathBuf::from(format!("/dev/block/mapper/{}", partition));
                if mapper_cand.exists() {
                    mapper_cand
                } else {
                    let _ = Self::send_response(ep_in, "FAIL", "block device not mapped");
                    return;
                }
            }
        };

        if let Ok(mut st) = status.lock() {
            st.status_text = format!("Flashing {}...", partition);
            st.logs.push(format!("Flashing to {:?}", target_block));
        }

        let _ = Self::send_response(ep_in, "INFO", "writing to device...");

        let mut out_file = match OpenOptions::new().write(true).open(&target_block) {
            Ok(f) => f,
            Err(e) => {
                let _ = Self::send_response(ep_in, "FAIL", &format!("cannot open partition: {}", e));
                return;
            }
        };

        let is_sparse = if download_buffer.len() >= std::mem::size_of::<SparseHeader>() {
            let magic = u32::from_le_bytes(download_buffer[0..4].try_into().unwrap());
            magic == SPARSE_HEADER_MAGIC
        } else {
            false
        };

        let res = if is_sparse {
            Self::unsparse_write(&mut out_file, download_buffer)
        } else {
            out_file.write_all(download_buffer).map(|_| ())
        };

        download_buffer.clear();
        download_buffer.shrink_to_fit();

        match res {
            Ok(_) => {
                if let Ok(mut st) = status.lock() {
                    st.status_text = format!("Flashed {} successfully.", partition);
                    st.logs.push(format!("Partition {} flash complete.", partition));
                }
                let _ = Self::send_response(ep_in, "OKAY", "");
            }
            Err(e) => {
                let _ = Self::send_response(ep_in, "FAIL", &format!("write error: {}", e));
            }
        }
    }

    fn unsparse_write(out: &mut File, buffer: &[u8]) -> std::io::Result<()> {
        let hdr: SparseHeader =
            unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const SparseHeader) };
        let blk_sz = hdr.blk_sz as u64;
        let mut cursor = hdr.file_hdr_sz as usize;

        for _ in 0..hdr.total_chunks {
            if cursor + std::mem::size_of::<ChunkHeader>() > buffer.len() {
                break;
            }
            let chdr: ChunkHeader = unsafe {
                std::ptr::read_unaligned(buffer[cursor..].as_ptr() as *const ChunkHeader)
            };
            cursor += hdr.chunk_hdr_sz as usize;

            let chunk_data_bytes = (chdr.chunk_sz as u64) * blk_sz;
            match chdr.chunk_type {
                0xCAC1 => {
                    let end = cursor + chunk_data_bytes as usize;
                    if end <= buffer.len() {
                        out.write_all(&buffer[cursor..end])?;
                    }
                    cursor = end;
                }
                0xCAC2 => {
                    let fill_val =
                        u32::from_le_bytes(buffer[cursor..cursor + 4].try_into().unwrap());
                    let fill_buf = fill_val.to_le_bytes();
                    let full_block = fill_buf.repeat((blk_sz / 4) as usize);
                    for _ in 0..chdr.chunk_sz {
                        out.write_all(&full_block)?;
                    }
                    cursor += 4;
                }
                0xCAC3 => {
                    out.seek(SeekFrom::Current(chunk_data_bytes as i64))?;
                }
                _ => {}
            }
        }
        out.flush()?;
        Ok(())
    }

    fn handle_erase(partition: &str, ep_in: &mut File, current_slot: &str) {
        if let Some(target_block) = crate::resolve_partition_block(partition, current_slot) {
            if let Ok(mut f) = OpenOptions::new().write(true).open(target_block) {
                let zero_buf = [0u8; 4096];
                let _ = f.write_all(&zero_buf);
                let _ = f.flush();
            }
        }
        let _ = Self::send_response(ep_in, "OKAY", "");
    }
}
