use rand::rngs::OsRng;
use rand::RngCore;

/// Generates 32 bytes (256 bits) of cryptographic entropy using the OS random source.
pub fn generate_entropy() -> [u8; 32] {
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);
    entropy
}