use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

/// Picks a random address from the in-memory derived address list.
///
/// Addresses are no longer read from the backup file (which is now encrypted and
/// only stores the mnemonic). They are re-derived from the seed at login and held
/// in memory for the duration of the session.
pub fn get_random_address(addresses: &[(Address, SecretKey)]) -> Result<String, String> {
    if addresses.is_empty() {
        return Err("No addresses available.".to_string());
    }
    let mut rng = OsRng;
    let (address, _) = addresses
        .choose(&mut rng)
        .ok_or_else(|| "Failed to select a random address.".to_string())?;
    Ok(address.to_string())
}