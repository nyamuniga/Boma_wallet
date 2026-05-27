use bitcoin::network::constants::Network;
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ExtendedPrivKey;

use crate::address_derivation::derive_address_range;

/// Generates 20 external receive addresses on BIP-84 path m/84'/{coin}'/0'/0/{i}.
pub fn generate_many_addresses(
    root_key: &ExtendedPrivKey,
    network: Network,
) -> Vec<(Address, SecretKey)> {
    derive_address_range(root_key, network, 0, 20)
}