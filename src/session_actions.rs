use std::path::Path;
use crate::get_random_address::get_random_address;
use crate::get_utxos::print_addresses;
use crate::password_input::read_password;
use crate::qr_display;
use crate::send_and_receive::{build_transaction, collect_send_params, import_utxos_from_csv};
use crate::session_state::SessionState;
use crate::store_backup::load_backup;
use crate::ui;
use crate::wallet_info;

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
    print_addresses("Receive  (m/44'/0'/0'/0/{i})", &state.receive_addresses);
    print_addresses("Change   (m/44'/0'/0'/1/{i})", &state.change_addresses);
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
    println!("  Receive addresses   {} (m/44'/0'/0'/0/{{i}})", state.receive_addresses.len());
    println!("  Change addresses    {} (m/44'/0'/0'/1/{{i}})", state.change_addresses.len());
    ui::section("Settings");
    println!("  Session timeout     {} minutes", state.cfg.session_timeout_secs / 60);
    println!("  Audit log           wallet_audit.log");
    ui::pause();
}

pub fn handle_export_xpub(state: &SessionState) {
    ui::header("", &format!("[{}] > Export xpub", state.fingerprint));
    match wallet_info::export_watch_wallet(&state.root_key, state.cfg.network) {
        Ok(()) => {
            ui::success("Exported to watch_wallet.txt");
            ui::info("This file is SAFE to copy to a hot machine — it cannot spend.");
            state.audit.log("XPUB_EXPORTED");
        }
        Err(e) => ui::error(&e),
    }
    ui::pause();
}

pub fn handle_export_descriptor(state: &SessionState) {
    ui::header("", &format!("[{}] > Export Descriptor", state.fingerprint));
    match wallet_info::export_descriptor(&state.root_key, state.cfg.network) {
        Ok(()) => {
            ui::success("Descriptor exported to wallet_descriptor.txt");
            ui::info("Import into Electrum or Sparrow to track balances.");
            state.audit.log("DESCRIPTOR_EXPORTED");
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
    match crate::store_backup::store_backup(&new_passphrase, &state.mnemonic_str, "backup.txt") {
        Ok(()) => {
            state.audit.log("PASSPHRASE_CHANGED");
            ui::success("Passphrase changed successfully. Backup file updated.");
        }
        Err(e) => ui::error(&format!("Failed to update passphrase: {}", e)),
    }
    ui::pause();
}
