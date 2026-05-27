use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

/// Generates 20 change addresses on the BIP-44 internal chain: m/44'/{coin}'/0'/1/{i}.
///
/// Change addresses are used as recipients for the "leftover" funds in a transaction.
/// Using a dedicated internal chain (index 1) keeps change addresses separate from
/// receive addresses, matching what Electrum, Sparrow, and hardware wallets do.
pub fn generate_change_addresses(
    root_key: &ExtendedPrivKey,
    network: Network,
) -> Vec<(Address, SecretKey)> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    let mut addresses = Vec::new();

    for i in 0..20u32 {
        // BIP-44 internal (change) chain: m/44'/{coin}'/0'/1/{i}
        let path_str = format!("m/44'/{}'/0'/1/{}", coin, i);
        let path = match DerivationPath::from_str(&path_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: could not parse change path {}: {}", path_str, e);
                continue;
            }
        };
        let child = match root_key.derive_priv(&secp, &path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Warning: could not derive change key {}: {}", i, e);
                continue;
            }
        };

        let priv_key = child.private_key;
        let pub_key = PublicKey::new(priv_key.public_key(&secp));
        let address = Address::p2pkh(&pub_key, network);
        addresses.push((address, priv_key));
    }

    addresses
}
