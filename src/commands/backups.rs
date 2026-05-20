//! `clear-backups` command.
//!
//! Mirrors `cmd_clear_backups` from `odin.sh`.

use crate::{
    config::AppConfig,
    error::Result,
    utils::display::{info, ok, section, separator_n, warn},
};
use colored::Colorize;
use std::fs;

pub async fn run_clear(config: &AppConfig) -> Result<()> {
    section("Clear Backups — Delete all backups in config/backups/");

    let backups_dir = config.backups_dir();
    println!();

    if !backups_dir.exists() {
        warn(&format!(
            "Backup directory not found: {}",
            backups_dir.display()
        ));
        warn("Nothing to delete.");
        return Ok(());
    }

    // Collect files
    let mut files: Vec<_> = fs::read_dir(&backups_dir)
        .map_err(crate::error::Error::Io)?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    files.sort();

    if files.is_empty() {
        warn(&format!("No files found in {}.", backups_dir.display()));
        info("Nothing to delete.");
        return Ok(());
    }

    let total = files.len();
    println!(
        "  {}Files to be deleted from \x1b[0;36m{}\x1b[0m{}:",
        "".bold(),
        backups_dir.display(),
        "".bold()
    );
    separator_n(44);

    for f in &files {
        let fname = f.file_name().unwrap_or_default().to_string_lossy();
        let fsize = fs::metadata(f)
            .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
            .unwrap_or_else(|_| "?".into());
        println!("  {:<40}  {}", fname.red(), fsize.yellow());
    }

    separator_n(44);
    println!(
        "  \x1b[1;33m⚠  This will permanently delete {} file(s).\x1b[0m",
        total
    );
    println!("  \x1b[1;33m   The config/backups/ directory itself will be preserved.\x1b[0m");
    println!();

    if !crate::utils::display::confirm(&format!(
        "\x1b[0;31m\x1b[1mDelete all {total} backup(s)? (y/N)\x1b[0m"
    )) {
        warn("Cancelled. No files were deleted.");
        return Ok(());
    }

    println!();
    info("Deleting backup files…");

    let mut deleted = 0usize;
    let mut failed = 0usize;

    for f in &files {
        let fname = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match fs::remove_file(f) {
            Ok(_) => {
                ok(&format!("Deleted: {fname}"));
                deleted += 1;
            }
            Err(e) => {
                crate::utils::display::err(&format!("Failed to delete {fname}: {e}"));
                failed += 1;
            }
        }
    }

    println!();
    separator_n(44);
    if failed == 0 {
        ok(&format!("{deleted} backup(s) deleted successfully."));
        ok(&format!(
            "Directory {} is now empty.",
            backups_dir.display()
        ));
    } else {
        warn(&format!(
            "{deleted} deleted, {failed} failed. Check permissions."
        ));
    }
    separator_n(44);
    Ok(())
}
