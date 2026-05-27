<div align="center">
  <img src="https://res.cloudinary.com/dlsbepbro/image/upload/v1779875405/boma-logo_yuncve.png" width="300" alt="BOMA Wallet Logo">
  <h1>BOMA Cold Wallet</h1>
  <p><b>A highly secure, offline-first Bitcoin Wallet available for Terminal and Desktop.</b></p>
</div>

---

BOMA (meaning "fortified enclosure" or "stronghold") is an air-gapped Bitcoin wallet designed for maximum security. It allows you to generate keys, receive funds, and sign transactions entirely offline, ensuring your private keys never touch the internet. 

BOMA strictly adheres to Bitcoin standards (BIP-39, BIP-32, BIP-44) for full interoperability with other major wallets like Electrum, Sparrow, and Ledger.

It is split into a modular workspace:
- **`boma-core`**: The secure cryptographic engine containing all logic, completely decoupled from any UI.
- **`boma-cli`**: A lightweight, hacker-friendly terminal interface.
- **`gui`**: A premium, responsive desktop application built with Tauri v2, React, and TailwindCSS v4.

## What BOMA Does

- **Creates Secure Wallets:** Generates a secure 12-word or 24-word Recovery Phrase using OS-level cryptographic entropy. It encrypts this phrase locally with AES-256-GCM so you don't have to type your phrase every time you log in.
- **Restores Existing Wallets:** Instantly restore any standard BIP-39 mnemonic (12 or 24 words), with optional 25th word passphrase support.
- **Receives Bitcoin:** Generates receive addresses and displays scannable QR codes in both the GUI and CLI, making it easy to send funds to your cold wallet from your phone.
- **Signs Transactions Offline:** Allows you to securely spend your Bitcoin without your private keys ever touching an internet-connected device.
- **Imports UTXOs:** Easily load UTXOs (Unspent Transaction Outputs) via CSV files for fast, offline transaction building.
- **Protects Memory:** Actively wipes sensitive materials (like the master seed) from your computer's RAM immediately after keys are derived.
- **Exports Watch-Only Data:** Exports an "xpub" file or standard descriptor which you can safely load onto an online computer (like Sparrow Wallet) to track your balances and receive payments.

> [!WARNING]
> **Never store your Recovery Phrase digitally.** Do not take a photo of it, and do not save it in a text file. If someone finds your Recovery Phrase, they can steal your Bitcoin. Only ever run BOMA on an offline, air-gapped machine.

## Build Instructions

To compile the applications yourself, you will need [Rust and Cargo](https://rustup.rs/) installed. To build the GUI, you also need [Node.js](https://nodejs.org/).

### Building the CLI
```bash
git clone <your-repo-url>
cd boma/cli
cargo build --release --bin boma-cli
```
The optimized binary will be generated at `target/release/boma-cli`.

### Building the GUI
```bash
cd boma/gui
npm install
npm run tauri build
```
This will compile the web assets and bundle the Tauri desktop application for your native operating system. To run it in development mode, use `npm run tauri dev`.

## Instructions & Usage

For maximum security, BOMA is designed to be run on an air-gapped computer.

1. **Get the Executable:** Build the application and move the `boma-cli` executable or GUI app bundle to a USB thumb drive.
2. **Go Offline:** Plug the USB drive into a computer that is **completely disconnected from the internet**.
3. **Run the Wallet:** Open the GUI app, or run the CLI executable in your terminal.

### How to Send Bitcoin (Offline Signing Workflow)
Because BOMA cannot connect to the internet to broadcast a transaction, you must follow this secure workflow to send funds:
1. Use an internet-connected device to look up your Bitcoin address on a block explorer and find the "UTXO" (the unspent chunk of Bitcoin you want to spend), or export a UTXO CSV from a watch-only wallet.
2. In the offline BOMA app, select "Sign transaction" (or load your UTXO CSV). Enter the UTXO details, the recipient address, and select a fee tier.
3. BOMA will mathematically sign the transaction and output a long string of text called "Raw Hex".
4. Copy that Hex text, transfer it back to your online device, and paste it into a transaction broadcaster (like `https://blockstream.info/tx/push`).

## Architecture Details
- **BIP-44 Standard:** Receive addresses derived at `m/44'/0'/0'/0/{i}` and change addresses at `m/44'/0'/0'/1/{i}`.
- **RBF (Replace-By-Fee):** Supported natively via sequence `0xFFFFFFFD`.
- **Dynamic Fee Estimation:** Built-in P2PKH vbyte calculator.
- **Mainnet / Testnet Support:** Instantly toggleable via settings.
- **Stateless Operation:** The wallet runs entirely in memory. The `backup.txt` file only contains the AES-GCM nonce, salt, and ciphertext. 

## Open Source & Contributing
BOMA is fully open-source. We believe security tools must be transparent and verifiable by the community. Contributions, issues, and feature requests are highly encouraged!

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
*BOMA: Your keys, your fortress.*
