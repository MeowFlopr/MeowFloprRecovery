pub mod dm;
pub mod fscrypt;
pub mod keys;

pub fn auto_decrypt() -> bool {
    println!("==========================================");
    println!("   MeowFloprRecovery (MRR) - Rust Crypto  ");
    println!("==========================================");

    keys::ensure_persist_mounted();
    let (dm_key, ce_key) = keys::get_active_keys();

    // Step 1: Mount /data
    if let Err(e) = fscrypt::ensure_data_mounted(&dm_key) {
        eprintln!("[MRR-CRYPTO] Failed to mount /data: {}", e);
        return false;
    }

    // Step 2: Always inject master key if standard directories aren't verified yet
    if !fscrypt::verify_decryption() {
        println!("[MRR-CRYPTO] Standard directories not visible yet, injecting CE Master Key...");
        if let Err(e) = fscrypt::inject_master_key_to_kernel(&ce_key) {
            eprintln!("[MRR-CRYPTO] Failed to inject master key: {}", e);
        }
    }

    // Step 3: Bind mount /data/media/0 -> /sdcard
    let _ = fscrypt::bind_mount_sdcard();

    // Step 4: Verify decryption
    let success = fscrypt::verify_decryption();
    if success {
        println!("[MRR-CRYPTO] Decryption SUCCESS! /sdcard is readable.");
    } else {
        eprintln!("[MRR-CRYPTO] Warning: verify_decryption failed.");
    }
    success
}
