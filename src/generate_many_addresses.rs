use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

/// Generates 20 external receive addresses on BIP-44 path m/44'/{coin}'/0'/0/{i}.
///
/// External chain (index 0) addresses are shared with senders.
/// Compatible with Electrum, Sparrow, Ledger, and any other BIP-44 wallet.
pub fn generate_many_addresses(
    root_key: &ExtendedPrivKey,
    network: Network,
) -> Vec<(Address, SecretKey)> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    let mut addresses = Vec::new();

    for i in 0..20u32 {
        let path_str = format!("m/44'/{}'/0'/0/{}", coin, i);
        let path = match DerivationPath::from_str(&path_str) {
            Ok(p) => p,
            Err(e) => { eprintln!("Warning: bad path {}: {}", path_str, e); continue; }
        };
        let child = match root_key.derive_priv(&secp, &path) {
            Ok(k) => k,
            Err(e) => { eprintln!("Warning: derive failed at {}: {}", i, e); continue; }
        };
        let priv_key = child.private_key;
        let pub_key  = PublicKey::new(priv_key.public_key(&secp));
        let address  = Address::p2pkh(&pub_key, network);
        addresses.push((address, priv_key));
    }

    addresses
}