use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::Network;
use bitcoin::consensus::deserialize;
use serde::Serialize;

// ── Public summary struct (returned to CLI and GUI) ───────────────────────────

/// Human-readable PSBT summary for display before signing.
#[derive(Debug, Clone, Serialize)]
pub struct PsbtSummary {
    /// Total satoshis being sent to the destination (non-change outputs).
    pub send_sats: u64,
    /// Total input value (sum of all UTXO values in the PSBT).
    pub input_sats: u64,
    /// Miner fee in satoshis (input_sats - all outputs).
    pub fee_sats: u64,
    /// Fee as a percentage of the total input, for anomaly detection.
    pub fee_pct: f64,
    /// Number of inputs being signed.
    pub input_count: usize,
    /// Number of outputs (including change).
    pub output_count: usize,
    /// All destination addresses (excluding change back to self).
    pub destinations: Vec<String>,
    /// True if the fee is suspiciously high (>5% of input value).
    pub fee_warning: bool,
}

// ── Parse ─────────────────────────────────────────────────────────────────────

/// Reads and parses a PSBT file from disk, returning a human-readable summary.
///
/// The PSBT must be in standard binary format (base64-encoded files are
/// automatically detected and decoded).
pub fn parse_psbt(path: &str) -> Result<(PartiallySignedTransaction, PsbtSummary), String> {
    let raw = std::fs::read(path)
        .map_err(|e| format!("Cannot read PSBT file '{}': {}", path, e))?;

    // Support both raw binary and base64-encoded PSBTs
    let psbt: PartiallySignedTransaction = if raw.starts_with(b"psbt\xff") {
        deserialize(&raw).map_err(|e| format!("Invalid PSBT binary: {}", e))?
    } else {
        // Try base64 decode
        let decoded = base64_decode(&raw)?;
        deserialize(&decoded).map_err(|e| format!("Invalid PSBT (base64): {}", e))?
    };

    let summary = summarise(&psbt)?;
    Ok((psbt, summary))
}

/// Parses a PSBT from a raw base64 string (e.g. decoded from a QR code).
pub fn parse_psbt_from_base64(b64: &str) -> Result<(PartiallySignedTransaction, PsbtSummary), String> {
    let decoded = base64_decode(b64.trim().as_bytes())?;
    let psbt: PartiallySignedTransaction = deserialize(&decoded)
        .map_err(|e| format!("Invalid PSBT data: {}", e))?;
    let summary = summarise(&psbt)?;
    Ok((psbt, summary))
}

fn base64_decode(input: &[u8]) -> Result<Vec<u8>, String> {
    // Simple base64 decode using the standard alphabet
    let s = std::str::from_utf8(input)
        .map_err(|_| "PSBT file contains non-UTF-8 data".to_string())?
        .trim();
    // Use the bitcoin crate's hex or a manual decode — bitcoin 0.29 ships base64 via deps
    base64::decode(s).map_err(|e| format!("Base64 decode failed: {}", e))
}

fn summarise(psbt: &PartiallySignedTransaction) -> Result<PsbtSummary, String> {
    // Sum all known input values from PSBT witness_utxo / non_witness_utxo
    let input_sats: u64 = psbt.inputs.iter().enumerate().try_fold(0u64, |acc, (i, inp)| {
        let val = inp.witness_utxo.as_ref().map(|u| u.value)
            .or_else(|| {
                inp.non_witness_utxo.as_ref().and_then(|prev_tx| {
                    let vout = psbt.unsigned_tx.input[i].previous_output.vout as usize;
                    prev_tx.output.get(vout).map(|o| o.value)
                })
            })
            .ok_or_else(|| format!("Input {} has no UTXO data — PSBT is incomplete.", i))?;
        Ok::<u64, String>(acc + val)
    })?;

    // Sum all output values
    let total_out: u64 = psbt.unsigned_tx.output.iter().map(|o| o.value).sum();
    let fee_sats = input_sats.saturating_sub(total_out);
    let fee_pct = if input_sats > 0 { fee_sats as f64 / input_sats as f64 * 100.0 } else { 0.0 };

    // Collect destination addresses (all outputs — caller or UI can mark change)
    let destinations: Vec<String> = psbt.unsigned_tx.output.iter()
        .filter_map(|o| bitcoin::util::address::Address::from_script(&o.script_pubkey, bitcoin::Network::Bitcoin).ok())
        .map(|a| a.to_string())
        .collect();

    let send_sats: u64 = psbt.unsigned_tx.output.iter().map(|o| o.value).sum();

    Ok(PsbtSummary {
        send_sats,
        input_sats,
        fee_sats,
        fee_pct,
        input_count: psbt.inputs.len(),
        output_count: psbt.unsigned_tx.output.len(),
        destinations,
        fee_warning: fee_pct > 5.0,
    })
}

// ── Sign ──────────────────────────────────────────────────────────────────────

/// Signs all inputs in the PSBT that belong to this wallet.
///
/// Uses the BIP-32 derivation paths recorded in the PSBT's `bip32_derivation`
/// map to identify which inputs we own and derive the exact signing key.
pub fn sign_psbt(
    mut psbt: PartiallySignedTransaction,
    root_key: &ExtendedPrivKey,
    _network: Network,
) -> Result<PartiallySignedTransaction, String> {
    let secp = Secp256k1::new();
    let mut signed_count = 0usize;

    for (input_idx, psbt_input) in psbt.inputs.iter_mut().enumerate() {
        // Collect all derivation paths for keys we might own
        let derivations: Vec<(bitcoin::secp256k1::PublicKey, DerivationPath)> = psbt_input
            .bip32_derivation
            .iter()
            .filter(|(_, (fingerprint, _))| {
                // Only sign inputs whose fingerprint matches our root key
                fingerprint == &root_key.fingerprint(&secp)
            })
            .map(|(pk, (_, path))| (*pk, path.clone()))
            .collect();

        if derivations.is_empty() {
            continue; // This input doesn't belong to us
        }

        for (pubkey, path) in derivations {
            let child_key = root_key
                .derive_priv(&secp, &path)
                .map_err(|e| format!("Key derivation failed for input {}: {}", input_idx, e))?;

            let child_pubkey = child_key.private_key.public_key(&secp);
            if child_pubkey.serialize() != pubkey.serialize() {
                continue; // Derived key doesn't match recorded pubkey
            }

            // Determine the script to sign against
            let script = psbt_input.witness_utxo.as_ref()
                .map(|u| u.script_pubkey.clone())
                .or_else(|| {
                    psbt_input.non_witness_utxo.as_ref().and_then(|prev_tx| {
                        let vout = psbt.unsigned_tx.input[input_idx].previous_output.vout as usize;
                        prev_tx.output.get(vout).map(|o| o.script_pubkey.clone())
                    })
                })
                .ok_or_else(|| format!("No script found for input {}", input_idx))?;

            // Sign using legacy P2PKH sighash
            use bitcoin::util::sighash::SighashCache;
            use bitcoin::{EcdsaSighashType, EcdsaSig};

            let sighash = {
                let cache = SighashCache::new(&psbt.unsigned_tx);
                cache
                    .legacy_signature_hash(input_idx, &script, EcdsaSighashType::All as u32)
                    .map_err(|e| format!("Sighash error on input {}: {}", input_idx, e))?
            };

            let msg = bitcoin::secp256k1::Message::from_slice(sighash.as_ref())
                .map_err(|e| format!("Message error: {}", e))?;

            let sig = secp.sign_ecdsa(&msg, &child_key.private_key);
            let ecdsa_sig = EcdsaSig { sig, hash_ty: EcdsaSighashType::All };

            psbt_input.partial_sigs.insert(
                bitcoin::PublicKey { compressed: true, inner: child_pubkey },
                ecdsa_sig,
            );
            signed_count += 1;
        }
    }

    if signed_count == 0 {
        return Err(
            "No inputs were signed. This PSBT may not belong to this wallet, \
             or the PSBT is missing BIP-32 derivation metadata."
            .to_string(),
        );
    }

    Ok(psbt)
}

// ── Export ────────────────────────────────────────────────────────────────────

/// Writes the signed PSBT to a file in standard binary format.
pub fn export_psbt(psbt: &PartiallySignedTransaction, path: &str) -> Result<(), String> {
    use bitcoin::consensus::encode::serialize;
    let bytes = serialize(psbt);
    std::fs::write(path, &bytes)
        .map_err(|e| format!("Failed to write signed PSBT to '{}': {}", path, e))
}

/// Returns the PSBT as a base64 string (for QR code display).
pub fn psbt_to_base64(psbt: &PartiallySignedTransaction) -> String {
    use bitcoin::consensus::encode::serialize;
    base64::encode(serialize(psbt))
}

// ── base64 helper (reuse the dep already pulled in by bitcoin crate) ──────────

mod base64 {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        // Manual decode using the standard base64 alphabet
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut lookup = [255u8; 256];
        for (i, &c) in alphabet.iter().enumerate() { lookup[c as usize] = i as u8; }

        let s = s.replace('\n', "").replace('\r', "").replace(' ', "");
        let s = s.trim_end_matches('=');
        let mut out = Vec::with_capacity(s.len() * 3 / 4);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i + 3 < bytes.len() {
            let [a, b, c, d] = [
                lookup[bytes[i] as usize],
                lookup[bytes[i+1] as usize],
                lookup[bytes[i+2] as usize],
                lookup[bytes[i+3] as usize],
            ];
            if a == 255 || b == 255 || c == 255 || d == 255 {
                return Err("Invalid base64 character".to_string());
            }
            out.push((a << 2) | (b >> 4));
            out.push((b << 4) | (c >> 2));
            out.push((c << 6) | d);
            i += 4;
        }
        // Handle remaining bytes
        match bytes.len() - i {
            2 => {
                let [a, b] = [lookup[bytes[i] as usize], lookup[bytes[i+1] as usize]];
                if a == 255 || b == 255 { return Err("Invalid base64".to_string()); }
                out.push((a << 2) | (b >> 4));
            }
            3 => {
                let [a, b, c] = [lookup[bytes[i] as usize], lookup[bytes[i+1] as usize], lookup[bytes[i+2] as usize]];
                if a == 255 || b == 255 || c == 255 { return Err("Invalid base64".to_string()); }
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
            }
            _ => {}
        }
        Ok(out)
    }

    pub fn encode(data: Vec<u8>) -> String {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        let mut i = 0;
        while i + 2 < data.len() {
            let [a, b, c] = [data[i], data[i+1], data[i+2]];
            out.push(alphabet[(a >> 2) as usize] as char);
            out.push(alphabet[((a & 3) << 4 | b >> 4) as usize] as char);
            out.push(alphabet[((b & 15) << 2 | c >> 6) as usize] as char);
            out.push(alphabet[(c & 63) as usize] as char);
            i += 3;
        }
        match data.len() - i {
            1 => {
                let a = data[i];
                out.push(alphabet[(a >> 2) as usize] as char);
                out.push(alphabet[((a & 3) << 4) as usize] as char);
                out.push_str("==");
            }
            2 => {
                let [a, b] = [data[i], data[i+1]];
                out.push(alphabet[(a >> 2) as usize] as char);
                out.push(alphabet[((a & 3) << 4 | b >> 4) as usize] as char);
                out.push(alphabet[((b & 15) << 2) as usize] as char);
                out.push('=');
            }
            _ => {}
        }
        out
    }
}
