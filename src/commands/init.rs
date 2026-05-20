//! `odin init` — interactive wizard to bootstrap a new Valheim server.
//!
//! Fetches docker-compose.yaml, valheim.env.example, and the full scripts/
//! directory from the upstream GitHub repository, prompts the user for key
//! values, and writes ready-to-use files in the current directory.

use crate::error::{Error, Result};
use colored::Colorize;
use dialoguer::{Confirm, Input, Password};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const REPO_RAW: &str = "https://raw.githubusercontent.com/RajaRakoto/odin-vsm/master";
const REPO_API: &str = "https://api.github.com/repos/RajaRakoto/odin-vsm/contents/scripts";

// ── Defaults ──────────────────────────────────────────────────────────────────

/// Static defaults that match the current valheim.env.example structure.
fn static_defaults() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // scheduling
    m.insert("UPDATE_CRON", "30 * * * *");
    m.insert("UPDATE_IF_IDLE", "true");
    m.insert("RESTART_CRON", "30 4 * * *");
    m.insert("RESTART_IF_IDLE", "true");
    m.insert("BACKUPS", "true");
    m.insert("BACKUPS_CRON", "5 * * * *");
    m.insert("BACKUPS_DIRECTORY", "/config/backups");
    m.insert("BACKUPS_MAX_AGE", "7");
    m.insert("BACKUPS_MAX_COUNT", "168");
    m.insert("BACKUPS_ZIP", "true");
    m.insert("BACKUPS_IF_IDLE", "true");
    // mods
    m.insert("BEPINEX", "false");
    m.insert("BEPINEXCFG_Logging_DOT_Console_Enabled", "false");
    m.insert("VALHEIM_PLUS", "false");
    // windows sync
    m.insert("WIN_SSH_PORT", "22");
    m.insert("WIN_SSH_KEY", "/root/.ssh/id_ed25519_valheim_win");
    // container
    m.insert("PUID", "1000");
    m.insert("PGID", "1000");
    // patch
    m.insert("APPLY_DLL_PATCH", "false");
    m.insert("PRE_SERVER_RUN_HOOK", "/scripts/apply-patch.sh");
    // extra
    m.insert("SERVER_PUBLIC", "false");
    m.insert("CROSSPLAY", "false");
    m
}

// ── Timezone detection ────────────────────────────────────────────────────────

fn detect_timezone() -> String {
    if let Ok(tz) = fs::read_to_string("/etc/timezone") {
        let tz = tz.trim().to_string();
        if !tz.is_empty() {
            return tz;
        }
    }

    if let Ok(link) = fs::read_link("/etc/localtime") {
        let s = link.to_string_lossy();
        if let Some(pos) = s.find("zoneinfo/") {
            let tz = &s[pos + "zoneinfo/".len()..];
            if !tz.is_empty() {
                return tz.to_string();
            }
        }
    }

    if let Ok(out) = std::process::Command::new("timedatectl")
        .args(["show", "--property=Timezone", "--value"])
        .output()
    {
        let tz = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !tz.is_empty() {
            return tz;
        }
    }

    "Etc/UTC".to_string()
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

async fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("odin-vsm/init")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| Error::network(e.to_string()))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::network(format!("GET {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(Error::network(format!(
            "GET {url} returned HTTP {}",
            resp.status()
        )));
    }

    resp.text().await.map_err(|e| Error::network(e.to_string()))
}

async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<bytes::Bytes> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::network(format!("GET {url}: {e}")))?;

    if !resp.status().is_success() {
        return Err(Error::network(format!(
            "GET {url} returned HTTP {}",
            resp.status()
        )));
    }

    resp.bytes()
        .await
        .map_err(|e| Error::network(e.to_string()))
}

// ── Scripts directory fetch ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GhEntry {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    download_url: Option<String>,
}

/// Fetch all files in scripts/ from GitHub and write them to ./scripts/.
/// Returns the list of filenames written.
async fn fetch_scripts(client: &reqwest::Client, dest: &Path) -> Result<Vec<String>> {
    let listing: Vec<GhEntry> = client
        .get(REPO_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| Error::network(format!("GitHub API: {e}")))?
        .json()
        .await
        .map_err(|e| Error::network(format!("GitHub API parse: {e}")))?;

    fs::create_dir_all(dest).map_err(|e| Error::other(format!("Cannot create scripts/: {e}")))?;

    let mut written = Vec::new();

    for entry in listing {
        if entry.kind != "file" {
            continue;
        }
        let url = match entry.download_url {
            Some(ref u) => u.clone(),
            None => continue,
        };

        let content = fetch_bytes(client, &url).await?;
        let file_path = dest.join(&entry.name);
        fs::write(&file_path, &content)
            .map_err(|e| Error::other(format!("Write {}: {e}", file_path.display())))?;

        // Preserve executable bit for shell scripts
        #[cfg(unix)]
        if entry.name.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&file_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&file_path, perms)?;
        }

        written.push(entry.name);
    }

    Ok(written)
}

// ── Env file rendering ────────────────────────────────────────────────────────

/// Substitute values into the env template, preserving all comments and structure.
fn render_env(template: &str, values: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(template.len() + 256);

    for line in template.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with('#') || trimmed.is_empty() {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            if let Some(val) = values.get(key) {
                out.push_str(key);
                out.push('=');
                out.push_str(val);
                out.push('\n');
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

// ── Interactive prompts ───────────────────────────────────────────────────────

fn prompt_str(prompt: &str, default: &str) -> Result<String> {
    Input::<String>::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()
        .map_err(|e| Error::other(e.to_string()))
}

fn prompt_password(prompt: &str) -> Result<String> {
    Password::new()
        .with_prompt(prompt)
        .with_confirmation("Confirm password", "Passwords do not match")
        .interact()
        .map_err(|e| Error::other(e.to_string()))
}

fn prompt_confirm(prompt: &str, default: bool) -> Result<bool> {
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|e| Error::other(e.to_string()))
}

// ── Public entry point ────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::other(format!("Cannot determine current directory: {e}")))?;

    println!();
    println!(
        "  {} {}",
        "odin init".cyan().bold(),
        "— Valheim Server Setup Wizard".bold()
    );
    println!("  {}", "─".repeat(44).cyan());
    println!();
    println!("  {} Fetching latest files from GitHub…", "→".cyan());

    let client = build_client().await?;

    let compose_url = format!("{REPO_RAW}/docker-compose.yaml");
    let env_url = format!("{REPO_RAW}/valheim.env.example");

    let (compose_content, env_template) = tokio::try_join!(
        fetch_text(&client, &compose_url),
        fetch_text(&client, &env_url)
    )?;

    println!("  {} docker-compose.yaml", "✔".green());
    println!("  {} valheim.env.example", "✔".green());
    println!();

    // ── Guard: existing files ─────────────────────────────────────────────────
    let compose_dest = cwd.join("docker-compose.yaml");
    let env_dest = cwd.join("valheim.env");
    let scripts_dest = cwd.join("scripts");

    let has_existing = compose_dest.exists() || env_dest.exists() || scripts_dest.exists();
    if has_existing {
        println!(
            "  {} Files/directories already exist in this directory:",
            "!".yellow().bold()
        );
        if compose_dest.exists() {
            println!("    • {}", "docker-compose.yaml".yellow());
        }
        if env_dest.exists() {
            println!("    • {}", "valheim.env".yellow());
        }
        if scripts_dest.exists() {
            println!("    • {}", "scripts/".yellow());
        }
        println!();
        let overwrite = prompt_confirm("Overwrite existing files?", false)?;
        if !overwrite {
            println!("  {} Aborted.", "✘".red());
            return Ok(());
        }
        println!();
    }

    // ── Detect timezone ───────────────────────────────────────────────────────
    let detected_tz = detect_timezone();

    // ── Interactive prompts ───────────────────────────────────────────────────
    println!(
        "  {}",
        "── Server Identity ──────────────────────────".bold()
    );
    println!();

    let server_name = prompt_str("  Server name (shown in browser)", "My Valheim Server")?;
    let world_name = prompt_str("  World name (save file, no extension)", "Dedicated")?;

    println!();
    println!(
        "  {}",
        "Server password must be at least 5 characters.".dimmed()
    );
    let server_pass = loop {
        let p = prompt_password("  Server password")?;
        if p.len() >= 5 {
            break p;
        }
        println!("  {} Password too short (minimum 5 characters).", "✘".red());
    };

    println!();
    println!(
        "  {}",
        "── Timezone ─────────────────────────────────".bold()
    );
    println!();
    println!("  {} Detected: {}", "→".cyan(), detected_tz.cyan());
    let tz = prompt_str("  Timezone (tz database name)", &detected_tz)?;

    println!();
    println!(
        "  {}",
        "── Optional: Windows World Sync ─────────────".bold()
    );
    println!();
    let setup_win_sync = prompt_confirm(
        "  Configure Windows → Linux world sync (odin sync-worlds)?",
        false,
    )?;

    let (win_user, win_host, win_ssh_user) = if setup_win_sync {
        println!();
        let wu = prompt_str("  Windows account name (C:\\Users\\<name>)", "")?;
        let wh = prompt_str("  Windows machine IP or hostname", "")?;
        let wsu = prompt_str("  SSH username on Windows", &wu)?;
        (wu, wh, wsu)
    } else {
        (String::new(), String::new(), String::new())
    };

    // ── Build values map ──────────────────────────────────────────────────────
    let defaults = static_defaults();
    let mut values: HashMap<&str, String> =
        defaults.iter().map(|(&k, &v)| (k, v.to_string())).collect();

    values.insert("SERVER_NAME", server_name);
    values.insert("WORLD_NAME", world_name);
    values.insert("SERVER_PASS", server_pass);
    values.insert("TZ", tz);

    if setup_win_sync {
        values.insert("WIN_USER", win_user);
        values.insert("WIN_HOST", win_host);
        values.insert("WIN_SSH_USER", win_ssh_user);
    }

    // ── Write config files ────────────────────────────────────────────────────
    write_file(&compose_dest, compose_content.as_bytes())?;
    write_file(&env_dest, render_env(&env_template, &values).as_bytes())?;

    // ── Fetch and write scripts/ ──────────────────────────────────────────────
    println!();
    println!("  {} Fetching scripts/ from GitHub…", "→".cyan());

    let script_files = fetch_scripts(&client, &scripts_dest).await?;
    for name in &script_files {
        println!("  {} scripts/{}", "✔".green(), name);
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    println!();
    println!("  {}", "─".repeat(44).cyan());
    println!();
    println!("  {} Setup complete!", "✔".green().bold());
    println!();
    println!("  Files written:");
    println!("    • docker-compose.yaml");
    println!("    • valheim.env");
    for name in &script_files {
        println!("    • scripts/{name}");
    }
    println!();
    println!("  Next steps:");
    println!(
        "    {}  Review valheim.env and adjust any remaining values.",
        "1.".cyan()
    );
    println!(
        "    {}  Run `odin health` to verify your environment.",
        "2.".cyan()
    );
    println!(
        "    {}  Run `odin start` to launch the server.",
        "3.".cyan()
    );
    println!();

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_file(path: &Path, content: &[u8]) -> Result<()> {
    fs::write(path, content)
        .map_err(|e| Error::other(format!("Failed to write {}: {e}", path.display())))
}
