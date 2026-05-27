use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha512;
use std::fs::File;
use std::io::{Write, BufWriter};

/// Encrypts the mnemonic with AES-256-GCM and writes the result to disk.
///
/// Security properties:
///   - The passphrase is NEVER stored — authentication is via successful decryption.
///   - Private keys are NOT stored — they are re-derived from the mnemonic at login.
///   - The encryption key is derived from the passphrase + a random 32-byte salt
///     using PBKDF2-HMAC-SHA512 with 100,000 iterations.
///   - Each save generates a fresh random salt and nonce, so the ciphertext
///     is different every time even for the same mnemonic.
///
/// Backup file format (no plaintext secrets):
///   SALT: <64 hex chars>
///   NONCE: <24 hex chars>
///   DATA: <hex-encoded AES-256-GCM ciphertext + 16-byte auth tag>
pub fn store_backup(passphrase: &str, mnemonic_str: &str, filename: &str) -> Result<(), String> {
    // Random 32-byte salt (for PBKDF2) and 12-byte nonce (for AES-GCM)
    let mut salt = [0u8; 32];
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    // Derive a 256-bit AES key from passphrase + salt via PBKDF2-HMAC-SHA512
    let mut key_bytes = [0u8; 32];
    pbkdf2::<Hmac<Sha512>>(passphrase.as_bytes(), &salt, 100_000, &mut key_bytes);

    // Encrypt the mnemonic; the 16-byte GCM auth tag is appended to ciphertext
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, mnemonic_str.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Write only the encrypted blob — no plaintext secrets on disk
    let file = File::create(filename)
        .map_err(|e| format!("Failed to create backup file: {}", e))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "SALT: {}", hex::encode(salt)).map_err(|e| e.to_string())?;
    writeln!(writer, "NONCE: {}", hex::encode(nonce_bytes)).map_err(|e| e.to_string())?;
    writeln!(writer, "DATA: {}", hex::encode(&ciphertext)).map_err(|e| e.to_string())?;

    println!("  ✓ Wallet encrypted and saved to '{}'.", filename);
    Ok(())
}

/// Decrypts the backup file using the given passphrase.
///
/// Returns `Ok(mnemonic)` on success.
/// Returns `Err` if the passphrase is wrong, the file is missing, or the data is corrupt.
/// A wrong passphrase causes AES-GCM authentication to fail — the error message does NOT
/// reveal whether the file is corrupt or the passphrase is wrong, to avoid oracle attacks.
pub fn load_backup(passphrase: &str, filename: &str) -> Result<String, String> {
    let contents = std::fs::read_to_string(filename)
        .map_err(|_| "No wallet found. Please create a new wallet first.".to_string())?;

    let mut salt_hex: Option<String> = None;
    let mut nonce_hex: Option<String> = None;
    let mut data_hex: Option<String> = None;

    for line in contents.lines() {
        if let Some(v) = line.strip_prefix("SALT: ") {
            salt_hex = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("NONCE: ") {
            nonce_hex = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("DATA: ") {
            data_hex = Some(v.trim().to_string());
        }
    }

    let salt = hex::decode(salt_hex.ok_or("Corrupted backup: missing SALT")?)
        .map_err(|_| "Corrupted backup: invalid SALT encoding".to_string())?;
    let nonce_bytes = hex::decode(nonce_hex.ok_or("Corrupted backup: missing NONCE")?)
        .map_err(|_| "Corrupted backup: invalid NONCE encoding".to_string())?;
    let ciphertext = hex::decode(data_hex.ok_or("Corrupted backup: missing DATA")?)
        .map_err(|_| "Corrupted backup: invalid DATA encoding".to_string())?;

    // Derive the same key from passphrase + salt
    let mut key_bytes = [0u8; 32];
    pbkdf2::<Hmac<Sha512>>(passphrase.as_bytes(), &salt, 100_000, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| "Cipher init failed.".to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Authentication failure = wrong passphrase or corrupted data
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Incorrect passphrase or corrupted backup.".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "Corrupted backup: invalid UTF-8.".to_string())
}