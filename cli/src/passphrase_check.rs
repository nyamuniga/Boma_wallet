use crate::ui;

// Re-export constants from the core module for backward compatibility
pub use boma_core::passphrase_strength::{MIN_SCORE, MIN_LABEL};

/// Returns `true` when the passphrase meets the minimum strength requirement.
/// Empty passphrases are rejected before this function is reached.
pub fn is_strong_enough(passphrase: &str) -> bool {
    boma_core::passphrase_strength::is_strong_enough(passphrase)
}

/// Prints a colour-coded strength bar and advice line.
pub fn display(passphrase: &str) {
    if passphrase.is_empty() {
        ui::warn("No passphrase set. Mnemonic alone protects your funds.");
        return;
    }
    let (tier, s, advice) = boma_core::passphrase_strength::score(passphrase);
    let label = boma_core::passphrase_strength::tier_label(tier);
    let max = 7usize;
    let bar_len = 24usize;
    let filled = ((s as usize) * bar_len / max).min(bar_len);

    let color = match tier {
        boma_core::passphrase_strength::Tier::VeryWeak   => ui::RED,
        boma_core::passphrase_strength::Tier::Weak       => ui::YELLOW,
        boma_core::passphrase_strength::Tier::Fair       => ui::YELLOW,
        boma_core::passphrase_strength::Tier::Strong     => ui::GREEN,
        boma_core::passphrase_strength::Tier::VeryStrong => ui::GREEN,
    };

    println!(
        "  Strength  {}{}{}{}{}  {}{}{}",
        color,
        "█".repeat(filled),
        ui::RESET,
        ui::DIM,
        "░".repeat(bar_len - filled),
        ui::RESET,
        color, label
    );
    println!("  {}{}  {}{}", ui::DIM, ui::CYAN, advice, ui::RESET);
}
