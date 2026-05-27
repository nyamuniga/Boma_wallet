<div align="center">
  <img src="https://res.cloudinary.com/dlsbepbro/image/upload/v1779875405/boma-logo_yuncve.png" width="300" alt="BOMA Wallet Logo">
  <h1>BOMA Cold Wallet</h1>
  <p><b>A highly secure, offline-first Bitcoin Wallet.</b></p>
</div>

---

BOMA (meaning "fortified enclosure" or "stronghold") is a terminal-based, air-gapped Bitcoin wallet designed for maximum security. It allows you to generate keys, receive funds, and sign transactions entirely offline, ensuring your private keys never touch the internet. 

BOMA is written entirely in Rust and strictly adheres to Bitcoin standards (BIP-39, BIP-32, BIP-44) for full interoperability with other major wallets like Electrum, Sparrow, and Ledger.

## What BOMA Does

- **Creates Secure Wallets:** Generates a secure 12-word or 24-word Recovery Phrase using OS-level cryptographic entropy. It encrypts this phrase locally with AES-256-GCM so you don't have to type your phrase every time you log in.
- **Receives Bitcoin:** Generates receive addresses and displays scannable QR codes right in your terminal, making it easy to send funds to your cold wallet from your phone.
- **Signs Transactions Offline:** Allows you to securely spend your Bitcoin without your private keys ever touching an internet-connected device.
- **Protects Memory:** Actively wipes sensitive materials (like the master seed) from your computer's RAM immediately after keys are derived.
- **Exports Watch-Only Data:** Exports an "xpub" file that contains no private keys, which you can safely load onto an online computer to track your balances and receive payments.

> [!WARNING]
> **Never store your Recovery Phrase digitally.** Do not take a photo of it, and do not save it in a text file. If someone finds your Recovery Phrase, they can steal your Bitcoin.

## Instructions & Usage

For maximum security, BOMA is designed to be run on an air-gapped computer.

1. **Get the Executable:** Build the application (instructions below) and move the `boma` executable file to a USB thumb drive.
2. **Go Offline:** Plug the USB drive into a computer that is **completely disconnected from the internet**.
3. **Run the Wallet:** Open your Terminal, drag the `boma` file into it, and press Enter. 

### How to Send Bitcoin (Offline Signing Workflow)
Because BOMA cannot connect to the internet to broadcast a transaction, you must follow this secure workflow to send funds:
1. Use an internet-connected device to look up your Bitcoin address on a block explorer and find the "UTXO" (the unspent chunk of Bitcoin you want to spend).
2. In the offline BOMA app, select "Sign transaction" and enter the UTXO details and the recipient address.
3. BOMA will mathematically sign the transaction and output a long string of text called "Raw Hex".
4. Copy that Hex text, transfer it back to your online device, and paste it into a transaction broadcaster (like `https://blockstream.info/tx/push`).

## Build Instructions

To compile the application yourself, you will need [Rust and Cargo](https://rustup.rs/) installed.

1. **Clone & Build:**
   ```bash
   git clone <your-repo-url>
   cd boma
   cargo build --release
   ```
2. **Locate Executable:**
   The highly optimized, standalone binary will be generated at:
   ```bash
   target/release/boma
   ```

## Architecture Details
- **BIP-44 Standard:** Receive addresses derived at `m/44'/0'/0'/0/{i}` and change addresses at `m/44'/0'/0'/1/{i}`.
- **RBF (Replace-By-Fee):** Supported natively via sequence `0xFFFFFFFD`.
- **Dynamic Fee Estimation:** Built-in P2PKH vbyte calculator.
- **Mainnet / Testnet Support:** Configurable via `wallet_config.txt`.
- **Stateless Operation:** The wallet runs entirely in memory. The `backup.txt` file only contains the AES-GCM nonce, salt, and ciphertext. 

## Open Source & Contributing
BOMA is fully open-source. We believe security tools must be transparent and verifiable by the community. Contributions, issues, and feature requests are highly encouraged!

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
*BOMA: Your keys, your fortress.*
