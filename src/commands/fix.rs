//! `fix` sub-commands.
//!
//! Mirrors `cmd_fix` from `odin.sh`.

use crate::{
    config::AppConfig,
    error::Result,
    utils::{
        display::{info, ok, section},
        fs::sudo_run,
    },
};
use colored::Colorize;

pub async fn run_permission(config: &AppConfig) -> Result<()> {
    section("Fix: Permissions");
    info("Fixing ownership and permissions on ./data and ./config…");

    let base = config.script_dir.to_string_lossy().to_string();
    sudo_run(&["chown", "-R", "1000:1000", &format!("{base}/data"), &format!("{base}/config")])?;
    sudo_run(&["chmod", "-R", "755", &format!("{base}/data"), &format!("{base}/config")])?;

    ok("Ownership set to 1000:1000 and permissions to 755 on ./data and ./config.");
    println!();
    println!("  {}  You can now run {}.", "→".cyan(), "odin start".bold());
    Ok(())
}
