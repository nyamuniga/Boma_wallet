use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::witness::Witness;
use bitcoin::consensus::encode::serialize;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::sighash::SighashCache;
use bitcoin::{EcdsaSighashType, OutPoint, PackedLockTime, Sequence, Transaction, TxIn, TxOut, Txid};
use std::str::FromStr;

use crate::ui;

const DUST_SATS: u64 = 546;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse a BTC-denominated string (e.g. "0.005") to satoshis.
pub fn btc_to_sats(s: &str) -> Result<u64, String> {
    let v: f64 = s.trim().parse()
        .map_err(|_| format!("'{}' is not a valid number — use format like 0.005", s.trim()))?;
    if v < 0.0 { return Err("Amount cannot be negative.".to_string()); }
    Ok((v * 100_000_000.0).round() as u64)
}

/// Estimate P2PKH transaction size in virtual bytes.
/// Formula: 10 (header) + 148×inputs + 34×outputs
pub fn estimate_vbytes(n_inputs: usize, n_outputs: usize) -> u64 {
    (10 + 148 * n_inputs + 34 * n_outputs) as u64
}

/// Returns (slow, standard, fast) fee in satoshis for a given vsize.
/// Rates: 2 / 10 / 25 sat/vbyte — update based on mempool conditions.
pub fn fee_tiers(vbytes: u64) -> (u64, u64, u64) {
    (vbytes * 2, vbytes * 10, vbytes * 25)
}

/// Check if `addr` belongs to this wallet (receive or change).
pub fn is_own_address(addr: &Address, receive: &[(Address, SecretKey)], change: &[(Address, SecretKey)]) -> bool {
    receive.iter().any(|(a, _)| a == addr) || change.iter().any(|(a, _)| a == addr)
}

// ── Imported UTXO ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub address: String,
}

/// Import UTXOs from a simple CSV file.
/// Format (one per line, no header): txid,vout,amount_btc,address
pub fn import_utxos_from_csv(path: &str) -> Result<Vec<Utxo>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|_| format!("Cannot read file '{}'", path))?;
    let mut utxos = Vec::new();
    for (lineno, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.splitn(4, ',').collect();
        if parts.len() < 4 {
            return Err(format!("Line {}: expected txid,vout,amount_btc,address", lineno + 1));
        }
        let vout: u32 = parts[1].trim().parse()
            .map_err(|_| format!("Line {}: invalid vout '{}'", lineno + 1, parts[1]))?;
        let amount_sats = btc_to_sats(parts[2])
            .map_err(|e| format!("Line {}: {}", lineno + 1, e))?;
        utxos.push(Utxo {
            txid: parts[0].trim().to_string(),
            vout,
            amount_sats,
            address: parts[3].trim().to_string(),
        });
    }
    if utxos.is_empty() {
        return Err("No UTXOs found in file.".to_string());
    }
    Ok(utxos)
}

// ── Transaction builder ───────────────────────────────────────────────────────

pub struct TxParams<'a> {
    pub from_address: &'a Address,
    pub from_key: &'a SecretKey,
    pub txid_str: String,
    pub vout: u32,
    pub input_sats: u64,
    pub to_address: Address,
    pub send_sats: u64,
    pub fee_sats: u64,
    pub change_address: &'a Address,
    pub use_rbf: bool,
    pub dry_run: bool,
}

/// Build (and optionally sign) a P2PKH transaction.
///
/// In dry-run mode, returns the unsigned hex to show the structure without signing.
/// RBF sets nSequence = 0xFFFFFFFD to signal replaceability.
pub fn build_transaction(p: &TxParams) -> Result<String, String> {
    let total = p.send_sats
        .checked_add(p.fee_sats)
        .ok_or("Amount overflow.")?;
    if total > p.input_sats {
        return Err(format!(
            "Insufficient funds.\n    Available: {} sats ({:.8} BTC)\n    Needed:    {} sats ({:.8} BTC)",
            p.input_sats, p.input_sats as f64 / 1e8,
            total, total as f64 / 1e8
        ));
    }
    let change_sats = p.input_sats - total;

    let txid = Txid::from_str(&p.txid_str)
        .map_err(|_| "Invalid Transaction ID.".to_string())?;

    let sequence = if p.use_rbf { Sequence(0xFFFF_FFFD) } else { Sequence::MAX };

    let txin = TxIn {
        previous_output: OutPoint { txid, vout: p.vout },
        script_sig: Builder::new().into_script(),
        sequence,
        witness: Witness::default(),
    };

    let mut outputs = vec![TxOut {
        value: p.send_sats,
        script_pubkey: p.to_address.script_pubkey(),
    }];

    if change_sats >= DUST_SATS {
        outputs.push(TxOut {
            value: change_sats,
            script_pubkey: p.change_address.script_pubkey(),
        });
    }

    let mut tx = Transaction {
        version: 1,
        lock_time: PackedLockTime::ZERO,
        input: vec![txin],
        output: outputs,
    };

    if p.dry_run {
        // Return the unsigned transaction hex with a "DRY RUN" prefix marker
        return Ok(format!("DRY_RUN:{}", hex::encode(serialize(&tx))));
    }

    // Sign the P2PKH input
    let secp = Secp256k1::new();
    let script_pubkey = p.from_address.script_pubkey();
    let sighash = {
        let cache = SighashCache::new(&tx);
        cache.legacy_signature_hash(0, &script_pubkey, EcdsaSighashType::All as u32)
            .map_err(|e| format!("Sighash failed: {}", e))?
    };

    let msg = Message::from_slice(sighash.as_ref())
        .map_err(|e| format!("Message error: {}", e))?;

    let sig = secp.sign_ecdsa(&msg, p.from_key);
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);

    let pubkey_bytes = p.from_key.public_key(&secp).serialize();
    tx.input[0].script_sig = Builder::new()
        .push_slice(&sig_bytes)
        .push_slice(&pubkey_bytes)
        .into_script();

    Ok(hex::encode(serialize(&tx)))
}

// ── Interactive send flow ─────────────────────────────────────────────────────

/// Full interactive guided send flow with all safety checks.
///
/// Collects the parameters and returns a populated TxParams struct.
pub fn collect_send_params<'a>(
    receive_addresses: &'a [(Address, SecretKey)],
    change_addresses: &'a [(Address, SecretKey)],
    preloaded_utxos: &[Utxo],
    dry_run: bool,
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
    println!("\n  {}Estimated tx size: {} vbytes{}", ui::DIM, vbytes, ui::RESET);
    println!("  Fee tiers (sat/vbyte):");
    println!("    {}[s]{}  Slow     ~2  sat/vbyte  →  {} sats  ({:.8} BTC)", ui::DIM, ui::RESET, slow, slow as f64/1e8);
    println!("    {}[n]{}  Normal  ~10  sat/vbyte  →  {} sats  ({:.8} BTC)", ui::DIM, ui::RESET, standard, standard as f64/1e8);
    println!("    {}[f]{}  Fast    ~25  sat/vbyte  →  {} sats  ({:.8} BTC)", ui::DIM, ui::RESET, fast, fast as f64/1e8);
    println!("    {}[m]{}  Enter manually", ui::DIM, ui::RESET);

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
    let change_address = if change_addresses.is_empty() {
        ui::warn("No change addresses available — change will go back to your sending address.");
        from_address
    } else {
        ui::info("This keeps your change on a separate internal address (BIP-44 standard).");
        &change_addresses[0].0
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