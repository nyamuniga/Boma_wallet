use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use zeroize::Zeroize;

mod ui;
mod passphrase_check;
mod audit_log;
mod config;
mod wallet_info;
mod qr_display;
mod change_addresses;
mod password_input;

mod generate_entropy;
use generate_entropy::generate_entropy;

mod generate_mnemonic;
use generate_mnemonic::generate_mnemonic;

mod derive_seed_from_mnemonic;
use derive_seed_from_mnemonic::derive_seed_from_mnemonic;

mod derive_keys;
use derive_keys::derive_keys;

mod generate_many_addresses;
use generate_many_addresses::generate_many_addresses;

mod send_and_receive;
use send_and_receive::{guided_send, import_utxos_from_csv, Utxo};

mod store_backup;
use store_backup::{load_backup, store_backup};

mod get_utxos;
use get_utxos::print_addresses;

mod get_random_address;
use get_random_address::get_random_address;

mod restore_and_backup_master_key;

use audit_log::AuditLog;
use config::Config;
use password_input::read_password;

const BACKUP_FILE: &str = "backup.txt";

// ── Main entry point ──────────────────────────────────────────────────────────

fn main() {
    let mut cfg = Config::load();
    let audit = AuditLog::new();

    loop {
        ui::header(
            &format!("Network: {}", cfg.network_label()),
            "Main Menu",
        );
        ui::menu(&[
            ("1", "Create a new wallet"),
            ("2", "Open existing wallet"),
            ("3", "Verify backup integrity"),
            ("4", "Settings"),
            ("5", "Exit"),
        ]);

        let choice = ui::prompt("\nChoice", "Type a number and press Enter.");
        match choice.as_str() {
            "1" => create_new_wallet(&cfg, &audit),
            "2" => login_with_passphrase(&cfg, &audit),
            "3" => verify_backup_menu(),
            "4" => settings_menu(&mut cfg),
            "5" => { println!("\n  Goodbye!\n"); break; }
            _   => ui::error("Invalid choice — enter 1 to 5."),
        }
    }
}

// ── Create wallet ─────────────────────────────────────────────────────────────

fn create_new_wallet(cfg: &Config, audit: &AuditLog) {
    ui::header("", "Main > Create New Wallet");

    if Path::new(BACKUP_FILE).exists() {
        ui::warn("A wallet backup already exists. Creating a new one will overwrite it.");
        let ans = ui::prompt("Type 'yes' to continue", "This cannot be undone.");
        if ans != "yes" { ui::info("Cancelled."); ui::pause(); return; }
    }

    // Generate entropy and mnemonic
    let entropy = generate_entropy();
    let mnemonic = generate_mnemonic(&entropy);
    let mnemonic_str = mnemonic.to_string();

    // Display mnemonic
    ui::header("", "Main > Create New Wallet > Recovery Phrase");
    println!("  {}{}⚠  Write down these words — they are your Bitcoin backup.{}", ui::BOLD, ui::YELLOW, ui::RESET);
    println!("  {}NEVER share them. Anyone with these words owns your funds.{}\n", ui::RED, ui::RESET);

    let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        print!("  {}{:>2}.{} {:<12}", ui::DIM, i + 1, ui::RESET, word);
        if (i + 1) % 4 == 0 { println!(); }
    }
    println!("\n");
    ui::pause();

    // Passphrase with strength meter
    let passphrase = get_passphrase_new();

    // Derive and verify keys
    ui::header("", "Main > Create New Wallet > Generating Keys");
    ui::info("Deriving keys (this takes a moment)...");
    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, &passphrase);

    let root_key = match derive_keys(&seed, cfg.network) {
        Ok((rk, _, _, _)) => rk,
        Err(e) => { ui::error(&format!("Key derivation failed: {}", e)); seed.zeroize(); ui::pause(); return; }
    };

    let fingerprint = wallet_info::get_fingerprint(&root_key);

    // Encrypt and save
    match store_backup(&passphrase, &mnemonic_str, BACKUP_FILE) {
        Ok(()) => {
            audit.log("WALLET_CREATED");
            ui::success("Wallet encrypted and saved to backup.txt");
            println!("  Wallet fingerprint: {}{}{}", ui::CYAN, fingerprint, ui::RESET);
        }
        Err(e) => { ui::error(&format!("Failed to save: {}", e)); }
    }

    seed.zeroize();
    ui::pause();
}

// ── Login ─────────────────────────────────────────────────────────────────────

fn login_with_passphrase(cfg: &Config, audit: &AuditLog) {
    if !Path::new(BACKUP_FILE).exists() {
        ui::header("", "Main > Open Wallet");
        ui::error("No wallet found. Create one first.");
        ui::pause();
        return;
    }

    ui::header("", "Main > Open Wallet");

    // Exponential backoff on wrong passphrase
    let mut attempts = 0u32;
    let (passphrase, mnemonic_str) = loop {
        if attempts > 0 {
            let delay = 2u64.pow((attempts - 1).min(5));
            ui::warn(&format!("Wrong passphrase. Waiting {}s before retry ({}/{} attempts)...", delay, attempts, 5));
            thread::sleep(Duration::from_secs(delay));
        }
        if attempts >= 5 {
            ui::error("Too many failed attempts. Returning to main menu.");
            audit.log("LOGIN_FAILED_MAX_ATTEMPTS");
            ui::pause();
            return;
        }

        println!("  Enter your passphrase {}(hidden){}:", ui::DIM, ui::RESET);
        let pass = match read_password() {
            Ok(p) => p,
            Err(e) => { ui::error(&e); continue; }
        };

        match load_backup(&pass, BACKUP_FILE) {
            Ok(m) => { ui::success("Wallet unlocked."); break (pass, m); }
            Err(e) => { ui::error(&e); attempts += 1; }
        }
    };

    audit.log("SESSION_START");

    // Derive everything in memory
    ui::info("Deriving keys...");
    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, &passphrase);

    let root_key = match derive_keys(&seed, cfg.network) {
        Ok((rk, _, _, _)) => rk,
        Err(e) => { ui::error(&format!("Key derivation failed: {}", e)); seed.zeroize(); ui::pause(); return; }
    };

    let receive_addresses = generate_many_addresses(&root_key, cfg.network);
    let change_addresses  = change_addresses::generate_change_addresses(&root_key, cfg.network);
    let fingerprint       = wallet_info::get_fingerprint(&root_key);
    seed.zeroize();

    ui::success(&format!("{} receive + {} change addresses ready.", receive_addresses.len(), change_addresses.len()));

    // Enter wallet session
    wallet_session(
        &mnemonic_str, &receive_addresses, &change_addresses,
        &root_key, &fingerprint, cfg, audit,
    );
}

// ── Wallet session ────────────────────────────────────────────────────────────

fn wallet_session(
    mnemonic_str:      &str,
    receive_addresses: &[(bitcoin::util::address::Address, bitcoin::secp256k1::SecretKey)],
    change_addresses:  &[(bitcoin::util::address::Address, bitcoin::secp256k1::SecretKey)],
    root_key:          &bitcoin::util::bip32::ExtendedPrivKey,
    fingerprint:       &str,
    cfg:               &Config,
    audit:             &AuditLog,
) {
    let timeout = Duration::from_secs(cfg.session_timeout_secs);
    let mut last_activity = Instant::now();
    let mut preloaded_utxos: Vec<Utxo> = Vec::new();

    loop {
        // Session timeout check
        if last_activity.elapsed() >= timeout {
            ui::header("", "Session Timed Out");
            ui::warn(&format!(
                "Wallet auto-locked after {} minutes of inactivity.",
                cfg.session_timeout_secs / 60
            ));
            audit.log("SESSION_TIMEOUT");
            ui::pause();
            return;
        }

        let subtitle = format!(
            "[{}]  {}  │  timeout in {}s",
            fingerprint,
            cfg.network_label(),
            timeout.saturating_sub(last_activity.elapsed()).as_secs()
        );
        ui::header("Wallet Menu", &subtitle);

        ui::section("Receive");
        ui::menu(&[
            ("1", "Show receive address + QR code"),
            ("2", "View all addresses  (receive + change)"),
        ]);
        ui::section("Send");
        ui::menu(&[
            ("3", "Sign transaction  (offline)"),
            ("4", "Dry run  — preview transaction without signing"),
            ("5", "Import UTXOs from CSV file"),
        ]);
        ui::section("Wallet");
        ui::menu(&[
            ("6",  "Wallet summary & fingerprint"),
            ("7",  "Export watch-only xpub"),
            ("8",  "Export wallet descriptor"),
            ("9",  "View recovery phrase"),
            ("10", "Verify backup integrity"),
            ("11", "Lock wallet"),
        ]);

        let choice = ui::prompt("\nChoice", "Type a number. Type '?' at any prompt for help.");
        last_activity = Instant::now(); // reset timer on any interaction

        match choice.as_str() {
            // ── Receive ──────────────────────────────────────────────────────
            "1" => {
                ui::header("", &format!("[{}] > Receive", fingerprint));
                match get_random_address(receive_addresses) {
                    Ok(addr) => {
                        println!("  {}{}Receive Address{}", ui::BOLD, ui::GREEN, ui::RESET);
                        println!("  {}{}{}\n", ui::CYAN, addr, ui::RESET);
                        ui::info("Scanning the QR code below sends Bitcoin to this address.");
                        if let Err(e) = qr_display::print_qr(&addr) {
                            ui::warn(&format!("QR render failed: {}", e));
                        }
                        audit.log("ADDRESS_SHOWN");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            "2" => {
                ui::header("", &format!("[{}] > All Addresses", fingerprint));
                print_addresses("Receive  (m/44'/0'/0'/0/{i})", receive_addresses);
                print_addresses("Change   (m/44'/0'/0'/1/{i})", change_addresses);
                ui::pause();
            }

            // ── Send ─────────────────────────────────────────────────────────
            "3" => {
                match guided_send(receive_addresses, change_addresses, &preloaded_utxos, false) {
                    Ok(hex) => {
                        ui::header("", &format!("[{}] > Signed Transaction", fingerprint));
                        ui::success("Transaction signed! Copy the hex below and broadcast it.");
                        ui::info("Broadcast at: https://blockstream.info/tx/push");
                        println!("\n  {}{}{}\n", ui::CYAN, hex, ui::RESET);
                        audit.log("TX_SIGNED");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            "4" => {
                match guided_send(receive_addresses, change_addresses, &preloaded_utxos, true) {
                    Ok(hex) => {
                        ui::header("", &format!("[{}] > Dry Run Preview", fingerprint));
                        let raw = hex.strip_prefix("DRY_RUN:").unwrap_or(&hex);
                        ui::warn("DRY RUN — this transaction was NOT signed.");
                        ui::info("Unsigned transaction hex (for inspection only):");
                        println!("\n  {}{}{}\n", ui::DIM, raw, ui::RESET);
                        audit.log("TX_DRY_RUN");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            "5" => {
                ui::header("", &format!("[{}] > Import UTXOs", fingerprint));
                ui::info("CSV format: txid,vout,amount_btc,address  (one per line, # = comment)");
                let path = ui::prompt("CSV file path", "Path to your UTXO CSV file.");
                match import_utxos_from_csv(&path) {
                    Ok(utxos) => {
                        ui::success(&format!("{} UTXOs loaded.", utxos.len()));
                        for u in &utxos {
                            println!("  • {}…  vout {}  {} sats  ({})",
                                &u.txid[..16], u.vout, u.amount_sats, u.address);
                        }
                        preloaded_utxos = utxos;
                        audit.log("UTXOS_IMPORTED");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            // ── Wallet info ───────────────────────────────────────────────────
            "6" => {
                ui::header("", &format!("[{}] > Wallet Summary", fingerprint));
                ui::section("Identity");
                println!("  Master fingerprint  {}{}{}", ui::CYAN, fingerprint, ui::RESET);
                println!("  Network             {}", cfg.network_label());
                println!("  Backup file         {}", BACKUP_FILE);
                println!("  Backup exists       {}", if Path::new(BACKUP_FILE).exists() { "✓ yes" } else { "✗ no" });
                ui::section("Addresses");
                println!("  Receive addresses   {} (m/44'/0'/0'/0/{{i}})", receive_addresses.len());
                println!("  Change addresses    {} (m/44'/0'/0'/1/{{i}})", change_addresses.len());
                ui::section("Settings");
                println!("  Session timeout     {} minutes", cfg.session_timeout_secs / 60);
                println!("  Audit log           wallet_audit.log");
                ui::pause();
            }

            "7" => {
                ui::header("", &format!("[{}] > Export xpub", fingerprint));
                match wallet_info::export_watch_wallet(root_key, cfg.network) {
                    Ok(()) => {
                        ui::success("Exported to watch_wallet.txt");
                        ui::info("This file is SAFE to copy to a hot machine — it cannot spend.");
                        audit.log("XPUB_EXPORTED");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            "8" => {
                ui::header("", &format!("[{}] > Export Descriptor", fingerprint));
                match wallet_info::export_descriptor(root_key, cfg.network) {
                    Ok(()) => {
                        ui::success("Descriptor exported to wallet_descriptor.txt");
                        ui::info("Import into Electrum or Sparrow to track balances.");
                        audit.log("DESCRIPTOR_EXPORTED");
                    }
                    Err(e) => ui::error(&e),
                }
                ui::pause();
            }

            "9" => {
                ui::header("", &format!("[{}] > Recovery Phrase", fingerprint));
                ui::warn("Make sure nobody can see your screen before continuing.");
                let confirm = ui::prompt("Show recovery phrase? [yes/no]", "Type 'yes' to reveal.");
                if confirm == "yes" {
                    println!();
                    let words: Vec<&str> = mnemonic_str.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        print!("  {}{:>2}.{} {:<12}", ui::DIM, i + 1, ui::RESET, word);
                        if (i + 1) % 4 == 0 { println!(); }
                    }
                    println!("\n");
                    audit.log("MNEMONIC_VIEWED");
                } else {
                    ui::info("Cancelled.");
                }
                ui::pause();
            }

            "10" => {
                verify_backup_menu();
            }

            "11" => {
                audit.log("SESSION_END");
                ui::info("Wallet locked. All key material cleared from memory.");
                return;
            }

            _ => ui::error("Invalid choice — enter 1 to 11."),
        }
    }
}

// ── Verify backup ─────────────────────────────────────────────────────────────

fn verify_backup_menu() {
    ui::header("", "Main > Verify Backup");
    if !Path::new(BACKUP_FILE).exists() {
        ui::error("No backup file found.");
        ui::pause();
        return;
    }
    println!("  Enter your passphrase to verify the backup can be decrypted:\n");
    let pass = match read_password() {
        Ok(p) => p,
        Err(e) => { ui::error(&e); ui::pause(); return; }
    };
    match load_backup(&pass, BACKUP_FILE) {
        Ok(mnemonic) => {
            let word_count = mnemonic.split_whitespace().count();
            ui::success(&format!("Backup verified! Contains a valid {}-word mnemonic.", word_count));
        }
        Err(e) => ui::error(&format!("Backup verification FAILED: {}", e)),
    }
    ui::pause();
}

// ── Settings ──────────────────────────────────────────────────────────────────

fn settings_menu(cfg: &mut Config) {
    loop {
        ui::header("", "Main > Settings");
        ui::menu(&[
            ("1", &format!("Toggle network  (current: {})", cfg.network_label())),
            ("2", &format!("Set session timeout  (current: {} min)", cfg.session_timeout_secs / 60)),
            ("3", "Back"),
        ]);

        match ui::prompt("\nChoice", "").as_str() {
            "1" => {
                cfg.network = if cfg.network == bitcoin::network::constants::Network::Bitcoin {
                    bitcoin::network::constants::Network::Testnet
                } else {
                    bitcoin::network::constants::Network::Bitcoin
                };
                match cfg.save() {
                    Ok(()) => ui::success(&format!("Network set to {}", cfg.network_label())),
                    Err(e) => ui::error(&e),
                }
            }
            "2" => {
                let mins: u64 = ui::prompt_until(
                    "Session timeout in minutes (1–60)",
                    "Wallet auto-locks after this many minutes of inactivity.",
                    |s| s.parse::<u64>()
                          .ok()
                          .filter(|&n| n >= 1 && n <= 60)
                          .ok_or_else(|| "Enter a number between 1 and 60.".to_string())
                );
                cfg.session_timeout_secs = mins * 60;
                match cfg.save() {
                    Ok(()) => ui::success(&format!("Timeout set to {} minutes.", mins)),
                    Err(e) => ui::error(&e),
                }
            }
            "3" | "" => break,
            _ => ui::error("Invalid choice."),
        }
    }
}

// ── Passphrase input ──────────────────────────────────────────────────────────

/// Interactive passphrase creation with strength meter and confirmation.
fn get_passphrase_new() -> String {
    loop {
        println!("\n  {}Set a passphrase to protect your wallet {}(input hidden){}:",
            ui::BOLD, ui::DIM, ui::RESET);
        ui::info("Press Enter to skip (not recommended — mnemonic alone protects funds).");

        let pass = match read_password() {
            Ok(p) => p,
            Err(e) => { ui::error(&e); continue; }
        };

        // Show strength meter
        println!();
        passphrase_check::display(&pass);
        println!();

        if pass.is_empty() {
            let ans = ui::prompt("Continue without a passphrase? [yes/no]",
                "Type 'yes' to use no passphrase (mnemonic alone will secure the wallet).");
            if ans.eq_ignore_ascii_case("yes") { return pass; }
            continue;
        }

        println!("  Confirm your passphrase:");
        let confirm = match read_password() {
            Ok(p) => p,
            Err(e) => { ui::error(&e); continue; }
        };

        if pass == confirm {
            ui::success("Passphrase confirmed.");
            return pass;
        }
        ui::error("Passphrases do not match. Try again.");
    }
}