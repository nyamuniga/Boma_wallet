use bip39::Mnemonic;

/// Converts raw entropy bytes into a BIP-39 mnemonic phrase.
///
/// Returns `Err` if `entropy` is not a valid BIP-39 entropy length (16, 20, 24, 28, or 32 bytes).
/// The bip39 crate handles checksumming and word-list lookup internally.
pub fn generate_mnemonic(entropy: &[u8]) -> Result<Mnemonic, String> {
    Mnemonic::from_entropy(entropy)
        .map_err(|e| format!("Failed to generate mnemonic from entropy: {}", e))
}