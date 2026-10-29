pub mod dm;
pub mod fscrypt;
pub mod keys;

pub fn auto_decrypt() -> bool {
    println!("==========================================");
    println!("   MeowFloprRecovery (MRR) - Rust Crypto  ");
    println!("==========================================");

    keys::ensure_persist_mounted();
    let (dm_key, ce_key) = match keys::get_active_keys() {
        Some(k) => k,
        None => {
            println!("[MRR-CRYPTO] No stored encryption keys found in persist storage.");
            return fscrypt::verify_decryption();
        }
    };

    if let Err(e) = fscrypt::ensure_data_mounted(&dm_key) {
        eprintln!("[MRR-CRYPTO] Failed to mount /data: {}", e);
        return false;
    }

    if !fscrypt::verify_decryption() {
        println!("[MRR-CRYPTO] Standard directories not visible yet, injecting CE Master Key...");
        if let Err(e) = fscrypt::inject_master_key_to_kernel(&ce_key) {
            eprintln!("[MRR-CRYPTO] Failed to inject master key: {}", e);
        }
    }

    let _ = fscrypt::bind_mount_sdcard();

    let success = fscrypt::verify_decryption();
    if success {
        println!("[MRR-CRYPTO] Decryption SUCCESS! /sdcard is readable.");
    } else {
        eprintln!("[MRR-CRYPTO] Warning: verify_decryption failed.");
    }
    success
}
