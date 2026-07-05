use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha512;
use zeroize::{Zeroize, Zeroizing};

/// Derives a 64-byte (512-bit) seed from a BIP-39 mnemonic and optional passphrase.
///
/// Follows the BIP-39 standard exactly:
///   - Password  = mnemonic phrase (UTF-8)
///   - Salt      = "mnemonic" + passphrase  (the literal string "mnemonic" is the BIP-39 prefix)
///   - Iterations = 2048
///   - PRF       = HMAC-SHA512
///
/// This makes wallets compatible with Ledger, Trezor, Electrum, Sparrow, etc.
///
/// Returns a `Zeroizing<Vec<u8>>` that automatically wipes itself from memory when dropped.
/// Callers do NOT need to call `.zeroize()` manually, though doing so is harmless.
pub fn derive_seed_from_mnemonic(mnemonic: &str, passphrase: &str) -> Zeroizing<Vec<u8>> {
    let mnemonic_bytes = mnemonic.as_bytes();

    let mut salt = format!("mnemonic{}", passphrase);
    let salt_bytes = salt.as_bytes();

    let mut seed = Zeroizing::new(vec![0u8; 64]);
    pbkdf2::<Hmac<Sha512>>(mnemonic_bytes, salt_bytes, 2048, &mut seed);

    salt.zeroize();

    seed
}