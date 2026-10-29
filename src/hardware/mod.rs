pub mod backlight;
pub mod battery;
pub mod dynamic_partitions;
pub mod installer;
pub mod power;
pub mod restore;
pub mod root;
pub mod slots;
pub mod storage;

pub trait ProgressCallback {
    fn step(&mut self, desc: &str, pct: f32);
    fn log(&mut self, line: &str);
}
