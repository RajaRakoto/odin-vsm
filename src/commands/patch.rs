//! DLL patch commands: apply-patch, verify-patch.
//!
//! ## How patching works
//!
//! The Docker image runs `PRE_SERVER_RUN_HOOK=/scripts/apply-patch.sh` before
//! every Valheim server start. That script reads `APPLY_DLL_PATCH` from the
//! container environment (injected by docker-compose from `valheim.env`).
//!
//! Docker only re-reads `valheim.env` on `docker compose up` (i.e. after a
//! `down`). A plain `restart` reuses the environment of the existing container,
//! so a change to `APPLY_DLL_PATCH` in `valheim.env` only takes effect after
//! the container is recreated.
//!
//! `odin apply-patch` handles this: it reads `APPLY_DLL_PATCH` from `valheim.env`
//! in real time, then performs `down` + `start` so the new value is injected
//! into the fresh container. The hook will then apply or skip the patch on the
//! next Valheim startup automatically.

use crate::{
    config::AppConfig,
    error::{Error, Result},
    utils::display::{confirm, info, ok, warn},
};
use std::{
    io::Read,
    process::{Command, Stdio},
};

const CONTAINER: &str = "valheim-server";
const TARGET_PATH: &str = "/opt/valheim/server/valheim_server_Data/Managed/assembly_valheim.dll";

// ── Public entry points ───────────────────────────────────────────────────────

/// Recreate the container so docker-compose picks up the current
/// `APPLY_DLL_PATCH` value from `valheim.env`.
///
/// The `PRE_SERVER_RUN_HOOK` will then apply or skip the patch automatically
/// on the next Valheim startup — no `docker cp` needed.
pub async fn run_apply(config: &AppConfig) -> Result<()> {
    if config.apply_dll_patch {
        let src = config.patch_dll_src();
        if !src.exists() {
            return Err(Error::other(format!(
                "Patch source not found: {}. Place your patched DLL at that path first.",
                src.display()
            )));
        }
        info("APPLY_DLL_PATCH=true  ->  patch will be applied on next server start.");
    } else {
        info("APPLY_DLL_PATCH=false  ->  patch will be skipped on next server start.");
    }

    let state = crate::commands::docker::container_state(CONTAINER);
    let container_exists = matches!(state.as_str(), "running" | "exited" | "paused");

    if container_exists {
        println!();
        warn("The container must be recreated for the new APPLY_DLL_PATCH value to take effect.");
        warn("This will run:  docker compose down  then  docker compose up -d");
        println!();
        if !confirm("Proceed with container recreation? (y/N)") {
            warn("Cancelled. APPLY_DLL_PATCH change will not take effect until the container is recreated.");
            return Ok(());
        }
        info("Stopping and removing the container...");
        crate::commands::docker::compose_down()?;
    }

    info("Starting container with updated environment...");
    crate::commands::docker::compose_up()?;
    println!();
    ok("Container started with the new APPLY_DLL_PATCH value.");
    ok("The PRE_SERVER_RUN_HOOK will apply or skip the patch when Valheim starts.");
    info("Monitor with:  odin logs | grep apply-patch");
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
    let src_size = std::fs::metadata(&src).map(|m| m.len()).unwrap_or(0);

    let dst_md5 = md5_in_container(TARGET_PATH)?;
    let dst_size = size_in_container(TARGET_PATH).unwrap_or(0);

    let sep = "------------------------------------------------------";
    println!("{sep}");
    println!(" Patch source   : {}", src.display());
    println!(" MD5            : {src_md5}");
    println!(" Size           : {src_size} bytes");
    println!("{sep}");
    println!(" Container target: {TARGET_PATH}");
    println!(" MD5            : {dst_md5}");
    println!(" Size           : {dst_size} bytes");
    println!("{sep}");

    if dst_md5 == "ABSENT" {
        warn("ABSENT - DLL not found inside the container.");
        return Err(Error::docker("Target DLL not found in container."));
    } else if src_md5 == dst_md5 {
        ok("OK - DLL correctly patched.");
    } else {
        warn("DIFF - DLL not patched or version mismatch. Run: odin apply-patch");
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
/// Returns `"ABSENT"` if the file does not exist inside the container.
fn md5_in_container(container_path: &str) -> Result<String> {
    let out = Command::new("docker")
        .args(["exec", CONTAINER, "md5sum", container_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::docker(format!("docker exec md5sum: {e}")))?;

    if !out.status.success() {
        return Ok("ABSENT".to_string());
    }

    // md5sum output: "<hash>  <path>"
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::docker("Unexpected md5sum output format"))
}

/// Get the byte size of a file inside the container via `docker exec stat`.
fn size_in_container(container_path: &str) -> Result<u64> {
    let out = Command::new("docker")
        .args(["exec", CONTAINER, "stat", "-c%s", container_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::docker(format!("docker exec stat: {e}")))?;

    if !out.status.success() {
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .trim()
        .parse::<u64>()
        .map_err(|_| Error::docker("Unexpected stat output format"))
}
