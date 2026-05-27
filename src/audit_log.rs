use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE: &str = "wallet_audit.log";

/// Appends a timestamped entry to wallet_audit.log.
/// The log contains action names only — never any key material.
pub struct AuditLog;

impl AuditLog {
    pub fn new() -> Self { AuditLog }

    pub fn log(&self, action: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
            let _ = writeln!(f, "[{ts}] {action}");
        }
    }
}
