//! Mod management commands: filter-mods, download-mods, install-mods, clear-mods.
//!
//! Mirrors `cmd_filter_mods`, `cmd_download_mods`, `cmd_install_mods`,
//! and `cmd_clear_mods` from `odin.sh`.

use crate::{
    api::thunderstore,
    config::AppConfig,
    error::{Error, Result},
    utils::{
        display::{confirm, err, info, ok, section, separator_n, warn},
        fs::{dir_is_empty, file_mtime_str, sudo_mkdir_p, sudo_rm_rf},
    },
};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

// ── Entry points ──────────────────────────────────────────────────────────────

pub async fn run_filter(config: &AppConfig) -> Result<()> {
    section("Filter Mods — Categorize mods via Thunderstore API");
    println!();

    let all_mods = read_mods_list(config)?;
    let client = Client::new();

    let mut cached_types: HashMap<String, String> = HashMap::new();
    if config.filtered_list_file().exists() {
        info(&format!(
            "Existing {} found — loading cached classifications…",
            "mods_list.filtered.txt".bold()
        ));
        cached_types = load_cached_filter(&config.filtered_list_file());
        let nc = cached_types.len();
        if nc > 0 {
            ok(&format!(
                "{nc} mod(s) already classified — skipping their API calls."
            ));
        } else {
            info("No usable cached classifications found — querying all mods.");
        }
        println!();
    }

    let mut mods_to_query: Vec<String> = Vec::new();
    let mut force_both: Vec<String> = Vec::new();

    for entry in &all_mods {
        if entry.ends_with("**") {
            force_both.push(entry.trim_end_matches("**").to_string());
        } else if entry.ends_with('*') {
            // skip entirely
        } else {
            mods_to_query.push(entry.clone());
        }
    }

    let mods_need_api: Vec<String> = mods_to_query
        .iter()
        .filter(|e| !cached_types.contains_key(*e))
        .cloned()
        .collect();

    let total_cached = mods_to_query.len() - mods_need_api.len();
    let total_api = mods_need_api.len();

    info(&format!(
        "Total mods in list   : {}",
        (mods_to_query.len() + force_both.len()).to_string().bold()
    ));
    if !force_both.is_empty() {
        info(&format!(
            "Forced as 'both' (**): {} (API skipped)",
            force_both.len().to_string().bold()
        ));
    }
    info(&format!(
        "Already classified   : {} (skipped)",
        total_cached.to_string().bold()
    ));
    info(&format!(
        "Querying Thunderstore: {} mods…",
        total_api.to_string().bold()
    ));
    if total_api > 0 {
        info("This may take a moment (one API call per mod).");
    }
    println!();

    let mut mod_server: Vec<String> = Vec::new();
    let mut mod_client: Vec<String> = Vec::new();
    let mut mod_both: Vec<String> = force_both.clone();
    let mut mod_unknown: Vec<String> = Vec::new();

    for entry in &mods_to_query {
        match cached_types.get(entry).map(|s| s.as_str()) {
            Some("server") => mod_server.push(entry.clone()),
            Some("client") => mod_client.push(entry.clone()),
            Some("both") => mod_both.push(entry.clone()),
            Some("unknown") => mod_unknown.push(entry.clone()),
            _ => {}
        }
    }

    let pb = make_progress_bar(mods_need_api.len() as u64);
    for entry in mods_need_api.iter() {
        pb.set_message(truncate(entry, 40));

        let (ns, name) = parse_mod_entry(entry);
        let category = thunderstore::classify_mod(&client, &ns, &name)
            .await
            .unwrap_or(thunderstore::ModCategory::Unknown);

        match category {
            thunderstore::ModCategory::ServerOnly => mod_server.push(entry.clone()),
            thunderstore::ModCategory::ClientOnly => mod_client.push(entry.clone()),
            thunderstore::ModCategory::Both => mod_both.push(entry.clone()),
            thunderstore::ModCategory::Unknown => mod_unknown.push(entry.clone()),
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    section("Classification Results");
    separator_n(44);
    println!(
        "  {}  Client-side only   : {}",
        "✘".red(),
        mod_client.len().to_string().bold()
    );
    println!(
        "  {}  Server-side only   : {}",
        "✔".green(),
        mod_server.len().to_string().bold()
    );
    println!(
        "  {}  Both (client+server): {}",
        "◈".green(),
        mod_both.len().to_string().bold()
    );
    println!(
        "  {}  Unknown category   : {}",
        "?".yellow(),
        mod_unknown.len().to_string().bold()
    );
    separator_n(44);

    let combined = dedup_list(
        mod_server
            .iter()
            .chain(mod_both.iter())
            .chain(mod_unknown.iter()),
    );

    println!();
    section("Writing Filtered List");

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut out = String::new();
    out.push_str(&format!("# Generated by odin filter-mods on {now}\n"));
    out.push_str(&format!(
        "# Client-only: {}  |  Server-only: {}  |  Both: {}  |  Unknown: {}\n#\n",
        mod_client.len(),
        mod_server.len(),
        mod_both.len(),
        mod_unknown.len()
    ));
    out.push_str("## Client-side only\n");
    for m in &mod_client {
        out.push_str(m);
        out.push('\n');
    }
    out.push('\n');
    out.push_str("## Server-side only\n");
    for m in &mod_server {
        out.push_str(m);
        out.push('\n');
    }
    out.push('\n');
    out.push_str("## Both\n");
    for m in &mod_both {
        out.push_str(m);
        out.push('\n');
    }
    out.push('\n');
    out.push_str("## Unknown\n");
    for m in &mod_unknown {
        out.push_str(m);
        out.push('\n');
    }
    out.push('\n');
    out.push_str("## Server-side + Both + Unknown (no duplicates)\n");
    for m in &combined {
        out.push_str(m);
        out.push('\n');
    }

    fs::write(config.filtered_list_file(), &out).map_err(Error::Io)?;
    ok(&format!(
        "Structured filtered list written: {}",
        config.filtered_list_file().display().to_string().bold()
    ));

    println!();
    if !mod_client.is_empty() {
        println!("  \x1b[0;31m\x1b[1mClient-side only:\x1b[0m");
        for m in &mod_client {
            println!("    {}  {}", "✘".red(), m);
        }
        println!();
    }
    if !mod_server.is_empty() {
        println!("  \x1b[0;32m\x1b[1mServer-side only:\x1b[0m");
        for m in &mod_server {
            println!("    {}  {}", "✔".green(), m);
        }
        println!();
    }
    if !mod_both.is_empty() {
        println!("  \x1b[0;32m\x1b[1mBoth (client + server):\x1b[0m");
        for m in &mod_both {
            println!("    {}  {}", "◈".green(), m);
        }
        println!();
    }
    if !mod_unknown.is_empty() {
        println!("  \x1b[1;33m\x1b[1mUnknown:\x1b[0m");
        for m in &mod_unknown {
            println!("    {}  {}", "?".yellow(), m);
        }
        println!();
    }

    println!(
        "  \x1b[0;36m\x1b[1mServer-side + Both + Unknown (no duplicates) — {} mods:\x1b[0m",
        combined.len()
    );
    for m in &combined {
        println!("    {}  {}", "◈".cyan(), m);
    }
    println!();

    separator_n(44);
    println!(
        "  \x1b[1;33m▶  Combined server-side + Both: {} mods (Unknown excluded)\x1b[0m",
        mod_server.len() + mod_both.len()
    );
    separator_n(44);

    if !confirm(&format!(
        "{}Replace mods_list.txt with the combined Server-side + Both selection? (y/N){}",
        "\x1b[1m", "\x1b[0m"
    )) {
        warn("mods_list.txt left unchanged.");
        println!();
        separator_n(44);
        println!(
            "  \x1b[1;33m▶  Next step:\x1b[0m Review {} and update {} manually,",
            "mods_list.filtered.txt".bold(),
            "mods_list.txt".bold()
        );
        println!(
            "  \x1b[0;36m   then run {} to install the filtered set.\x1b[0m",
            "odin install-mods".bold()
        );
        separator_n(44);
        return Ok(());
    }

    let mut final_list = dedup_list(mod_server.iter().chain(mod_both.iter()));
    let mut unknown_included = 0usize;

    if !mod_unknown.is_empty() {
        println!();
        println!(
            "  \x1b[1;33m⚠  {} Unknown mod(s) were not automatically categorized:\x1b[0m",
            mod_unknown.len()
        );
        separator_n(44);
        for m in &mod_unknown {
            println!("    {}  {}", "?".yellow(), m);
        }
        separator_n(44);
        println!(
            "  \x1b[0;36mIncluding them preserves the previous behaviour (safe default).\x1b[0m"
        );
        println!();

        if confirm(&format!(
            "{}Also include Unknown mods in mods_list.txt? (y/N){}",
            "\x1b[1;33m", "\x1b[0m"
        )) {
            let existing: HashSet<String> = final_list.iter().cloned().collect();
            for m in &mod_unknown {
                if !existing.contains(m) {
                    final_list.push(m.clone());
                    unknown_included += 1;
                }
            }
            ok(&format!(
                "{unknown_included} unknown mod(s) added to the selection."
            ));
        } else {
            warn("Unknown mods excluded from mods_list.txt. Review manually if needed.");
        }
    }

    println!();
    section("Updating mods_list.txt");

    let mut list_out = String::new();
    list_out.push_str(&format!("# Updated by odin filter-mods on {now}\n"));
    list_out.push_str(&format!(
        "# Server-side: {}  |  Both: {}  |  Unknown included: {}  |  Total: {}\n#\n",
        mod_server.len(),
        mod_both.len(),
        unknown_included,
        final_list.len()
    ));
    for m in &final_list {
        list_out.push_str(m);
        list_out.push('\n');
    }

    fs::write(config.mods_list_file(), &list_out).map_err(Error::Io)?;
    ok(&format!(
        "mods_list.txt updated with {} mod(s).",
        final_list.len().to_string().bold()
    ));

    println!();
    separator_n(44);
    println!("  \x1b[1;36m\x1b[1mFinal recap:\x1b[0m");
    println!(
        "  {}  Server-side only     : {}",
        "✔".green(),
        mod_server.len().to_string().bold()
    );
    println!(
        "  {}  Both (client+server) : {}",
        "◈".green(),
        mod_both.len().to_string().bold()
    );
    println!(
        "  {}  Removed (client-only): {}",
        "✘".red(),
        mod_client.len().to_string().bold()
    );
    println!(
        "  {}  Unknown mods         : {}",
        "?".yellow(),
        mod_unknown.len().to_string().bold()
    );
    println!(
        "  {}  Total in mods_list   : {}",
        "◈".cyan(),
        final_list.len().to_string().bold()
    );
    separator_n(44);
    println!();
    println!(
        "  \x1b[1;33m▶  Next step:\x1b[0m Run {} to install the updated mod list.",
        "odin install-mods".bold()
    );
    separator_n(44);
    Ok(())
}

// ── download-mods ─────────────────────────────────────────────────────────────

pub async fn run_download(config: &AppConfig) -> Result<()> {
    section("Downloading Valheim mods to cache  [API-resolved, always latest]");
    let all_mods = read_mods_list(config)?;

    fs::create_dir_all(config.mods_cache_dir()).map_err(Error::Io)?;

    let (changes, purge) = detect_version_changes(&all_mods, config.mods_cache_dir());
    if !confirm_migration(&changes, &purge) {
        return Ok(());
    }
    purge_old_zips(&purge);

    let client = Client::new();
    let (downloaded, cached, skipped, failed) =
        download_mods(&all_mods, config.mods_cache_dir(), &client).await;

    separator_n(44);
    ok(&format!(
        "Download phase complete → {}",
        config.mods_cache_dir().display()
    ));
    println!(
        "  \x1b[0;32m↓  Freshly downloaded  : {}",
        downloaded.len().to_string().bold()
    );
    println!(
        "  \x1b[0;36m◈  Already in cache    : {}\x1b[0m",
        cached.len().to_string().bold()
    );
    if !changes.is_empty() {
        println!(
            "  \x1b[1;33m↑  Versions migrated   : {}\x1b[0m",
            changes.len().to_string().bold()
        );
    }
    if !skipped.is_empty() {
        println!(
            "  \x1b[1;33m⊘  Marked (*) ignored  : {}\x1b[0m",
            skipped.len().to_string().bold()
        );
    }
    if !failed.is_empty() {
        println!(
            "  \x1b[0;31m✘  Failed              : {}\x1b[0m",
            failed.len().to_string().bold()
        );
    }
    separator_n(44);
    println!(
        "\n  \x1b[0;36m→\x1b[0m  Run {} to extract the cached packages.\n",
        "odin install-mods".bold()
    );
    Ok(())
}

// ── install-mods ──────────────────────────────────────────────────────────────

pub async fn run_install(config: &AppConfig) -> Result<()> {
    section("Installing Valheim mods  [API-resolved, always latest]");
    let all_mods = read_mods_list(config)?;

    fs::create_dir_all(config.mods_cache_dir()).map_err(Error::Io)?;

    let (changes, purge) = detect_version_changes(&all_mods, config.mods_cache_dir());
    if !confirm_migration(&changes, &purge) {
        return Ok(());
    }
    purge_old_zips(&purge);

    let plugins_dir = config.plugins_dir();
    if plugins_dir.is_dir() && !dir_is_empty(&plugins_dir) {
        println!();
        warn(&format!(
            "Plugins directory is not empty: {}",
            plugins_dir.display()
        ));
        warn("Its contents will be fully deleted before extraction.");
        if !confirm("Replace existing mods (y/N)?") {
            warn("Installation cancelled. Existing mods preserved.");
            return Ok(());
        }
    }

    sudo_rm_rf(&plugins_dir)?;
    sudo_mkdir_p(&plugins_dir)?;
    ok(&format!(
        "Plugins directory ready: {}",
        plugins_dir.display()
    ));

    let client = Client::new();
    let (downloaded, cached, skipped, failed) =
        download_mods(&all_mods, config.mods_cache_dir(), &client).await;

    let mut to_extract: Vec<String> = Vec::new();
    to_extract.extend(downloaded.iter().cloned());
    to_extract.extend(cached.iter().cloned());

    if to_extract.is_empty() {
        warn("No mods to extract.");
        return Ok(());
    }

    section(&format!("Extracting to {}", plugins_dir.display()));

    let pb = make_progress_bar(to_extract.len() as u64);
    for entry in to_extract.iter() {
        pb.set_message(truncate(entry, 40));
        let zip_file = config.mods_cache_dir().join(format!("{entry}.zip"));
        if !zip_file.exists() {
            pb.println(format!("  ⊘ Archive missing: {}", zip_file.display()));
        } else {
            sudo_7z_extract(&zip_file, &plugins_dir)?;
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    separator_n(44);
    ok(&format!(" Extraction complete → {}", plugins_dir.display()));
    println!(
        "  \x1b[0;32m✔  Total installed     : {}\x1b[0m",
        to_extract.len().to_string().bold()
    );
    println!(
        "  \x1b[0;36m◈  From cache          : {}\x1b[0m",
        cached.len().to_string().bold()
    );
    println!(
        "  \x1b[0;32m↓  Freshly downloaded  : {}\x1b[0m",
        downloaded.len().to_string().bold()
    );
    if !changes.is_empty() {
        println!(
            "  \x1b[1;33m↑  Versions migrated   : {}\x1b[0m",
            changes.len().to_string().bold()
        );
    }
    if !skipped.is_empty() {
        println!(
            "  \x1b[1;33m⊘  Marked (*) ignored  : {}\x1b[0m",
            skipped.len().to_string().bold()
        );
    }
    if !failed.is_empty() {
        println!(
            "  \x1b[0;31m✘  Failed              : {}\x1b[0m",
            failed.len().to_string().bold()
        );
    }
    separator_n(44);
    println!(
        "\n  \x1b[1;33m⚠  Remember to run\x1b[0m {} to fetch the latest {} files.\n",
        "git pull".bold(),
        ".cfg".bold()
    );
    Ok(())
}

// ── clear-mods ────────────────────────────────────────────────────────────────

pub async fn run_clear(config: &AppConfig) -> Result<()> {
    section("Clear Mods — Full cleanup (mods + server data)");

    let backups_dir = config.backups_dir();
    let worlds_local = config.worlds_local_dir();
    let data_dir = config.data_dir();
    let container = "valheim-server";

    println!();
    section("Step 1/5 — Server shutdown");
    let state = crate::commands::docker::container_state(container);
    if state == "running" {
        warn("The server is currently running.");
        info("Performing docker compose down before cleanup…");
        crate::commands::docker::compose_down()?;
        ok("Server stopped and container removed.");
    } else {
        info(&format!(
            "Server is not running (state: {state}). No action needed."
        ));
    }

    println!();
    section("Step 2/5 — Important notice");
    println!("  \x1b[1;33m⚠  WARNING: This operation includes deleting \x1b[0;31m./data/*\x1b[1;33m.\x1b[0m");
    println!("  \x1b[1;33m   This folder contains the Valheim server installation.\x1b[0m");
    println!("  \x1b[1;33m   It will be fully recreated on next \x1b[1modin start.\x1b[0m");
    println!("  \x1b[0;36m   Your world files (./config/worlds_local) will be backed up\x1b[0m");
    println!("  \x1b[0;36m   automatically BEFORE any deletion occurs.\x1b[0m");
    separator_n(44);

    println!();
    section("Step 3/5 — Automatic world backup");
    fs::create_dir_all(&backups_dir).map_err(Error::Io)?;

    if worlds_local.is_dir() && !dir_is_empty(&worlds_local) {
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let backup_name = format!("worlds-{ts}.zip");
        let backup_path = backups_dir.join(&backup_name);

        info(&format!(
            "Backing up {} → {}…",
            worlds_local.display(),
            backup_path.display()
        ));

        let status = sudo_run_bool(&[
            "7z",
            "a",
            &backup_path.to_string_lossy(),
            &format!("{}/.", worlds_local.display()),
            "-tzip",
            "-y",
        ])?;
        if !status {
            return Err(Error::other(
                "Backup failed! Aborting to protect world data.",
            ));
        }
        ok(&format!("Backup created: {}", backup_name.bold()));
    } else {
        warn("worlds_local is empty or absent — no backup needed.");
    }

    println!();
    section("Step 4/5 — Interactive cleanup");

    let bepinex_dir = config
        .plugins_dir()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| config.script_dir.join("config/bepinex"));

    let items: Vec<(&str, PathBuf, bool)> = vec![
        (
            "Mods list file        (mods_list.txt)",
            config.mods_list_file(),
            false,
        ),
        (
            "Mods cache dir        (mods_cache/*)",
            config.mods_cache_dir(),
            true,
        ),
        ("Server data dir       (data/*)", data_dir.clone(), true),
        (
            "BepInEx config dir    (config/bepinex/*)",
            bepinex_dir,
            true,
        ),
    ];

    println!();
    println!("  \x1b[0;36mEach item will be presented for individual confirmation.\x1b[0m");
    separator_n(44);

    let mut deleted = 0usize;
    let mut skipped = 0usize;
    let mut absent = 0usize;

    for (label, path, is_dir) in &items {
        println!();
        let exists = if *is_dir {
            path.is_dir()
        } else {
            path.is_file()
        };
        if !exists {
            warn(label);
            info(&format!("  Not found, skipping: {}", path.display()));
            absent += 1;
            continue;
        }

        info(&format!("{}{}{}", "\x1b[1m", label, "\x1b[0m"));
        if *is_dir {
            let count = fs::read_dir(path).map(|rd| rd.count()).unwrap_or(0);
            info(&format!("  Path : {}", path.display()));
            info(&format!("  Items: {} file(s)/folder(s)", count));
        } else {
            let size = fs::metadata(path)
                .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                .unwrap_or_else(|_| "?".into());
            info(&format!("  Path : {}", path.display()));
            info(&format!("  Size : {size}"));
        }

        if confirm(&format!(
            "{}Delete this {}? (y/N){}",
            "\x1b[0;31m",
            if *is_dir { "dir" } else { "file" },
            "\x1b[0m"
        )) {
            if *is_dir {
                sudo_rm_rf(path)?;
            } else {
                fs::remove_file(path).map_err(Error::Io)?;
            }
            ok(&format!("Deleted: {}", path.display()));
            deleted += 1;
        } else {
            warn(&format!("Skipped: {}", path.display()));
            skipped += 1;
        }
    }

    println!();
    separator_n(44);
    println!(
        "  \x1b[0;32m✔  Deleted : {}\x1b[0m",
        deleted.to_string().bold()
    );
    println!(
        "  \x1b[1;33m⊘  Skipped : {}\x1b[0m",
        skipped.to_string().bold()
    );
    if absent > 0 {
        println!(
            "  \x1b[0;36m◈  Absent  : {}\x1b[0m",
            absent.to_string().bold()
        );
    }
    separator_n(44);

    println!();
    section("Step 5/5 — Restore latest world backup");

    let latest_backup = find_latest_backup(&backups_dir);
    if let Some(ref bp) = latest_backup {
        let bp_name = bp
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let bp_date = file_mtime_str(bp);
        println!();
        info("Latest backup available:");
        println!("  \x1b[0;32m  \x1b[1m{}\x1b[0m", bp_name.bold());
        println!("  \x1b[0;36m  Created: {}\x1b[0m", bp_date);
        println!();

        if confirm(&format!(
            "{}Restore this backup to ./config/worlds_local? (n/Y){}",
            "\x1b[0;32m", "\x1b[0m"
        )) {
            if worlds_local.exists() {
                sudo_rm_rf(&worlds_local)?;
                info("Removed existing worlds_local.");
            }
            sudo_mkdir_p(&worlds_local)?;
            info(&format!(
                "Extracting {bp_name} → {}…",
                worlds_local.display()
            ));
            let ok_flag = sudo_run_bool(&[
                "7z",
                "x",
                &bp.to_string_lossy(),
                &format!("-o{}", worlds_local.display()),
                "-y",
            ])?;
            if ok_flag {
                ok(&format!(
                    "World restored successfully from {}.",
                    bp_name.bold()
                ));
                ok("Your latest Valheim progress is preserved.");
            } else {
                err(&format!(
                    "Extraction failed. Please restore manually from: {}",
                    bp.display()
                ));
            }
        } else {
            warn(&format!(
                "Restore skipped. Backup remains available in {}.",
                backups_dir.display()
            ));
        }
    } else {
        warn(&format!(
            "No backup found in {}. Skipping restore.",
            backups_dir.display()
        ));
    }

    println!();
    separator_n(44);
    if deleted > 0 {
        ok("Cleanup complete.");
    } else {
        info("Nothing was deleted.");
    }
    println!();
    println!(
        "  \x1b[1;33m▶  Next step:\x1b[0m Run {} to launch a vanilla Valheim server (no mods).",
        "odin start".bold()
    );
    println!(
        "  \x1b[0;36m   To reinstall mods afterwards, run {}.\x1b[0m",
        "odin install-mods".bold()
    );
    separator_n(44);
    Ok(())
}

// ── Core download logic ───────────────────────────────────────────────────────

async fn download_mods(
    all_mods: &[String],
    cache_dir: PathBuf,
    client: &Client,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut downloaded: Vec<String> = Vec::new();
    let mut cached: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    let total = all_mods.len();
    section(&format!(
        "Downloading  ({total} entries)  [API-resolved, always latest]"
    ));

    let pb = make_progress_bar(total as u64);

    for raw_entry in all_mods.iter() {
        let mut entry = raw_entry.clone();

        if entry.ends_with("**") {
            entry = entry.trim_end_matches("**").to_string();
        }

        pb.set_message(truncate(&entry, 40));

        if entry.ends_with('*') {
            skipped.push(entry.trim_end_matches('*').to_string());
            pb.inc(1);
            continue;
        }

        let dash_count = entry.chars().filter(|&c| c == '-').count();
        if dash_count < 2 {
            pb.println(format!(
                "  ⊘ Invalid entry (not Author-Mod-Version format): '{entry}'"
            ));
            skipped.push(entry.clone());
            pb.inc(1);
            continue;
        }

        let (ns, name) = parse_mod_entry(&entry);
        let resolved = thunderstore::resolve_mod(client, &ns, &name, &entry).await;
        let resolved_name = resolved.full_name.clone();
        let resolved_url = resolved.download_url.clone();
        let base = strip_version(&resolved_name);

        let dest = cache_dir.join(format!("{resolved_name}.zip"));

        if let Some(cached_zip) = find_cached_zip(&cache_dir, &base) {
            let cached_name = cached_zip
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if cached_name == resolved_name {
                if zip_valid(&cached_zip) {
                    cached.push(resolved_name.clone());
                    pb.inc(1);
                    continue;
                } else {
                    pb.println(format!(
                        "  ⊘ Corrupt cached zip, re-downloading: {}",
                        cached_zip.display()
                    ));
                    let _ = fs::remove_file(&cached_zip);
                }
            } else {
                let _ = fs::remove_file(&cached_zip);
            }
        }

        if wget_download(&resolved_url, &dest) {
            if zip_valid(&dest) {
                downloaded.push(resolved_name.clone());
            } else {
                pb.println(format!(
                    "  ✘ Downloaded archive is invalid: {resolved_name}"
                ));
                let _ = fs::remove_file(&dest);
                failed.push(format!("{entry}  [corrupt archive]"));
            }
        } else {
            let _ = fs::remove_file(&dest);
            failed.push(format!("{entry}  [network error / not found]"));
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    separator_n(44);
    println!(
        "  \x1b[0;32m✔\x1b[0m  Downloaded: {}",
        downloaded.len().to_string().bold()
    );
    if !cached.is_empty() {
        println!(
            "  \x1b[0;36m◈\x1b[0m  Cached (skipped): {}",
            cached.len().to_string().bold()
        );
    }
    if !skipped.is_empty() {
        println!(
            "  \x1b[1;33m⊘\x1b[0m  Marked (*) / invalid: {}",
            skipped.len().to_string().bold()
        );
    }
    if !failed.is_empty() {
        println!(
            "  \x1b[0;31m✘\x1b[0m  Failed: {}",
            failed.len().to_string().bold()
        );
    }
    separator_n(44);

    if !failed.is_empty() {
        for f in &failed {
            err(&format!("  ↳ {f}"));
        }
        println!();
        warn("Failed entries above: verify the Author-ModName-Version format in mods_list.txt");
        warn("and confirm the package exists at: https://thunderstore.io/c/valheim/");
    }

    (downloaded, cached, skipped, failed)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_mods_list(config: &AppConfig) -> Result<Vec<String>> {
    if !config.mods_list_file().exists() {
        return Err(Error::other(format!(
            "File not found: {}",
            config.mods_list_file().display()
        )));
    }
    let content = fs::read_to_string(config.mods_list_file()).map_err(Error::Io)?;
    let mods: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    if mods.is_empty() {
        return Err(Error::other("Mod list is empty."));
    }
    Ok(mods)
}

fn parse_mod_entry(entry: &str) -> (String, String) {
    let base = strip_version(entry);
    let mut parts = base.splitn(2, '-');
    let ns = parts.next().unwrap_or("").to_string();
    let name = parts.next().unwrap_or("").to_string();
    (ns, name)
}

fn strip_version(entry: &str) -> String {
    match entry.rfind('-') {
        Some(i) => entry[..i].to_string(),
        None => entry.to_string(),
    }
}

fn find_cached_zip(cache_dir: &Path, base: &str) -> Option<PathBuf> {
    let pattern = format!("{base}-");
    fs::read_dir(cache_dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with(&pattern) && n.ends_with(".zip")
        })
        .map(|e| e.path())
        .next()
}

fn zip_valid(path: &Path) -> bool {
    Command::new("zip")
        .args(["-T", &path.to_string_lossy()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn wget_download(url: &str, dest: &Path) -> bool {
    Command::new("wget")
        .args([
            "--quiet",
            "--timeout=60",
            "--tries=3",
            &format!("--output-document={}", dest.display()),
            url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn detect_version_changes(mods: &[String], cache_dir: PathBuf) -> (Vec<String>, Vec<PathBuf>) {
    let mut changes: Vec<String> = Vec::new();
    let mut purge: Vec<PathBuf> = Vec::new();

    for raw in mods {
        let mut entry = raw.clone();
        if entry.ends_with("**") {
            entry = entry.trim_end_matches("**").to_string();
        }
        if entry.ends_with('*') {
            continue;
        }
        if entry.chars().filter(|&c| c == '-').count() < 2 {
            continue;
        }

        let base = strip_version(&entry);
        if let Some(old_zip) = find_cached_zip(&cache_dir, &base) {
            let old_name = old_zip
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if old_name != entry {
                changes.push(format!(
                    "{old_name}  →  {entry} (will resolve latest via API)"
                ));
                purge.push(old_zip);
            }
        }
    }
    (changes, purge)
}

fn confirm_migration(changes: &[String], purge: &[PathBuf]) -> bool {
    if changes.is_empty() {
        return true;
    }
    println!();
    println!(
        "  \x1b[1;33m\x1b[1m⚠  Version differences detected between cache and mods_list.txt:\x1b[0m"
    );
    separator_n(44);
    for c in changes {
        println!("  \x1b[0;36m◈\x1b[0m  {}", c);
    }
    separator_n(44);
    warn("These mods will be updated and their old versions removed from cache.");
    if !confirm("Confirm migration (y/N)?") {
        warn("Migration cancelled. Script keeps current cache and stops.");
        return false;
    }
    purge_old_zips(purge);
    println!();
    true
}

fn purge_old_zips(paths: &[PathBuf]) {
    for p in paths {
        let _ = fs::remove_file(p);
        info(&format!(
            "Cache purged: {}",
            p.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
}

fn load_cached_filter(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let content = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return map,
    };

    let mut current_section = String::new();
    for line in content.lines() {
        match line {
            "## Client-side only" => current_section = "client".into(),
            "## Server-side only" => current_section = "server".into(),
            "## Both" => current_section = "both".into(),
            "## Unknown" => current_section = "unknown".into(),
            s if s.starts_with("## Server-side + Both") => current_section = String::new(),
            s if s.starts_with('#') || s.is_empty() => {}
            entry => {
                if !current_section.is_empty() {
                    map.insert(entry.to_string(), current_section.clone());
                }
            }
        }
    }
    map
}

fn dedup_list<'a, I: Iterator<Item = &'a String>>(iter: I) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in iter {
        if seen.insert(item.clone()) {
            out.push(item.clone());
        }
    }
    out
}

fn find_latest_backup(backups_dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(backups_dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("worlds-") && n.ends_with(".zip")
        })
        .map(|e| e.path())
        .collect();
    files.sort();
    files.into_iter().last()
}

/// Run a privileged command and return its success status (without failing on non-zero exit).
fn sudo_run_bool(args: &[&str]) -> Result<bool> {
    let uid = unsafe { libc::getuid() };
    let status = if uid == 0 {
        Command::new(args[0]).args(&args[1..]).status()
    } else {
        Command::new("sudo").args(args).status()
    };
    match status {
        Ok(s) => Ok(s.success()),
        Err(e) => Err(Error::other(format!("exec error: {e}"))),
    }
}

fn sudo_7z_extract(zip: &Path, dest: &Path) -> Result<()> {
    let zip_s = zip.to_string_lossy().to_string();
    let out_arg = format!("-o{}", dest.display());
    // suppress 7z's verbose per-file output — only the progress bar is shown
    let uid = unsafe { libc::getuid() };
    let status = if uid == 0 {
        Command::new("7z")
            .args(["x", &zip_s, &out_arg, "-y"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    } else {
        Command::new("sudo")
            .args(["7z", "x", &zip_s, &out_arg, "-y"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(Error::other(format!(
            "7z extract failed (exit {:?})",
            s.code()
        ))),
        Err(e) => Err(Error::other(format!("exec error: {e}"))),
    }
}

// ── Progress bar ──────────────────────────────────────────────────────────────

fn make_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("  [{bar:40.cyan/blue}] {pos:>3}/{len}  {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{s:<max$}")
    } else {
        format!("{}…", &s[..max - 1])
    }
}
