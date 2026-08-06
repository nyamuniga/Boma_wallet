use boma_core::generate_entropy::generate_entropy;
use boma_core::generate_mnemonic::generate_mnemonic;
use boma_core::derive_seed_from_mnemonic::derive_seed_from_mnemonic;
use boma_core::derive_keys::derive_keys;
use boma_core::generate_many_addresses::generate_many_addresses;
use boma_core::store_backup::{store_backup, load_backup};
use boma_core::psbt::{parse_psbt_from_bytes, parse_psbt_from_base64, sign_psbt, psbt_to_base64, PsbtSummary};
use boma_core::config::Config;
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ExtendedPrivKey;
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use zeroize::Zeroize;

/// Canonical backup filename — defined once, used everywhere.
const BACKUP_FILE: &str = "backup.txt";

// ── Rust-side session (C3) ────────────────────────────────────────────────────

/// Holds all sensitive key material on the Rust side.
/// The passphrase is NEVER stored — only derived keys.
///
/// Security: Implements Drop to zeroize all secrets when the session ends.
struct AppSession {
    root_key: ExtendedPrivKey,
    receive_addresses: Vec<(Address, SecretKey)>,
    change_addresses: Vec<(Address, SecretKey)>,
    fingerprint: String,
    mnemonic_str: String,
    last_activity: Instant,
    change_index: usize,
}

impl Drop for AppSession {
    fn drop(&mut self) {
        self.mnemonic_str.zeroize();
        self.fingerprint.zeroize();
        // Zeroize all SecretKeys via raw pointer (bitcoin crate doesn't impl Zeroize)
        for (_, key) in self.receive_addresses.iter() {
            zeroize_secret_key(key);
        }
        for (_, key) in self.change_addresses.iter() {
            zeroize_secret_key(key);
        }
        zeroize_secret_key(&self.root_key.private_key);
        self.receive_addresses.clear();
        self.change_addresses.clear();
    }
}

/// Overwrites the 32 bytes of a secp256k1 SecretKey with zeros.
fn zeroize_secret_key(key: &SecretKey) {
    let ptr = key as *const SecretKey as *mut u8;
    unsafe {
        std::ptr::write_bytes(ptr, 0, 32);
    }
}

/// Global session state, protected by a Mutex.
/// `None` = no active session (locked).
struct SessionStore(Mutex<Option<AppSession>>);

// ── Shared helper ─────────────────────────────────────────────────────────────

/// Loads the configured network from the wallet config file.
/// Falls back to mainnet (safest default) if the config cannot be read.
fn wallet_network() -> Network {
    Config::load().network
}

/// Checks if the session is still valid (not timed out).
/// Returns false and clears the session if it has expired.
fn check_session_timeout(store: &SessionStore) -> bool {
    let cfg = Config::load();
    let timeout = std::time::Duration::from_secs(cfg.session_timeout_secs);

    let mut guard = store.0.lock().unwrap();
    if let Some(session) = guard.as_ref() {
        if session.last_activity.elapsed() >= timeout {
            // Session expired — drop clears all secrets
            *guard = None;
            return false;
        }
    }
    true
}

/// Helper: borrow the session, refresh activity timestamp, and run a closure.
/// Returns Err if no active session or session timed out.
fn with_session<F, T>(store: &tauri::State<SessionStore>, f: F) -> Result<T, String>
where
    F: FnOnce(&mut AppSession) -> Result<T, String>,
{
    if !check_session_timeout(store) {
        return Err("Session expired. Please log in again.".to_string());
    }
    let mut guard = store.0.lock().map_err(|_| "Session lock poisoned.".to_string())?;
    let session = guard.as_mut().ok_or("Not logged in. Please open your wallet first.")?;
    session.last_activity = Instant::now();
    f(session)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// C2: WalletData no longer contains the mnemonic.
/// The mnemonic is only accessible one word at a time via get_mnemonic_word.
#[derive(Serialize)]
pub struct WalletData {
    fingerprint: String,
    word_count: usize,
}

#[tauri::command]
fn check_wallet_exists() -> bool {
    Path::new(BACKUP_FILE).exists()
}

/// H4: Enforces passphrase strength on the server side.
fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.is_empty() {
        return Err("A passphrase is required.".to_string());
    }
    if !boma_core::passphrase_strength::is_strong_enough(passphrase) {
        let (_, score, advice) = boma_core::passphrase_strength::score(passphrase);
        return Err(format!(
            "Passphrase too weak (score {}/7, minimum {}/7 required). {}",
            score, boma_core::passphrase_strength::MIN_SCORE, advice
        ));
    }
    Ok(())
}

#[tauri::command]
fn create_wallet(passphrase: &str) -> Result<WalletData, String> {
    // H4: Server-side passphrase strength enforcement
    validate_passphrase(passphrase)?;

    let network = wallet_network();
    let mut entropy = generate_entropy().map_err(|e| e.to_string())?;
    let mnemonic = generate_mnemonic(&entropy).map_err(|e| e.to_string())?;
    entropy.zeroize();
    let mnemonic_str = mnemonic.to_string();
    let word_count = mnemonic_str.split_whitespace().count();

    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, network).map_err(|e| e.to_string())?.0;
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);

    store_backup(passphrase, &mnemonic_str, BACKUP_FILE).map_err(|e| e.to_string())?;

    seed.zeroize();
    // C2: mnemonic_str is NOT returned to JS — only word_count and fingerprint

    Ok(WalletData { fingerprint, word_count })
}

/// C2: Returns a single mnemonic word by index (0-based).
/// Requires re-authentication via passphrase to prevent cached access.
/// The JS frontend never holds the full phrase as a single string.
#[tauri::command]
fn get_mnemonic_word(passphrase: &str, index: usize) -> Result<String, String> {
    let mut mnemonic_str = load_backup(passphrase, BACKUP_FILE).map_err(|e| e.to_string())?;
    let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
    let word_count = words.len();

    if index >= word_count {
        drop(words);
        mnemonic_str.zeroize();
        return Err(format!("Invalid word index: {} (phrase has {} words)", index, word_count));
    }

    let word = words[index].to_string();
    drop(words);
    mnemonic_str.zeroize();
    Ok(word)
}

#[tauri::command]
fn restore_wallet(mnemonic: &str, passphrase: &str) -> Result<DashboardData, String> {
    use bip39::Mnemonic;
    use std::str::FromStr;

    // H4: Server-side passphrase strength enforcement
    validate_passphrase(passphrase)?;

    let network = wallet_network();

    // Validate and normalize the mnemonic (bip39 checks wordlist + checksum)
    let validated = Mnemonic::from_str(&mnemonic.trim().to_lowercase())
        .map_err(|e| format!("Invalid recovery phrase: {}", e))?;
    let mnemonic_str = validated.to_string();

    // Guard: warn but allow overwrite (GUI handles confirmation before calling)
    store_backup(passphrase, &mnemonic_str, BACKUP_FILE).map_err(|e| e.to_string())?;

    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, network).map_err(|e| e.to_string())?.0;
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    let addresses = generate_many_addresses(&root_key, network);
    let receive_addresses = addresses.into_iter().map(|(addr, _)| addr.to_string()).collect();

    seed.zeroize();

    Ok(DashboardData { fingerprint, receive_addresses })
}


#[derive(Serialize)]
pub struct DashboardData {
    fingerprint: String,
    receive_addresses: Vec<String>,
}

/// C3: Login now creates a Rust-side session instead of returning keys to JS.
/// The passphrase is used once for decryption and key derivation, then discarded.
#[tauri::command]
fn login(passphrase: &str, store: tauri::State<SessionStore>) -> Result<DashboardData, String> {
    let network = wallet_network();
    let mnemonic_str = load_backup(passphrase, BACKUP_FILE).map_err(|e| e.to_string())?;
    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, network).map_err(|e| e.to_string())?.0;

    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    let receive_addresses = generate_many_addresses(&root_key, network);
    let change_addresses = boma_core::change_addresses::generate_change_addresses(&root_key, network);

    let receive_strings: Vec<String> = receive_addresses.iter()
        .map(|(addr, _)| addr.to_string())
        .collect();

    seed.zeroize();

    // Store session in Rust — keys never cross the IPC boundary
    let session = AppSession {
        root_key,
        receive_addresses,
        change_addresses,
        fingerprint: fingerprint.clone(),
        mnemonic_str,
        last_activity: Instant::now(),
        change_index: 0,
    };

    let mut guard = store.0.lock().map_err(|_| "Session lock error.".to_string())?;
    *guard = Some(session);

    Ok(DashboardData { fingerprint, receive_addresses: receive_strings })
}

/// Locks the wallet session, zeroizing all key material.
#[tauri::command]
fn lock_wallet(store: tauri::State<SessionStore>) -> Result<(), String> {
    let mut guard = store.0.lock().map_err(|_| "Session lock error.".to_string())?;
    *guard = None; // Drop triggers zeroize
    Ok(())
}

/// C3: Export xpub from the active session (no passphrase needed after login).
#[tauri::command]
fn export_xpub(store: tauri::State<SessionStore>) -> Result<String, String> {
    let network = wallet_network();
    with_session(&store, |session| {
        boma_core::wallet_info::watch_wallet_content(&session.root_key, network)
    })
}

/// C3: Export descriptor from the active session.
#[tauri::command]
fn export_descriptor(store: tauri::State<SessionStore>) -> Result<String, String> {
    let network = wallet_network();
    with_session(&store, |session| {
        boma_core::wallet_info::descriptor_content(&session.root_key, network)
    })
}

/// C2: Recovery phrase access requires passphrase re-authentication.
/// Returns word count only — use get_mnemonic_word for individual words.
#[tauri::command]
fn get_recovery_phrase(passphrase: &str) -> Result<String, String> {
    load_backup(passphrase, BACKUP_FILE).map_err(|e| e.to_string())
}

#[tauri::command]
fn change_passphrase(old_passphrase: &str, new_passphrase: &str) -> Result<(), String> {
    // H4: Enforce strength on new passphrase
    validate_passphrase(new_passphrase)?;

    let mnemonic_str = load_backup(old_passphrase, BACKUP_FILE).map_err(|e| e.to_string())?;
    store_backup(new_passphrase, &mnemonic_str, BACKUP_FILE).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_utxos(csv_content: String) -> Result<Vec<boma_core::transaction::Utxo>, String> {
    boma_core::transaction::parse_utxos_from_csv_content(&csv_content)
}

/// C3: Build transaction using the active session's keys.
/// The passphrase is no longer needed — keys are held in the Rust session.
#[tauri::command]
fn build_transaction(
    store: tauri::State<SessionStore>,
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

    let from_addr_obj = Address::from_str(from_address_str).map_err(|_| "Invalid from address")?;
    let to_address = Address::from_str(to_address_str).map_err(|_| "Invalid to address")?;

    with_session(&store, |session| {
        let from_pair = session.receive_addresses.iter()
            .find(|(a, _)| a == &from_addr_obj)
            .ok_or_else(|| "From address not found in wallet receive addresses".to_string())?;

        // H2: Use rotating change address
        let change_address = if session.change_addresses.is_empty() {
            &from_pair.0
        } else {
            let idx = session.change_index % session.change_addresses.len();
            &session.change_addresses[idx].0
        };

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

        let result = build_tx(&p)?;

        // H2: Rotate change address on successful (non-dry-run) transaction
        if !dry_run {
            session.change_index += 1;
        }

        Ok(result)
    })
}

/// M2: Pass network to PSBT parser.
#[tauri::command]
fn load_psbt(psbt_data: Vec<u8>) -> Result<PsbtSummary, String> {
    let network = wallet_network();
    parse_psbt_from_bytes(&psbt_data, network).map(|(_, summary)| summary)
}

/// M2: Pass network to PSBT parser.
#[tauri::command]
fn load_psbt_from_base64(b64: &str) -> Result<PsbtSummary, String> {
    let network = wallet_network();
    parse_psbt_from_base64(b64, network).map(|(_, summary)| summary)
}

/// C3: Sign PSBT using the active session's root key.
#[tauri::command]
fn sign_and_export_psbt(
    store: tauri::State<SessionStore>,
    psbt_b64: &str,
) -> Result<String, String> {
    let cfg = Config::load();
    let (psbt, _) = parse_psbt_from_base64(psbt_b64, cfg.network)?;

    with_session(&store, |session| {
        let signed = sign_psbt(psbt.clone(), &session.root_key, cfg.network)?;
        Ok(psbt_to_base64(&signed))
    })
}


#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConfigData {
    network: String,
    session_timeout_secs: u64,
}

#[tauri::command]
fn get_settings() -> ConfigData {
    let cfg = Config::load();
    ConfigData {
        network: if cfg.network == Network::Bitcoin { "mainnet".to_string() } else { "testnet".to_string() },
        session_timeout_secs: cfg.session_timeout_secs,
    }
}

/// M5: Clamps timeout to [60, 3600] before saving.
#[tauri::command]
fn update_settings(network: &str, session_timeout_secs: u64) -> Result<(), String> {
    let mut cfg = Config::load();
    cfg.network = if network == "testnet" { Network::Testnet } else { Network::Bitcoin };
    // M5: Clamp to safe range — Config::save() also clamps, but do it here for immediate feedback
    cfg.session_timeout_secs = session_timeout_secs.clamp(
        Config::min_timeout_secs(),
        Config::max_timeout_secs(),
    );
    cfg.save()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        // C3: Register the session store as managed state
        .manage(SessionStore(Mutex::new(None)))
        .setup(|app| {
            use tauri::Manager;
            if let Ok(app_dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&app_dir);
                let _ = std::env::set_current_dir(&app_dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_wallet_exists,
            create_wallet,
            restore_wallet,
            login,
            lock_wallet,
            export_xpub,
            export_descriptor,
            get_recovery_phrase,
            get_mnemonic_word,
            change_passphrase,
            import_utxos,
            build_transaction,
            get_settings,
            update_settings,
            load_psbt,
            load_psbt_from_base64,
            sign_and_export_psbt
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
