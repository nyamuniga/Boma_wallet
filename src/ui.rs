use std::io::{self, Write};

// ── ANSI codes ────────────────────────────────────────────────────────────────
pub const RESET:  &str = "\x1b[0m";
pub const BOLD:   &str = "\x1b[1m";
pub const DIM:    &str = "\x1b[2m";
pub const RED:    &str = "\x1b[31m";
pub const GREEN:  &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN:   &str = "\x1b[36m";
pub const ORANGE: &str = "\x1b[38;5;214m"; // Bitcoin orange

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
    let t = format!("₿  BOMA Cold Wallet     │  v0.3");
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
