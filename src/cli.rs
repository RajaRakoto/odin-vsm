//! CLI interface for odin — Valheim Server Manager.
//!
//! Mirrors every sub-command from the original `odin.sh` Bash script.

use clap::{Parser, Subcommand};

/// odin — Valheim Server Manager
#[derive(Parser, Debug)]
#[command(
    name = "odin",
    version,
    author,
    about = "Valheim Server Manager — manage your Dockerized Valheim server"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// All available sub-commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ── Diagnostic ────────────────────────────────────────────────────────────
    /// Full environment diagnostic (system, Docker, config, ports, mods, …).
    /// Recommended before first use.
    Health,

    // ── Fixes ─────────────────────────────────────────────────────────────────
    /// Apply quick fixes for known issues.
    Fix {
        #[command(subcommand)]
        sub: FixSub,
    },

    // ── Docker server ─────────────────────────────────────────────────────────
    /// Start the server (docker compose up -d).
    Start,

    /// Graceful stop — waits up to 2 minutes for the world to save.
    Stop,

    /// Restart the container.
    Restart,

    /// Remove the container (config/ and data/ volumes are preserved).
    Down,

    /// Stream container logs.
    Logs {
        /// Number of lines to show (default: 50).
        #[arg(default_value = "50")]
        lines: usize,
    },

    /// Show full server status (passwords hidden).
    Status,

    /// Show full server status with passwords revealed.
    StatusPassword,

    /// Pull the latest Docker image and restart.
    Update,

    /// Trigger a manual backup via Supervisor.
    Backup,

    /// Delete all backup files in config/backups/ (interactive).
    ClearBackups,

    /// Archive the project to ~/valheim-server.bak.zip.
    Snapshot,

    /// Open an interactive shell inside the container.
    Shell,

    // ── Worlds ────────────────────────────────────────────────────────────────
    /// Interactively list and restore a world backup from config/backups/.
    RestoreWorlds,

    /// Sync worlds from Windows to Linux via rclone SFTP (destructive).
    SyncWorlds {
        /// Show the sync-worlds setup guide instead of running the sync.
        #[arg(long)]
        help_guide: bool,
    },

    // ── Mods ─────────────────────────────────────────────────────────────────
    /// Query Thunderstore API and classify each mod (server/client/both/unknown).
    FilterMods,

    /// Download all mods in mods_list.txt to mods_cache/ (no extraction).
    DownloadMods,

    /// Download (if needed) and install mods from mods_list.txt to plugins/.
    InstallMods,

    /// Full cleanup: docker down, world backup, interactive deletion.
    ClearMods,

    // ── DLL patch ────────────────────────────────────────────────────────────
    /// Copy patches/assembly_valheim.dll into the running container (idempotent).
    ApplyPatch,

    /// Verify whether the patched DLL is active inside the container.
    VerifyPatch,
}

/// Sub-commands for `fix`.
#[derive(Subcommand, Debug)]
pub enum FixSub {
    /// Fix ownership and permissions on ./data and ./config (chown 1000:1000, chmod 755).
    Permission,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_start() {
        let cli = Cli::try_parse_from(["odin", "start"]).unwrap();
        assert!(matches!(cli.command, Commands::Start));
    }

    #[test]
    fn parse_logs_default() {
        let cli = Cli::try_parse_from(["odin", "logs"]).unwrap();
        match cli.command {
            Commands::Logs { lines } => assert_eq!(lines, 50),
            _ => panic!("expected Logs"),
        }
    }

    #[test]
    fn parse_logs_custom() {
        let cli = Cli::try_parse_from(["odin", "logs", "100"]).unwrap();
        match cli.command {
            Commands::Logs { lines } => assert_eq!(lines, 100),
            _ => panic!("expected Logs"),
        }
    }

    #[test]
    fn parse_fix_permission() {
        let cli = Cli::try_parse_from(["odin", "fix", "permission"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Fix {
                sub: FixSub::Permission
            }
        ));
    }

    #[test]
    fn parse_sync_worlds_help() {
        let cli = Cli::try_parse_from(["odin", "sync-worlds", "--help-guide"]).unwrap();
        match cli.command {
            Commands::SyncWorlds { help_guide } => assert!(help_guide),
            _ => panic!("expected SyncWorlds"),
        }
    }

    #[test]
    fn parse_apply_patch() {
        let cli = Cli::try_parse_from(["odin", "apply-patch"]).unwrap();
        assert!(matches!(cli.command, Commands::ApplyPatch));
    }

    #[test]
    fn parse_verify_patch() {
        let cli = Cli::try_parse_from(["odin", "verify-patch"]).unwrap();
        assert!(matches!(cli.command, Commands::VerifyPatch));
    }
}