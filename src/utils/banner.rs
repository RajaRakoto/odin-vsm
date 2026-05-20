//! ASCII-art startup banner and custom help output.

use colored::Colorize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print the odin startup banner to stdout.
pub fn print_banner() {
    println!();
    println!("  {}", " ██████╗ ██████╗ ██╗███╗  ██╗".cyan().bold());
    println!("  {}", "██╔═══██╗██╔══██╗██║████╗ ██║".cyan().bold());
    println!("  {}", "██║   ██║██║  ██║██║██╔██╗██║".cyan().bold());
    println!("  {}", "██║   ██║██║  ██║██║██║╚████║".cyan().bold());
    println!("  {}", " ██████╔╝██████╔╝██║██║ ╚███║".cyan().bold());
    println!("  {}", " ╚═════╝ ╚═════╝ ╚═╝╚═╝  ╚══╝".cyan().bold());
    println!();
    println!("  {}", "      Valheim Server Manager".bold());
    println!(
        "  {}  {}  {}",
        format!("      v{VERSION}").cyan(),
        "·".yellow().bold(),
        "by Z3R0D4Y".cyan()
    );
    println!();
    println!("  {}", "─".repeat(34).cyan());
    println!();
}

/// Print the full custom help page (banner already printed by caller).
pub fn print_help() {
    let u = "Usage:".bold();
    println!("  {u} ./odin {}", "<command> [options]".cyan());
    println!();

    section_header("Setup");
    cmd("init", "Bootstrap a new Valheim server interactively");
    sub("Fetches latest docker-compose.yaml + valheim.env.example from GitHub");
    sub("Prompts for SERVER_NAME, WORLD_NAME, SERVER_PASS, TZ, …");
    arrow("Run once in an empty directory before first use");
    println!();

    section_header("Diagnostic");
    cmd("health", "Full environment diagnostic");
    sub("Checks: system, dependencies, Docker, volumes,");
    sub("config files, ports, network, steamcmd fixes");
    arrow("Recommended before first use");
    println!();

    section_header("Fixes");
    cmd("fix <sub>", "Apply a quick fix for known issues");
    cmd("fix permission", "Fix ownership and permissions on ./data and ./config");
    arrow("Use when the container cannot write to volumes");
    println!();

    section_header("Docker Server");
    cmd("start", "Start the server (docker compose up -d)");
    cmd("stop", "Graceful stop (waits for save, max 2 min)");
    cmd("restart", "Restart the container");
    cmd("down", "Remove the container (volumes preserved)");
    cmd("logs [N]", "Stream logs (default: 50 lines)");
    cmd("status", "Show full server status (passwords hidden)");
    cmd("status-password", "Show full server status with passwords revealed");
    cmd("update", "Pull latest image and restart");
    cmd("backup", "Trigger a manual backup");
    cmd("clear-backups", "Delete all files in config/backups/ (with confirmation)");
    cmd("snapshot", "Archive the project to ~/valheim-server.bak.zip");
    cmd("shell", "Open a shell inside the container");
    println!();

    section_header("Worlds");
    cmd("restore-worlds", "Interactively list and restore a world backup");
    sub("from config/backups/ (numbered list, latest highlighted)");
    cmd("sync-worlds", "Sync worlds from Windows to Linux via rclone SFTP");
    sub("Checks: no players connected, Valheim.exe not running on Windows");
    sub("Requires: WIN_HOST, WIN_SSH_USER, WIN_SSH_KEY in valheim.env");
    arrow("Destructive: server worlds are overwritten with Windows save");
    cmd("sync-worlds --help-guide", "Show the sync-worlds quick-guide (setup & workflow)");
    println!();

    section_header("Mods");
    cmd("filter-mods", "Classify mods (server-side / both / client-only / unknown)");
    sub("Queries Thunderstore API; * = ignore, ** = force as both");
    arrow("Step 1: run on your raw mods_list.txt");

    cmd("download-mods", "Download all mods in mods_list.txt to mods_cache/");
    sub("Always fetches latest version from Thunderstore API");
    arrow("Step 2 (optional): pre-populate cache before install");

    cmd("install-mods", "Download and install mods from mods_list.txt to plugins/");
    sub("Installs server-side and both-side mods only");
    arrow("Step 3: run after filter-mods");

    cmd("clear-mods", "Full cleanup: stop server, backup worlds, remove mods");
    arrow("Interactive — choose what to delete");
    println!();

    section_header("DLL Patch");
    cmd("apply-patch", "Apply APPLY_DLL_PATCH change from valheim.env");
    sub("Recreates the container (down + start) so docker-compose");
    sub("re-reads valheim.env with the new APPLY_DLL_PATCH value.");
    sub("The PRE_SERVER_RUN_HOOK then applies or skips the patch");
    sub("automatically on Valheim startup.");
    arrow("Required after every APPLY_DLL_PATCH change in valheim.env");
    arrow("docker restart alone does NOT re-read valheim.env");

    cmd("verify-patch", "Check whether the patched DLL is active in the container");
    sub("Compares MD5 + size of local patch source vs DLL inside container");
    arrow("Use after apply-patch to confirm the patch is in effect");
    println!();

    println!("  {}", "─".repeat(52).cyan());
    println!(
        "  {} Join Game → Join IP → {}",
        "→".cyan(),
        "localhost:2456".bold()
    );
    println!(
        "  {} Steam: View → Servers → Favorites → {}",
        "→".cyan(),
        "127.0.0.1:2457".bold()
    );
    println!();
}

// ── Internal layout helpers ───────────────────────────────────────────────────

fn section_header(title: &str) {
    let line = format!("── {} ", title);
    let total = 54usize;
    let dashes = total.saturating_sub(line.len());
    println!("  {}{}", line.bold(), "─".repeat(dashes).cyan());
}

fn cmd(name: &str, desc: &str) {
    println!("    {:<24}  {}", name.bold(), desc);
}

fn sub(text: &str) {
    println!("    {}  {}", " ".repeat(24), text.dimmed());
}

fn arrow(text: &str) {
    println!(
        "    {}  {} {}",
        " ".repeat(24),
        "→".cyan(),
        text.cyan()
    );
}
