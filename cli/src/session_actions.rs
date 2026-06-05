use std::path::Path;
use boma_core::get_random_address::get_random_address;
use crate::get_utxos::print_addresses;
use crate::password_input::read_password;
use crate::qr_display;
use boma_core::transaction::{build_transaction, import_utxos_from_csv};
use crate::send_and_receive::collect_send_params;
use crate::session_state::SessionState;
use boma_core::store_backup::load_backup;
use crate::ui;


pub fn handle_receive_address(state: &SessionState) {
    ui::header("", &format!("[{}] > Receive", state.fingerprint));
    match get_random_address(&state.receive_addresses) {
        Ok(addr) => {
            println!("  {}{}Receive Address{}", ui::BOLD, ui::GREEN, ui::RESET);
            println!("  {}{}{}\n", ui::CYAN, addr, ui::RESET);
            ui::info("Scanning the QR code below sends Bitcoin to this address.");
            if let Err(e) = qr_display::print_qr(&addr) {
                ui::warn(&format!("QR render failed: {}", e));
            }
            state.audit.log("ADDRESS_SHOWN");
        }
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_view_all_addresses(state: &SessionState) {
    ui::header("", &format!("[{}] > All Addresses", state.fingerprint));
    print_addresses("Receive  (m/84'/0'/0'/0/{i})", &state.receive_addresses);
    print_addresses("Change   (m/84'/0'/0'/1/{i})", &state.change_addresses);
    ui::pause();
}

pub fn handle_sign_transaction(state: &SessionState) {
    let params_res = collect_send_params(
        &state.receive_addresses,
        &state.change_addresses,
        &state.preloaded_utxos,
        false,
    );

    match params_res {
        Ok(params) => match build_transaction(&params) {
            Ok(hex) => {
                ui::header("", &format!("[{}] > Signed Transaction", state.fingerprint));
                ui::success("Transaction signed! Copy the hex below and broadcast it.");
                ui::info("Broadcast at: https://blockstream.info/tx/push");
                println!("\n  {}{}{}\n", ui::CYAN, hex, ui::RESET);
                state.audit.log("TX_SIGNED");
            }
            Err(e) => ui::error(&e),
        },
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_dry_run(state: &SessionState) {
    let params_res = collect_send_params(
        &state.receive_addresses,
        &state.change_addresses,
        &state.preloaded_utxos,
        true,
    );

    match params_res {
        Ok(params) => match build_transaction(&params) {
            Ok(hex) => {
                ui::header("", &format!("[{}] > Dry Run Preview", state.fingerprint));
                let raw = hex.strip_prefix("DRY_RUN:").unwrap_or(&hex);
                ui::warn("DRY RUN — this transaction was NOT signed.");
                ui::info("Unsigned transaction hex (for inspection only):");
                println!("\n  {}{}{}\n", ui::DIM, raw, ui::RESET);
                state.audit.log("TX_DRY_RUN");
            }
            Err(e) => ui::error(&e),
        },
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_import_utxos(state: &mut SessionState) {
    ui::header("", &format!("[{}] > Import UTXOs", state.fingerprint));
    ui::info("CSV format: txid,vout,amount_btc,address  (one per line, # = comment)");
    let path = ui::prompt("CSV file path", "Path to your UTXO CSV file.");
    match import_utxos_from_csv(&path) {
        Ok(utxos) => {
            ui::success(&format!("{} UTXOs loaded.", utxos.len()));
            for u in &utxos {
                println!("  • {}…  vout {}  {} sats  ({})",
                    &u.txid[..16], u.vout, u.amount_sats, u.address);
            }
            state.preloaded_utxos = utxos;
            state.audit.log("UTXOS_IMPORTED");
        }
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_wallet_summary(state: &SessionState) {
    ui::header("", &format!("[{}] > Wallet Summary", state.fingerprint));
    ui::section("Identity");
    println!("  Master fingerprint  {}{}{}", ui::CYAN, state.fingerprint, ui::RESET);
    println!("  Network             {}", state.cfg.network_label());
    println!("  Backup file         backup.txt");
    println!("  Backup exists       {}", if Path::new("backup.txt").exists() { "✓ yes" } else { "✗ no" });
    ui::section("Addresses");
    println!("  Receive addresses   {} (m/84'/0'/0'/0/{{i}})", state.receive_addresses.len());
    println!("  Change addresses    {} (m/84'/0'/0'/1/{{i}})", state.change_addresses.len());

    ui::section("Settings");
    println!("  Session timeout     {} minutes", state.cfg.session_timeout_secs / 60);
    println!("  Audit log           wallet_audit.log");
    ui::pause();
}

pub fn handle_export_xpub(state: &SessionState) {
    ui::header("", &format!("[{}] > Export xpub", state.fingerprint));
    match boma_core::wallet_info::export_watch_wallet(&state.root_key, state.cfg.network, "watch_wallet.txt") {
        Ok(()) => {
            state.audit.log("XPUB_EXPORTED");
            ui::success("Watch-only wallet exported to watch_wallet.txt.");
            ui::info("This file is SAFE to copy to a hot machine — it cannot spend.");
        }
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_export_descriptor(state: &SessionState) {
    ui::header("", &format!("[{}] > Export Descriptor", state.fingerprint));
    match boma_core::wallet_info::export_descriptor(&state.root_key, state.cfg.network, "wallet_descriptor.txt") {
        Ok(()) => {
            state.audit.log("DESCRIPTOR_EXPORTED");
            ui::success("Descriptor exported to wallet_descriptor.txt.");
            ui::info("Import into Electrum or Sparrow to track balances.");
        }
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_view_phrase(state: &SessionState) {
    ui::header("", &format!("[{}] > Recovery Phrase", state.fingerprint));
    ui::warn("Make sure nobody can see your screen before continuing.");
    let confirm = ui::prompt("Show recovery phrase? [yes/no]", "Type 'yes' to reveal.");
    if confirm == "yes" {
        println!();
        let words: Vec<&str> = state.mnemonic_str.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            print!("  {}{:>2}.{} {:<12}", ui::DIM, i + 1, ui::RESET, word);
            if (i + 1) % 4 == 0 { println!(); }
        }
        println!("\n");
        state.audit.log("MNEMONIC_VIEWED");
    } else {
        ui::info("Cancelled.");
    }
    ui::pause();
}

pub fn handle_verify_backup() {
    ui::header("", "Main > Verify Backup");
    if !Path::new("backup.txt").exists() {
        ui::error("No backup file found.");
        ui::pause();
        return;
    }
    println!("  Enter your passphrase to verify the backup can be decrypted:\n");
    let pass = match read_password() {
        Ok(p) => p,
        Err(e) => { ui::error(&e); ui::pause(); return; }
    };
    match load_backup(&pass, "backup.txt") {
        Ok(mnemonic) => {
            let word_count = mnemonic.split_whitespace().count();
            ui::success(&format!("Backup verified! Contains a valid {}-word mnemonic.", word_count));
        }
        Err(e) => ui::error(&format!("Backup verification FAILED: {}", e)),
    }
    ui::pause();
}

pub fn handle_change_passphrase(state: &mut SessionState) {
    ui::header("", &format!("[{}] > Change Passphrase", state.fingerprint));
    ui::warn("This will re-encrypt your backup file with a new passphrase.");
    ui::info("You will need the new passphrase to open the wallet next time.");
    
    let confirm = ui::prompt("Continue? [yes/no]", "Type 'yes' to set a new passphrase.");
    if confirm != "yes" {
        ui::info("Cancelled.");
        ui::pause();
        return;
    }

    let new_passphrase = crate::get_passphrase_new();
    match boma_core::store_backup::store_backup(&new_passphrase, &state.mnemonic_str, "backup.txt") {
        Ok(()) => {
            state.audit.log("PASSPHRASE_CHANGED");
            ui::success("Passphrase changed successfully. Backup file updated.");
        }
        Err(e) => ui::error(&format!("Failed to update passphrase: {}", e)),
    }
    ui::pause();
}

pub fn handle_restore_wallet(cfg: &boma_core::config::Config, audit: &crate::audit_log::AuditLog) {
    use bip39::Mnemonic;
    use std::str::FromStr;
    use boma_core::derive_seed_from_mnemonic::derive_seed_from_mnemonic;
    use boma_core::derive_keys::derive_keys;

    ui::header("", "Main > Restore from Recovery Phrase");
    ui::warn("Only do this on an OFFLINE, air-gapped machine.");
    ui::warn("Never enter your seed phrase on an internet-connected device.");
    println!();
    ui::info("Enter your 24 recovery words separated by spaces, then press Enter.");
    ui::info("Words are case-insensitive. Press '?' for help.");
    println!();

    // Check for existing wallet
    if Path::new("backup.txt").exists() {
        ui::warn("A wallet backup already exists. Restoring will OVERWRITE it permanently.");
        let ans = ui::prompt("Type 'yes' to continue", "This cannot be undone.");
        if ans != "yes" {
            ui::info("Cancelled.");
            ui::pause();
            return;
        }
    }

    // Collect and validate mnemonic
    let mnemonic_str: String = ui::prompt_until(
        "Recovery phrase (12 or 24 words)",
        "Type all words separated by spaces. Each word must be from the BIP-39 wordlist.",
        |input| {
            let normalised = input.trim().to_lowercase();
            Mnemonic::from_str(&normalised)
                .map(|m| m.to_string())
                .map_err(|e| format!("Invalid phrase: {}", e))
        },
    );

    // Set passphrase for the restored wallet
    println!();
    ui::info("Set a passphrase to protect this restored wallet.");
    ui::info("If your original wallet used a BIP-39 passphrase (25th word), enter it here.");
    let passphrase = crate::get_passphrase_new();

    // Verify keys can be derived before saving
    ui::info("Verifying and deriving keys...");
    let seed = derive_seed_from_mnemonic(&mnemonic_str, &passphrase);
    let root_key = match derive_keys(&seed, cfg.network) {
        Ok((rk, _, _, _)) => rk,
        Err(e) => { ui::error(&format!("Key derivation failed: {}", e)); ui::pause(); return; }
    };

    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);

    match boma_core::store_backup::store_backup(&passphrase, &mnemonic_str, "backup.txt") {
        Ok(()) => {
            audit.log("WALLET_RESTORED");
            ui::success("Wallet restored and encrypted successfully!");
            println!("  Wallet fingerprint: {}{}{}", ui::CYAN, fingerprint, ui::RESET);
            ui::info("You can now open your wallet with option 2 from the main menu.");
        }
        Err(e) => ui::error(&format!("Failed to save: {}", e)),
    }
    ui::pause();
}

pub fn handle_sign_psbt(state: &SessionState) {
    use boma_core::psbt::{parse_psbt, parse_psbt_from_base64, sign_psbt, export_psbt, psbt_to_base64};

    ui::header("", &format!("[{}] > Sign PSBT", state.fingerprint));
    ui::info("Input method:");
    ui::menu(&[
        ("1", "Load .psbt file from disk"),
        ("2", "Paste base64 PSBT string (e.g. from QR code)"),
    ]);

    let choice = ui::prompt("Choice", "1 = file, 2 = paste");
    let parse_result = match choice.trim() {
        "1" => {
            let path = ui::prompt("Path to .psbt file", "Full file path, e.g. /media/usb/unsigned.psbt");
            parse_psbt(&path)
        }
        "2" => {
            ui::info("Paste the base64 PSBT string and press Enter:");
            let b64 = ui::prompt("Base64 PSBT", "Paste here.");
            parse_psbt_from_base64(&b64)
        }
        _ => { ui::error("Invalid choice."); ui::pause(); return; }
    };

    let (psbt, summary) = match parse_result {
        Ok(r) => r,
        Err(e) => { ui::error(&format!("Failed to parse PSBT: {}", e)); ui::pause(); return; }
    };

    // Display summary
    println!();
    ui::section("Transaction Summary");
    println!("  Inputs        {}", summary.input_count);
    println!("  Outputs       {}", summary.output_count);
    println!("  Total in      {} sats  ({:.8} BTC)", summary.input_sats, summary.input_sats as f64 / 1e8);
    println!("  Sending       {} sats  ({:.8} BTC)", summary.send_sats, summary.send_sats as f64 / 1e8);
    println!("  Miner fee     {}{} sats  ({:.4}%){}",
        if summary.fee_warning { ui::YELLOW } else { ui::RESET },
        summary.fee_sats, summary.fee_pct,
        ui::RESET
    );
    if summary.fee_warning {
        ui::warn("⚠  Fee is unusually HIGH (>5% of input). Verify before signing!");
    }
    println!();
    ui::section("Destination Addresses");
    for addr in &summary.destinations {
        println!("  {}{}{}", ui::CYAN, addr, ui::RESET);
    }
    println!();

    let confirm = ui::prompt("Sign this transaction? [yes/no]", "Type 'yes' to authorize signing.");
    if confirm != "yes" {
        ui::info("Signing cancelled.");
        ui::pause();
        return;
    }

    let signed = match sign_psbt(psbt, &state.root_key, state.cfg.network) {
        Ok(p) => p,
        Err(e) => { ui::error(&format!("Signing failed: {}", e)); ui::pause(); return; }
    };

    ui::success("PSBT signed successfully!");
    println!();
    ui::info("Export options:");
    ui::menu(&[
        ("1", "Save as .psbt file"),
        ("2", "Display as base64 (copy for QR)"),
        ("3", "Both"),
    ]);

    let export_choice = ui::prompt("Choice", "How to export the signed PSBT.");
    match export_choice.trim() {
        "1" | "3" => {
            let out_path = ui::prompt("Output path", "e.g. /media/usb/signed.psbt");
            match export_psbt(&signed, &out_path) {
                Ok(()) => ui::success(&format!("Signed PSBT saved to '{}'.", out_path)),
                Err(e) => ui::error(&e),
            }
            if export_choice.trim() == "1" { state.audit.log("PSBT_SIGNED"); ui::pause(); return; }
        }
        _ => {}
    }
    if export_choice.trim() == "2" || export_choice.trim() == "3" {
        let b64 = psbt_to_base64(&signed);
        println!();
        ui::info("Base64 signed PSBT (import into Sparrow or scan as QR):");
        println!("\n  {}{}{}\n", ui::CYAN, b64, ui::RESET);
    }

    state.audit.log("PSBT_SIGNED");
    ui::pause();
}
