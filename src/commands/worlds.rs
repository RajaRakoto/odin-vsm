//! `restore-worlds` and `sync-worlds` commands.
//!
//! Mirrors `cmd_restore_worlds` and `cmd_sync_worlds` from `odin.sh`.

use crate::{
    config::AppConfig,
    error::{Error, Result},
    utils::{
        display::{confirm, err, info, ok, section, separator_n, warn},
        fs::{file_mtime_str, sudo_mkdir_p, sudo_rm_rf},
    },
};
use colored::Colorize;
use std::{fs, path::Path, process::Command};

const CONTAINER: &str = "valheim-server";

// =============================================================================
// RESTORE WORLDS
// =============================================================================

pub async fn run_restore(config: &AppConfig) -> Result<()> {
    section("Restore Worlds — Interactive backup selection");
    let sep = 44;
    let backups_dir = config.backups_dir();
    println!();

    let mut files: Vec<_> = match fs::read_dir(&backups_dir) {
        Ok(rd) => rd
            .flatten()
            .filter(|e| {
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && e.file_name().to_string_lossy().starts_with("worlds-")
            })
            .map(|e| e.path())
            .collect(),
        Err(_) => vec![],
    };
    files.sort();

    if files.is_empty() {
        warn(&format!("No backups found in {}.", backups_dir.display()));
        info("Backups are created automatically by odin clear-mods.");
        return Ok(());
    }

    let total = files.len();
    let latest_idx = total - 1;

    println!(
        "  {}Available backups in \x1b[0;36m{}\x1b[0m{}:",
        "".bold(),
        backups_dir.display(),
        "".bold()
    );
    separator_n(sep);

    for (i, f) in files.iter().enumerate() {
        let bname = f.file_name().unwrap_or_default().to_string_lossy().to_string();
        let bdate = file_mtime_str(f);
        let num = i + 1;
        if i == latest_idx {
            println!(
                "  \x1b[0;32m\x1b[1m{:2})  {:<38}  {}  ◀ latest\x1b[0m",
                num, bname, bdate
            );
        } else {
            println!("  \x1b[0;36m{:2})\x1b[0m  {:<38}  \x1b[0;36m{}\x1b[0m", num, bname, bdate);
        }
    }

    separator_n(sep);
    println!("  \x1b[0;36m◀ latest\x1b[0m = most recent backup (recommended)");
    println!("  \x1b[0;36mNumbers\x1b[0m  = enter the number of the backup to restore");
    println!();

    print!("  \x1b[1mEnter backup number to restore [1-{total}]: \x1b[0m");
    use std::io::Write;
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    let selection: usize = input.parse().unwrap_or(0);
    if selection < 1 || selection > total {
        return Err(Error::validation(format!(
            "Invalid selection: '{input}'. Must be a number between 1 and {total}."
        )));
    }

    let chosen = &files[selection - 1];
    let chosen_name = chosen.file_name().unwrap_or_default().to_string_lossy().to_string();
    let chosen_date = file_mtime_str(chosen);

    println!();
    info(&format!("Selected: {}", chosen_name.bold()));
    info(&format!("Created : {chosen_date}"));
    println!();

    if !confirm("\x1b[1;33mRestore this backup to ./config/worlds_local? (y/N)\x1b[0m") {
        warn("Restore cancelled. No changes made.");
        return Ok(());
    }

    let worlds_local = config.worlds_local_dir();
    extract_backup(chosen, &worlds_local)?;

    println!();
    ok(&format!("World restored successfully from {}.", chosen_name.bold()));
    ok("Your Valheim progress has been restored.");
    println!();
    println!("  \x1b[1;33m▶  Next step:\x1b[0m Run {} to launch the server.", "odin start".bold());
    Ok(())
}

// =============================================================================
// SYNC WORLDS
// =============================================================================

pub async fn run_sync(config: &AppConfig, help_guide: bool) -> Result<()> {
    if help_guide {
        print_sync_guide();
        return Ok(());
    }

    require_cmd("rclone")?;
    require_cmd("ssh")?;

    section("Sync Worlds  [ Windows → Linux ]");

    info("Checking environment variables…");
    check_sync_env(config)?;

    println!();
    info(&format!("Windows host    : {}", config.win_host));
    info(&format!("SSH user         : {}  (port {})", config.win_ssh_user, config.win_ssh_port));
    info(&format!("SSH key          : {}", config.win_ssh_key.display()));
    info(&format!("Source (Windows) : {}", config.worlds_src_remote()));
    info(&format!("Destination      : {}", config.worlds_local_dir().display()));
    println!();

    section("Pre-flight checks");
    check_no_players()?;
    check_client_files_unlocked(config)?;

    println!();
    println!("  \x1b[1;36m╔══════════════════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[1;36m║                  ⚠   SYNC WARNING   ⚠                        ║\x1b[0m");
    println!("  \x1b[1;36m╚══════════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("  \x1b[1;33mThis operation will perform a \x1b[1mdestructive one-way sync\x1b[0m\x1b[1;33m:\x1b[0m");
    println!();
    println!(
        "  \x1b[0;36m  Source  →\x1b[0m  Windows : \x1b[1m{}\x1b[0m",
        config.worlds_src_remote()
    );
    println!(
        "  \x1b[0;31m  Dest    →\x1b[0m  Server  : \x1b[1m{}\x1b[0m",
        config.worlds_local_dir().display()
    );
    println!();
    println!("  \x1b[1;33m  · All files on the server will be \x1b[1moverwritten\x1b[0m\x1b[1;33m.\x1b[0m");
    println!("  \x1b[1;33m  · Files absent from Windows will be \x1b[1mdeleted\x1b[0m\x1b[1;33m on the server.\x1b[0m");
    println!("  \x1b[1;33m  · Run \x1b[1modin backup\x1b[0m\x1b[1;33m first if you need a server-side snapshot.\x1b[0m");
    println!();
    println!("  \x1b[1mPre-flight checks passed.  The server is ready to receive the sync.\x1b[0m");
    println!();

    if !confirm("\x1b[0;31m\x1b[1mProceed with destructive sync? This cannot be undone. [y/N]\x1b[0m") {
        warn("Sync cancelled by user.");
        return Ok(());
    }

    section("Preparing destination");
    let dst = config.worlds_local_dir();
    if !dst.exists() {
        info(&format!("Creating {}…", dst.display()));
        sudo_mkdir_p(&dst)?;
        ok("Directory created.");
    } else {
        ok("Destination directory exists.");
    }

    section("Transferring files");

    let sftp_src = format!(":sftp:{}", config.worlds_src_remote());
    let dst_str = dst.to_string_lossy().to_string();
    let port_str = config.win_ssh_port.to_string();

    let status = Command::new("rclone")
        .args([
            "sync",
            &sftp_src,
            &dst_str,
            &format!("--sftp-host={}", config.win_host),
            &format!("--sftp-user={}", config.win_ssh_user),
            &format!("--sftp-port={port_str}"),
            &format!("--sftp-key-file={}", config.win_ssh_key.display()),
            "--sftp-shell-type=cmd",
            "--checksum",
            "--transfers=4",
            "--progress",
            "-v",
        ])
        .status()
        .map_err(|e| Error::other(format!("rclone: {e}")))?;

    println!();
    if !status.success() {
        return Err(Error::other("rclone sync failed"));
    }
    ok("Sync complete — server worlds_local is now an exact mirror of Windows.");
    println!();
    info(&format!("You can start the server with:  {}", "odin start".bold()));
    println!();
    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

fn extract_backup(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        sudo_rm_rf(dst)?;
        info("Removed existing worlds_local.");
    }
    sudo_mkdir_p(dst)?;
    info(&format!("Extracting {} → {}…", src.display(), dst.display()));

    let status = Command::new("7z")
        .args(["x", &src.to_string_lossy(), &format!("-o{}", dst.display()), "-y"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| Error::other(format!("7z: {e}")))?;

    if !status.success() {
        return Err(Error::other(format!(
            "Extraction failed. Please restore manually from: {}",
            src.display()
        )));
    }
    Ok(())
}

fn require_cmd(cmd: &str) -> Result<()> {
    let found = Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !found {
        return Err(Error::other(format!("Missing command: {cmd}")));
    }
    Ok(())
}

fn check_sync_env(config: &AppConfig) -> Result<()> {
    let mut ok_flag = true;

    if config.win_user.is_empty() {
        err("Variable WIN_USER is not set in valheim.env.");
        ok_flag = false;
    }
    if config.win_host.is_empty() {
        err("Variable WIN_HOST is not set in valheim.env.");
        ok_flag = false;
    }
    if config.win_ssh_user.is_empty() {
        err("Variable WIN_SSH_USER is not set in valheim.env.");
        ok_flag = false;
    }
    if !config.win_ssh_key.as_os_str().is_empty() && !config.win_ssh_key.exists() {
        err(&format!("SSH private key not found: {}", config.win_ssh_key.display()));
        ok_flag = false;
    }

    if !ok_flag {
        return Err(Error::validation(
            "Missing required variables — see sync-worlds --help-guide.",
        ));
    }
    Ok(())
}

fn check_no_players() -> Result<()> {
    let state = crate::commands::docker::container_state(CONTAINER);
    if state != "running" {
        ok("Server container is not running — safe to sync.");
        return Ok(());
    }

    let logs = Command::new("docker")
        .args(["logs", "--tail=200", CONTAINER])
        .output();

    let mut count = 0u32;
    if let Ok(out) = logs {
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        for line in text.lines().rev() {
            let lower = line.to_lowercase();
            if lower.contains("there are") && lower.contains("player") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, w) in parts.iter().enumerate() {
                    if w.to_lowercase() == "are" {
                        if let Some(n) = parts.get(i + 1).and_then(|s| s.parse::<u32>().ok()) {
                            count = n;
                            break;
                        }
                    }
                }
                break;
            }
        }
    }

    if count > 0 {
        return Err(Error::validation(format!(
            "There are {count} player(s) currently connected. Ask them to disconnect first."
        )));
    }
    ok("No players connected — safe to sync.");
    Ok(())
}

fn check_client_files_unlocked(config: &AppConfig) -> Result<()> {
    let port_str = config.win_ssh_port.to_string();
    let key_str = config.win_ssh_key.to_string_lossy().to_string();

    let ssh_opts: &[&str] = &[
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=10",
        "-o", "StrictHostKeyChecking=accept-new",
        "-p", &port_str,
        "-i", &key_str,
    ];

    let remote = format!("{}@{}", config.win_ssh_user, config.win_host);

    let out = Command::new("ssh")
        .args(ssh_opts)
        .arg(&remote)
        .arg(r#"tasklist /FI "IMAGENAME eq valheim.exe" /NH 2>nul"#)
        .output();

    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout).to_lowercase();
        if text.contains("valheim.exe") {
            return Err(Error::validation(format!(
                "Valheim.exe is currently running on Windows ({}). Close it first.",
                config.win_host
            )));
        }
    }
    ok("Valheim is not running on Windows — save files are unlocked.");
    Ok(())
}

fn print_sync_guide() {
    println!();
    println!("  \x1b[1;36m╔══════════════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[1;36m║          sync-worlds  —  Quick Guide                     ║\x1b[0m");
    println!("  \x1b[1;36m╚══════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("  \x1b[1mPurpose\x1b[0m");
    println!("    One-way destructive sync of Valheim save files from a Windows");
    println!("    machine to this Linux server via rclone SFTP.");
    println!();
    println!("  \x1b[1;36m── Required tools ───────────────────────────────────────────\x1b[0m");
    println!("    \x1b[0;36mrclone\x1b[0m       File transfer over SFTP");
    println!("    \x1b[0;36mtailscale\x1b[0m    Encrypted VPN tunnel");
    println!("    \x1b[0;36mssh\x1b[0m          Pre-flight check: verifies Valheim.exe is not running");
    println!();
    println!("  \x1b[1;36m── Required variables in valheim.env ────────────────────────\x1b[0m");
    println!("    \x1b[0;36mWIN_USER\x1b[0m       Windows account name");
    println!("    \x1b[0;36mWIN_HOST\x1b[0m       Windows machine IP (Tailscale IP recommended)");
    println!("    \x1b[0;36mWIN_SSH_USER\x1b[0m   SSH login on Windows");
    println!("    \x1b[0;36mWIN_SSH_PORT\x1b[0m   SSH port on Windows (default: 22)");
    println!("    \x1b[0;36mWIN_SSH_KEY\x1b[0m    Absolute path to the private key on this server");
    println!();
    println!("  \x1b[1;36m── Recommended workflow ─────────────────────────────────────\x1b[0m");
    println!("    \x1b[1;33m1.\x1b[0m  Close Valheim on Windows");
    println!("    \x1b[1;33m2.\x1b[0m  \x1b[0;36modin backup\x1b[0m");
    println!("    \x1b[1;33m3.\x1b[0m  \x1b[0;36modin sync-worlds\x1b[0m  ← destructive");
    println!("    \x1b[1;33m4.\x1b[0m  \x1b[0;36modin start\x1b[0m");
    println!();
    println!("  \x1b[0;31m⚠  The server destination is overwritten and extra files are deleted.\x1b[0m");
    println!();
}
