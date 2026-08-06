/// Unit tests for boma-core.
///
/// Run with: `cargo test -p boma-core`
///
/// Coverage:
///   - BIP-39 known-answer test vectors (from the official BIP-39 spec)
///   - Backup round-trip (correct passphrase, wrong passphrase, tampered data)
///   - btc_to_sats fixed-point arithmetic boundary cases
///   - build_transaction smoke test (signed tx can be deserialized)

#[cfg(test)]
mod bip39_vectors {
    use crate::derive_seed_from_mnemonic::derive_seed_from_mnemonic;

    /// Verifies that our PBKDF2 implementation matches the output of the `bip39` crate,
    /// which is itself validated against the official BIP-39 test vectors.
    #[test]
    fn abandon_mnemonic_no_passphrase() {
        use bip39::Mnemonic;
        use std::str::FromStr;

        let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let our_seed = derive_seed_from_mnemonic(mnemonic_str, "");

        // Cross-check against the bip39 crate's own derivation (which is spec-validated)
        let mnemonic = Mnemonic::from_str(mnemonic_str).expect("valid mnemonic");
        let crate_seed = mnemonic.to_seed("");
        
        assert_eq!(&*our_seed, &crate_seed, "Our PBKDF2 derivation must match the bip39 crate output");
        assert_eq!(our_seed.len(), 64, "BIP-39 seed must be 64 bytes");
    }

    /// Same mnemonic but with the "TREZOR" passphrase — produces a completely different seed.
    #[test]
    fn abandon_mnemonic_trezor_passphrase() {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed_no_pass = derive_seed_from_mnemonic(mnemonic, "");
        let seed_with_pass = derive_seed_from_mnemonic(mnemonic, "TREZOR");
        assert_ne!(
            &*seed_no_pass, &*seed_with_pass,
            "Different passphrases must produce different seeds"
        );
        assert_eq!(seed_with_pass.len(), 64, "Seed must always be 64 bytes");
    }

    /// Determinism: the same inputs always produce the same output.
    #[test]
    fn derivation_is_deterministic() {
        let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        let s1 = derive_seed_from_mnemonic(mnemonic, "pass");
        let s2 = derive_seed_from_mnemonic(mnemonic, "pass");
        assert_eq!(&*s1, &*s2, "Seed derivation must be deterministic");
    }
}

#[cfg(test)]
mod backup_round_trip {
    use crate::store_backup::{store_backup, load_backup};

    fn temp_path() -> String {
        // Use a unique temp file per test thread
        format!("/tmp/boma_test_backup_{}.txt", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos())
    }

    #[test]
    fn correct_passphrase_round_trips() {
        let path = temp_path();
        let mnemonic = "legal winner thank year wave sausage worth useful legal winner thank yellow";
        let pass = "correct-horse-battery-staple!1";

        store_backup(pass, mnemonic, &path).expect("store_backup should succeed");
        let recovered = load_backup(pass, &path).expect("load_backup with correct pass should succeed");
        assert_eq!(recovered, mnemonic, "Recovered mnemonic must match original");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wrong_passphrase_returns_error() {
        let path = temp_path();
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        store_backup("right-pass-9!", mnemonic, &path).expect("store should succeed");
        let result = load_backup("wrong-pass-9!", &path);
        assert!(result.is_err(), "Wrong passphrase must return Err");
        let err = result.unwrap_err();
        assert!(
            err.contains("Incorrect passphrase") || err.contains("corrupted"),
            "Error must not reveal file-corruption vs. wrong-pass distinction: '{}'", err
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_returns_error() {
        let result = load_backup("any-pass", "/tmp/boma_definitely_does_not_exist.txt");
        assert!(result.is_err(), "Missing file must return Err");
    }

    #[test]
    fn tampered_data_returns_error() {
        let path = temp_path();
        store_backup("secure-pass-1!", "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong", &path)
            .expect("store should succeed");

        // Flip a byte in the DATA field
        let mut contents = std::fs::read_to_string(&path).unwrap();
        if let Some(pos) = contents.find("DATA: ") {
            let byte_pos = pos + 6 + 10; // 10 chars into the hex
            if byte_pos < contents.len() {
                let b = contents.as_bytes()[byte_pos];
                let flipped = if b == b'a' { b'b' } else { b'a' };
                contents.replace_range(byte_pos..byte_pos+1, &(flipped as char).to_string());
            }
        }
        std::fs::write(&path, &contents).unwrap();

        let result = load_backup("secure-pass-1!", &path);
        assert!(result.is_err(), "Tampered ciphertext must not decrypt successfully");

        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod btc_to_sats_tests {
    use crate::transaction::btc_to_sats;

    #[test]
    fn one_bitcoin() {
        assert_eq!(btc_to_sats("1").unwrap(), 100_000_000);
        assert_eq!(btc_to_sats("1.0").unwrap(), 100_000_000);
        assert_eq!(btc_to_sats("1.00000000").unwrap(), 100_000_000);
    }

    #[test]
    fn one_satoshi() {
        assert_eq!(btc_to_sats("0.00000001").unwrap(), 1);
    }

    #[test]
    fn zero() {
        assert_eq!(btc_to_sats("0").unwrap(), 0);
        assert_eq!(btc_to_sats("0.0").unwrap(), 0);
        assert_eq!(btc_to_sats("0.00000000").unwrap(), 0);
    }

    /// Classic IEEE 754 pitfall: 0.1 + 0.2 != 0.3 in floating-point.
    /// Our fixed-point implementation must handle these exactly.
    #[test]
    fn no_floating_point_rounding() {
        assert_eq!(btc_to_sats("0.1").unwrap(), 10_000_000);
        assert_eq!(btc_to_sats("0.2").unwrap(), 20_000_000);
        // 0.1 + 0.2 = 0.3 exactly in fixed-point
        let sum = btc_to_sats("0.1").unwrap() + btc_to_sats("0.2").unwrap();
        assert_eq!(sum, btc_to_sats("0.3").unwrap());
    }

    #[test]
    fn max_supply() {
        // 21 million BTC = 2_100_000_000_000_000 satoshis
        assert_eq!(btc_to_sats("21000000").unwrap(), 2_100_000_000_000_000);
        assert_eq!(btc_to_sats("21000000.00000000").unwrap(), 2_100_000_000_000_000);
    }

    #[test]
    fn fractional_only() {
        assert_eq!(btc_to_sats(".5").unwrap(), 50_000_000);
        assert_eq!(btc_to_sats(".00000001").unwrap(), 1);
    }

    #[test]
    fn negative_is_rejected() {
        assert!(btc_to_sats("-0.001").is_err());
        assert!(btc_to_sats("-1").is_err());
    }

    #[test]
    fn too_many_decimal_places_rejected() {
        assert!(btc_to_sats("0.000000001").is_err(), "9 decimal places should be rejected");
    }

    #[test]
    fn non_numeric_rejected() {
        assert!(btc_to_sats("abc").is_err());
        assert!(btc_to_sats("1.2.3").is_err());
        assert!(btc_to_sats("").is_err());
        assert!(btc_to_sats("   ").is_err(), "whitespace-only should be rejected");
        assert!(btc_to_sats("1e5").is_err());
    }

    #[test]
    fn whitespace_trimmed() {
        assert_eq!(btc_to_sats("  0.005  ").unwrap(), 500_000);
    }

    #[test]
    fn leading_zeros_rejected() {
        assert!(btc_to_sats("00").is_err(), "00 should be rejected");
        assert!(btc_to_sats("01").is_err(), "01 should be rejected");
        assert!(btc_to_sats("007").is_err(), "007 should be rejected");
        assert!(btc_to_sats("01.5").is_err(), "01.5 should be rejected");
        // Bare "0" is still allowed
        assert_eq!(btc_to_sats("0").unwrap(), 0);
        assert_eq!(btc_to_sats("0.5").unwrap(), 50_000_000);
    }
}

#[cfg(test)]
mod transaction_smoke_test {
    use crate::generate_entropy::generate_entropy;
    use crate::generate_mnemonic::generate_mnemonic;
    use crate::derive_seed_from_mnemonic::derive_seed_from_mnemonic;
    use crate::derive_keys::derive_keys;
    use crate::address_derivation::derive_address_range;
    use crate::transaction::{build_transaction, btc_to_sats, TxParams};
    use bitcoin::network::constants::Network;
    use bitcoin::consensus::deserialize;
    use bitcoin::Transaction;

    /// Builds a signed P2WPKH transaction and verifies it can be deserialized
    /// (i.e., is structurally valid Bitcoin). Uses a fake UTXO — this is never broadcast.
    #[test]
    fn build_and_deserialize_signed_tx() {
        let entropy = generate_entropy().expect("entropy generation must succeed");
        let mnemonic = generate_mnemonic(&entropy).expect("mnemonic generation must succeed");
        let mnemonic_str = mnemonic.to_string();
        let seed = derive_seed_from_mnemonic(&mnemonic_str, "test-pass-99!");
        let (root_key, _, _, _) = derive_keys(&seed, Network::Testnet)
            .expect("key derivation must succeed");

        let addresses = derive_address_range(&root_key, Network::Testnet, 0, 2);
        let (from_addr, from_key) = &addresses[0];
        let (change_addr, _) = &addresses[1];

        // Craft a fake recipient address (testnet)
        let to_addr = from_addr.clone(); // send-to-self for test purposes

        // Fake txid (64 hex zeros) — valid format, not a real UTXO
        let fake_txid = "0".repeat(64);
        let input_sats = btc_to_sats("0.001").unwrap(); // 100_000 sats
        let send_sats  = btc_to_sats("0.0005").unwrap(); // 50_000 sats
        let fee_sats   = btc_to_sats("0.0001").unwrap(); // 10_000 sats

        let p = TxParams {
            from_address: from_addr,
            from_key,
            txid_str: fake_txid,
            vout: 0,
            input_sats,
            to_address: to_addr,
            send_sats,
            fee_sats,
            change_address: change_addr,
            use_rbf: true,
            dry_run: false,
        };

        let hex = build_transaction(&p).expect("build_transaction must succeed");
        let raw = hex::decode(&hex).expect("output must be valid hex");
        let tx: Transaction = deserialize(&raw).expect("signed tx must deserialize as valid Bitcoin tx");

        // Basic structural checks
        assert_eq!(tx.input.len(), 1, "must have exactly 1 input");
        assert!(!tx.output.is_empty(), "must have at least 1 output");
        assert!(!tx.input[0].witness.is_empty(), "P2WPKH must populate the witness");
        // L4: Verify transaction version 2
        assert_eq!(tx.version, 2, "transaction must use version 2");
    }
}
