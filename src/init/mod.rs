pub mod mounts;
pub mod modules;

pub fn early_boot() -> Result<(), std::io::Error> {
    mounts::early_mount_vfs()?;
    modules::init_touch_hardware();
    Ok(())
}
