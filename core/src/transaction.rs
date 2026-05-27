use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::witness::Witness;
use bitcoin::consensus::encode::serialize;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::sighash::SighashCache;
use bitcoin::{EcdsaSighashType, OutPoint, PackedLockTime, Sequence, Transaction, TxIn, TxOut, Txid};
use std::str::FromStr;

pub const DUST_SATS: u64 = 546;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse a BTC-denominated string (e.g. "0.005") to satoshis.
pub fn btc_to_sats(s: &str) -> Result<u64, String> {
    let v: f64 = s.trim().parse()
        .map_err(|_| format!("'{}' is not a valid number — use format like 0.005", s.trim()))?;
    if v < 0.0 { return Err("Amount cannot be negative.".to_string()); }
    Ok((v * 100_000_000.0).round() as u64)
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

/// Build (and optionally sign) a P2WPKH transaction.
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

    // Sign the P2WPKH input
    let secp = Secp256k1::new();
    let pub_key = bitcoin::PublicKey::new(p.from_key.public_key(&secp));
    
    // For P2WPKH, the scriptCode used for the sighash is actually the P2PKH script pubkey
    // of the same public key.
    let script_code = Address::p2pkh(&pub_key, p.from_address.network).script_pubkey();
    
    let sighash = {
        let mut cache = SighashCache::new(&tx);
        cache.segwit_signature_hash(0, &script_code, p.input_sats, EcdsaSighashType::All)
            .map_err(|e| format!("Sighash failed: {}", e))?
    };

    let msg = Message::from_slice(sighash.as_ref())
        .map_err(|e| format!("Message error: {}", e))?;

    let sig = secp.sign_ecdsa(&msg, p.from_key);
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);

    let pubkey_bytes = p.from_key.public_key(&secp).serialize();
    
    // In SegWit, the scriptSig is empty and signatures go into the Witness field
    tx.input[0].script_sig = Builder::new().into_script();
    
    let mut witness = Witness::new();
    witness.push(&sig_bytes);
    witness.push(&pubkey_bytes);
    tx.input[0].witness = witness;

    Ok(hex::encode(serialize(&tx)))
}
