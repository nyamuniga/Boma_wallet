use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

/// Derives the master root key and the first BIP-84 receive address.
///
/// Derivation path: m/84'/{coin}'/0'/0/0
/// - coin = 0 for mainnet, 1 for testnet
pub fn derive_keys(
    seed: &[u8],
    network: Network,
) -> Result<(ExtendedPrivKey, SecretKey, bitcoin::PublicKey, Address), String> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };

    let root_key = ExtendedPrivKey::new_master(network, seed)
        .map_err(|e| format!("Failed to create master key: {}", e))?;

    let path = DerivationPath::from_str(&format!("m/84'/{}'/0'/0/0", coin))
        .map_err(|e| format!("Invalid derivation path: {}", e))?;

    let child = root_key
        .derive_priv(&secp, &path)
        .map_err(|e| format!("Failed to derive child key: {}", e))?;

    let priv_key = child.private_key;
    let pub_key = PublicKey::new(priv_key.public_key(&secp));
    let address = Address::p2wpkh(&pub_key, network)
        .map_err(|e| format!("Failed to create P2WPKH address: {}", e))?;

    Ok((root_key, priv_key, pub_key, address))
}