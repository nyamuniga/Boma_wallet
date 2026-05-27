use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use crate::ui;

/// Prints all derived receive addresses with their indices.
pub fn print_addresses(label: &str, addresses: &[(Address, SecretKey)]) {
    ui::section(label);
    if addresses.is_empty() {
        ui::warn("No addresses found.");
        return;
    }
    for (i, (addr, _)) in addresses.iter().enumerate() {
        println!("  {}[{:>2}]{}  {}", ui::ORANGE, i, ui::RESET, addr);
    }
}