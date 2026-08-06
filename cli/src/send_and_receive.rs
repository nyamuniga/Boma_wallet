use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use std::str::FromStr;

use boma_core::transaction::{
    btc_to_sats, estimate_vbytes, fee_tiers, is_own_address, Utxo, TxParams, DUST_SATS
};
use crate::ui;

// ── Interactive send flow ─────────────────────────────────────────────────────

/// Full interactive guided send flow with all safety checks.
///
/// Collects the parameters and returns a populated TxParams struct.
/// `change_index` selects which change address to use (rotated after each tx).
pub fn collect_send_params<'a>(
    receive_addresses: &'a [(Address, SecretKey)],
    change_addresses: &'a [(Address, SecretKey)],
    preloaded_utxos: &[Utxo],
    dry_run: bool,
    change_index: usize,
) -> Result<TxParams<'a>, String> {
    let crumb = if dry_run { "Wallet > Dry Run" } else { "Wallet > Sign Transaction" };
    ui::header("", crumb);

    if dry_run {
        ui::warn("DRY RUN — This transaction will NOT be signed.");
        println!();
    } else {
        println!("  Sign a transaction offline. Broadcast the resulting hex at:");
        ui::info("https://blockstream.info/tx/push");
        println!();
    }

    // ── Step 1: Spending address ──────────────────────────────────────────────
    ui::section("Step 1/6 — Spending address");
    println!("  Which of your addresses holds the Bitcoin to send?\n");
    for (i, (addr, _)) in receive_addresses.iter().enumerate() {
        println!("  {}[{:>2}]{}  {}", ui::ORANGE, i, ui::RESET, addr);
    }
    let from_idx: usize = ui::prompt_until(
        "Address number (? for help)",
        "Enter the index shown in [brackets] next to the address holding your Bitcoin.",
        |s| s.parse::<usize>()
              .ok()
              .filter(|&n| n < receive_addresses.len())
              .ok_or_else(|| format!("Enter a number between 0 and {}.", receive_addresses.len() - 1))
    );
    let (from_address, from_key) = &receive_addresses[from_idx];

    // ── Step 2: UTXO source ───────────────────────────────────────────────────
    ui::section("Step 2/6 — UTXO to spend");
    let (txid_str, vout, input_sats) = if !preloaded_utxos.is_empty() {
        println!("  Preloaded UTXOs:\n");
        for (i, u) in preloaded_utxos.iter().enumerate() {
            println!("  {}[{:>2}]{}  {} sats  ({})  {}…",
                ui::ORANGE, i, ui::RESET,
                u.amount_sats, u.address, &u.txid[..16]);
        }
        println!("  {}[m ]{}  Enter manually", ui::ORANGE, ui::RESET);
        let sel = ui::prompt("Choice (number or 'm')", "Select a preloaded UTXO or type 'm' to enter manually.");

        if sel.to_lowercase() != "m" {
            if let Ok(idx) = sel.parse::<usize>() {
                if idx < preloaded_utxos.len() {
                    let u = &preloaded_utxos[idx];
                    (u.txid.clone(), u.vout, u.amount_sats)
                } else {
                    ui::warn("Invalid selection — switching to manual entry.");
                    manual_utxo_entry()?
                }
            } else {
                manual_utxo_entry()?
            }
        } else {
            manual_utxo_entry()?
        }
    } else {
        manual_utxo_entry()?
    };

    // ── Step 3: RBF ───────────────────────────────────────────────────────────
    ui::section("Step 3/6 — Replace-By-Fee (RBF)");
    let use_rbf = {
        let ans = ui::prompt(
            "Enable RBF? [y/N]",
            "RBF allows you to increase the fee later if the transaction is stuck. Recommended: yes."
        );
        ans.to_lowercase() == "y" || ans.to_lowercase() == "yes"
    };

    // ── Step 4: Recipient ─────────────────────────────────────────────────────
    ui::section("Step 4/6 — Recipient");
    let to_address: Address = ui::prompt_until(
        "Recipient Bitcoin address (? for help)",
        "The Bitcoin address you are sending to. Double-check every character.",
        |s| Address::from_str(s).map_err(|_| "Invalid Bitcoin address — double-check it.".to_string())
    );

    // Address reuse warning
    if is_own_address(&to_address, receive_addresses, change_addresses) {
        ui::warn("This is one of your own addresses. Are you sure you want to send to yourself?");
        let confirm = ui::prompt("Continue? [y/N]", "Type 'y' to proceed anyway.");
        if confirm.to_lowercase() != "y" {
            return Err("Transaction cancelled — recipient was your own address.".to_string());
        }
    }

    // ── Step 5: Amounts ───────────────────────────────────────────────────────
    ui::section("Step 5/6 — Amounts");
    let send_sats: u64 = ui::prompt_until(
        "Amount to send (BTC, e.g. 0.005)",
        "How much Bitcoin to send to the recipient. Must be less than your UTXO minus fee.",
        |s| btc_to_sats(s).and_then(|v| if v > 0 { Ok(v) } else { Err("Amount must be > 0.".to_string()) })
    );

    // Fee estimation
    let n_outputs = if input_sats > send_sats + DUST_SATS { 2 } else { 1 };
    let vbytes = estimate_vbytes(1, n_outputs);
    let (slow, standard, fast) = fee_tiers(vbytes);
    ui::print_fee_tiers(vbytes, slow, standard, fast);

    let fee_sats: u64 = {
        let sel = ui::prompt("Fee choice [s/n/f/m]", "Pick a fee tier or type 'm' to enter a custom amount.");
        match sel.to_lowercase().as_str() {
            "s" => slow,
            "n" => standard,
            "f" => fast,
            _ => ui::prompt_until(
                "Custom fee (BTC)",
                "Enter your fee in BTC, e.g. 0.0002",
                |s| btc_to_sats(s).and_then(|v| if v > 0 { Ok(v) } else { Err("Fee must be > 0.".to_string()) })
            ),
        }
    };

    // ── Step 6: Change address ────────────────────────────────────────────────
    ui::section("Step 6/6 — Change address");
    // H2: Use rotating change address instead of always index 0
    let change_address = if change_addresses.is_empty() {
        ui::warn("No change addresses available — change will go back to your sending address.");
        from_address
    } else {
        let idx = change_index % change_addresses.len();
        ui::info(&format!("Change will go to internal address index {} (auto-rotated).", idx));
        &change_addresses[idx].0
    };

    let change_sats = input_sats.saturating_sub(send_sats + fee_sats);


    // ── Summary ───────────────────────────────────────────────────────────────
    ui::print_transaction_summary(
        from_address,
        &to_address,
        send_sats,
        fee_sats,
        change_sats,
        change_address,
        use_rbf,
        dry_run,
    );


    let action = if dry_run { "preview" } else { "sign" };
    let confirm = ui::prompt(
        &format!("{}? [yes/no]", if dry_run { "Preview" } else { "Sign and export" }),
        &format!("Type 'yes' to {} this transaction.", action)
    );
    if confirm.to_lowercase() != "yes" {
        return Err("Transaction cancelled.".to_string());
    }

    Ok(TxParams {
        from_address,
        from_key,
        txid_str,
        vout,
        input_sats,
        to_address,
        send_sats,
        fee_sats,
        change_address,
        use_rbf,
        dry_run,
    })
}

fn manual_utxo_entry() -> Result<(String, u32, u64), String> {
    println!();
    ui::info("Find these details on a block explorer (e.g. blockstream.info).");

    let txid_str: String = ui::prompt_until(
        "Transaction ID (64 hex chars)",
        "The full 64-character transaction hash of the Bitcoin you received.",
        |s| {
            if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(s.to_string())
            } else {
                Err("Transaction ID must be exactly 64 hex characters.".to_string())
            }
        }
    );

    let vout: u32 = ui::prompt_until(
        "Output index / vout (usually 0)",
        "Which output in that transaction sent funds to you? Usually 0.",
        |s| s.parse::<u32>().map_err(|_| "Enter a whole number like 0 or 1.".to_string())
    );

    let amount_sats: u64 = ui::prompt_until(
        "Amount you received in that output (BTC)",
        "How much Bitcoin was in that specific output? E.g. 0.005",
        |s| btc_to_sats(s).and_then(|v| if v > 0 { Ok(v) } else { Err("Must be > 0.".to_string()) })
    );

    Ok((txid_str, vout, amount_sats))
}