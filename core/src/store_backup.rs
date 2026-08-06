use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, Params};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs::File;
use std::io::{Write, BufWriter};
use zeroize::Zeroize;

// ── Argon2id parameters ────────────────────────────────────────────────────
// 64 MB memory, 3 iterations, 1-way parallelism, 32-byte output.
// Adjust only after benchmarking; lowering memory cost weakens brute-force resistance.
const ARGON2_MEM_KB:   u32   = 65_536; // 64 MiB
const ARGON2_ITERS:    u32   = 3;
const ARGON2_PARALLEL: u32   = 1;
const ARGON2_KEY_LEN:  usize = 32;     // 256-bit AES key

/// Expected byte lengths for cryptographic parameters.
const SALT_LEN:  usize = 32; // 256-bit Argon2 salt
const NONCE_LEN: usize = 12; // 96-bit AES-GCM nonce

/// Encrypts the mnemonic with AES-256-GCM and writes the result to disk.
///
/// Security properties:
///   - The passphrase is NEVER stored — authentication is via successful decryption.
///   - Private keys are NOT stored — they are re-derived from the mnemonic at login.
///   - The encryption key is derived from the passphrase + a random 32-byte salt
///     using Argon2id with 64MB memory cost, 3 iterations, and 1 degree of parallelism.
///   - Each save generates a fresh random salt and nonce, so the ciphertext
///     is different every time even for the same mnemonic.
///   - File permissions are set to 0600 (owner-only) on Unix systems.
///
/// Backup file format (no plaintext secrets):
///   SALT: <64 hex chars>
///   NONCE: <24 hex chars>
///   DATA: <hex-encoded AES-256-GCM ciphertext + 16-byte auth tag>
pub fn store_backup(passphrase: &str, mnemonic_str: &str, filename: &str) -> Result<(), String> {
    // Random 32-byte salt (for Argon2id key derivation) and 12-byte nonce (for AES-GCM)
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    // Derive a 256-bit AES key from passphrase + salt via Argon2id
    let mut key_bytes = [0u8; ARGON2_KEY_LEN];
    let params = Params::new(ARGON2_MEM_KB, ARGON2_ITERS, ARGON2_PARALLEL, Some(ARGON2_KEY_LEN))
        .map_err(|e| format!("Argon2 params error: {}", e))?;
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );
    argon2.hash_password_into(passphrase.as_bytes(), &salt, &mut key_bytes)
        .map_err(|e| format!("Argon2 failed: {}", e))?;

    // Encrypt the mnemonic; the 16-byte GCM auth tag is appended to ciphertext
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, mnemonic_str.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Zeroize all key material immediately after use
    key_bytes.zeroize();

    // Write only the encrypted blob — no plaintext secrets on disk
    let file = File::create(filename)
        .map_err(|e| format!("Failed to create backup file: {}", e))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "SALT: {}", hex::encode(&salt)).map_err(|e| e.to_string())?;
    writeln!(writer, "NONCE: {}", hex::encode(&nonce_bytes)).map_err(|e| e.to_string())?;
    writeln!(writer, "DATA: {}", hex::encode(&ciphertext)).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    // Zeroize salt and nonce after writing
    salt.zeroize();
    nonce_bytes.zeroize();

    // ── Set restrictive file permissions (Unix only) ──────────────────────
    // Owner read/write only (0600) — prevents other local users from copying
    // the encrypted blob for offline brute-force attacks.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(filename, perms)
            .map_err(|e| format!("Failed to set file permissions on '{}': {}", filename, e))?;
    }

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

    // ── Decode and validate parameter lengths ─────────────────────────────
    // Use a generic error message for all validation failures to prevent
    // oracle attacks that could distinguish corruption from wrong passphrase.
    let generic_err = "Incorrect passphrase or corrupted backup.";

    let salt = hex::decode(salt_hex.ok_or("Corrupted backup: missing SALT")?)
        .map_err(|_| "Corrupted backup: invalid SALT encoding".to_string())?;
    if salt.len() != SALT_LEN {
        return Err(format!("Corrupted backup: SALT must be {} bytes, got {}", SALT_LEN, salt.len()));
    }

    let nonce_bytes = hex::decode(nonce_hex.ok_or("Corrupted backup: missing NONCE")?)
        .map_err(|_| "Corrupted backup: invalid NONCE encoding".to_string())?;
    if nonce_bytes.len() != NONCE_LEN {
        // Return generic error — don't reveal exact validation failure to potential attacker
        return Err(generic_err.to_string());
    }

    let ciphertext = hex::decode(data_hex.ok_or("Corrupted backup: missing DATA")?)
        .map_err(|_| "Corrupted backup: invalid DATA encoding".to_string())?;

    // ── Key derivation ────────────────────────────────────────────────────
    let mut key_bytes = [0u8; ARGON2_KEY_LEN];
    let params = Params::new(ARGON2_MEM_KB, ARGON2_ITERS, ARGON2_PARALLEL, Some(ARGON2_KEY_LEN))
        .map_err(|e| format!("Argon2 params error: {}", e))?;
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );
    argon2.hash_password_into(passphrase.as_bytes(), &salt, &mut key_bytes)
        .map_err(|_| generic_err.to_string())?;

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|_| generic_err.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Authentication failure = wrong passphrase or corrupted data
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| generic_err.to_string())?;

    // Zeroize key material after use
    key_bytes.zeroize();

    String::from_utf8(plaintext).map_err(|_| "Corrupted backup: invalid UTF-8.".to_string())
}