use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

static USER_BRIGHTNESS: AtomicI32 = AtomicI32::new(2500);
static SCREEN_POWER: AtomicBool = AtomicBool::new(true);

const BACKLIGHT_CANDIDATES: [&str; 4] = [
    "/sys/class/backlight/panel0-backlight/brightness",
    "/sys/class/leds/lcd-backlight/brightness",
    "/sys/class/backlight/mtk_leds-ktd/brightness",
    "/sys/class/leds/sc6607-backlight/brightness",
];

pub fn set_brightness(val: i32) {
    let clamped = val.clamp(100, 10000);
    USER_BRIGHTNESS.store(clamped, Ordering::Relaxed);
    if SCREEN_POWER.load(Ordering::Relaxed) {
        write_raw_brightness(clamped);
    }
}

pub fn get_brightness() -> i32 {
    USER_BRIGHTNESS.load(Ordering::Relaxed)
}

pub fn set_screen_power(on: bool) {
    SCREEN_POWER.store(on, Ordering::Relaxed);
    if on {
        let level = USER_BRIGHTNESS.load(Ordering::Relaxed);
        write_raw_brightness(level);
        println!("[MRR-DISPLAY] Screen power set to ON (brightness={})", level);
    } else {
        write_raw_brightness(0);
        println!("[MRR-DISPLAY] Screen power set to OFF (sleep)");
    }
}

pub fn is_screen_power() -> bool {
    SCREEN_POWER.load(Ordering::Relaxed)
}

fn write_raw_brightness(val: i32) {
    for p in BACKLIGHT_CANDIDATES {
        if Path::new(p).exists() {
            let _ = fs::write(p, format!("{}\n", val));
            return;
        }
    }
    let _ = fs::write(BACKLIGHT_CANDIDATES[0], format!("{}\n", val));
}
