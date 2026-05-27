use bip39::Mnemonic;

/// Converts raw entropy bytes into a BIP-39 mnemonic phrase.
/// The bip39 crate handles checksumming and word-list lookup internally.
pub fn generate_mnemonic(entropy: &[u8]) -> Mnemonic {
    Mnemonic::from_entropy(entropy).expect("Failed to generate mnemonic from entropy")
}