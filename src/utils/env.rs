//! Utilities for reading and parsing `valheim.env`.

use std::fs;
use std::path::Path;

/// Read a single variable from a `.env`-style file.
///
/// Ignores comment lines and trims trailing inline comments and whitespace,
/// mirroring the `_env_get` helper in `odin.sh`.
pub fn env_get(path: &Path, key: &str, default: &str) -> String {
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix(&format!("{key}=")) {
                // Strip inline comment and surrounding whitespace
                let val = rest.split(" #")
                    .next()
                    .unwrap_or(rest).split("\t#")
                    .next()
                    .unwrap_or(rest)
                    .trim();
                return val.to_string();
            }
        }
    }
    default.to_string()
}