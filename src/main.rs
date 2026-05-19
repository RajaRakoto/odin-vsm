//! Binary entry point — bootstraps env/config and dispatches CLI commands.

use clap::Parser;
use odin::{
    cli::{Cli, Commands, FixSub},
    commands,
    config::AppConfig,
    utils::banner::{print_banner, print_help},
};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // Load valheim.env from the directory where the binary lives (or CWD).
    let script_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let env_path = script_dir.join("valheim.env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    } else {
        dotenvy::dotenv().ok();
    }

    let config = match AppConfig::from_env(&script_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  ✘ Config error: {e}");
            std::process::exit(1);
        }
    };

    // Initialize logging (RUST_LOG controls verbosity).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .without_time()
        .with_target(false)
        .init();

    // No arguments → show banner + custom help and exit cleanly.
    if std::env::args().len() == 1 {
        print_banner();
        print_help();
        std::process::exit(0);
    }

    let cli = Cli::parse();

    // Most commands print the banner; raw pass-throughs (logs, shell) skip it.
    let show_banner = !matches!(cli.command, Commands::Logs { .. } | Commands::Shell);
    if show_banner {
        print_banner();
    }

    if let Err(e) = dispatch(cli, &config).await {
        eprintln!("  ✘ {e}");
        std::process::exit(1);
    }
}

async fn dispatch(cli: Cli, config: &AppConfig) -> odin::error::Result<()> {
    match cli.command {
        // ── Diagnostic ───────────────────────────────────────────────────────
        Commands::Health => commands::health::run(config).await,

        // ── Fixes ────────────────────────────────────────────────────────────
        Commands::Fix { sub } => match sub {
            FixSub::Permission => commands::fix::run_permission(config).await,
        },

        // ── Docker server ────────────────────────────────────────────────────
        Commands::Start => {
            if config.apply_dll_patch {
                commands::patch::run_apply(config).await?;
            }
            commands::docker::run_start(config).await
        }
        Commands::Stop => commands::docker::run_stop(config).await,
        Commands::Restart => commands::docker::run_restart(config).await,
        Commands::Down => commands::docker::run_down(config).await,
        Commands::Logs { lines } => commands::docker::run_logs(config, lines).await,
        Commands::Update => commands::docker::run_update(config).await,
        Commands::Backup => commands::docker::run_backup(config).await,
        Commands::Snapshot => commands::docker::run_snapshot(config).await,
        Commands::Shell => commands::docker::run_shell(config).await,

        // ── Status ───────────────────────────────────────────────────────────
        Commands::Status => commands::status::run(config, false).await,
        Commands::StatusPassword => commands::status::run(config, true).await,

        // ── Backups ──────────────────────────────────────────────────────────
        Commands::ClearBackups => commands::backups::run_clear(config).await,

        // ── Worlds ───────────────────────────────────────────────────────────
        Commands::RestoreWorlds => commands::worlds::run_restore(config).await,
        Commands::SyncWorlds { help_guide } => {
            commands::worlds::run_sync(config, help_guide).await
        }

        // ── Mods ─────────────────────────────────────────────────────────────
        Commands::FilterMods => commands::mods::run_filter(config).await,
        Commands::DownloadMods => commands::mods::run_download(config).await,
        Commands::InstallMods => commands::mods::run_install(config).await,
        Commands::ClearMods => commands::mods::run_clear(config).await,

        // ── DLL patch ────────────────────────────────────────────────────────
        Commands::ApplyPatch => commands::patch::run_apply(config).await,
        Commands::VerifyPatch => commands::patch::run_verify(config).await,
    }
}
