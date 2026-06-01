# BOMA Cold Wallet — Codebase Walkthrough & Rust Learning Guide

Welcome to the **BOMA Cold Wallet** codebase documentation. This document is structured specifically to help you learn Rust by explaining what every file, function, and line of code is doing in detail.

The project is split into three main components:
1. **`core`**: The library containing core cryptographic logic, Bitcoin key derivation (BIP-32, BIP-39, BIP-84), backup encryption, transaction creation, and PSBT signing.
2. **`cli`**: A terminal-based user interface that walks the user through wallet actions.
3. **`gui`**: A Tauri-based desktop GUI shell that wraps the core library functions.

---

## Part 1: Core Library (`core/src/`)

The core library contains the core logic and does not depend on any UI framework.

### 1. `generate_entropy.rs`

This file is responsible for generating cryptographically secure random bytes (entropy) from the OS kernel, which will act as the source of randomness for creating seed phrases.

```rust
use rand::rngs::OsRng;
use rand::RngCore;

/// Generates 32 bytes (256 bits) of cryptographic entropy using the OS random source.
pub fn generate_entropy() -> [u8; 32] {
    let mut entropy = [0u8; 32];
    OsRng.fill_bytes(&mut entropy);
    entropy
}
```

#### Line-by-Line Explanation:

- **Line 1: `use rand::rngs::OsRng;`**
  Imports the `OsRng` struct from the `rand` crate. `OsRng` interfaces with the host operating system's secure entropy generator (such as `/dev/urandom` on Unix or `BCryptGenRandom` on Windows).
- **Line 2: `use rand::RngCore;`**
  Imports the `RngCore` trait (interface). In Rust, to use methods defined inside a trait (like `fill_bytes`), that trait must be brought into scope.
- **Line 5: `pub fn generate_entropy() -> [u8; 32] {`**
  Declares a **public** function named `generate_entropy`. It takes no arguments and returns a fixed-size array of 32 unsigned 8-bit integers (`[u8; 32]`). Fixed-size arrays reside on the stack rather than the heap, making them fast and easy to zeroize.
- **Line 6: `let mut entropy = [0u8; 32];`**
  Declares a mutable (`mut`) local variable named `entropy` and initializes it as a 32-byte array containing only zeros (`0u8` is a literal for a byte with value 0). Mutability is required because we need to modify this array's values on the next line.
- **Line 7: `OsRng.fill_bytes(&mut entropy);`**
  Calls the `fill_bytes` method of the `OsRng` generator. It passes a mutable reference (`&mut entropy`) to our array. Passing a reference allows the method to mutate our array directly without copying it.
- **Line 8: `entropy`**
  Returns the `entropy` array. In Rust, if the last line of a block does not end with a semicolon, it is treated as an expression and its value is returned.

---

### 2. `generate_mnemonic.rs`

This file converts raw entropy bytes into readable words (BIP-39 mnemonic seed phrase).

```rust
use bip39::Mnemonic;

/// Converts raw entropy bytes into a BIP-39 mnemonic phrase.
pub fn generate_mnemonic(entropy: &[u8]) -> Mnemonic {
    Mnemonic::from_entropy(entropy).expect("Failed to generate mnemonic from entropy")
}
```

#### Line-by-Line Explanation:

- **Line 1: `use bip39::Mnemonic;`**
  Imports the `Mnemonic` struct from the `bip39` crate.
- **Line 5: `pub fn generate_mnemonic(entropy: &[u8]) -> Mnemonic {`**
  Declares a public function that accepts a reference to a slice of bytes (`&[u8]`) and returns a `Mnemonic` struct. Slices are "views" into memory, allowing us to pass array segments of any size without copying data.
- **Line 6: `Mnemonic::from_entropy(entropy).expect("Failed to generate mnemonic from entropy")`**
  Calls the associated constructor function `from_entropy` on the `Mnemonic` struct. This checks the entropy length, calculates a SHA-256 checksum, appends it, and maps the resulting bits to words from the BIP-39 wordlist.
  It returns a `Result<Mnemonic, Error>`. The `.expect(...)` method unwraps this Result: if it's `Ok`, it returns the inner `Mnemonic`; if it's `Err`, it crashes (panics) the program immediately with the specified message.

---

### 3. `derive_seed_from_mnemonic.rs`

This file converts the 12- or 24-word recovery phrase (along with an optional passphrase) into a 512-bit seed using the standard PBKDF2 function.

```rust
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha512;
use zeroize::{Zeroize, Zeroizing};

pub fn derive_seed_from_mnemonic(mnemonic: &str, passphrase: &str) -> Vec<u8> {
    let mnemonic_bytes = mnemonic.as_bytes();

    let mut salt = format!("mnemonic{}", passphrase);
    let salt_bytes = salt.as_bytes();

    let mut seed = Zeroizing::new(vec![0u8; 64]);
    pbkdf2::<Hmac<Sha512>>(mnemonic_bytes, salt_bytes, 2048, &mut seed);

    salt.zeroize();
    
    let out = seed.to_vec();
    out
}
```

#### Line-by-Line Explanation:

- **Line 1: `use hmac::Hmac;`**
  Imports the Hash-based Message Authentication Code implementation, required by the key derivation function PBKDF2.
- **Line 2: `use pbkdf2::pbkdf2;`**
  Imports the `pbkdf2` function which performs key stretching.
- **Line 3: `use sha2::Sha512;`**
  Imports the SHA-512 hashing algorithm.
- **Line 4: `use zeroize::{Zeroize, Zeroizing};`**
  Imports memory zeroization tools. This is a critical security practice in crypto wallets: we overwrite sensitive data in RAM with zeros once we are done, ensuring it cannot be read from memory dumps or recovered by other processes.
- **Line 6: `pub fn derive_seed_from_mnemonic(mnemonic: &str, passphrase: &str) -> Vec<u8> {`**
  Takes references to the mnemonic string slice and passphrase string slice and returns a dynamic byte vector (`Vec<u8>`).
- **Line 7: `let mnemonic_bytes = mnemonic.as_bytes();`**
  Converts the string slice `&str` into a byte slice `&[u8]`. PBKDF2 works on raw bytes.
- **Line 9: `let mut salt = format!("mnemonic{}", passphrase);`**
  Constructs a new string using the `format!` macro. The BIP-39 standard defines the PBKDF2 salt as the string literal `"mnemonic"` concatenated with the optional user passphrase.
- **Line 10: `let salt_bytes = salt.as_bytes();`**
  Gets the byte slice of the salt.
- **Line 13: `let mut seed = Zeroizing::new(vec![0u8; 64]);`**
  Initializes a 64-byte (512 bits) vector filled with zeros. We wrap it inside the `Zeroizing` smart pointer. When `seed` falls out of scope, `Zeroizing` automatically overrides the underlying vector memory with zeros before deallocating it.
- **Line 14: `pbkdf2::<Hmac<Sha512>>(mnemonic_bytes, salt_bytes, 2048, &mut seed);`**
  Runs the key derivation. The turbofish syntax `::<Hmac<Sha512>>` specifies that PBKDF2 should use HMAC-SHA512. It hashes the mnemonic bytes using the salt bytes over 2048 rounds (iterations), storing the resulting 64 bytes into `seed`.
- **Line 16: `salt.zeroize();`**
  Explicitly zeroizes the local salt variable since it contains the passphrase.
- **Line 19: `let out = seed.to_vec();`**
  Clones the derived seed into a normal `Vec<u8>` to return to the caller.
- **Line 20: `out`**
  Implicitly returns the seed.

---

### 4. `derive_keys.rs`

This file creates a BIP-32 root private key and derives key components for the very first wallet address.

```rust
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

pub fn derive_keys(
    seed: &[u8],
    network: Network,
) -> Result<(ExtendedPrivKey, SecretKey, bitcoin::PublicKey, Address), String> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };

    let root_key = ExtendedPrivKey::new_master(network, seed)
        .map_err(|e| format!("Failed to create master key: {}", e))?;

    let path = DerivationPath::from_str(&format!("m/84'/{}'/0'/0/0", coin))
        .map_err(|e| format!("Invalid derivation path: {}", e))?;

    let child = root_key
        .derive_priv(&secp, &path)
        .map_err(|e| format!("Failed to derive child key: {}", e))?;

    let priv_key = child.private_key;
    let pub_key = PublicKey::new(priv_key.public_key(&secp));
    let address = Address::p2wpkh(&pub_key, network)
        .map_err(|e| format!("Failed to create P2WPKH address: {}", e))?;

    Ok((root_key, priv_key, pub_key, address))
}
```

#### Line-by-Line Explanation:

- **Line 12: `pub fn derive_keys(...) -> Result<..., String> {`**
  Defines a function that returns a `Result`. If derivation is successful, it returns `Ok` containing a tuple with:
  1. `ExtendedPrivKey`: The root master extended private key (xprv).
  2. `SecretKey`: The private key of the first address.
  3. `bitcoin::PublicKey`: The public key of the first address.
  4. `Address`: The parsed P2WPKH Bech32 address.
  If any step fails, it returns an `Err(String)`.
- **Line 16: `let secp = Secp256k1::new();`**
  Creates a new elliptic curve context instance of the `secp256k1` library, used to perform ECDSA mathematical operations (like public key calculation).
- **Line 17: `let coin = if network == Network::Bitcoin { 0 } else { 1 };`**
  If the network is Mainnet, the coin index is `0`. If it's Testnet, it is `1` (according to BIP-44 rules).
- **Line 19: `let root_key = ExtendedPrivKey::new_master(network, seed)`**
  Generates the master extended private key (xprv) using HMAC-SHA512 on the seed.
- **Line 20: `.map_err(|e| format!("Failed to create master key: {}", e))?;`**
  `.map_err` converts a BIP-32 specific error into a user-friendly `String`. The trailing `?` operator is a Rust feature: if the result is `Ok(x)`, it binds `x` to `root_key`; if the result is `Err(e)`, the function returns early from this line, passing the error up.
- **Line 22: `let path = DerivationPath::from_str(&format!("m/84'/{}'/0'/0/0", coin))`**
  Parses the derivation path string. BIP-84 specifies native SegWit (Bech32) addresses under the format: `m / 84' / coin_type' / account' / change / address_index`. Here we derive the first receive address: index `0/0`.
- **Line 23: `.map_err(|e| format!("Invalid derivation path: {}", e))?;`**
  Converts parsing errors to `String` and uses `?` to handle failures.
- **Line 25: `let child = root_key.derive_priv(&secp, &path)`**
  Derives the child extended private key at the parsed path.
- **Line 29: `let priv_key = child.private_key;`**
  Extracts the raw 32-byte secret private key from the child key structure.
- **Line 30: `let pub_key = PublicKey::new(priv_key.public_key(&secp));`**
  Computes the public key corresponding to `priv_key` on the Secp256k1 curve and wraps it in the `bitcoin::PublicKey` type.
- **Line 31: `let address = Address::p2wpkh(&pub_key, network)`**
  Creates a Pay-to-Witness-Public-Key-Hash (native SegWit) address from the public key.
- **Line 34: `Ok((root_key, priv_key, pub_key, address))`**
  Returns the tuple packed in `Ok`.

---

### 5. `address_derivation.rs`

This file is a shared helper to derive a sequential list of addresses (e.g. 20 receive addresses or 20 change addresses) from the root master key.

```rust
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::PublicKey;
use std::str::FromStr;

pub fn derive_address_range(
    root_key: &ExtendedPrivKey,
    network: Network,
    chain_index: u32,
    count: u32,
) -> Vec<(Address, SecretKey)> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    let mut addresses = Vec::new();

    for i in 0..count {
        let path_str = format!("m/84'/{}'/0'/{}/{}", coin, chain_index, i);
        let path = match DerivationPath::from_str(&path_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: could not parse path {}: {}", path_str, e);
                continue;
            }
        };
        let child = match root_key.derive_priv(&secp, &path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("Warning: could not derive key at index {}: {}", i, e);
                continue;
            }
        };

        let priv_key = child.private_key;
        let pub_key = PublicKey::new(priv_key.public_key(&secp));
        let address = match Address::p2wpkh(&pub_key, network) {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Warning: could not create P2WPKH address at index {}: {}", i, e);
                continue;
            }
        };
        addresses.push((address, priv_key));
    }

    addresses
}
```

#### Line-by-Line Explanation:

- **Line 11: `pub fn derive_address_range(...) -> Vec<(Address, SecretKey)> {`**
  Takes references to the root master key, the bitcoin network type, a `chain_index` (0 for external/receive, 1 for internal/change), and `count` (number of addresses to generate). It returns a vector of tuples containing address and its corresponding private key.
- **Line 17: `let secp = Secp256k1::new();`**
  Initializes the Secp256k1 context.
- **Line 18: `let coin = if network == Network::Bitcoin { 0 } else { 1 };`**
  BIP-44 coin type variable.
- **Line 19: `let mut addresses = Vec::new();`**
  Creates an empty vector to store our generated address/key pairs.
- **Line 21: `for i in 0..count {`**
  A loop that runs from index `0` up to `count - 1`.
- **Line 22: `let path_str = format!("m/84'/{}'/0'/{}/{}", coin, chain_index, i);`**
  Creates the hierarchical deterministic path string. E.g. index 3 receive on testnet is `m/84'/1'/0'/0/3`.
- **Line 23: `let path = match DerivationPath::from_str(&path_str) { ... }`**
  Attempts to parse the path string. We use `match` (pattern matching) to handle errors.
- **Lines 24–28: `Ok(p) => p, Err(e) => { ... continue; }`**
  If parsing returns `Ok`, we extract the path `p`. If it returns `Err`, we log a warning to `stderr` and execute `continue` to skip this index and proceed to the next iteration.
- **Lines 30–36: `let child = match root_key.derive_priv(&secp, &path) { ... }`**
  Derives the child private key at the parsed path, using pattern matching to skip failure indexes.
- **Line 38: `let priv_key = child.private_key;`**
  Extracts the private key.
- **Line 39: `let pub_key = PublicKey::new(priv_key.public_key(&secp));`**
  Derives the public key.
- **Lines 40–46: `let address = match Address::p2wpkh(&pub_key, network) { ... }`**
  Creates the SegWit address, gracefully bypassing any index formatting errors.
- **Line 47: `addresses.push((address, priv_key));`**
  Appends the tuple of `(address, priv_key)` to our vector.
- **Line 50: `addresses`**
  Returns the completed vector.

---

### 6. `change_addresses.rs` & 7. `generate_many_addresses.rs`

These files wrap `derive_address_range` for standard receive and change addresses.

```rust
// change_addresses.rs
pub fn generate_change_addresses(
    root_key: &ExtendedPrivKey,
    network: Network,
) -> Vec<(Address, SecretKey)> {
    derive_address_range(root_key, network, 1, 20)
}
```

```rust
// generate_many_addresses.rs
pub fn generate_many_addresses(
    root_key: &ExtendedPrivKey,
    network: Network,
) -> Vec<(Address, SecretKey)> {
    derive_address_range(root_key, network, 0, 20)
}
```

#### Line-by-Line Explanation:
These functions call `derive_address_range` requesting exactly 20 addresses:
- `generate_change_addresses` requests chain index `1` (internal change addresses).
- `generate_many_addresses` requests chain index `0` (external receive addresses).

---

### 8. `get_random_address.rs`

This file is used to get a single random address from our generated list. This is useful for displaying a single address to the user to receive funds without exposing their entire list.

```rust
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::address::Address;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;

pub fn get_random_address(addresses: &[(Address, SecretKey)]) -> Result<String, String> {
    if addresses.is_empty() {
        return Err("No addresses available.".to_string());
    }
    let mut rng = OsRng;
    let (address, _) = addresses
        .choose(&mut rng)
        .ok_or_else(|| "Failed to select a random address.".to_string())?;
    Ok(address.to_string())
}
```

#### Line-by-Line Explanation:
- **Line 11: `pub fn get_random_address(addresses: &[(Address, SecretKey)]) -> Result<String, String> {`**
  Takes a reference to a slice of address/private key tuples and returns a `Result` containing the random address string or an error string.
- **Lines 12–14: `if addresses.is_empty() { return Err(...); }`**
  Guard check: if the list is empty, returns an early error to prevent panic.
- **Line 15: `let mut rng = OsRng;`**
  Creates an instance of the OS-level secure random number generator.
- **Line 16: `let (address, _) = addresses`**
  Starts a pattern match to extract the address part of the chosen tuple (ignoring the private key using `_`).
- **Line 17: `.choose(&mut rng)`**
  Chooses a random element from the slice. `choose` is a helper method on slices provided by the `SliceRandom` trait. It returns an `Option` (`Some(&element)` or `None`).
- **Line 18: `.ok_or_else(|| "Failed to select a random address.".to_string())?;`**
  Converts the `Option` into a `Result`. If it's `Some(x)`, it returns `Ok(x)`; if it's `None`, it evaluates the closure to produce an `Err`. The trailing `?` unwraps the chosen element or returns the error early.
- **Line 19: `Ok(address.to_string())`**
  Returns the address formatted as a String wrapper in `Ok`.

---

### 9. `config.rs`

This handles parsing and writing configuration settings (like active Network and Session Timeout) to the file `wallet_config.txt`.

```rust
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
}
```

#### Line-by-Line Explanation:
- **Line 7: `pub struct Config { ... }`**
  Defines a public struct grouping configuration fields: `network` (Mainnet/Testnet) and `session_timeout_secs` (how long the wallet can remain idle in CLI).
- **Lines 12–19: `impl Default for Config { ... }`**
  Implements the standard `Default` trait. This allows creating a default configuration using `Config::default()`.
- **Line 22: `pub fn load() -> Self {`**
  Loads settings from disk. If the file is missing, it returns the defaults.
- **Line 24: `if let Ok(contents) = fs::read_to_string(CONFIG_FILE) {`**
  Uses `if let` pattern matching. If `fs::read_to_string` successfully reads the file, it binds the contents to `contents` and executes the block. If not (e.g. file doesn't exist), it skips the block, falling back to default values.
- **Line 25: `for line in contents.lines() {`**
  Loops through each line of the configuration file.
- **Line 27: `if line.is_empty() || line.starts_with('#') { continue; }`**
  Skips blank lines and comments.
- **Line 28: `if let Some((key, val)) = line.split_once('=') {`**
  Splits the line on the `=` sign. E.g. `network=mainnet` becomes key `"network"`, value `"mainnet"`.
- **Lines 29–43: `match key.trim() { ... }`**
  Uses pattern matching to parse the values.
- **Line 38: `if let Ok(n) = val.trim().parse::<u64>() {`**
  Attempts to parse the string value into an unsigned 64-bit integer (`u64`). If parsing succeeds, it assigns it to the configuration struct.
- **Line 50: `pub fn save(&self) -> Result<(), String> {`**
  Saves the current configuration instance to disk.
- **Line 56: `let mut f = fs::File::create(CONFIG_FILE)...`**
  Attempts to create/overwrite the configuration file, transforming errors into a readable `String` description and returning early on failures.
- **Line 58: `f.write_all(contents.as_bytes())...`**
  Writes the formatted text string out as bytes to disk.

---

### 10. `wallet_info.rs`

This file handles exporting metadata that does not compromise security (such as the Master Fingerprint, Extended Public Key, and Output Descriptors) to hot-wallet tracking files.

```rust
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use std::io::Write;
use std::str::FromStr;

pub fn get_fingerprint(root_key: &ExtendedPrivKey) -> String {
    let secp = Secp256k1::new();
    let xpub = ExtendedPubKey::from_priv(&secp, root_key);
    hex::encode(xpub.fingerprint().as_bytes())
}

pub fn get_account_xpub(root_key: &ExtendedPrivKey, network: Network) -> Result<String, String> {
    let secp = Secp256k1::new();
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    let path = DerivationPath::from_str(&format!("m/84'/{}'/0'", coin))
        .map_err(|e| e.to_string())?;
    let account_key = root_key
        .derive_priv(&secp, &path)
        .map_err(|e| e.to_string())?;
    let xpub = ExtendedPubKey::from_priv(&secp, &account_key);
    Ok(xpub.to_string())
}

pub fn get_descriptor(root_key: &ExtendedPrivKey, network: Network) -> Result<String, String> {
    let xpub = get_account_xpub(root_key, network)?;
    Ok(format!("wpkh({}/0/*)", xpub))
}
```

#### Line-by-Line Explanation:
- **Line 9: `pub fn get_fingerprint(root_key: &ExtendedPrivKey) -> String {`**
  Calculates the master key fingerprint.
- **Line 11: `let xpub = ExtendedPubKey::from_priv(&secp, root_key);`**
  Derives the extended public key (xpub) corresponding to the extended private key (xprv).
- **Line 12: `hex::encode(xpub.fingerprint().as_bytes())`**
  Calculates the fingerprint (first 4 bytes of hash160 of public key) and encodes it as a hex string.
- **Line 17: `pub fn get_account_xpub(...) -> Result<String, String> {`**
  Derives the account-level extended public key at standard BIP-84 path `m/84'/coin'/0'`. This key is shared with hot tracking apps so they can generate all receive and change addresses to view balances, but they cannot spend.
- **Line 20: `let path = DerivationPath::from_str(&format!("m/84'/{}'/0'", coin))...`**
  Parses the account-level derivation path.
- **Line 22: `let account_key = root_key.derive_priv(&secp, &path)...`**
  Derives the account-level private key.
- **Line 25: `let xpub = ExtendedPubKey::from_priv(&secp, &account_key);`**
  Converts this account key into a public key (xpub).
- **Line 31: `pub fn get_descriptor(...) -> Result<String, String> {`**
  Returns the descriptor representing native SegWit receive addresses: `wpkh(XPUB/0/*)`.
- **Line 38: `pub fn export_watch_wallet(...) -> Result<(), String> {`**
  Writes a formatted tracking text file to disk (containing fingerprint, xpub, and descriptors) for external tools.

---

### 11. `store_backup.rs`

This file is responsible for encrypting the recovery phrase on disk using Argon2id for key stretching and AES-256-GCM for authenticated encryption.

```rust
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, Params};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs::File;
use std::io::{Write, BufWriter};
use zeroize::Zeroize;

pub fn store_backup(passphrase: &str, mnemonic_str: &str, filename: &str) -> Result<(), String> {
    let mut salt = [0u8; 32];
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    let mut key_bytes = [0u8; 32];
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(65536, 3, 1, Some(32)).unwrap(),
    );
    argon2.hash_password_into(passphrase.as_bytes(), &salt, &mut key_bytes)
        .map_err(|e| format!("Argon2 failed: {}", e))?;

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, mnemonic_str.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    key_bytes.zeroize();

    let file = File::create(filename)
        .map_err(|e| format!("Failed to create backup file: {}", e))?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "SALT: {}", hex::encode(salt)).map_err(|e| e.to_string())?;
    writeln!(writer, "NONCE: {}", hex::encode(nonce_bytes)).map_err(|e| e.to_string())?;
    writeln!(writer, "DATA: {}", hex::encode(&ciphertext)).map_err(|e| e.to_string())?;

    Ok(())
}
```

#### Line-by-Line Explanation:
- **Lines 26–27: `let mut salt = [0u8; 32]; let mut nonce_bytes = [0u8; 12];`**
  Allocates local buffers for key stretching (salt) and encryption (AES nonce).
- **Lines 28–29: `OsRng.fill_bytes(&mut salt); OsRng.fill_bytes(&mut nonce_bytes);`**
  Fills salt and nonce with cryptographically secure random bytes from the OS.
- **Line 32: `let mut key_bytes = [0u8; 32];`**
  Creates a mutable buffer to store the derived 256-bit AES key.
- **Lines 33–37: `let argon2 = Argon2::new(...)`**
  Configures the Argon2id hashing parameters. Memory cost is set to 64MB (`65536` KB), 3 iterations, and 1 parallel thread.
- **Line 38: `argon2.hash_password_into(passphrase.as_bytes(), &salt, &mut key_bytes)`**
  Derives the AES key by hashing the passphrase with the salt.
- **Line 42: `let cipher = Aes256Gcm::new_from_slice(&key_bytes)...`**
  Initializes the AES-256-GCM cipher instance with the derived key.
- **Line 44: `let nonce = Nonce::from_slice(&nonce_bytes);`**
  Converts the raw nonce bytes to the required GCM-specific `Nonce` wrapper.
- **Line 45: `let ciphertext = cipher.encrypt(nonce, mnemonic_str.as_bytes())...`**
  Encrypts the mnemonic bytes. The output ciphertext includes a 16-byte authentication tag appended at the end.
- **Line 49: `key_bytes.zeroize();`**
  Overwrites the key buffer in RAM with zeros to erase sensitive key material.
- **Line 52: `let file = File::create(filename)...`**
  Creates the backup text file on disk.
- **Line 54: `let mut writer = BufWriter::new(file);`**
  Wraps the file inside a buffered writer for efficiency.
- **Lines 55–57: `writeln!(writer, "SALT: {}", hex::encode(salt))...`**
  Writes the salt, nonce, and ciphertext out to the text file as hex-encoded lines.

#### `load_backup`:
Loads the file and does the reverse process (Argon2id derivation -> AES-256-GCM decryption).
- If decryption fails, it returns `Incorrect passphrase or corrupted backup`. This message is kept generic on purpose to avoid leaking clues that might aid timing or padding oracle attacks.

---

### 12. `transaction.rs`

This file builds, signs, and serializes P2WPKH transactions.

```rust
use bitcoin::blockdata::script::Builder;
use bitcoin::blockdata::witness::Witness;
use bitcoin::consensus::encode::serialize;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::util::address::Address;
use bitcoin::util::sighash::SighashCache;
use bitcoin::{EcdsaSighashType, OutPoint, PackedLockTime, Sequence, Transaction, TxIn, TxOut, Txid};
use std::str::FromStr;

pub const DUST_SATS: u64 = 546;

pub fn btc_to_sats(s: &str) -> Result<u64, String> {
    let v: f64 = s.trim().parse()
        .map_err(|_| format!("'{}' is not a valid number", s.trim()))?;
    if v < 0.0 { return Err("Amount cannot be negative.".to_string()); }
    Ok((v * 100_000_000.0).round() as u64)
}
```

#### Line-by-Line Explanation:
- **Line 15: `pub fn btc_to_sats(s: &str) -> Result<u64, String> {`**
  Parses a user input amount (e.g. `"0.001"`) into Satoshi integer units (`100000`).
- **Line 16: `let v: f64 = s.trim().parse()...`**
  Trims spacing and parses the string into a floating-point number.
- **Line 19: `Ok((v * 100_000_000.0).round() as u64)`**
  Converts BTC to Satoshis by multiplying by 100,000,000, rounding to handle floating-point precision issues, and casting to `u64`.

#### `build_transaction`:
```rust
pub fn build_transaction(p: &TxParams) -> Result<String, String> {
    // 1. Guard check and compute change
    let total = p.send_sats.checked_add(p.fee_sats).ok_or("Amount overflow.")?;
    if total > p.input_sats { return Err("Insufficient funds.".to_string()); }
    let change_sats = p.input_sats - total;

    // 2. Setup inputs
    let txid = Txid::from_str(&p.txid_str).map_err(|_| "Invalid Tx ID".to_string())?;
    let sequence = if p.use_rbf { Sequence(0xFFFF_FFFD) } else { Sequence::MAX };
    let txin = TxIn {
        previous_output: OutPoint { txid, vout: p.vout },
        script_sig: Builder::new().into_script(),
        sequence,
        witness: Witness::default(),
    };

    // 3. Setup outputs
    let mut outputs = vec![TxOut {
        value: p.send_sats,
        script_pubkey: p.to_address.script_pubkey(),
    }];
    if change_sats >= DUST_SATS {
        outputs.push(TxOut {
            value: change_sats,
            script_pubkey: p.change_address.script_pubkey(),
        });
    }

    let mut tx = Transaction {
        version: 1,
        lock_time: PackedLockTime::ZERO,
        input: vec![txin],
        output: outputs,
    };

    if p.dry_run {
        return Ok(format!("DRY_RUN:{}", hex::encode(serialize(&tx))));
    }

    // 4. Sign the segwit input
    let secp = Secp256k1::new();
    let pub_key = bitcoin::PublicKey::new(p.from_key.public_key(&secp));
    let script_code = Address::p2pkh(&pub_key, p.from_address.network).script_pubkey();
    
    let sighash = {
        let mut cache = SighashCache::new(&tx);
        cache.segwit_signature_hash(0, &script_code, p.input_sats, EcdsaSighashType::All)
            .map_err(|e| format!("Sighash failed: {}", e))?
    };

    let msg = Message::from_slice(sighash.as_ref()).map_err(|e| e.to_string())?;
    let sig = secp.sign_ecdsa(&msg, p.from_key);
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(EcdsaSighashType::All as u8);

    // 5. Place signature in witness
    let mut witness = Witness::new();
    witness.push(&sig_bytes);
    witness.push(&pub_key.public_key.serialize());
    tx.input[0].witness = witness;

    Ok(hex::encode(serialize(&tx)))
}
```
- **Line 102: `.checked_add(...)`**: Prevents overflow attacks when adding the send amount and fees.
- **Line 115: `Sequence(0xFFFF_FFFD)`**: Signals BIP-125 Replace-By-Fee (RBF) support by setting sequence below `0xFFFFFFFF`.
- **Line 119: `script_sig: Builder::new().into_script()`**: In SegWit, the legacy `scriptSig` is left empty.
- **Line 129: `DUST_SATS`**: Prevents creating change outputs smaller than 546 satoshis, which nodes would reject as dust.
- **Line 154: `script_code`**: When generating a signature hash for a P2WPKH input, the signature engine expects the script template formatted like a legacy P2PKH script.
- **Line 158: `SighashCache::new(&tx)`**: Computes the double-SHA256 digest of the transaction components according to BIP-143.
- **Line 175: `witness.push`**: Appends the signature and public key to the Witness stack. SegWit inputs read keys from here rather than `scriptSig`.
- **Line 179: `serialize(&tx)`**: Converts the transaction structure into Bitcoin consensus byte format, which is then hex-encoded.

---

### 13. `psbt.rs`

This file is responsible for reading, summarizing, signing, and exporting **Partially Signed Bitcoin Transactions (PSBTs)**. This is the main way offline hardware wallets interact with external coordinator applications.

- **`parse_psbt`**: Reads file bytes and checks if they start with the `psbt\xff` magic bytes. If yes, it deserializes the binary data. If not, it attempts base64 decoding first.
- **`summarise`**: Scans the input transactions or `witness_utxo` arrays, sums up input values, sums up outputs, calculates transaction fees, and tracks destination addresses. It sets a flag if fees exceed 5% of inputs.
- **`sign_psbt`**:
  - Scans the inputs in the PSBT.
  - Checks the input derivation path fingerprint (`bip32_derivation`). If it matches the fingerprint of our master key, we own this input.
  - Derives the exact private key for that path.
  - Signs the input using SegWit or Legacy signature caches.
  - Inserst the signature inside the input's `partial_sigs` map.
  - Returns the signed PSBT structure.
- **`base64` submodule**: Implements standard base64 decoding and encoding without relying on external crates.

---

## Part 2: Command Line Interface (`cli/src/`)

The CLI directory contains a text user interface.

### 1. `main.rs`
Acts as the central router. It draws the main menu in a loop:
1. **Create wallet**: Generates entropy -> shows mnemonic -> prompts for passphrase -> derives keys -> saves encrypted backup.txt.
2. **Open wallet**: Prompts for password, performs load verification, derives keys, sets session state.
3. **Verify backup**: Decrypts backup.txt to check password correctness.
4. **Settings**: Changes network or session timeout.
5. **Restore wallet**: Takes seed phrase input -> validates BIP-39 words -> encrypts new backup.

### 2. `session_state.rs`
Defines the `SessionState` struct that acts as an in-memory database during the session (holding keys and derived addresses). It uses zeroization for memory cleanup.

### 3. `session_actions.rs`
Contains the actions triggered by menu items:
- **`handle_receive_address`**: Displays a random receive address and prints a unicode QR code in the terminal.
- **`handle_sign_psbt`**: Interactive flow to load a PSBT file from a USB drive or paste it as base64, review the summary, sign it, and export it.
- **`handle_change_passphrase`**: Re-encrypts the mnemonic with a new passphrase.

### 4. `send_and_receive.rs`
Implements the interactive steps of building a transaction manually:
1. Address selection.
2. UTXO selection (preloaded CSV or manual entry).
3. RBF configuration toggle.
4. Destination address input (with verification against address reuse).
5. Amount & Fee Selection (displays fee tiers: slow, standard, fast).
6. Change address setup.

### 5. `ui.rs`
Terminal printing helpers. Defines ANSI color constants (e.g. `\x1b[31m` for Red, `\x1b[0m` to Reset styling) and prints styled blocks (headers, success marks, tables, and dividers).

### 6. `passphrase_check.rs`
Calculates password strength. It checks length, uppercase characters, digits, symbols, and non-ASCII characters to compute a score out of 7, displaying a colored bar indicator.

### 7. `qr_display.rs`
Generates a QR code in the terminal. It converts character modules into unicode blocks: `█` for dark blocks and ` ` for light spaces.

### 8. `password_input.rs`
Uses the `rpassword` crate to read terminal inputs without echoing them to the screen (hiding the password as it is typed).

### 9. `audit_log.rs`
Appends timestamped actions to `wallet_audit.log` (e.g. `[1717106720] SESSION_START`). It never logs keys or mnemonic data.

---

## Part 3: Tauri Desktop App Wrapper (`gui/src-tauri/src/`)

This directory allows compiling the core engine as a desktop application.

### 1. `lib.rs`

This file bridges frontend JavaScript/TypeScript calls to backend Rust logic using Tauri commands.

```rust
#[tauri::command]
fn create_wallet(passphrase: &str) -> Result<WalletData, String> {
    let entropy = generate_entropy();
    let mnemonic = generate_mnemonic(&entropy);
    let mnemonic_str = mnemonic.to_string();
    
    let mut seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);
    let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;
    let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);
    
    store_backup(passphrase, &mnemonic_str, "backup.txt").map_err(|e| e.to_string())?;
    
    seed.zeroize();
    
    Ok(WalletData { mnemonic: mnemonic_str, fingerprint })
}
```

#### Line-by-Line Explanation:
- **Line 19: `#[tauri::command]`**
  An attribute macro. It tells the Tauri framework to generate serialization code for this function, allowing it to be invoked from frontend JS/TS code via `invoke("create_wallet", { passphrase: "..." })`.
- **Line 20: `fn create_wallet(passphrase: &str) -> Result<WalletData, String> {`**
  Declares the command function.
- **Line 21: `let entropy = generate_entropy();`**
  Generates random entropy bytes.
- **Line 22: `let mnemonic = generate_mnemonic(&entropy);`**
  Generates the mnemonic words.
- **Line 23: `let mnemonic_str = mnemonic.to_string();`**
  Converts the mnemonic structure into a space-separated word string.
- **Line 25: `let mut seed = derive_seed_from_mnemonic(&mnemonic_str, passphrase);`**
  Derives the 512-bit seed.
- **Line 26: `let root_key = derive_keys(&seed, Network::Bitcoin).map_err(|e| e.to_string())?.0;`**
  Derives the BIP-32 root key for the Mainnet Bitcoin network. We select index `.0` of the returned tuple (the `ExtendedPrivKey`).
- **Line 27: `let fingerprint = boma_core::wallet_info::get_fingerprint(&root_key);`**
  Calculates the master key fingerprint.
- **Line 29: `store_backup(passphrase, &mnemonic_str, "backup.txt").map_err(|e| e.to_string())?;`**
  Encrypts and writes the recovery phrase to `backup.txt`.
- **Line 31: `seed.zeroize();`**
  Zeroes out seed bytes in memory.
- **Line 33: `Ok(WalletData { mnemonic: mnemonic_str, fingerprint })`**
  Returns the serialized response structure to the frontend.

#### IPC Commands defined:
- `check_wallet_exists`: Checks if `backup.txt` exists.
- `restore_wallet`: Recreate wallet from a recovery phrase input.
- `login`: Decrypts `backup.txt`, derives keys, and loads receive address strings in memory.
- `export_xpub` / `export_descriptor`: Outputs watch-only configurations.
- `get_recovery_phrase`: Returns plaintext seed words (after validating password).
- `change_passphrase`: Re-encrypts wallet data.
- `import_utxos`: Parses UTXOs CSV files.
- `build_transaction`: Builds raw transactions.
- `load_psbt` / `load_psbt_from_base64` / `sign_and_export_psbt`: Handles PSBT workflows.
- `get_settings` / `update_settings`: Controls configuration state.

### 2. `main.rs` & `build.rs`
- **`main.rs`**: Passes entry-point execution directly to the Tauri runtime library: `gui_lib::run()`.
- **`build.rs`**: Compile-time script that runs `tauri_build::build()`, bundling icons and native desktop assets.
