use sha2::{Sha256, Digest};
use std::fs::{OpenOptions, File};
use std::io::{Write, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE: &str = "wallet_audit.log";

/// Well-known genesis hash for the first entry in the hash chain.
/// This is SHA-256("BOMA_AUDIT_LOG_GENESIS").
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Appends a timestamped, hash-chained entry to wallet_audit.log.
///
/// Each entry includes the SHA-256 hash of the previous entry, forming a
/// tamper-evident chain. Deleting or modifying any entry breaks the chain,
/// making post-hoc manipulation detectable.
///
/// Format: [timestamp] prev_hash action
///
/// The log contains action names only — never any key material.
pub struct AuditLog;

impl AuditLog {
    pub fn new() -> Self { AuditLog }

    pub fn log(&self, action: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Read the hash of the last entry (or use genesis)
        let prev_hash = Self::last_entry_hash().unwrap_or_else(|| GENESIS_HASH.to_string());

        // Format the new entry
        let entry = format!("[{}] {} {}", ts, prev_hash, action);

        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
            let _ = writeln!(f, "{}", entry);
        }
    }

    /// Reads the last line of the audit log and computes its SHA-256 hash.
    /// Returns `None` if the log doesn't exist or is empty.
    fn last_entry_hash() -> Option<String> {
        let file = File::open(LOG_FILE).ok()?;
        let reader = BufReader::new(file);
        let mut last_line: Option<String> = None;

        for line in reader.lines() {
            if let Ok(l) = line {
                let trimmed = l.trim().to_string();
                if !trimmed.is_empty() {
                    last_line = Some(trimmed);
                }
            }
        }

        last_line.map(|l| hex::encode(Sha256::digest(l.as_bytes())))
    }

    /// Verifies the integrity of the entire audit log hash chain.
    ///
    /// Returns `Ok(entry_count)` if the chain is intact.
    /// Returns `Err(description)` if any entry has been tampered with.
    #[allow(dead_code)]
    pub fn verify_chain() -> Result<usize, String> {
        let file = File::open(LOG_FILE)
            .map_err(|_| "Audit log not found.".to_string())?;
        let reader = BufReader::new(file);

        let mut prev_hash = GENESIS_HASH.to_string();
        let mut count = 0usize;

        for (lineno, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Read error at line {}: {}", lineno + 1, e))?;
            let line = line.trim().to_string();
            if line.is_empty() { continue; }

            // Parse: [timestamp] hash action
            // Find the hash field (second space-separated token)
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 3 {
                return Err(format!("Malformed entry at line {}", lineno + 1));
            }

            let recorded_hash = parts[1];
            if recorded_hash != prev_hash {
                return Err(format!(
                    "Hash chain broken at line {} — log has been tampered with.", lineno + 1
                ));
            }

            // This entry's hash becomes the next entry's prev_hash
            prev_hash = hex::encode(Sha256::digest(line.as_bytes()));
            count += 1;
        }

        Ok(count)
    }
}
