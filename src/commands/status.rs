//! `status` and `status-password` commands.
//!
//! Mirrors `cmd_status` from `odin.sh`.

use crate::{
    config::{bool_label, bool_onoff, cron_human, AppConfig},
    error::Result,
    utils::{
        display::warn,
        net::{external_ip, internal_ips},
    },
};
use colored::Colorize;
use std::process::Command;

const CONTAINER: &str = "valheim-server";

pub async fn run(config: &AppConfig, show_passwords: bool) -> Result<()> {
    // Container state
    let container_state = crate::commands::docker::container_state(CONTAINER);

    if !config.env_file().exists() {
        warn(&format!(
            "File {} not found — some values will be N/A.",
            config.env_file().display()
        ));
    }

    // Formatted server status
    let server_status = match container_state.as_str() {
        "running" => "Active and Running".green().bold().to_string(),
        "exited" => "Stopped (exited)".red().bold().to_string(),
        "absent" => "Container not found".red().bold().to_string(),
        other => other.yellow().bold().to_string(),
    };

    // Passwords: mask or reveal
    let display_pass = if show_passwords {
        config.server_pass.clone().yellow().bold().to_string()
    } else {
        "*".repeat(config.server_pass.len())
    };
    let display_sup_pass = if show_passwords {
        config.supervisor_http_pass.clone().yellow().bold().to_string()
    } else {
        "*".repeat(config.supervisor_http_pass.len())
    };

    // IPs
    let ext_ip = if container_state == "running" {
        external_ip(CONTAINER)
    } else {
        "N/A (server offline)".into()
    };
    let int_ip = internal_ips();

    // Port from docker inspect (fallback 2456)
    let server_port = detect_port().unwrap_or_else(|| "2456".into());

    // Cron human labels
    let update_human = if config.update_cron.is_empty() {
        "Disabled".into()
    } else {
        cron_human(&config.update_cron)
    };
    let restart_human = if config.restart_cron.is_empty() {
        "Disabled".into()
    } else {
        cron_human(&config.restart_cron)
    };
    let backup_human = if config.backups_cron.is_empty() {
        "Disabled".into()
    } else {
        cron_human(&config.backups_cron)
    };

    let sep = format!("  {}", "─".repeat(52).cyan());
    let title = if show_passwords {
        "       Valheim Server — Status (passwords visible)  "
    } else {
        "           Valheim Server — Status                "
    };

    println!();
    println!("  \x1b[1;36m╔══════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[1;36m║{}\x1b[1;36m║\x1b[0m", title);
    println!("  \x1b[1;36m╚══════════════════════════════════════════════════╝\x1b[0m");
    println!("{sep}");

    row("Current Valheim Instance:", &config.server_name);
    row("Server Status:", &server_status);
    println!("{sep}");
    row("Server Name:", &config.server_name);
    row("Server Password:", &display_pass);
    row("World Name:", &config.world_name);
    println!("{sep}");
    row("Supervisor:", bool_label(config.supervisor_http));
    row("Supervisor Password:", &display_sup_pass);
    row("BepInEx:", bool_label(config.bepinex));
    println!("{sep}");
    row("Internet:", "Enabled");
    row("External IP:", &ext_ip);
    row("Internal IP:", &int_ip);
    row("Server Port:", &server_port);
    row("Timezone:", &config.tz);
    println!("{sep}");
    row("Auto Update  (server):", &update_human);
    row("Auto Restart (server):", &restart_human);
    row("Auto Backup  (server):", &backup_human);
    println!("{sep}");
    row("Public Listing:", bool_onoff(config.server_public));
    row("Crossplay status:", bool_label(config.crossplay));
    println!("{sep}");

    if show_passwords {
        println!("  \x1b[1;33m⚠  Passwords are visible. Keep this output private.\x1b[0m");
    } else {
        println!(
            "  \x1b[1;33m⚠  Passwords hidden — use\x1b[0m {} \x1b[1;33mto reveal them.\x1b[0m",
            "odin status-password".bold()
        );
    }
    println!();
    Ok(())
}

fn row(label: &str, value: &str) {
    println!("  {:<28}  {}", label.bold(), value);
}

fn detect_port() -> Option<String> {
    let out = Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{range $p,$conf := .NetworkSettings.Ports}}{{$p}} {{end}}",
            CONTAINER,
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    // Find first "NNNN/udp" pattern
    for token in s.split_whitespace() {
        if token.ends_with("/udp") {
            if let Some(port) = token.split('/').next() {
                return Some(port.to_string());
            }
        }
    }
    None
}