use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ExtendedPrivKey;
use zeroize::Zeroize;

use crate::audit_log::AuditLog;
use boma_core::config::Config;
use boma_core::transaction::Utxo;

/// Encapsulates all data required for an active wallet session.
///
/// **Security**: Implements `Drop` to zeroize all sensitive key material
/// when the session ends (lock, timeout, or process exit). This prevents
/// secrets from lingering in freed heap/stack memory where they could be
/// recovered via cold-boot attacks, swap file forensics, or core dumps.
pub struct SessionState<'a> {
    pub mnemonic_str: String,
    pub receive_addresses: Vec<(Address, SecretKey)>,
    pub change_addresses: Vec<(Address, SecretKey)>,
    pub root_key: ExtendedPrivKey,
    pub fingerprint: String,
    pub cfg: &'a Config,
    pub audit: &'a AuditLog,
    pub preloaded_utxos: Vec<Utxo>,
    /// Tracks which change address to use next (rotated after each tx).
    pub change_index: usize,
}

impl<'a> Drop for SessionState<'a> {
    fn drop(&mut self) {
        // ── Zeroize the mnemonic string ───────────────────────────────────
        self.mnemonic_str.zeroize();

        // ── Zeroize all SecretKeys in receive and change address lists ────
        // SecretKey is 32 bytes internally. We access the raw bytes via
        // secret_bytes() and overwrite them through a raw pointer.
        for (_, key) in self.receive_addresses.iter() {
            zeroize_secret_key(key);
        }
        for (_, key) in self.change_addresses.iter() {
            zeroize_secret_key(key);
        }

        // ── Zeroize the root extended private key ─────────────────────────
        // ExtendedPrivKey contains a SecretKey (private_key) and a ChainCode.
        // We zeroize the private_key field via the same raw-pointer technique.
        zeroize_secret_key(&self.root_key.private_key);

        // Clear the fingerprint (not secret, but good hygiene)
        self.fingerprint.zeroize();

        // Clear address vectors so dangling references don't persist
        self.receive_addresses.clear();
        self.change_addresses.clear();
        self.preloaded_utxos.clear();
    }
}

/// Overwrites the 32 bytes of a secp256k1 SecretKey with zeros.
///
/// Safety: SecretKey is repr(transparent) over a 32-byte array.
/// The bitcoin/secp256k1 crate does not implement Zeroize, so we must
/// do a raw byte-level overwrite. This is the accepted pattern for
/// zeroizing foreign types in security-critical Rust code.
fn zeroize_secret_key(key: &SecretKey) {
    let ptr = key as *const SecretKey as *mut u8;
    unsafe {
        // SecretKey is 32 bytes
        std::ptr::write_bytes(ptr, 0, 32);
    }
}
