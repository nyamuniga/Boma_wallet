use boma_core::generate_entropy::generate_entropy;
use boma_core::generate_mnemonic::generate_mnemonic;
use boma_core::derive_seed_from_mnemonic::derive_seed_from_mnemonic;
use boma_core::derive_keys::derive_keys;
use boma_core::generate_many_addresses::generate_many_addresses;
use boma_core::store_backup::{store_backup, load_backup};
use bitcoin::network::constants::Network;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct WalletData {
    mnemonic: String,
    fingerprint: String,
}

#[tauri::command]
fn check_wallet_exists() -> bool {
    Path::new("backup.txt").exists()
}

#[tauri::command]
fn create_wallet(passphrase: &str) -> Result<WalletData, String> {
    let entropy = generate_entropy();
    let mnemonic = generate_mnemonic(&entropy);
    let mnemonic_str = mnemonic.to_string();
    
    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    
    store_backup(passphrase, &mnemonic_str, "backup.txt").map_err(|e| e.to_string())?;
    
    Ok(WalletData { mnemonic: mnemonic_str, fingerprint })
}

#[tauri::command]
fn restore_wallet(mnemonic: &str, passphrase: &str) -> Result<DashboardData, String> {
    use bip39::Mnemonic;
    use std::str::FromStr;

    // Validate and normalize the mnemonic (bip39 checks wordlist + checksum)
    let validated = Mnemonic::from_str(&mnemonic.trim().to_lowercase())
        .map_err(|e| format!("Invalid recovery phrase: {}", e))?;
    let mnemonic_str = validated.to_string();

    // Guard: warn but allow overwrite (GUI handles confirmation before calling)
    store_backup(passphrase, &mnemonic_str, "backup.txt").map_err(|e| e.to_string())?;

    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    let addresses = generate_many_addresses(&root_key, Network::Bitcoin);
    let receive_addresses = addresses.into_iter().map(|(addr, _)| addr.to_string()).collect();

    Ok(DashboardData { fingerprint, receive_addresses })
}


#[derive(Serialize)]
pub struct DashboardData {
    fingerprint: String,
    receive_addresses: Vec<String>,
}

#[tauri::command]
fn login(passphrase: &str) -> Result<DashboardData, String> {
    let mnemonic_str = load_backup(passphrase, "backup.txt").map_err(|e| e.to_string())?;
    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    let addresses = generate_many_addresses(&root_key, Network::Bitcoin);
    
    let receive_addresses = addresses.into_iter().map(|(addr, _)| addr.to_string()).collect();
    
    Ok(DashboardData { fingerprint, receive_addresses })
}

#[tauri::command]
fn export_xpub(passphrase: &str, save_path: &str) -> Result<(), String> {
    let mnemonic_str = load_backup(passphrase, "backup.txt").map_err(|e| e.to_string())?;
    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    boma_core::wallet_info::export_watch_wallet(&root_key, Network::Bitcoin, save_path)
}

#[tauri::command]
fn export_descriptor(passphrase: &str, save_path: &str) -> Result<(), String> {
    let mnemonic_str = load_backup(passphrase, "backup.txt").map_err(|e| e.to_string())?;
    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    boma_core::wallet_info::export_descriptor(&root_key, Network::Bitcoin, save_path)
}

#[tauri::command]
fn get_recovery_phrase(passphrase: &str) -> Result<String, String> {
    load_backup(passphrase, "backup.txt").map_err(|e| e.to_string())
}

#[tauri::command]
fn change_passphrase(old_passphrase: &str, new_passphrase: &str) -> Result<(), String> {
    let mnemonic_str = load_backup(old_passphrase, "backup.txt").map_err(|e| e.to_string())?;
    store_backup(new_passphrase, &mnemonic_str, "backup.txt").map_err(|e| e.to_string())
}

#[tauri::command]
fn import_utxos(path: &str) -> Result<Vec<boma_core::transaction::Utxo>, String> {
    boma_core::transaction::import_utxos_from_csv(path)
}

#[tauri::command]
fn build_transaction(
    passphrase: &str,
    txid_str: String,
    vout: u32,
    input_sats: u64,
    from_address_str: &str,
    to_address_str: &str,
    send_sats: u64,
    fee_sats: u64,
    use_rbf: bool,
    dry_run: bool,
) -> Result<String, String> {
    use bitcoin::util::address::Address;
    use std::str::FromStr;
    use boma_core::transaction::{TxParams, build_transaction as build_tx};
    
    let mnemonic_str = load_backup(passphrase, "backup.txt").map_err(|e| e.to_string())?;
    let seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    
    let receive_addresses = generate_many_addresses(&root_key, Network::Bitcoin);
    let change_addresses = boma_core::change_addresses::generate_change_addresses(&root_key, Network::Bitcoin);
    
    let from_addr_obj = Address::from_str(from_address_str).map_err(|_| "Invalid from address")?;
    let to_address = Address::from_str(to_address_str).map_err(|_| "Invalid to address")?;
    
    let from_pair = receive_addresses.iter()
        .find(|(a, _)| a == &from_addr_obj)
        .ok_or_else(|| "From address not found in wallet receive addresses".to_string())?;
        
    let change_address = change_addresses.first().map(|(a, _)| a).unwrap_or(&from_pair.0);
    
    let p = TxParams {
        from_address: &from_pair.0,
        from_key: &from_pair.1,
        txid_str,
        vout,
        input_sats,
        to_address,
        send_sats,
        fee_sats,
        change_address,
        use_rbf,
        dry_run,
    };
    
    build_tx(&p)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConfigData {
    network: String,
    session_timeout_secs: u64,
}

#[tauri::command]
fn get_settings() -> ConfigData {
    let cfg = boma_core::config::Config::load();
    ConfigData {
        network: if cfg.network == Network::Bitcoin { "mainnet".to_string() } else { "testnet".to_string() },
        session_timeout_secs: cfg.session_timeout_secs,
    }
}

#[tauri::command]
fn update_settings(network: &str, session_timeout_secs: u64) -> Result<(), String> {
    let mut cfg = boma_core::config::Config::load();
    cfg.network = if network == "testnet" { Network::Testnet } else { Network::Bitcoin };
    cfg.session_timeout_secs = session_timeout_secs;
    cfg.save()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // macOS App bundles are read-only. We change the CWD to ~/.boma
    // so that backup.txt and wallet_config.txt write to a persistent, writable location.
    if let Some(mut path) = dirs::home_dir() {
        path.push(".boma");
        let _ = std::fs::create_dir_all(&path);
        let _ = std::env::set_current_dir(&path);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_wallet_exists,
            create_wallet,
            restore_wallet,
            login,
            export_xpub,
            export_descriptor,
            get_recovery_phrase,
            change_passphrase,
            import_utxos,
            build_transaction,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
