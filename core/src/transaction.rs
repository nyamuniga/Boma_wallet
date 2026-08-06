use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::witness::Witness;
use bitcoin::consensus::encode::serialize;
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::sighash::SighashCache;
use bitcoin::{EcdsaSighashType, OutPoint, PackedLockTime, Sequence, Transaction, TxIn, TxOut, Txid};
use std::str::FromStr;

pub const DUST_SATS: u64 = 546;

/// Returns the BIP-44/84 coin type for the given network.
/// Used to build derivation paths: m/84'/{coin}'/...
/// coin = 0 for mainnet, 1 for testnet.
pub fn coin_type(network: Network) -> u32 {
    if network == Network::Bitcoin { 0 } else { 1 }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Converts a BTC-denominated string (e.g. "0.00500000") to satoshis using fixed-point
/// integer arithmetic — no floating-point involved, so there is no IEEE 754 rounding risk.
///
/// Accepts up to 8 decimal places. Rejects negative values, empty inputs, malformed strings,
/// and integer portions with leading zeros (except bare "0" before a decimal point).
pub fn btc_to_sats(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let err = || format!("'{}' is not a valid BTC amount — use format like 0.00500000", s);

    if s.is_empty() {
        return Err("Amount cannot be empty.".to_string());
    }
    if s.starts_with('-') {
        return Err("Amount cannot be negative.".to_string());
    }

    // Split on the decimal point (optional)
    let (int_str, frac_str) = match s.find('.') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None      => (s, ""),
    };

    // Validate: only ASCII digits allowed
    if !int_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(err());
    }
    if !frac_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(err());
    }
    if frac_str.len() > 8 {
        return Err("Too many decimal places — Bitcoin has at most 8.".to_string());
    }

    // ── L3: Reject leading zeros in the integer portion ───────────────────
    // "00", "01", "007" are ambiguous and potentially mask the true amount.
    // Only bare "" (before ".5") and "0" are allowed.
    if int_str.len() > 1 && int_str.starts_with('0') {
        return Err(format!(
            "'{}' has leading zeros — remove them to avoid ambiguity.", s
        ));
    }

    // Integer BTC portion → satoshis (multiply by 10^8)
    let int_val: u64 = if int_str.is_empty() {
        0
    } else {
        int_str.parse::<u64>().map_err(|_| err())?
    };
    let int_sats = int_val
        .checked_mul(100_000_000)
        .ok_or_else(|| "Amount too large.".to_string())?;

    // Fractional portion: right-pad with zeros to exactly 8 digits, then parse
    let mut frac_padded = frac_str.to_string();
    while frac_padded.len() < 8 {
        frac_padded.push('0');
    }
    let frac_sats: u64 = frac_padded.parse().map_err(|_| err())?;

    int_sats.checked_add(frac_sats)
        .ok_or_else(|| "Amount too large.".to_string())
}


/// Estimate Native SegWit (P2WPKH) transaction size in virtual bytes.
/// Formula: 10.5 (header) + 68×inputs + 31×outputs
pub fn estimate_vbytes(n_inputs: usize, n_outputs: usize) -> u64 {
    (10.5 + 68.0 * n_inputs as f64 + 31.0 * n_outputs as f64).ceil() as u64
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub address: String,
}

/// Parse UTXOs from CSV string content.
/// Format (one per line, no header): txid,vout,amount_btc,address
pub fn parse_utxos_from_csv_content(contents: &str) -> Result<Vec<Utxo>, String> {
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

/// Import UTXOs from a simple CSV file.
/// Format (one per line, no header): txid,vout,amount_btc,address
pub fn import_utxos_from_csv(path: &str) -> Result<Vec<Utxo>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|_| format!("Cannot read file '{}'", path))?;
    parse_utxos_from_csv_content(&contents)
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

/// Build (and optionally sign) a single-input P2WPKH transaction.
///
/// In dry-run mode, returns the unsigned hex to show the structure without signing.
/// RBF sets nSequence = 0xFFFFFFFD to signal replaceability.
/// Uses transaction version 2 (required for BIP-68 relative timelocks).
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

    // L4: Use transaction version 2 — modern standard, required for BIP-68
    let mut tx = Transaction {
        version: 2,
        lock_time: PackedLockTime::ZERO,
        input: vec![txin],
        output: outputs,
    };

    if p.dry_run {
        // Return the unsigned transaction hex with a "DRY RUN" prefix marker
        return Ok(format!("DRY_RUN:{}", hex::encode(serialize(&tx))));
    }

    // Sign the P2WPKH input
    sign_p2wpkh_input(&mut tx, 0, p.from_key, p.from_address, p.input_sats)?;

    Ok(hex::encode(serialize(&tx)))
}

// ── Multi-input transaction builder (H3) ──────────────────────────────────────

/// Parameters for a multi-input P2WPKH transaction.
pub struct MultiTxParams<'a> {
    /// Each input: (UTXO, signing key, source address).
    pub inputs: Vec<(&'a Utxo, &'a SecretKey, &'a Address)>,
    pub to_address: Address,
    pub send_sats: u64,
    pub fee_sats: u64,
    pub change_address: &'a Address,
    pub use_rbf: bool,
    pub dry_run: bool,
}

/// Build (and optionally sign) a multi-input P2WPKH transaction.
///
/// This allows consolidating funds spread across multiple UTXOs into a single
/// transaction. Each input is signed independently with its own private key.
/// Uses transaction version 2 (BIP-68 compatible).
pub fn build_multi_input_transaction(p: &MultiTxParams) -> Result<String, String> {
    if p.inputs.is_empty() {
        return Err("At least one input is required.".to_string());
    }

    // Sum all input values
    let total_input: u64 = p.inputs.iter()
        .map(|(utxo, _, _)| utxo.amount_sats)
        .try_fold(0u64, |acc, v| acc.checked_add(v))
        .ok_or("Input sum overflow.")?;

    let total_needed = p.send_sats
        .checked_add(p.fee_sats)
        .ok_or("Amount overflow.")?;

    if total_needed > total_input {
        return Err(format!(
            "Insufficient funds.\n    Available: {} sats ({:.8} BTC)\n    Needed:    {} sats ({:.8} BTC)",
            total_input, total_input as f64 / 1e8,
            total_needed, total_needed as f64 / 1e8
        ));
    }
    let change_sats = total_input - total_needed;

    let sequence = if p.use_rbf { Sequence(0xFFFF_FFFD) } else { Sequence::MAX };

    // Build inputs
    let mut tx_inputs = Vec::with_capacity(p.inputs.len());
    for (utxo, _, _) in &p.inputs {
        let txid = Txid::from_str(&utxo.txid)
            .map_err(|_| format!("Invalid Transaction ID: {}", &utxo.txid))?;
        tx_inputs.push(TxIn {
            previous_output: OutPoint { txid, vout: utxo.vout },
            script_sig: Builder::new().into_script(),
            sequence,
            witness: Witness::default(),
        });
    }

    // Build outputs
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
        version: 2,
        lock_time: PackedLockTime::ZERO,
        input: tx_inputs,
        output: outputs,
    };

    if p.dry_run {
        return Ok(format!("DRY_RUN:{}", hex::encode(serialize(&tx))));
    }

    // Sign each input with its corresponding key
    for (idx, (utxo, key, addr)) in p.inputs.iter().enumerate() {
        sign_p2wpkh_input(&mut tx, idx, key, addr, utxo.amount_sats)?;
    }

    Ok(hex::encode(serialize(&tx)))
}

// ── Shared P2WPKH signing logic ──────────────────────────────────────────────

/// Signs a single P2WPKH input in-place.
///
/// Computes the BIP-143 segwit sighash and inserts the signature + pubkey
/// into the witness field at the given input index.
fn sign_p2wpkh_input(
    tx: &mut Transaction,
    input_idx: usize,
    secret_key: &SecretKey,
    from_address: &Address,
    input_sats: u64,
) -> Result<(), String> {
    let secp = Secp256k1::new();
    let pub_key = bitcoin::PublicKey::new(secret_key.public_key(&secp));

    // For P2WPKH, the scriptCode used for the sighash is the P2PKH script pubkey
    let script_code = Address::p2pkh(&pub_key, from_address.network).script_pubkey();

    let sighash = {
        let mut cache = SighashCache::new(&*tx);
        cache.segwit_signature_hash(input_idx, &script_code, input_sats, EcdsaSighashType::All)
            .map_err(|e| format!("Sighash failed on input {}: {}", input_idx, e))?
    };

    let msg = Message::from_slice(sighash.as_ref())
        .map_err(|e| format!("Message error: {}", e))?;

    let sig = secp.sign_ecdsa(&msg, secret_key);
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);

    let pubkey_bytes = secret_key.public_key(&secp).serialize();

    // In SegWit, the scriptSig is empty and signatures go into the Witness field
    tx.input[input_idx].script_sig = Builder::new().into_script();

    let mut witness = Witness::new();
    witness.push(&sig_bytes);
    witness.push(&pubkey_bytes);
    tx.input[input_idx].witness = witness;

    Ok(())
}
