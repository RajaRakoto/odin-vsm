//! Filesystem and process helpers shared across commands.

use crate::error::{Error, Result};
use std::{fs, path::Path, process::Command};

/// Run a privileged command — as-is if root, prefixed with `sudo` otherwise.
pub fn sudo_run(args: &[&str]) -> Result<()> {
    let uid = unsafe { libc::getuid() };
    let status = if uid == 0 {
        Command::new(args[0]).args(&args[1..]).status()
    } else {
        Command::new("sudo").args(args).status()
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(Error::other(format!(
            "command {:?} failed (exit {:?})",
            args,
            s.code()
        ))),
        Err(e) => Err(Error::other(format!("exec error: {e}"))),
    }
}

/// `rm -rf <path>` with sudo elevation when not root.
pub fn sudo_rm_rf(path: &Path) -> Result<()> {
    sudo_run(&["rm", "-rf", &path.to_string_lossy()])
}

/// `mkdir -p <path>` with sudo elevation when not root.
pub fn sudo_mkdir_p(path: &Path) -> Result<()> {
    sudo_run(&["mkdir", "-p", &path.to_string_lossy()])
}

/// Returns `true` if `path` does not exist or is an empty directory.
pub fn dir_is_empty(path: &Path) -> bool {
    if !path.is_dir() {
        return true;
    }
    fs::read_dir(path)
        .map(|mut rd| rd.next().is_none())
        .unwrap_or(true)
}

/// Format a file's mtime as a human-readable string.
pub fn file_mtime_str(path: &Path) -> String {
    use std::time::SystemTime;
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| {
            let secs = t
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            chrono::DateTime::from_timestamp(secs as i64, 0)
                .unwrap_or_default()
                .format("%d %b %Y  %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| "unknown date".into())
}
