use std::io::{self, Write};

// ── ANSI codes ────────────────────────────────────────────────────────────────
pub const RESET:  &str = "\x1b[0m";
pub const BOLD:   &str = "\x1b[1m";
pub const DIM:    &str = "\x1b[2m";
pub const RED:    &str = "\x1b[31m";
pub const GREEN:  &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN:   &str = "\x1b[36m";
pub const ORANGE: &str = "\x1b[38;2;165;81;48m"; // #a55130 from logo

// ── Screen control ────────────────────────────────────────────────────────────

pub fn clear() {
    print!("\x1B[2J\x1B[H");
    io::stdout().flush().ok();
}

// ── Styled output ─────────────────────────────────────────────────────────────

pub fn success(msg: &str) {
    println!("  {}{}✓  {}{}", BOLD, GREEN, msg, RESET);
}
pub fn error(msg: &str) {
    println!("  {}{}✗  {}{}", BOLD, RED, msg, RESET);
}
pub fn warn(msg: &str) {
    println!("  {}{}⚠  {}{}", BOLD, YELLOW, msg, RESET);
}
pub fn info(msg: &str) {
    println!("  {}{}ℹ  {}{}", DIM, CYAN, msg, RESET);
}

// ── Layout helpers ────────────────────────────────────────────────────────────

/// Clears screen and draws the top header bar.
/// `breadcrumb` is shown on the subtitle line, e.g. "Main > Send > Step 2/5"
pub fn header(title: &str, breadcrumb: &str) {
    clear();
    let w = 54usize;
    println!("\n  {}{}┌{}┐{}", BOLD, ORANGE, "─".repeat(w), RESET);
    let t = "₿  BOMA Cold Wallet     │  v0.3".to_string();
    println!("  {}{}│ {:<w$}│{}", BOLD, ORANGE, t, RESET, w = w - 1);
    if !breadcrumb.is_empty() {
        println!("  {}{}│ {:<w$}│{}", DIM, ORANGE, breadcrumb, RESET, w = w - 1);
    }
    if !title.is_empty() {
        println!("  {}{}│ {:<w$}│{}", BOLD, CYAN, title, RESET, w = w - 1);
    }
    println!("  {}{}└{}┘{}\n", BOLD, ORANGE, "─".repeat(w), RESET);
}

pub fn section(title: &str) {
    println!("\n  {}{}{}{}", BOLD, CYAN, title, RESET);
    println!("  {}{}{}", DIM, "─".repeat(46), RESET);
}

pub fn divider() {
    println!("  {}{}{}", DIM, "─".repeat(52), RESET);
}

pub fn pause() {
    println!("\n  {}Press Enter to continue...{}", DIM, RESET);
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}

// ── Prompt ────────────────────────────────────────────────────────────────────

/// Prints a styled prompt and reads a trimmed line from stdin.
/// If the user types `?`, prints `help_text` and re-prompts.
pub fn prompt(label: &str, help_text: &str) -> String {
    loop {
        print!("  {}▶{}  {}: ", BOLD, RESET, label);
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).expect("Failed to read input");
        let trimmed = buf.trim().to_string();
        if trimmed == "?" {
            println!("  {}{}  {}{}", DIM, CYAN, help_text, RESET);
            continue;
        }
        return trimmed;
    }
}

/// Prompt that loops until the closure returns Ok(T), displaying Err(msg) on failure.
pub fn prompt_until<F, T>(label: &str, help_text: &str, parse: F) -> T
where
    F: Fn(&str) -> Result<T, String>,
{
    loop {
        let input = prompt(label, help_text);
        match parse(&input) {
            Ok(val) => return val,
            Err(e) => error(&e),
        }
    }
}

// ── Menu builder ──────────────────────────────────────────────────────────────

/// Prints a numbered menu. `items` is a list of `(number_label, description)`.
pub fn menu(items: &[(&str, &str)]) {
    for (num, desc) in items {
        println!("  {}{}{}{}  {}", BOLD, ORANGE, num, RESET, desc);
    }
}

// ── Application-specific formatting ───────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn print_transaction_summary(
    from: &bitcoin::util::address::Address,
    to: &bitcoin::util::address::Address,
    send_sats: u64,
    fee_sats: u64,
    change_sats: u64,
    change_address: &bitcoin::util::address::Address,
    use_rbf: bool,
    dry_run: bool,
) {
    let has_change = change_sats >= 546; // DUST_SATS
    println!();
    divider();
    println!("  {}{}Transaction Summary{}", BOLD, CYAN, RESET);
    divider();
    println!("  From:      {}", from);
    println!("  To:        {}{}{}", GREEN, to, RESET);
    println!("  Send:      {:.8} BTC  ({} sats)", send_sats as f64/1e8, send_sats);
    println!("  Fee:       {:.8} BTC  ({} sats)", fee_sats as f64/1e8, fee_sats);
    if has_change {
        println!("  Change:    {:.8} BTC  ({} sats)  → {}", change_sats as f64/1e8, change_sats, change_address);
    } else if change_sats > 0 {
        println!("  Change:    {} sats  {}(below dust — absorbed into fee){}", change_sats, DIM, RESET);
    }
    println!("  RBF:       {}", if use_rbf { "yes — fee can be bumped later" } else { "no" });
    if dry_run {
        println!("  {}{}MODE: DRY RUN — will NOT be signed{}", BOLD, YELLOW, RESET);
    }
    divider();
}

/// Prints the warning header when displaying a new mnemonic phrase.
pub fn print_mnemonic_warning() {
    println!("  {}{}\u{26a0}  Write down these words \u{2014} they are your Bitcoin backup.{}", BOLD, YELLOW, RESET);
    println!("  {}NEVER share them. Anyone with these words owns your funds.{}\n", RED, RESET);
}

/// Prints the fee tier table during transaction building.
pub fn print_fee_tiers(vbytes: u64, slow: u64, standard: u64, fast: u64) {
    println!("\n  {}Estimated tx size: {} vbytes{}", DIM, vbytes, RESET);
    println!("  Fee tiers (sat/vbyte):");
    println!("    {}[s]{}  Slow     ~2  sat/vbyte  \u{2192}  {} sats  ({:.8} BTC)", DIM, RESET, slow,     slow as f64 / 1e8);
    println!("    {}[n]{}  Normal  ~10  sat/vbyte  \u{2192}  {} sats  ({:.8} BTC)", DIM, RESET, standard, standard as f64 / 1e8);
    println!("    {}[f]{}  Fast    ~25  sat/vbyte  \u{2192}  {} sats  ({:.8} BTC)", DIM, RESET, fast,     fast as f64 / 1e8);
    println!("    {}[m]{}  Enter manually", DIM, RESET);
}
