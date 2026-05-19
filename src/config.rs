//! Application configuration.
//!
//! Reads `valheim.env` (via dotenvy) and exposes typed fields used by
//! every command.  All values have sensible defaults that match the
//! Bash script.
//!
//! # Environment variables (valheim.env)
//! | Variable              | Default              | Description                                |
//! |-----------------------|----------------------|--------------------------------------------|
//! | SERVER_NAME           | My Server            | Valheim server display name                |
//! | WORLD_NAME            | Dedicated            | World save file name                       |
//! | SERVER_PASS           | (empty)              | Server password (≥ 5 chars required)       |
//! | SERVER_PUBLIC         | false                | List server publicly on Steam              |
//! | TZ                    | Etc/UTC              | Timezone for cron expressions              |
//! | UPDATE_CRON           | (empty)              | Auto-update cron schedule                  |
//! | RESTART_CRON          | (empty)              | Auto-restart cron schedule                 |
//! | BACKUPS_CRON          | (empty)              | Backup cron schedule                       |
//! | CROSSPLAY             | false                | Enable crossplay                           |
//! | SUPERVISOR_HTTP       | false                | Enable Supervisor web interface            |
//! | SUPERVISOR_HTTP_PASS  | (empty)              | Supervisor password                        |
//! | BEPINEX               | false                | Enable BepInEx mod loader                  |
//! | WIN_USER              | current user         | Windows account name                       |
//! | WIN_HOST              | (empty)              | Windows machine IP/hostname                |
//! | WIN_SSH_USER          | WIN_USER             | SSH user on Windows                        |
//! | WIN_SSH_PORT          | 22                   | SSH port on Windows                        |
//! | WIN_SSH_KEY           | (empty)              | Absolute path to SSH private key           |
//! | APPLY_DLL_PATCH       | false                | Auto-apply patches/assembly_valheim.dll    |

use crate::error::{Error, Result};
use std::env;
use std::path::{Path, PathBuf};

/// Top-level application configuration derived from `valheim.env`.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // Server identity
    pub server_name: String,
    pub world_name: String,
    pub server_pass: String,
    pub server_public: bool,

    // Timezone + cron
    pub tz: String,
    pub update_cron: String,
    pub restart_cron: String,
    pub backups_cron: String,

    // Features
    pub crossplay: bool,
    pub supervisor_http: bool,
    pub supervisor_http_pass: String,
    pub bepinex: bool,
    pub apply_dll_patch: bool,

    // Windows sync
    pub win_user: String,
    pub win_host: String,
    pub win_ssh_user: String,
    pub win_ssh_port: u16,
    pub win_ssh_key: PathBuf,

    // Derived paths (based on script directory)
    pub script_dir: PathBuf,
}

impl AppConfig {
    /// Load config from the environment (after loading `valheim.env` in main).
    pub fn from_env(script_dir: &Path) -> Result<Self> {
        let current_user = env::var("USER")
            .or_else(|_| env::var("LOGNAME"))
            .unwrap_or_else(|_| "root".into());

        let win_user = env::var("WIN_USER").unwrap_or_else(|_| current_user.clone());
        let win_ssh_user = env::var("WIN_SSH_USER").unwrap_or_else(|_| win_user.clone());

        let win_ssh_port: u16 = env::var("WIN_SSH_PORT")
            .unwrap_or_else(|_| "22".into())
            .parse()
            .map_err(|_| Error::config("WIN_SSH_PORT must be a valid port number"))?;

        Ok(Self {
            server_name: env::var("SERVER_NAME").unwrap_or_else(|_| "My Server".into()),
            world_name: env::var("WORLD_NAME").unwrap_or_else(|_| "Dedicated".into()),
            server_pass: env::var("SERVER_PASS").unwrap_or_default(),
            server_public: parse_bool(&env::var("SERVER_PUBLIC").unwrap_or_default()),
            tz: env::var("TZ").unwrap_or_else(|_| "Etc/UTC".into()),
            update_cron: env::var("UPDATE_CRON").unwrap_or_default(),
            restart_cron: env::var("RESTART_CRON").unwrap_or_default(),
            backups_cron: env::var("BACKUPS_CRON").unwrap_or_default(),
            crossplay: parse_bool(&env::var("CROSSPLAY").unwrap_or_default()),
            supervisor_http: parse_bool(&env::var("SUPERVISOR_HTTP").unwrap_or_default()),
            supervisor_http_pass: env::var("SUPERVISOR_HTTP_PASS").unwrap_or_default(),
            bepinex: parse_bool(&env::var("BEPINEX").unwrap_or_default()),
            apply_dll_patch: parse_bool(&env::var("APPLY_DLL_PATCH").unwrap_or_default()),
            win_user,
            win_host: env::var("WIN_HOST").unwrap_or_default(),
            win_ssh_user,
            win_ssh_port,
            win_ssh_key: PathBuf::from(env::var("WIN_SSH_KEY").unwrap_or_default()),
            script_dir: script_dir.to_path_buf(),
        })
    }

    /// Path to `valheim.env` in the script directory.
    pub fn env_file(&self) -> PathBuf {
        self.script_dir.join("valheim.env")
    }

    /// Path to the mods list file.
    pub fn mods_list_file(&self) -> PathBuf {
        self.script_dir.join("mods_list.txt")
    }

    /// Path to the mods cache directory.
    pub fn mods_cache_dir(&self) -> PathBuf {
        self.script_dir.join("mods_cache")
    }

    /// Path to `config/bepinex/plugins`.
    pub fn plugins_dir(&self) -> PathBuf {
        self.script_dir.join("config/bepinex/plugins")
    }

    /// Path to `config/worlds_local`.
    pub fn worlds_local_dir(&self) -> PathBuf {
        self.script_dir.join("config/worlds_local")
    }

    /// Path to `config/backups`.
    pub fn backups_dir(&self) -> PathBuf {
        self.script_dir.join("config/backups")
    }

    /// Path to `data/`.
    pub fn data_dir(&self) -> PathBuf {
        self.script_dir.join("data")
    }

    /// Path to `patches/` (DLL patch source directory).
    pub fn patches_dir(&self) -> PathBuf {
        self.script_dir.join("patches")
    }

    /// Path to the patched DLL source file.
    pub fn patch_dll_src(&self) -> PathBuf {
        self.patches_dir().join("assembly_valheim.dll")
    }

    /// Windows `worlds_local` SFTP source path.
    pub fn worlds_src_remote(&self) -> String {
        format!(
            "C:/Users/{}/AppData/LocalLow/IronGate/Valheim/worlds_local",
            self.win_user
        )
    }

    /// Filtered mod list file.
    pub fn filtered_list_file(&self) -> PathBuf {
        self.script_dir.join("mods_list.filtered.txt")
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse common boolean strings: true/1/yes/on → true, everything else → false.
pub fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().trim(), "true" | "1" | "yes" | "on")
}

/// Human-readable label for a bool: Enabled / Disabled.
pub fn bool_label(b: bool) -> &'static str {
    if b { "Enabled" } else { "Disabled" }
}

/// Human-readable label for a bool: On / Off.
pub fn bool_onoff(b: bool) -> &'static str {
    if b { "On" } else { "Off" }
}

/// Translate a cron expression into human-readable text.
///
/// Covers the same common cases as `_cron_human` in the Bash script.
pub fn cron_human(cron: &str) -> String {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() != 5 {
        return cron.to_string();
    }
    let (min, hr, dom, mon, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    // */N * * * *  → Every N minutes
    if let Some(n) = min.strip_prefix("*/") {
        if hr == "*" && dom == "*" && mon == "*" && dow == "*" {
            return format!("Every {} minutes", n);
        }
    }

    // * * * * *  → Every minute
    if min == "*" && hr == "*" && dom == "*" && mon == "*" && dow == "*" {
        return "Every minute".into();
    }

    // N * * * *  → Every hour at minute N
    if hr == "*" && dom == "*" && mon == "*" && dow == "*" {
        if let Ok(m) = min.parse::<u32>() {
            return format!("Every hour at minute {}", m);
        }
    }

    // N */H * * *  → Every Hh at minute N
    if let Some(h_str) = hr.strip_prefix("*/") {
        if dom == "*" && mon == "*" && dow == "*" {
            if let (Ok(m), Ok(h)) = (min.parse::<u32>(), h_str.parse::<u32>()) {
                return format!("Every {}h at minute {}", h, m);
            }
        }
    }

    // N H * * *  → Daily at HH:MM
    if dom == "*" && mon == "*" && dow == "*" {
        if let (Ok(m), Ok(h)) = (min.parse::<u32>(), hr.parse::<u32>()) {
            return format!("Daily at {:02}:{:02}", h, m);
        }
    }

    // N H * * D  → Every <day> at HH:MM
    if dom == "*" && mon == "*" {
        if let (Ok(m), Ok(h), Ok(d)) =
            (min.parse::<u32>(), hr.parse::<u32>(), dow.parse::<usize>())
        {
            let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
            let day_name = days.get(d).copied().unwrap_or("day");
            return format!("Every {} at {:02}:{:02}", day_name, h, m);
        }
    }

    // N H D M *  → Monthly on day D at HH:MM
    if dow == "*" {
        if let (Ok(m), Ok(h), Ok(d)) =
            (min.parse::<u32>(), hr.parse::<u32>(), dom.parse::<u32>())
        {
            return format!("Monthly on day {} at {:02}:{:02}", d, h, m);
        }
    }

    // Fallback: return raw cron
    cron.to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_variants() {
        assert!(parse_bool("true"));
        assert!(parse_bool("1"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool(""));
    }

    #[test]
    fn cron_human_cases() {
        assert_eq!(cron_human("30 * * * *"), "Every hour at minute 30");
        assert_eq!(cron_human("30 4 * * *"), "Daily at 04:30");
        assert_eq!(cron_human("5 * * * *"), "Every hour at minute 5");
        assert_eq!(cron_human("*/15 * * * *"), "Every 15 minutes");
    }
}