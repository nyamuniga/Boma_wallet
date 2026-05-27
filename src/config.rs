use bitcoin::network::constants::Network;
use std::fs;
use std::io::Write;

const CONFIG_FILE: &str = "wallet_config.txt";

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
    pub fn load() -> Self {
        let mut cfg = Config::default();
        if let Ok(contents) = fs::read_to_string(CONFIG_FILE) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
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
        }
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let network_str = if self.network == Network::Bitcoin { "mainnet" } else { "testnet" };
        let contents = format!(
            "# BOMA Cold Wallet Configuration\nnetwork={}\nsession_timeout_secs={}\n",
            network_str, self.session_timeout_secs
        );
        let mut f = fs::File::create(CONFIG_FILE)
            .map_err(|e| format!("Failed to save config: {}", e))?;
        f.write_all(contents.as_bytes())
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    pub fn network_label(&self) -> &'static str {
        if self.network == Network::Bitcoin { "Mainnet ₿" } else { "Testnet ₿" }
    }

    /// BIP-44 coin type (0 = mainnet, 1 = testnet)
    #[allow(dead_code)]
    pub fn coin_type(&self) -> u32 {
        if self.network == Network::Bitcoin { 0 } else { 1 }
    }
}
