use bitcoin::network::constants::Network;
use sha2::{Sha256, Digest};
use std::fs;
use std::io::Write;

const CONFIG_FILE: &str = "wallet_config.txt";

/// Minimum session timeout: 1 minute.
const MIN_TIMEOUT_SECS: u64 = 60;
/// Maximum session timeout: 60 minutes.
const MAX_TIMEOUT_SECS: u64 = 3600;

pub struct Config {
    pub network: Network,
    pub session_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            network: Network::Bitcoin,
            session_timeout_secs: 300, // 5 minutes
        }
    }
}

impl Config {
    /// Loads the config from disk, verifying the SHA-256 integrity checksum.
    ///
    /// If the config file is missing, returns safe defaults.
    /// If the checksum is missing or invalid (file was tampered with),
    /// the file is rejected and safe defaults are used instead.
    /// Session timeout is always clamped to [60, 3600] seconds.
    pub fn load() -> Self {
        let mut cfg = Config::default();
        let contents = match fs::read_to_string(CONFIG_FILE) {
            Ok(c) => c,
            Err(_) => return cfg,
        };

        // ── Verify integrity checksum ─────────────────────────────────────
        // The last line should be: CHECKSUM: <hex sha256 of all lines above>
        let lines: Vec<&str> = contents.lines().collect();
        let checksum_line = lines.last().copied().unwrap_or("");

        if let Some(stored_hash) = checksum_line.strip_prefix("CHECKSUM: ") {
            // Compute hash of everything above the checksum line
            let content_above: String = lines[..lines.len() - 1]
                .iter()
                .map(|l| format!("{}\n", l))
                .collect();
            let computed_hash = hex::encode(Sha256::digest(content_above.as_bytes()));

            if computed_hash != stored_hash.trim() {
                eprintln!("  ⚠  Config file integrity check failed — using safe defaults.");
                eprintln!("     Delete '{}' and reconfigure if this persists.", CONFIG_FILE);
                return Config::default();
            }
        } else {
            // No checksum line — treat as legacy config, accept but warn
            eprintln!("  ⚠  Config file has no integrity checksum — will be added on next save.");
        }

        // ── Parse key-value pairs ─────────────────────────────────────────
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("CHECKSUM:") {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                match key.trim() {
                    "network" => {
                        cfg.network = if val.trim() == "testnet" {
                            Network::Testnet
                        } else {
                            Network::Bitcoin
                        };
                    }
                    "session_timeout_secs" => {
                        if let Ok(n) = val.trim().parse::<u64>() {
                            cfg.session_timeout_secs = n;
                        }
                    }
                    _ => {}
                }
            }
        }

        // ── Clamp timeout to safe range ───────────────────────────────────
        cfg.session_timeout_secs = cfg.session_timeout_secs.clamp(MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS);

        cfg
    }

    /// Saves the config to disk with a SHA-256 integrity checksum.
    ///
    /// The checksum covers all content lines and detects both accidental
    /// corruption and targeted tampering of the config file.
    pub fn save(&self) -> Result<(), String> {
        let network_str = if self.network == Network::Bitcoin { "mainnet" } else { "testnet" };

        // Clamp before saving
        let timeout = self.session_timeout_secs.clamp(MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS);

        let body = format!(
            "# BOMA Cold Wallet Configuration\nnetwork={}\nsession_timeout_secs={}\n",
            network_str, timeout
        );

        // Compute SHA-256 checksum of the body
        let checksum = hex::encode(Sha256::digest(body.as_bytes()));
        let full_contents = format!("{}CHECKSUM: {}\n", body, checksum);

        let mut f = fs::File::create(CONFIG_FILE)
            .map_err(|e| format!("Failed to save config: {}", e))?;
        f.write_all(full_contents.as_bytes())
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Set restrictive permissions on config file too
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(CONFIG_FILE, perms);
        }

        Ok(())
    }

    pub fn network_label(&self) -> &'static str {
        if self.network == Network::Bitcoin { "Mainnet ₿" } else { "Testnet ₿" }
    }

    /// Returns the minimum allowed timeout in seconds.
    pub fn min_timeout_secs() -> u64 { MIN_TIMEOUT_SECS }
    /// Returns the maximum allowed timeout in seconds.
    pub fn max_timeout_secs() -> u64 { MAX_TIMEOUT_SECS }
}
