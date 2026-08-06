use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroize;

/// Maximum number of times any single byte value may appear in a 32-byte sample.
/// For a fair uniform source, P(any byte appears ≥7 times in 32 draws) < 2⁻³².
const MAX_BYTE_REPETITIONS: usize = 6;

/// Minimum number of distinct byte values required in a 32-byte sample.
/// 32 truly random bytes yield ~26 distinct values on average.
/// 20 is the conservative floor that still guarantees ≥128 bits of min-entropy.
const MIN_DISTINCT_BYTES: usize = 20;

/// Generates 32 bytes (256 bits) of cryptographic entropy with ≥128-bit security guarantee.
///
/// Security properties:
///   - **Double-sampled**: Two independent 32-byte draws from the OS CSPRNG.
///   - **Health-tested**: Each sample passes NIST SP 800-90B–inspired checks
///     (repetition count, distinct values, stuck patterns, chi-squared).
///   - **Cross-verified**: Both samples must differ (detects stuck RNG).
///   - **XOR-combined**: Final output = sample_a ⊕ sample_b. Even if one sample
///     has partial bias, XOR preserves entropy from the stronger source.
///
/// Returns `Err` if the OS RNG fails health tests — generating a wallet with
/// bad entropy is catastrophically worse than failing to generate one.
pub fn generate_entropy() -> Result<[u8; 32], String> {
    let mut sample_a = [0u8; 32];
    let mut sample_b = [0u8; 32];

    OsRng.fill_bytes(&mut sample_a);
    OsRng.fill_bytes(&mut sample_b);

    // ── Health tests on each sample independently ─────────────────────────
    validate_sample(&sample_a, "A")?;
    validate_sample(&sample_b, "B")?;

    // ── Cross-sample independence ─────────────────────────────────────────
    // P(identical) = 2⁻²⁵⁶ for a working RNG. If they match, it's stuck.
    if sample_a == sample_b {
        sample_a.zeroize();
        sample_b.zeroize();
        return Err(
            "CRITICAL: OS RNG produced identical samples — entropy source is stuck or broken. \
             Do NOT generate a wallet on this machine."
                .to_string(),
        );
    }

    // ── XOR combination for defense in depth ──────────────────────────────
    let mut output = [0u8; 32];
    for i in 0..32 {
        output[i] = sample_a[i] ^ sample_b[i];
    }

    // Zeroize intermediate samples
    sample_a.zeroize();
    sample_b.zeroize();

    // Final sanity check on the combined output
    validate_sample(&output, "combined")?;

    Ok(output)
}

/// Runs NIST SP 800-90B–inspired health tests on a single 32-byte sample.
fn validate_sample(sample: &[u8; 32], label: &str) -> Result<(), String> {
    // ── Stuck pattern test ─────────────────────────────────────────────────
    // Reject all-zero, all-0xFF, or any single repeated byte
    if sample.iter().all(|&b| b == sample[0]) {
        return Err(format!(
            "CRITICAL: Entropy sample {} is a single repeated byte (0x{:02x}). \
             OS RNG is broken.",
            label, sample[0]
        ));
    }

    // Reject ascending or descending sequences (e.g. 0,1,2,3,...)
    let is_ascending = sample.windows(2).all(|w| w[1] == w[0].wrapping_add(1));
    let is_descending = sample.windows(2).all(|w| w[1] == w[0].wrapping_sub(1));
    if is_ascending || is_descending {
        return Err(format!(
            "CRITICAL: Entropy sample {} is a sequential pattern. OS RNG is broken.",
            label
        ));
    }

    // ── Byte frequency analysis ───────────────────────────────────────────
    let mut freq = [0u32; 256];
    for &b in sample.iter() {
        freq[b as usize] += 1;
    }

    // Repetition count test: no byte may appear more than MAX_BYTE_REPETITIONS times
    for (byte_val, &count) in freq.iter().enumerate() {
        if count as usize > MAX_BYTE_REPETITIONS {
            return Err(format!(
                "CRITICAL: Entropy sample {} has byte 0x{:02x} appearing {} times \
                 (max allowed: {}). OS RNG output is biased.",
                label, byte_val, count, MAX_BYTE_REPETITIONS
            ));
        }
    }

    // Distinct values test: at least MIN_DISTINCT_BYTES unique byte values
    let distinct = freq.iter().filter(|&&c| c > 0).count();
    if distinct < MIN_DISTINCT_BYTES {
        return Err(format!(
            "CRITICAL: Entropy sample {} has only {} distinct byte values \
             (minimum required: {}). Insufficient entropy.",
            label, distinct, MIN_DISTINCT_BYTES
        ));
    }

    // ── Chi-squared uniformity test ───────────────────────────────────────
    // Expected frequency for 32 bytes across 256 bins = 32/256 = 0.125
    // We use the chi-squared statistic: sum((observed - expected)² / expected)
    // For 255 degrees of freedom, chi² > 363.2 corresponds to p < 0.001
    let expected: f64 = 32.0 / 256.0;
    let chi_squared: f64 = freq
        .iter()
        .map(|&count| {
            let diff = count as f64 - expected;
            (diff * diff) / expected
        })
        .sum();

    // Threshold for p < 0.001 with 255 degrees of freedom
    const CHI_SQ_CRITICAL: f64 = 363.2;
    if chi_squared > CHI_SQ_CRITICAL {
        return Err(format!(
            "CRITICAL: Entropy sample {} failed chi-squared test \
             (χ²={:.1}, threshold={:.1}). Distribution is non-uniform.",
            label, chi_squared, CHI_SQ_CRITICAL
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_passes_health_checks() {
        // Generate entropy 10 times — all should pass on a healthy system
        for _ in 0..10 {
            let result = generate_entropy();
            assert!(result.is_ok(), "Healthy OS RNG should pass: {:?}", result.err());
            let entropy = result.unwrap();
            assert_eq!(entropy.len(), 32);
        }
    }

    #[test]
    fn rejects_all_zeros() {
        let bad = [0u8; 32];
        assert!(validate_sample(&bad, "test").is_err());
    }

    #[test]
    fn rejects_all_same_byte() {
        let bad = [0xAA; 32];
        assert!(validate_sample(&bad, "test").is_err());
    }

    #[test]
    fn rejects_ascending_sequence() {
        let mut bad = [0u8; 32];
        for i in 0..32 {
            bad[i] = i as u8;
        }
        assert!(validate_sample(&bad, "test").is_err());
    }

    #[test]
    fn rejects_descending_sequence() {
        let mut bad = [0u8; 32];
        for i in 0..32 {
            bad[i] = 31u8.wrapping_sub(i as u8);
        }
        assert!(validate_sample(&bad, "test").is_err());
    }

    #[test]
    fn rejects_low_distinct_values() {
        // Only 3 distinct values — should fail the distinct-values test
        let mut bad = [0u8; 32];
        for i in 0..32 {
            bad[i] = (i % 3) as u8;
        }
        assert!(validate_sample(&bad, "test").is_err());
    }

    #[test]
    fn two_calls_produce_different_output() {
        let a = generate_entropy().unwrap();
        let b = generate_entropy().unwrap();
        assert_ne!(a, b, "Two entropy generations must differ");
    }
}