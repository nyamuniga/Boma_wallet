<div align="center">
  <img src="/Users/mac/.gemini/antigravity-ide/brain/0d5e60e9-5e87-4dd1-a92b-6a529fbc9313/boma_wallet_logo_1779874020050.png" width="300" alt="BOMA Wallet Logo">
  <h1>BOMA Cold Wallet</h1>
  <p><b>A highly secure, offline-first Bitcoin storage solution.</b></p>
</div>

---

BOMA (meaning "fortified enclosure" or "stronghold") is a terminal-based, air-gapped Bitcoin wallet designed for maximum security. It allows you to generate keys, receive funds, and sign transactions entirely offline, ensuring your private keys never touch the internet.

---

## 🛡️ For Users (No Coding Experience Required)

BOMA is designed to be run on a secure, offline computer. It helps you generate a "Recovery Phrase" (your master key) and creates an encrypted backup file so you don't have to type your phrase every time.

### How to use BOMA securely:

1. **Get the executable:** Move the `boma` application file to a USB thumb drive.
2. **Go offline:** Plug the USB drive into a computer that is **completely disconnected from the internet** (and preferably will never connect again).
3. **Run the wallet:**
   - On Mac/Linux, open your Terminal, drag the `boma` file into it, and press Enter.
   - On Windows, double-click the `boma.exe` file or run it via Command Prompt.

### Core Features
- **Create a New Wallet:** Generates a secure 12-word or 24-word Recovery Phrase. **Write this down on paper.** BOMA will encrypt this phrase with a passphrase of your choosing and save it to a local `backup.txt` file.
- **Receive Bitcoin:** View your receive addresses. BOMA even generates scannable QR codes right in your terminal so you can easily send funds to your cold wallet from your phone.
- **Send Bitcoin (Offline Signing):** To send funds out of BOMA, you follow a secure "air-gapped" workflow:
  1. Use an internet-connected device to find the "UTXO" (the specific chunk of Bitcoin you want to spend) via a block explorer.
  2. In BOMA (offline), enter those details and the recipient address.
  3. BOMA will sign the transaction and give you a long string of text (Hex).
  4. Copy that text to your online device and paste it into a transaction broadcaster (like `https://blockstream.info/tx/push`).
- **Watch-Only Wallet:** You can export an "xpub" file. This file contains no private keys and is 100% safe to put on an online computer to track your balances without risking your funds.

> [!WARNING]
> **Never store your Recovery Phrase digitally.** Do not take a photo of it, and do not save it in a text file. If someone finds your Recovery Phrase, they can steal your Bitcoin.

---

## 💻 For Developers

BOMA is written entirely in Rust and strictly adheres to Bitcoin standards (BIP-39, BIP-32, BIP-44) for full interoperability with other major wallets like Electrum, Sparrow, and Ledger.

### Tech Stack & Security
- **Language:** Rust (edition 2021)
- **Cryptography:**
  - `bitcoin` crate (v0.29) for standard key derivation and P2PKH tx signing via `SighashCache`.
  - `aes-gcm` (AES-256-GCM) for authenticated encryption of the local backup file.
  - OS-level entropy (`OsRng`) for mnemonic generation.
- **Memory Safety:** Sensitive materials (like the master seed) are actively wiped from RAM after key derivation using the `zeroize` crate.
- **Dependencies:** Minimal footprint. CLI interactions use raw ANSI codes and built-in OS tools (like `stty` on Unix) to prevent dependency bloat.

### Build Instructions

You will need [Rust and Cargo](https://rustup.rs/) installed.

1. **Clone & Build:**
   ```bash
   git clone <your-repo-url>
   cd boma
   cargo build --release
   ```
2. **Run:**
   The optimized, standalone binary will be placed in `target/release/boma`.
   ```bash
   ./target/release/boma
   ```

### Architecture Highlights
- **BIP-44 Standard:** Receive addresses derived at `m/44'/0'/0'/0/{i}` and change addresses at `m/44'/0'/0'/1/{i}`.
- **RBF (Replace-By-Fee):** Supported natively via sequence `0xFFFFFFFD`.
- **Dynamic Fee Estimation:** Built-in P2PKH vbyte calculator.
- **Mainnet / Testnet Support:** Configurable via `wallet_config.txt`.
- **Stateless Operation:** The wallet runs entirely in memory. The `backup.txt` file only contains the AES-GCM nonce, salt, and ciphertext. Authentication acts as the decryption key.

### Developer Roadmap (Future Enhancements)
- PSBT (Partially Signed Bitcoin Transactions - BIP-174) support.
- Native SegWit (bech32) address derivation.
- Multi-sig (BIP-45) capabilities.

---
*BOMA: Your keys, your fortress.*
