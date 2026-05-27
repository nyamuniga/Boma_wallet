use std::io::{self, Write};
use std::process::Command;

/// Reads a password from stdin without echoing characters to the terminal.
///
/// Uses the macOS/Unix `stty` utility to temporarily disable terminal echo,
/// reads one line, then restores echo. No external crates required.
pub fn read_password() -> Result<String, String> {
    // Disable echo so keystrokes are invisible
    Command::new("stty")
        .arg("-echo")
        .status()
        .map_err(|e| format!("Failed to disable terminal echo: {}", e))?;

    let mut input = String::new();
    let result = io::stdin().read_line(&mut input);

    // Always re-enable echo, even if reading failed
    Command::new("stty")
        .arg("echo")
        .status()
        .ok();

    // Print a newline since Enter wasn't echoed
    println!();
    io::stdout().flush().ok();

    result.map_err(|e| format!("Failed to read input: {}", e))?;
    Ok(input.trim().to_string())
}
