//! Docker server commands: start, stop, restart, down, logs, update, backup, snapshot, shell.
//!
//! Each function mirrors the corresponding `cmd_*` in `odin.sh`.

use crate::{
    config::AppConfig,
    error::Result,
    utils::display::{info, ok, warn},
};
use std::process::{Command, Stdio};

const CONTAINER: &str = "valheim-server";

// ── Guards ────────────────────────────────────────────────────────────────────

/// Abort with a helpful message if the Docker daemon is not reachable.
fn require_docker() -> Result<()> {
    let ok = Command::new("docker")
        .args(["info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return Err(crate::error::Error::docker(
            "Docker daemon is not running (sudo systemctl start docker).",
        ));
    }
    Ok(())
}

/// Run `docker compose <args>` inheriting stdout/stderr.
fn compose(args: &[&str]) -> Result<()> {
    let status = Command::new("docker")
        .arg("compose")
        .args(args)
        .status()
        .map_err(|e| crate::error::Error::docker(e.to_string()))?;
    if !status.success() {
        return Err(crate::error::Error::docker(format!(
            "docker compose {} failed (exit {:?})",
            args.join(" "),
            status.code()
        )));
    }
    Ok(())
}

// ── Commands ──────────────────────────────────────────────────────────────────

pub async fn run_start(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    info("Starting the server…");
    compose(&["up", "-d"])?;
    info("Logs: odin logs");
    Ok(())
}

pub async fn run_stop(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    info("Graceful shutdown (waiting for save, max 2 min)…");
    compose(&["stop"])?;
    Ok(())
}

pub async fn run_restart(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    compose(&["restart"])?;
    Ok(())
}

pub async fn run_down(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    warn("Removing container (config/ and data/ volumes preserved).");
    compose(&["down"])?;
    Ok(())
}

pub async fn run_logs(_config: &AppConfig, lines: usize) -> Result<()> {
    require_docker()?;
    let lines_str = lines.to_string();
    let status = Command::new("docker")
        .args(["compose", "logs", "-f", "--tail", &lines_str])
        .status()
        .map_err(|e| crate::error::Error::docker(e.to_string()))?;
    if !status.success() {
        return Err(crate::error::Error::docker("docker compose logs failed"));
    }
    Ok(())
}

pub async fn run_update(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    info("Pulling latest image and restarting…");
    compose(&["pull"])?;
    compose(&["up", "-d"])?;
    Ok(())
}

pub async fn run_backup(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    info("Triggering a manual backup…");
    let status = Command::new("docker")
        .args([
            "exec",
            CONTAINER,
            "supervisorctl",
            "restart",
            "valheim-backup",
        ])
        .status()
        .map_err(|e| crate::error::Error::docker(e.to_string()))?;
    if !status.success() {
        return Err(crate::error::Error::docker(
            "supervisorctl restart valheim-backup failed",
        ));
    }
    ok("Backup in progress — result in config/backups/");
    Ok(())
}

pub async fn run_snapshot(config: &AppConfig) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let archive = format!("{home}/valheim-server.bak.zip");
    info(&format!("Archiving project to {archive}…"));

    let script_dir = config.script_dir.to_string_lossy().to_string();
    let status = Command::new("zip")
        .args([
            "-0",
            "-r",
            &archive,
            ".",
            "--exclude",
            "*.log",
            "--exclude",
            ".git/*",
        ])
        .current_dir(&script_dir)
        .status()
        .map_err(|e| crate::error::Error::other(format!("zip: {e}")))?;

    if !status.success() {
        return Err(crate::error::Error::other("zip snapshot failed"));
    }

    // Get archive size
    let size = std::fs::metadata(&archive)
        .map(|m| format!("{:.1} MB", m.len() as f64 / 1_048_576.0))
        .unwrap_or_else(|_| "?".into());
    ok(&format!("Snapshot: {archive} ({size})"));
    Ok(())
}

pub async fn run_shell(_config: &AppConfig) -> Result<()> {
    require_docker()?;
    let status = Command::new("docker")
        .args(["exec", "-it", CONTAINER, "bash"])
        .status()
        .map_err(|e| crate::error::Error::docker(e.to_string()))?;
    if !status.success() {
        return Err(crate::error::Error::docker("docker exec shell failed"));
    }
    Ok(())
}

// ── Helpers (used by other modules) ──────────────────────────────────────────

/// Returns the container state string (running / exited / absent / …).
pub fn container_state(container: &str) -> String {
    let out = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Status}}", container])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "absent".to_string(),
    }
}

/// Run `docker compose down` (used by clear-mods and apply-patch).
pub fn compose_down() -> Result<()> {
    compose(&["down"])
}

/// Run `docker compose up -d` (used by apply-patch).
pub fn compose_up() -> Result<()> {
    compose(&["up", "-d"])
}
