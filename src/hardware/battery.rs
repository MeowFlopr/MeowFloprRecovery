use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryInfo {
    pub percent: u32,
    pub temp_c: i32,
    pub is_charging: bool,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            percent: 0,
            temp_c: 0,
            is_charging: false,
        }
    }
}

pub fn get_battery_info() -> BatteryInfo {
    let mut info = BatteryInfo::default();

    let cap_paths = [
        "/sys/class/power_supply/battery/capacity",
        "/sys/class/power_supply/mtk-gauge/capacity",
        "/sys/class/power_supply/gauge/capacity",
    ];
    for p in cap_paths {
        if let Ok(cap_str) = fs::read_to_string(p) {
            if let Ok(val) = cap_str.trim().parse::<u32>() {
                info.percent = val.min(100);
                break;
            }
        }
    }

    let temp_paths = [
        "/sys/class/power_supply/battery/temp",
        "/sys/class/power_supply/mtk-gauge/temp",
    ];
    for p in temp_paths {
        if let Ok(temp_str) = fs::read_to_string(p) {
            if let Ok(val) = temp_str.trim().parse::<i32>() {
                info.temp_c = if val > 100 { val / 10 } else { val };
                break;
            }
        }
    }

    let status_paths = [
        "/sys/class/power_supply/battery/status",
        "/sys/class/power_supply/mtk-gauge/status",
    ];
    for p in status_paths {
        if let Ok(status_str) = fs::read_to_string(p) {
            let s = status_str.trim().to_lowercase();
            info.is_charging = s == "charging" || s == "full";
            break;
        }
    }

    info
}
