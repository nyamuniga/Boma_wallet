use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ExtendedPrivKey;

use crate::audit_log::AuditLog;
use boma_core::config::Config;
use boma_core::transaction::Utxo;

/// Encapsulates all data required for an active wallet session,
/// preventing functions from having too many arguments (Clean Code).
pub struct SessionState<'a> {
    pub mnemonic_str: String,
    pub receive_addresses: Vec<(Address, SecretKey)>,
    pub change_addresses: Vec<(Address, SecretKey)>,
    pub root_key: ExtendedPrivKey,
    pub fingerprint: String,
    pub cfg: &'a Config,
    pub audit: &'a AuditLog,
    pub preloaded_utxos: Vec<Utxo>,
}
