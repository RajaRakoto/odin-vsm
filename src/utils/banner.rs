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
    cmd("filter-mods", "Query Thunderstore API and classify each mod as:");
    sub("server-side / both / client-only / unknown");
    sub("(entries marked * are ignored, ** are forced as both)");
    sub("Generates mods_list.filtered.txt and updates mods_list.txt");
    arrow("Step 1: run this on your raw unfiltered mods_list.txt");

    cmd("download-mods", "Download all mods listed in mods_list.txt to mods_cache/");
    sub("Resolves the latest version via Thunderstore API (ignores");
    sub("the version number in mods_list.txt — always fetches latest)");
    sub("(entries marked * are skipped, ** are treated as normal)");
    sub("Validates zip integrity; no extraction performed");
    arrow("Step 2 (optional): pre-populate cache before install");

    cmd("install-mods", "Download (if needed) and install mods listed in mods_list.txt");
    sub("Resolves the latest version via Thunderstore API (ignores");
    sub("the version number in mods_list.txt — always fetches latest)");
    sub("(entries marked * are skipped, ** are treated as normal)");
    sub("Calls download-mods internally before any extraction");
    arrow("Step 3: installs server-side / both / unknown mods");

    cmd("clear-mods", "Full cleanup with auto world backup (5 steps):");
    sub("· docker down if server is running");
    sub("· backup worlds_local → config/backups/ (auto)");
    sub("· mods_list.txt / mods_cache/ / data/ / config/bepinex/");
    sub("· offer to restore the latest world backup");
    println!();

    section_header("DLL Patch");
    cmd("apply-patch", "Copy patches/assembly_valheim.dll into the container");
    sub("Idempotent: skipped if checksums already match");
    sub("Requires: patches/assembly_valheim.dll on the host");
    arrow("Also runs automatically on 'odin start' when APPLY_DLL_PATCH=true");

    cmd("verify-patch", "Check whether the patched DLL is active in the container");
    sub("Compares MD5 of local patch source vs DLL inside the container");
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
