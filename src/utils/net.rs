//! Network utilities: internal / external IP resolution.

use std::process::Command;

/// Return the machine's internal IP addresses (space-separated), or "N/A".
pub fn internal_ips() -> String {
    let out = Command::new("hostname").arg("-I").output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { "N/A".into() } else { trimmed }
        }
        Err(_) => "N/A".into(),
    }
}

/// Try to resolve the external IP via the running container, then curl fallback.
///
/// Returns "N/A" if both fail.
pub fn external_ip(container: &str) -> String {
    // Try via docker exec first
    let via_docker = Command::new("docker")
        .args(["exec", container, "wget", "-qO-", "ifconfig.me/ip"])
        .output();
    if let Ok(o) = via_docker {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout);
            let ip = s.split_whitespace().next().unwrap_or("").to_string();
            if !ip.is_empty() {
                return ip;
            }
        }
    }

    // Fallback: local curl
    let via_curl = Command::new("curl")
        .args(["-sf", "--max-time", "3", "ifconfig.me/ip"])
        .output();
    if let Ok(o) = via_curl {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout);
            let ip = s.split_whitespace().next().unwrap_or("").to_string();
            if !ip.is_empty() {
                return ip;
            }
        }
    }

    "N/A".into()
}