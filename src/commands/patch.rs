//! DLL patch commands: apply-patch, verify-patch.
//!
//! `apply-patch` copies `patches/assembly_valheim.dll` into the running
//! container using `docker cp`, replacing the stock DLL only when the MD5
//! checksums differ (idempotent).
//!
//! `verify-patch` computes the MD5 of the local patch source and the DLL
//! currently inside the container and reports whether they match.

use crate::{
    config::AppConfig,
    error::{Error, Result},
    utils::display::{info, ok, warn},
};
use std::{
    io::Read,
    process::{Command, Stdio},
};

const CONTAINER: &str = "valheim-server";
const TARGET_PATH: &str =
    "/opt/valheim/server/valheim_server_Data/Managed/assembly_valheim.dll";

// ── Public entry points ───────────────────────────────────────────────────────

/// Apply the patched DLL to the running container (idempotent).
pub async fn run_apply(config: &AppConfig) -> Result<()> {
    let src = config.patch_dll_src();

    if !src.exists() {
        return Err(Error::other(format!(
            "Patch source not found: {}",
            src.display()
        )));
    }

    require_container_running()?;

    let src_md5 = md5_local(&src)?;
    let dst_md5 = md5_in_container(TARGET_PATH)?;

    if src_md5 == dst_md5 {
        ok("DLL already patched — checksums match, nothing to do.");
        return Ok(());
    }

    info(&format!("Applying patch: {} → container:{TARGET_PATH}", src.display()));

    let cp_src = format!("{}:{TARGET_PATH}", CONTAINER);
    // docker cp <local_file> <container>:<path>
    let status = Command::new("docker")
        .args(["cp", &src.to_string_lossy(), &cp_src])
        .status()
        .map_err(|e| Error::docker(format!("docker cp: {e}")))?;

    if !status.success() {
        return Err(Error::docker("docker cp failed — is the container running?"));
    }

    // Fix permissions inside the container
    let chmod_status = Command::new("docker")
        .args(["exec", CONTAINER, "chmod", "644", TARGET_PATH])
        .status()
        .map_err(|e| Error::docker(format!("docker exec chmod: {e}")))?;

    if !chmod_status.success() {
        warn("chmod 644 inside container failed — patch was copied but permissions may be wrong.");
    }

    ok("DLL patched successfully.");
    Ok(())
}

/// Verify whether the patched DLL is active inside the container.
pub async fn run_verify(config: &AppConfig) -> Result<()> {
    let src = config.patch_dll_src();

    if !src.exists() {
        return Err(Error::other(format!(
            "Patch source not found: {}",
            src.display()
        )));
    }

    require_container_running()?;

    let src_md5 = md5_local(&src)?;
    let dst_md5 = md5_in_container(TARGET_PATH)?;

    info(&format!("Local  MD5 : {src_md5}"));
    info(&format!("Remote MD5 : {dst_md5}"));

    if src_md5 == dst_md5 {
        ok("Patch verified — DLL inside container matches the patch source.");
    } else {
        warn("Patch NOT applied — checksums differ. Run: odin apply-patch");
    }

    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn require_container_running() -> Result<()> {
    let out = Command::new("docker")
        .args(["inspect", "--format", "{{.State.Status}}", CONTAINER])
        .output()
        .map_err(|e| Error::docker(format!("docker inspect: {e}")))?;

    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if state != "running" {
        return Err(Error::docker(format!(
            "Container '{CONTAINER}' is not running (state: {state}). Start it first: odin start"
        )));
    }
    Ok(())
}

/// Compute MD5 of a local file using the `md5` crate.
fn md5_local(path: &std::path::Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| Error::other(format!("Cannot open {}: {e}", path.display())))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| Error::other(format!("Cannot read {}: {e}", path.display())))?;
    let digest = md5::compute(&buf);
    Ok(format!("{:x}", digest))
}

/// Compute MD5 of a file inside the container via `docker exec md5sum`.
fn md5_in_container(container_path: &str) -> Result<String> {
    let out = Command::new("docker")
        .args(["exec", CONTAINER, "md5sum", container_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::docker(format!("docker exec md5sum: {e}")))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(Error::docker(format!(
            "md5sum inside container failed: {stderr}"
        )));
    }

    // md5sum output: "<hash>  <path>"
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::docker("Unexpected md5sum output format"))
}
