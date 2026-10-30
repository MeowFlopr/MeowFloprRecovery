use std::fs;

#[derive(Debug, Clone, Copy)]
pub struct BatteryInfo {
    pub percent: u32,
    pub temp_c: i32,
    pub is_charging: bool,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            percent: 85,
            temp_c: 30,
            is_charging: false,
        }
    }
}

pub fn get_battery_info() -> BatteryInfo {
    let mut info = BatteryInfo::default();

    // 1. Capacity
    if let Ok(cap_str) = fs::read_to_string("/sys/class/power_supply/battery/capacity") {
        if let Ok(val) = cap_str.trim().parse::<u32>() {
            info.percent = val.min(100);
        }
    }

    // 2. Temperature (raw in tenths of a degree Celsius, e.g. 310 -> 31°C)
    if let Ok(temp_str) = fs::read_to_string("/sys/class/power_supply/battery/temp") {
        if let Ok(val) = temp_str.trim().parse::<i32>() {
            info.temp_c = val / 10;
        }
    }

    // 3. Charging status
    if let Ok(status_str) = fs::read_to_string("/sys/class/power_supply/battery/status") {
        let s = status_str.trim().to_lowercase();
        info.is_charging = s == "charging" || s == "full";
    }

    info
}
