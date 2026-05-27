
use bitcoin::util::bip32::ExtendedPrivKey;
use std::str::FromStr;
use std::str;

#[allow(dead_code)]
pub fn backup_master_key(root_key: &ExtendedPrivKey) -> String {
    root_key.to_string()
}

#[allow(dead_code)]
pub fn restore_master_key(backup: &str) -> ExtendedPrivKey {
    ExtendedPrivKey::from_str(backup).unwrap()
}