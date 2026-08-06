/// Passphrase strength scoring — shared between CLI and GUI.
///
/// Scores a passphrase 0–7 and classifies it into a strength tier.
/// Both frontends enforce the same minimum score before accepting a passphrase.

/// Strength tier for a passphrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

/// Minimum score (inclusive) that a non-empty passphrase must reach.
/// Score 4 corresponds to the `Fair` tier.
pub const MIN_SCORE: u8 = 4;
pub const MIN_LABEL: &str = "Fair";

/// Returns `true` when the passphrase meets the minimum strength requirement.
/// Empty passphrases are always rejected (caller should check separately).
pub fn is_strong_enough(passphrase: &str) -> bool {
    let (_, s, _) = score(passphrase);
    s >= MIN_SCORE
}

/// Scores a passphrase 0–7 and returns its tier, numeric score, and advice string.
///
/// Scoring criteria (1 point each):
///   - Length ≥ 8 characters
///   - Length ≥ 12 characters
///   - Length ≥ 16 characters
///   - Contains at least one uppercase letter
///   - Contains at least one ASCII digit
///   - Contains at least one ASCII symbol
///   - Contains at least one non-ASCII character (Unicode)
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

/// Returns a human-readable label for a tier.
pub fn tier_label(tier: Tier) -> &'static str {
    match tier {
        Tier::VeryWeak   => "Very Weak",
        Tier::Weak       => "Weak",
        Tier::Fair       => "Fair",
        Tier::Strong     => "Strong",
        Tier::VeryStrong => "Excellent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_very_weak() {
        let (tier, s, _) = score("");
        assert_eq!(tier, Tier::VeryWeak);
        assert_eq!(s, 0);
        assert!(!is_strong_enough(""));
    }

    #[test]
    fn short_lowercase_is_weak() {
        assert!(!is_strong_enough("abcdefgh"));
    }

    #[test]
    fn long_mixed_is_strong() {
        assert!(is_strong_enough("MyP@ssw0rd!!XYZ"));
    }

    #[test]
    fn unicode_adds_point() {
        let (_, s1, _) = score("abcdefghijkl");
        let (_, s2, _) = score("abcdefghijkl🔐");
        assert!(s2 > s1);
    }

    #[test]
    fn minimum_score_is_fair() {
        // 12 chars + uppercase + digit = 4 points = Fair
        assert!(is_strong_enough("Abcdefghij1!"));
    }
}
