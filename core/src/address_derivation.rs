use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

/// Shared helper to derive a sequence of addresses on a specific BIP-84 chain.
///
/// `chain_index`: 0 for external (receive), 1 for internal (change).
pub fn derive_address_range(
    root_key: &ExtendedPrivKey,
    network: Network,
    chain_index: u32,
    count: u32,
) -> Vec<(Address, SecretKey)> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    let mut addresses = Vec::new();

    for i in 0..count {
        let path_str = format!("m/84'/{}'/0'/{}/{}", coin, chain_index, i);
        let path = match DerivationPath::from_str(&path_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: could not parse path {}: {}", path_str, e);
                continue;
            }
        };
        let child = match root_key.derive_priv(&secp, &path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Warning: could not derive key at index {}: {}", i, e);
                continue;
            }
        };

        let priv_key = child.private_key;
        let pub_key = PublicKey::new(priv_key.public_key(&secp));
        let address = match Address::p2wpkh(&pub_key, network) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Warning: could not create P2WPKH address at index {}: {}", i, e);
                continue;
            }
        };
        addresses.push((address, priv_key));
    }

    addresses
}
