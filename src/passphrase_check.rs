use crate::ui;

/// Strength tier for a passphrase.
pub enum Tier { VeryWeak, Weak, Fair, Strong, VeryStrong }

/// Scores a passphrase 0–7 and returns its tier and advice.
pub fn score(passphrase: &str) -> (Tier, u8, &'static str) {
    let mut s: u8 = 0;
    let n = passphrase.len();

    if n >= 8  { s += 1; }
    if n >= 12 { s += 1; }
    if n >= 16 { s += 1; }
    if passphrase.chars().any(|c| c.is_uppercase())           { s += 1; }
    if passphrase.chars().any(|c| c.is_ascii_digit())         { s += 1; }
    if passphrase.chars().any(|c| "!@#$%^&*()-_=+[]{}|;:',.<>/?`~".contains(c)) { s += 1; }
    if passphrase.chars().any(|c| (c as u32) > 127)           { s += 1; }

    let (tier, advice) = match s {
        0..=1 => (Tier::VeryWeak,  "Too short. Use at least 12 characters."),
        2..=3 => (Tier::Weak,      "Add uppercase letters, numbers, or symbols."),
        4     => (Tier::Fair,      "Good start — try making it longer."),
        5..=6 => (Tier::Strong,    "Strong passphrase!"),
        _     => (Tier::VeryStrong,"Excellent passphrase!"),
    };
    (tier, s, advice)
}

/// Prints a colour-coded strength bar and advice line.
pub fn display(passphrase: &str) {
    if passphrase.is_empty() {
        ui::warn("No passphrase set. Mnemonic alone protects your funds.");
        return;
    }
    let (tier, s, advice) = score(passphrase);
    let max = 7usize;
    let bar_len = 24usize;
    let filled = ((s as usize) * bar_len / max).min(bar_len);

    let (color, label) = match tier {
        Tier::VeryWeak   => (ui::RED,    "Very Weak"),
        Tier::Weak       => (ui::YELLOW, "Weak     "),
        Tier::Fair       => (ui::YELLOW, "Fair     "),
        Tier::Strong     => (ui::GREEN,  "Strong   "),
        Tier::VeryStrong => (ui::GREEN,  "Excellent"),
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
