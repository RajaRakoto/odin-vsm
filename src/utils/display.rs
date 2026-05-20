//! Terminal display helpers — mirrors `info`, `ok`, `warn`, `err`, `section`,
//! and `_confirm` from `odin.sh`.

use colored::Colorize;
use std::io::Write;

// ── Single-line printers ──────────────────────────────────────────────────────

/// Print an info line: `  ▸ <msg>` in cyan.
pub fn info(msg: &str) {
    println!("  {} {}", "▸".cyan(), msg);
}

/// Print a success line: `  ✔ <msg>` in green.
pub fn ok(msg: &str) {
    println!("  {} {}", "✔".green(), msg);
}

/// Print a warning line: `  ⊘ <msg>` in yellow.
pub fn warn(msg: &str) {
    println!("  {} {}", "⊘".yellow(), msg);
}

/// Print an error line: `  ✘ <msg>` in red (to stderr).
pub fn err(msg: &str) {
    eprintln!("  {} {}", "✘".red(), msg);
}

// ── Structure ─────────────────────────────────────────────────────────────────

/// Print a bold-cyan section header: `\n► <title>`.
/// Matches `section()` in `odin.sh`.
pub fn section(title: &str) {
    println!("\n{} {}", "►".cyan().bold(), title.bold());
}

/// Print a horizontal separator of `n` `─` characters, indented by two spaces.
pub fn separator_n(n: usize) {
    println!("  {}", "─".repeat(n).cyan());
}

// ── Confirmation prompt ───────────────────────────────────────────────────────

/// Ask the user for confirmation.
///
/// Prints `  <prompt> ` and reads a line from stdin.  Returns `true` only
/// when the user types `y` or `Y`.  Any other input (including empty) is
/// treated as "No".
///
/// The `prompt` string may contain ANSI escape codes; they are passed
/// through unchanged so callers can style the question themselves.
pub fn confirm(prompt: &str) -> bool {
    print!("  {prompt} ");
    std::io::stdout().flush().ok();

    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(_) => matches!(line.trim().to_lowercase().as_str(), "y"),
        Err(_) => {
            warn("Cannot read user input (non-interactive mode?) — treating as 'No'.");
            false
        }
    }
}
