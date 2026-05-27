use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use zeroize::Zeroize;
use bitcoin::network::constants::Network;

mod ui;
mod passphrase_check;
mod audit_log;
mod qr_display;
mod password_input;
mod session_state;
mod session_actions;

use boma_core::generate_entropy::generate_entropy;
use boma_core::generate_mnemonic::generate_mnemonic;
use boma_core::derive_seed_from_mnemonic::derive_seed_from_mnemonic;
use boma_core::derive_keys::derive_keys;
use boma_core::generate_many_addresses::generate_many_addresses;
use boma_core::change_addresses;
use boma_core::store_backup::{load_backup, store_backup};
use boma_core::wallet_info;
use boma_core::config::Config;

mod send_and_receive;
mod get_utxos;
mod restore_and_backup_master_key;

use audit_log::AuditLog;
use password_input::read_password;
use session_state::SessionState;

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
            ("5", "Restore from recovery phrase"),
            ("6", "Exit"),
        ]);

        let choice = ui::prompt("\nChoice", "Type a number and press Enter.");
        match choice.as_str() {
            "1" => create_new_wallet(&cfg, &audit),
            "2" => login_with_passphrase(&cfg, &audit),
            "3" => session_actions::handle_verify_backup(),
            "4" => settings_menu(&mut cfg),
            "5" => session_actions::handle_restore_wallet(&cfg, &audit),
            "6" => { println!("\n  Goodbye!\n"); break; }
            _   => ui::error("Invalid choice — enter 1 to 6."),
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
    ui::print_mnemonic_warning();

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

    let mut state = SessionState {
        mnemonic_str,
        receive_addresses,
        change_addresses,
        root_key,
        fingerprint,
        cfg,
        audit,
        preloaded_utxos: Vec::new(),
    };

    wallet_session(&mut state);
}

// ── Wallet session ────────────────────────────────────────────────────────────

fn wallet_session(state: &mut SessionState) {
    let timeout = Duration::from_secs(state.cfg.session_timeout_secs);
    let mut last_activity = Instant::now();

    loop {
        if last_activity.elapsed() >= timeout {
            ui::header("", "Session Timed Out");
            ui::warn(&format!(
                "Wallet auto-locked after {} minutes of inactivity.",
                state.cfg.session_timeout_secs / 60
            ));
            state.audit.log("SESSION_TIMEOUT");
            ui::pause();
            return;
        }

        let subtitle = format!(
            "[{}]  {}  │  timeout in {}s",
            state.fingerprint,
            state.cfg.network_label(),
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
            ("3", "Sign PSBT File (Recommended)"),
            ("4", "[Advanced] Sign raw transaction manually"),
            ("5", "[Advanced] Dry run preview"),
            ("6", "[Advanced] Import UTXOs from CSV"),
        ]);
        ui::section("Wallet");
        ui::menu(&[
            ("7",  "Wallet summary & fingerprint"),
            ("8",  "Export watch-only xpub"),
            ("9",  "Export wallet descriptor"),
            ("10", "View recovery phrase"),
            ("11", "Verify backup integrity"),
            ("12", "Change passphrase"),
            ("13", "Lock wallet"),
        ]);

        let choice = ui::prompt("\nChoice", "Type a number. Type '?' at any prompt for help.");
        last_activity = Instant::now();

        match choice.as_str() {
            "1"  => session_actions::handle_receive_address(state),
            "2"  => session_actions::handle_view_all_addresses(state),
            "3"  => session_actions::handle_sign_psbt(state),
            "4"  => session_actions::handle_sign_transaction(state),
            "5"  => session_actions::handle_dry_run(state),
            "6"  => session_actions::handle_import_utxos(state),
            "7"  => session_actions::handle_wallet_summary(state),
            "8"  => session_actions::handle_export_xpub(state),
            "9"  => session_actions::handle_export_descriptor(state),
            "10" => session_actions::handle_view_phrase(state),
            "11" => session_actions::handle_verify_backup(),
            "12" => session_actions::handle_change_passphrase(state),
            "13" => {
                state.audit.log("SESSION_END");
                ui::info("Wallet locked. All key material cleared from memory.");
                return;
            }

            _ => ui::error("Invalid choice — enter 1 to 13."),
        }
    }
}



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
                cfg.network = if cfg.network == Network::Bitcoin {
                    Network::Testnet
                } else {
                    Network::Bitcoin
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
                          .filter(|&n| (1..=60).contains(&n))
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
pub fn get_passphrase_new() -> String {
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