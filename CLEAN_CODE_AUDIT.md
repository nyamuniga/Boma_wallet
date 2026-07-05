# Clean Code Audit — BOMA Wallet v0.3.0

This audit evaluates the BOMA Wallet codebase against standard Clean Code principles (e.g., SOLID principles, DRY, meaningful naming, error handling).

## ✅ What We Are Doing Well (Successes)

1. **Separation of Concerns (Modularity)**
   The project is heavily modularized into small, focused files rather than a monolithic `main.rs`. Modules like `ui.rs`, `config.rs`, `wallet_info.rs`, and `change_addresses.rs` have clear, singular responsibilities.

2. **Graceful Error Handling**
   There are no `panic!`, `.unwrap()`, or `.expect()` calls handling user input. All functions gracefully return `Result<T, String>`, and errors are presented to the user via the `ui::error()` wrapper, preventing sudden crashes.

3. **DRY (Don't Repeat Yourself) in UI**
   The creation of `ui::prompt_until<F, T>` is an excellent example of DRY. Instead of writing `loop { match ... }` every time we need validated user input, the logic is abstracted into a higher-order function that takes a closure.

4. **Meaningful Naming**
   Variable and function names are highly descriptive. Names like `derive_seed_from_mnemonic`, `btc_to_sats`, and `is_own_address` immediately convey their purpose without requiring inline comments.

5. **Security & Principle of Least Privilege**
   Memory is actively managed (e.g., `seed.zeroize()`) the moment it is no longer needed. Private keys are never passed into functions that only require public keys.

---

## ⚠️ Areas for Improvement (Clean Code Violations)

While the code is robust, it violates a few strict Clean Code principles, primarily around function size and the Single Responsibility Principle (SRP).

### 1. The `wallet_session` Function is Too Large (Violates SRP)
**File:** `src/main.rs` (approx. 180 lines)
- **The Issue:** The `wallet_session` function handles the menu display, user input routing, and the execution of 11 different distinct actions. This breaks the Single Responsibility Principle.
- **The Fix:** Extract the match arms into separate functions (e.g., `handle_receive()`, `handle_export_xpub()`, etc.) or move the entire session loop into a dedicated `session.rs` module.

### 2. Too Many Function Arguments
**File:** `src/main.rs`
- **The Issue:** `wallet_session` takes 7 arguments (`mnemonic_str`, `receive_addresses`, `change_addresses`, `root_key`, `fingerprint`, `cfg`, `audit`). Clean Code dictates that functions should ideally have 0–2 arguments, and 3+ should be avoided.
- **The Fix:** Group these related arguments into a struct called `WalletContext` or `SessionState` and pass a reference to that single struct.

### 3. Mixing UI with Business Logic
**File:** `src/send_and_receive.rs` (in `guided_send`)
- **The Issue:** The `guided_send` function is over 150 lines long. It heavily mixes terminal UI logic (`println!`, `ui::prompt`) with the core business logic of formatting and parsing transaction parameters. 
- **The Fix:** The UI collection phase (gathering the `TxParams` struct) should be completely separated from the transaction building phase. A pure UI function should return a populated `TxParams`, which is then passed to `build_transaction`.

### 4. Direct `println!` Usage Bypassing the UI Module
**Files:** `src/main.rs`, `src/send_and_receive.rs`
- **The Issue:** While a `ui.rs` module exists to standardize output, there are still many raw `println!` macros scattered through the code (e.g., printing the transaction summary).
- **The Fix:** Move complex layout printing (like the transaction summary box or the UTXO list) into dedicated functions inside `ui.rs` or a `views.rs` module to keep the terminal styling logic isolated.

### 5. Minor Code Duplication in Derivation Loops
**Files:** `src/generate_many_addresses.rs` and `src/change_addresses.rs`
- **The Issue:** Both files contain an almost identical `for i in 0..20u32` loop that formats a string, parses the path, derives the key, and handles the error. 
- **The Fix:** Abstract the loop into a single function `derive_address_range(root_key, path_prefix, count)` that both the receive and change modules can call.

---

## 🎯 Recommended Refactoring Plan

If we decide to prioritize a code cleanup phase in the future, these would be the exact steps:

1. Create a `SessionState` struct to hold keys, addresses, and config.
2. Extract the massive `match` block in `main.rs` into a `MenuRouter` or distinct handler functions.
3. Split `guided_send` into `ui::collect_send_params()` and a pure `transaction::build()`.
4. Replace all remaining raw `println!` calls with formatted `ui::` helpers.
5. Consolidate address derivation logic into a shared helper function.

*(Note: As requested, no code changes have been made during this audit).*
