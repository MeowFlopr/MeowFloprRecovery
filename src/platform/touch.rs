use std::ffi::{CStr, CString};
use std::path::Path;
use std::time::Instant;

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const KEY_POWER: u16 = 116;
const BTN_TOUCH: u16 = 0x14a;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct InputAbsInfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchState {
    Released,
    Pressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchEvent {
    pub x: i32,
    pub y: i32,
    pub state: TouchState,
}

pub struct InputManager {
    touch_fd: libc::c_int,
    power_fd: libc::c_int,
    last_x: i32,
    last_y: i32,
    max_x: i32,
    max_y: i32,
    display_width: i32,
    display_height: i32,
    active_slot: i32,
    touch_state: TouchState,
    last_activity: Instant,
}

fn test_bit(bit: usize, array: &[u8]) -> bool {
    let byte_idx = bit / 8;
    let bit_idx = bit % 8;
    if byte_idx < array.len() {
        (array[byte_idx] & (1 << bit_idx)) != 0
    } else {
        false
    }
}

#[cfg(target_env = "musl")]
type IoctlReq = libc::c_int;
#[cfg(not(target_env = "musl"))]
type IoctlReq = libc::c_ulong;

fn probe_input_devices() -> (libc::c_int, libc::c_int, i32, i32) {
    let mut touch_fd = -1;
    let mut power_fd = -1;
    let mut max_x = 1220;
    let mut max_y = 2712;

    let eviocgname = 0x81004506u32 as IoctlReq;
    let eviocgbit_types = 0x80084520u32 as IoctlReq;
    let eviocgbit_key = 0x80404521u32 as IoctlReq;
    let eviocgbit_abs = 0x80404523u32 as IoctlReq;
    let eviocgabs_x = 0x80184575u32 as IoctlReq;
    let eviocgabs_y = 0x80184576u32 as IoctlReq;

    for i in 0..32 {
        let dev_path = format!("/dev/input/event{}", i);
        if !Path::new(&dev_path).exists() {
            continue;
        }

        let c_path = match CString::new(dev_path.clone()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let fd = unsafe {
            libc::open(
                c_path.as_ptr(),
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            continue;
        }

        let mut name_buf = [0u8; 256];
        let _ = unsafe { libc::ioctl(fd, eviocgname, name_buf.as_mut_ptr()) };
        let name_str = CStr::from_bytes_until_nul(&name_buf)
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Unknown Input Device".to_string());

        let mut ev_types = [0u8; 8];
        let _ = unsafe { libc::ioctl(fd, eviocgbit_types, ev_types.as_mut_ptr()) };

        let has_keys = test_bit(EV_KEY as usize, &ev_types);
        let has_abs = test_bit(EV_ABS as usize, &ev_types);

        if has_abs && touch_fd < 0 {
            let mut abs_bits = [0u8; 64];
            let _ = unsafe { libc::ioctl(fd, eviocgbit_abs, abs_bits.as_mut_ptr()) };

            let has_mt_x = test_bit(ABS_MT_POSITION_X as usize, &abs_bits);
            let has_raw_x = test_bit(ABS_X as usize, &abs_bits);

            if has_mt_x || has_raw_x {
                touch_fd = fd;
                println!(
                    "[MRR-INPUT] Discovered Touchscreen on {}: \"{}\"",
                    dev_path, name_str
                );

                let mut abs_info_x = InputAbsInfo::default();
                let mut abs_info_y = InputAbsInfo::default();
                let ret_x = unsafe { libc::ioctl(fd, eviocgabs_x, &mut abs_info_x) };
                let ret_y = unsafe { libc::ioctl(fd, eviocgabs_y, &mut abs_info_y) };

                if ret_x == 0 && abs_info_x.maximum > 0 {
                    max_x = abs_info_x.maximum;
                }
                if ret_y == 0 && abs_info_y.maximum > 0 {
                    max_y = abs_info_y.maximum;
                }

                println!(
                    "[MRR-INPUT] Touch axis calibration bounds: X[0..{}], Y[0..{}]",
                    max_x, max_y
                );
                continue;
            }
        }

        if has_keys && power_fd < 0 {
            let mut key_bits = [0u8; 64];
            let _ = unsafe { libc::ioctl(fd, eviocgbit_key, key_bits.as_mut_ptr()) };

            if test_bit(KEY_POWER as usize, &key_bits) {
                power_fd = fd;
                println!(
                    "[MRR-INPUT] Discovered Power Key on {}: \"{}\"",
                    dev_path, name_str
                );
                continue;
            }
        }

        unsafe { libc::close(fd) };
    }

    if touch_fd < 0 {
        let fallback_touch = b"/dev/input/event3\0";
        touch_fd = unsafe {
            libc::open(
                fallback_touch.as_ptr() as *const libc::c_char,
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if touch_fd >= 0 {
            println!("[MRR-INPUT] Touchscreen fallback connected (/dev/input/event3)");
        }
    }

    if power_fd < 0 {
        let fallback_pwr = b"/dev/input/event1\0";
        power_fd = unsafe {
            libc::open(
                fallback_pwr.as_ptr() as *const libc::c_char,
                libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if power_fd >= 0 {
            println!("[MRR-INPUT] Power Key fallback connected (/dev/input/event1)");
        }
    }

    (touch_fd, power_fd, max_x, max_y)
}

impl InputManager {
    pub fn new() -> Self {
        let (touch_fd, power_fd, max_x, max_y) = probe_input_devices();

        Self {
            touch_fd,
            power_fd,
            last_x: 0,
            last_y: 0,
            max_x,
            max_y,
            display_width: 1220,
            display_height: 2712,
            active_slot: 0,
            touch_state: TouchState::Released,
            last_activity: Instant::now(),
        }
    }

    pub fn set_display_size(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.display_width = width as i32;
            self.display_height = height as i32;
        }
    }

    #[inline]
    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }

    pub fn scale_coordinate(val: i32, max_bound: i32, target_dim: i32) -> i32 {
        let bound = if max_bound > 0 { max_bound } else { target_dim };
        let scaled = (val.max(0) as f64 / bound as f64) * target_dim as f64;
        (scaled as i32).clamp(0, target_dim.saturating_sub(1))
    }

    pub fn poll_events(&mut self) -> (bool, Option<TouchEvent>) {
        let mut toggle_screen = false;

        if self.power_fd >= 0 {
            let mut ev: libc::input_event = unsafe { std::mem::zeroed() };
            let ev_size = std::mem::size_of::<libc::input_event>();

            loop {
                let n = unsafe {
                    libc::read(
                        self.power_fd,
                        &mut ev as *mut _ as *mut libc::c_void,
                        ev_size,
                    )
                };
                if n as usize != ev_size {
                    break;
                }

                if ev.type_ == EV_KEY && ev.code == KEY_POWER && ev.value == 0 {
                    toggle_screen = true;
                    self.last_activity = Instant::now();
                    println!("[MRR-POWER] Power key released -> Requesting display toggle");
                }
            }
        }

        if self.touch_fd < 0 {
            return (toggle_screen, None);
        }

        let mut ev: libc::input_event = unsafe { std::mem::zeroed() };
        let ev_size = std::mem::size_of::<libc::input_event>();
        let mut has_new_touch = false;

        loop {
            let n = unsafe {
                libc::read(
                    self.touch_fd,
                    &mut ev as *mut _ as *mut libc::c_void,
                    ev_size,
                )
            };
            if n as usize != ev_size {
                break;
            }

            has_new_touch = true;
            self.last_activity = Instant::now();

            if ev.type_ == EV_ABS {
                match ev.code {
                    ABS_MT_SLOT => {
                        self.active_slot = ev.value;
                    }
                    ABS_MT_POSITION_X if self.active_slot == 0 => {
                        self.last_x = Self::scale_coordinate(ev.value, self.max_x, self.display_width);
                    }
                    ABS_MT_POSITION_Y if self.active_slot == 0 => {
                        self.last_y = Self::scale_coordinate(ev.value, self.max_y, self.display_height);
                    }
                    ABS_MT_TRACKING_ID if self.active_slot == 0 => {
                        self.touch_state = if ev.value == -1 {
                            TouchState::Released
                        } else {
                            TouchState::Pressed
                        };
                    }
                    _ => {}
                }
            } else if ev.type_ == EV_KEY && ev.code == BTN_TOUCH {
                self.touch_state = if ev.value == 0 {
                    TouchState::Released
                } else {
                    TouchState::Pressed
                };
            }
        }

        if has_new_touch {
            let event = TouchEvent {
                x: self.last_x,
                y: self.last_y,
                state: self.touch_state,
            };
            (toggle_screen, Some(event))
        } else {
            (toggle_screen, None)
        }
    }
}

impl Drop for InputManager {
    fn drop(&mut self) {
        if self.touch_fd >= 0 {
            unsafe { libc::close(self.touch_fd) };
        }
        if self.power_fd >= 0 {
            unsafe { libc::close(self.power_fd) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_coordinate() {
        let x = InputManager::scale_coordinate(500, 1000, 1220);
        assert_eq!(x, 610);

        let clamped_over = InputManager::scale_coordinate(1500, 1000, 1220);
        assert_eq!(clamped_over, 1219);

        let clamped_under = InputManager::scale_coordinate(-50, 1000, 1220);
        assert_eq!(clamped_under, 0);
    }
}
