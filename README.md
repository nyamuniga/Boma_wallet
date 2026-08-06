<div align="center">
  <img src="https://res.cloudinary.com/dlsbepbro/image/upload/v1779875405/boma-logo_yuncve.png" width="300" alt="BOMA Wallet Logo">
  <h1>BOMA Cold Wallet</h1>
  <p><b>A highly secure, offline-first Bitcoin Wallet available for Terminal and Desktop.</b></p>
</div>

---

BOMA (meaning "fortified enclosure" or "stronghold") is an air-gapped Bitcoin wallet designed for maximum security. It allows you to generate keys, receive funds, and sign transactions entirely offline, ensuring your private keys never touch the internet. 

BOMA strictly adheres to Bitcoin standards (BIP-39, BIP-84 for native SegWit, PSBTs) for full interoperability with other major wallets like Electrum, Sparrow, and Ledger.

It is split into a modular workspace:
- **`boma-core`**: The secure cryptographic engine containing all logic, strictly enforcing memory hygiene and entropy health checks.
- **`boma-cli`**: A lightweight, hacker-friendly terminal interface.
- **`gui`**: A premium, responsive desktop application built with Tauri v2, React, and TailwindCSS v4.

## Core Features & Security Guarantees

- **Cryptographically Guaranteed Entropy:** Recovery Phrases are generated using a double-sampling OS RNG strategy verified against NIST SP 800-90B health tests (evaluating distinct values, repetition, and chi-squared uniformity). If the OS entropy degrades, the wallet refuses to generate keys rather than risk your funds.
- **Defense-in-Depth Memory Hygiene:** Sensitive material (mnemonics, private keys, decrypted payloads) are deterministically zeroized from RAM immediately upon session lock, timeout, or exit to mitigate cold-boot forensics.
- **PSBT (Partially Signed Bitcoin Transactions):** Natively parses, validates, and signs PSBTs offline, supporting complex multi-input derivations and hardware wallet interoperability workflows.
- **Change Address Rotation:** Automatically rotates change outputs across the `m/84'/0'/0'/1/*` derivation path, preserving on-chain privacy by never reusing change addresses.
- **Tamper-Evident Auditing:** The CLI maintains a cryptographic hash chain log of all activities. Any post-hoc modification or deletion of log entries immediately breaks the chain and alerts the user.

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
1. Export your wallet's xpub or descriptor from BOMA to an internet-connected, watch-only wallet (like Sparrow or Electrum).
2. Build your transaction in the watch-only wallet, and export it as a **PSBT**.
3. Transfer the PSBT to your offline BOMA app via USB.
4. Review and sign the PSBT in BOMA. BOMA will mathematically sign the transaction and output a finalized PSBT (or raw hex).
5. Transfer the signed transaction back to your online device and broadcast it to the network!

## Architecture Details
- **BIP-84 Standard:** Native SegWit (P2WPKH) receive addresses derived at `m/84'/0'/0'/0/{i}` and change addresses at `m/84'/0'/0'/1/{i}`.
- **Passphrase Validation:** Server-side enforcement of complex passphrases to ensure offline brute-force resistance.
- **RBF (Replace-By-Fee):** Supported natively via sequence `0xFFFFFFFD` and transaction Version 2.
- **Mainnet / Testnet Support:** Instantly toggleable via settings.
- **Stateless Operation:** The wallet runs entirely in memory. The `backup.txt` file is restricted (0600 permissions) and only contains the AES-256-GCM nonce, salt, and ciphertext. 

## Open Source & Contributing
BOMA is fully open-source. We believe security tools must be transparent and verifiable by the community. Contributions, issues, and feature requests are highly encouraged!

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
*BOMA: Your keys, your fortress.*
