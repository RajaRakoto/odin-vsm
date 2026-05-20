//! `health` command — full environment diagnostic.
//!
//! Mirrors `cmd_health` from `odin.sh`.
//! Runs 8 sections of checks and summarises the results.

use crate::{
    config::AppConfig,
    error::Result,
    utils::{env::env_get, fs::dir_is_empty},
};
use colored::Colorize;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

const CONTAINER: &str = "valheim-server";
const VALHEIM_IMAGE: &str = "ghcr.io/community-valheim-tools/valheim-server";

struct Health {
    checks: u32,
    errors: u32,
    warnings: u32,
}

impl Health {
    fn new() -> Self {
        Self {
            checks: 0,
            errors: 0,
            warnings: 0,
        }
    }

    fn check(&mut self, label: &str, status: CheckStatus, detail: &str) {
        self.checks += 1;
        match status {
            CheckStatus::Ok => {
                let d = if detail.is_empty() {
                    String::new()
                } else {
                    format!(" \x1b[0;36m{detail}\x1b[0m")
                };
                println!("  {} {:<52}{}", "✔".green(), label, d);
            }
            CheckStatus::Warn => {
                self.warnings += 1;
                let d = if detail.is_empty() {
                    String::new()
                } else {
                    format!(" \x1b[1;33m{detail}\x1b[0m")
                };
                println!("  {} {:<52}{}", "⊘".yellow(), label, d);
            }
            CheckStatus::Err => {
                self.errors += 1;
                let d = if detail.is_empty() {
                    String::new()
                } else {
                    format!(" \x1b[0;31m{detail}\x1b[0m")
                };
                println!("  {} {:<52}{}", "✘".red(), label, d);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CheckStatus {
    Ok,
    Warn,
    Err,
}

fn hsep() {
    println!("  {}", "─".repeat(60).cyan());
}

fn cmd_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_cmd(prog: &str, args: &[&str]) -> String {
    Command::new(prog)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn docker_active() -> bool {
    Command::new("docker")
        .args(["info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub async fn run(config: &AppConfig) -> Result<()> {
    let mut h = Health::new();

    println!();
    println!("  \x1b[1;36m╔══════════════════════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[1;36m║          Valheim Server — Health Diagnostic              ║\x1b[0m");
    println!("  \x1b[1;36m╚══════════════════════════════════════════════════════════╝\x1b[0m");
    println!();
    println!("  \x1b[0;36mRecommended first command before any use.\x1b[0m");
    println!("  \x1b[0;36mChecks all system, Docker, and project prerequisites.\x1b[0m");
    println!();

    section_1_system(&mut h);
    section_2_binaries(&mut h);
    section_3_docker(&mut h);
    section_4_config(config, &mut h);
    section_5_volumes(config, &mut h);
    section_6_mods(config, &mut h);
    section_7_network(&mut h);
    section_8_steamcmd(config, &mut h);

    hsep();
    println!();
    println!("  \x1b[1mChecks performed: \x1b[0;36m{}\x1b[0m", h.checks);
    if h.errors == 0 && h.warnings == 0 {
        println!("  \x1b[0;32m\x1b[1m✔  Environment is perfectly healthy — ready to use.\x1b[0m");
        println!();
        println!("  \x1b[0;36m→\x1b[0m  Next step: \x1b[1modin start\x1b[0m");
    } else if h.errors == 0 {
        println!(
            "  \x1b[1;33m\x1b[1m⊘  {} warning(s) — functional but worth monitoring.\x1b[0m",
            h.warnings
        );
        println!();
        println!("  \x1b[0;36m→\x1b[0m  Next step: \x1b[1modin start\x1b[0m  \x1b[1;33m(review the ⊘ above)\x1b[0m");
    } else {
        println!("  \x1b[0;31m\x1b[1m✘  {} critical error(s) · {} warning(s) — fix before continuing.\x1b[0m", h.errors, h.warnings);
        println!();
        println!("  \x1b[0;31m→\x1b[0m  Fix the \x1b[0;31m✘\x1b[0m above before running \x1b[1modin start\x1b[0m.");
    }
    println!();
    hsep();
    println!();
    Ok(())
}

// ── Section 1 ─────────────────────────────────────────────────────────────────

fn section_1_system(h: &mut Health) {
    println!("\n{} {}", "►".cyan().bold(), "1/8 · System & user".bold());
    hsep();

    let os_name = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
        .unwrap_or_else(|| run_cmd("uname", &["-s"]));
    h.check("Operating system", CheckStatus::Ok, &os_name);

    let kver = run_cmd("uname", &["-r"]);
    let parts: Vec<u64> = kver
        .split('.')
        .take(2)
        .filter_map(|s| s.parse().ok())
        .collect();
    let (kmaj, kmin) = (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
    );
    if kmaj > 4 || (kmaj == 4 && kmin >= 11) {
        h.check("Kernel ≥ 4.11 (overlay2)", CheckStatus::Ok, &kver);
    } else {
        h.check(
            "Kernel ≥ 4.11 (overlay2)",
            CheckStatus::Warn,
            &format!("{kver} — update recommended"),
        );
    }

    let cur_user = run_cmd("id", &["-un"]);
    let uid_s = run_cmd("id", &["-u"]);
    h.check(
        "Current user",
        CheckStatus::Ok,
        &format!("{cur_user} (uid={uid_s})"),
    );
    let uid: u32 = uid_s.parse().unwrap_or(1);

    let groups = run_cmd("id", &["-nG", &cur_user]);
    if uid == 0 {
        h.check(
            "Docker group",
            CheckStatus::Ok,
            "root — native Docker access",
        );
    } else if groups.split_whitespace().any(|g| g == "docker") {
        h.check("Docker group", CheckStatus::Ok, "member of 'docker' group");
    } else {
        h.check(
            "Docker group",
            CheckStatus::Warn,
            &format!("not in 'docker' group → sudo usermod -aG docker {cur_user}"),
        );
    }

    if uid == 0 {
        h.check(
            "sudo / root access",
            CheckStatus::Ok,
            "root — no sudo needed",
        );
    } else if Command::new("sudo")
        .args(["-n", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        h.check(
            "sudo / root access",
            CheckStatus::Ok,
            "passwordless sudo available",
        );
    } else if cmd_exists("sudo") {
        h.check(
            "sudo / root access",
            CheckStatus::Warn,
            "sudo available but password required",
        );
    } else {
        h.check(
            "sudo / root access",
            CheckStatus::Err,
            "sudo not found — some operations will fail",
        );
    }

    let ncpu: u32 = fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.lines().filter(|l| l.starts_with("processor")).count() as u32)
        .unwrap_or(0);
    if ncpu >= 4 {
        h.check(
            "CPU cores (recommended ≥ 4)",
            CheckStatus::Ok,
            &format!("{ncpu} cores detected"),
        );
    } else if ncpu >= 2 {
        h.check(
            "CPU cores (recommended ≥ 4)",
            CheckStatus::Warn,
            &format!("{ncpu} cores — limited performance"),
        );
    } else {
        h.check(
            "CPU cores (recommended ≥ 4)",
            CheckStatus::Err,
            &format!("{ncpu} core(s) — insufficient for Valheim"),
        );
    }

    let ram_kb: u64 = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0);
    let ram_gb = ram_kb / 1024 / 1024;
    if ram_gb >= 8 {
        h.check(
            "RAM (recommended ≥ 8 GB)",
            CheckStatus::Ok,
            &format!("{ram_gb} GB detected"),
        );
    } else if ram_gb >= 4 {
        h.check(
            "RAM (recommended ≥ 8 GB)",
            CheckStatus::Warn,
            &format!("{ram_gb} GB — Valheim idle ≈ 2.8 GB"),
        );
    } else {
        h.check(
            "RAM (recommended ≥ 8 GB)",
            CheckStatus::Err,
            &format!("{ram_gb} GB — insufficient"),
        );
    }

    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".into());
    let df_out = run_cmd("df", &["-k", &cwd]);
    let df_avail: u64 = df_out
        .lines()
        .last()
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let df_gb = df_avail / 1024 / 1024;
    if df_gb >= 10 {
        h.check(
            "Free disk space ≥ 10 GB",
            CheckStatus::Ok,
            &format!("{df_gb} GB available"),
        );
    } else if df_gb >= 5 {
        h.check(
            "Free disk space ≥ 10 GB",
            CheckStatus::Warn,
            &format!("{df_gb} GB — monitor closely"),
        );
    } else {
        h.check(
            "Free disk space ≥ 10 GB",
            CheckStatus::Err,
            &format!("{df_gb} GB — risk of disk full"),
        );
    }
}

// ── Section 2 ─────────────────────────────────────────────────────────────────

fn section_2_binaries(h: &mut Health) {
    println!(
        "\n{} {}",
        "►".cyan().bold(),
        "2/8 · Binaries & required dependencies".bold()
    );
    hsep();

    let checks: &[(&str, CheckStatus, &str)] = &[
        (
            "docker",
            CheckStatus::Err,
            "MISSING — required for all commands",
        ),
        (
            "curl",
            CheckStatus::Err,
            "MISSING — required for external IP, filter-mods",
        ),
        (
            "wget",
            CheckStatus::Err,
            "MISSING — required for install-mods",
        ),
        (
            "7z",
            CheckStatus::Err,
            "MISSING — required for install-mods, clear-mods (p7zip-full)",
        ),
        ("zip", CheckStatus::Warn, "MISSING — required for snapshot"),
        ("unzip", CheckStatus::Warn, "MISSING — utility tool"),
        (
            "jq",
            CheckStatus::Warn,
            "MISSING — filter-mods is now native Rust (jq no longer required)",
        ),
        (
            "rclone",
            CheckStatus::Err,
            "MISSING — required for sync-worlds",
        ),
    ];
    for (cmd, miss, miss_msg) in checks {
        if cmd_exists(cmd) {
            let ver = cmd_version(cmd);
            h.check(
                &format!("Binary: {cmd}"),
                CheckStatus::Ok,
                if ver.is_empty() { "(available)" } else { &ver },
            );
        } else {
            h.check(&format!("Binary: {cmd}"), *miss, miss_msg);
        }
    }

    if cmd_exists("tailscale") {
        let ver = cmd_version("tailscale");
        h.check(
            "Binary: tailscale (sync-worlds VPN)",
            CheckStatus::Ok,
            if ver.is_empty() { "(available)" } else { &ver },
        );
    } else {
        h.check(
            "Binary: tailscale (sync-worlds VPN)",
            CheckStatus::Warn,
            "MISSING — required for secure tunnel",
        );
    }

    if cmd_exists("ssh") {
        let ver = Command::new("ssh")
            .arg("-V")
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stderr)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default();
        h.check(
            "Binary: ssh (sync-worlds pre-flight)",
            CheckStatus::Ok,
            if ver.is_empty() { "(available)" } else { &ver },
        );
    } else {
        h.check(
            "Binary: ssh (sync-worlds pre-flight)",
            CheckStatus::Warn,
            "MISSING — required for Valheim.exe check",
        );
    }

    let compose_ok = Command::new("docker")
        .args(["compose", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if compose_ok {
        let ver = run_cmd("docker", &["compose", "version", "--short"]);
        h.check("docker compose (plugin v2)", CheckStatus::Ok, &ver);
    } else if cmd_exists("docker-compose") {
        h.check(
            "docker compose (plugin v2)",
            CheckStatus::Warn,
            "docker-compose v1 detected — migrate to plugin v2",
        );
    } else {
        h.check(
            "docker compose (plugin v2)",
            CheckStatus::Err,
            "MISSING — required for start/stop/restart/update",
        );
    }
}

fn cmd_version(cmd: &str) -> String {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .chars()
                .take(30)
                .collect()
        })
        .unwrap_or_default()
}

// ── Section 3 ─────────────────────────────────────────────────────────────────

fn section_3_docker(h: &mut Health) {
    println!(
        "\n{} {}",
        "►".cyan().bold(),
        "3/8 · Docker daemon & configuration".bold()
    );
    hsep();

    if docker_active() {
        let ver = run_cmd("docker", &["info", "--format", "{{.ServerVersion}}"]);
        h.check("Docker daemon active", CheckStatus::Ok, &ver);

        let driver = run_cmd("docker", &["info", "--format", "{{.Driver}}"]);
        if driver == "overlay2" {
            h.check(
                "Storage driver (overlay2 recommended)",
                CheckStatus::Ok,
                &driver,
            );
        } else {
            h.check(
                "Storage driver (overlay2 recommended)",
                CheckStatus::Warn,
                &format!("{driver} — overlay2 is preferred"),
            );
        }

        let cgroup = run_cmd("docker", &["info", "--format", "{{.CgroupVersion}}"]);
        if cgroup == "2" {
            h.check(
                "Cgroups v2 (resource limits)",
                CheckStatus::Ok,
                "cgroups v2 active",
            );
        } else {
            h.check(
                "Cgroups v2 (resource limits)",
                CheckStatus::Warn,
                &format!("cgroups v{cgroup} — compose limits may be ignored"),
            );
        }

        let img_tag = format!("{VALHEIM_IMAGE}:latest");
        let img_ok = Command::new("docker")
            .args(["image", "inspect", &img_tag])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if img_ok {
            let size = run_cmd(
                "docker",
                &["image", "inspect", &img_tag, "--format", "{{.Size}}"],
            );
            let mb = size.parse::<u64>().unwrap_or(0) / 1024 / 1024;
            let date: String = run_cmd(
                "docker",
                &["image", "inspect", &img_tag, "--format", "{{.Created}}"],
            )
            .chars()
            .take(10)
            .collect();
            h.check(
                "Valheim Docker image (local)",
                CheckStatus::Ok,
                &format!("{mb} MB — created on {date}"),
            );
        } else {
            h.check(
                "Valheim Docker image (local)",
                CheckStatus::Warn,
                "not pulled — will be downloaded on first 'start' (~1 GB)",
            );
        }

        let state = crate::commands::docker::container_state(CONTAINER);
        match state.as_str() {
            "running" => h.check(
                &format!("Container {CONTAINER}"),
                CheckStatus::Ok,
                "running (already started)",
            ),
            "exited" => h.check(
                &format!("Container {CONTAINER}"),
                CheckStatus::Warn,
                "exited — run 'odin start' to launch",
            ),
            "absent" => h.check(
                &format!("Container {CONTAINER}"),
                CheckStatus::Ok,
                "absent — will be created on next 'start'",
            ),
            other => h.check(
                &format!("Container {CONTAINER}"),
                CheckStatus::Warn,
                &format!("state: {other}"),
            ),
        }

        let nets = run_cmd("docker", &["network", "ls", "--format", "{{.Name}}"]);
        if nets.lines().any(|l| l == "bridge") {
            h.check(
                "Default Docker 'bridge' network",
                CheckStatus::Ok,
                "present",
            );
        } else {
            h.check(
                "Default Docker 'bridge' network",
                CheckStatus::Warn,
                "absent — check Docker network configuration",
            );
        }
    } else {
        h.check(
            "Docker daemon active",
            CheckStatus::Err,
            "INACTIVE — run: sudo systemctl start docker",
        );
        h.check("Storage driver", CheckStatus::Err, "N/A (daemon stopped)");
        h.check(
            "Valheim Docker image",
            CheckStatus::Err,
            "N/A (daemon stopped)",
        );
        h.check(
            &format!("Container {CONTAINER}"),
            CheckStatus::Err,
            "N/A (daemon stopped)",
        );
    }
}

// ── Section 4 ─────────────────────────────────────────────────────────────────

fn section_4_config(config: &AppConfig, h: &mut Health) {
    println!(
        "\n{} {}",
        "►".cyan().bold(),
        "4/8 · Project configuration files".bold()
    );
    hsep();

    let env_file = config.env_file();
    if env_file.exists() {
        h.check(
            "valheim.env file",
            CheckStatus::Ok,
            &env_file.display().to_string(),
        );

        let srv_name = env_get(&env_file, "SERVER_NAME", "");
        if !srv_name.is_empty() {
            h.check("  SERVER_NAME", CheckStatus::Ok, &srv_name);
        } else {
            h.check(
                "  SERVER_NAME",
                CheckStatus::Warn,
                "not set — using default 'My Server'",
            );
        }

        let srv_pass = env_get(&env_file, "SERVER_PASS", "");
        let plen = srv_pass.len();
        if plen >= 5 {
            h.check(
                "  SERVER_PASS (≥ 5 chars)",
                CheckStatus::Ok,
                &format!("{plen} characters"),
            );
        } else if plen > 0 {
            h.check(
                "  SERVER_PASS (≥ 5 chars)",
                CheckStatus::Err,
                &format!("{plen} character(s) — server will refuse to start!"),
            );
        } else {
            h.check(
                "  SERVER_PASS (≥ 5 chars)",
                CheckStatus::Err,
                "empty — the server will refuse to start!",
            );
        }

        let world_name = env_get(&env_file, "WORLD_NAME", "");
        if !world_name.is_empty() {
            h.check("  WORLD_NAME", CheckStatus::Ok, &world_name);
        } else {
            h.check(
                "  WORLD_NAME",
                CheckStatus::Warn,
                "not set — using 'Dedicated'",
            );
        }

        let tz_val = env_get(&env_file, "TZ", "Etc/UTC");
        if Path::new(&format!("/usr/share/zoneinfo/{tz_val}")).exists() {
            h.check("  TZ (valid timezone)", CheckStatus::Ok, &tz_val);
        } else {
            h.check(
                "  TZ (valid timezone)",
                CheckStatus::Warn,
                &format!("{tz_val} — not found in /usr/share/zoneinfo"),
            );
        }

        let bepinex = env_get(&env_file, "BEPINEX", "false");
        let vplus = env_get(&env_file, "VALHEIM_PLUS", "false");
        let bep_on = crate::config::parse_bool(&bepinex);
        let vp_on = crate::config::parse_bool(&vplus);
        if bep_on && vp_on {
            h.check(
                "  BEPINEX/VALHEIM_PLUS (mutually exclusive)",
                CheckStatus::Err,
                "Both enabled — not allowed!",
            );
        } else {
            h.check(
                "  BEPINEX/VALHEIM_PLUS (mutually exclusive)",
                CheckStatus::Ok,
                &format!(
                    "BEPINEX={}  VALHEIM_PLUS={}",
                    crate::config::bool_label(bep_on),
                    crate::config::bool_label(vp_on)
                ),
            );
        }

        let sup_http = env_get(&env_file, "SUPERVISOR_HTTP", "false");
        let sup_pass = env_get(&env_file, "SUPERVISOR_HTTP_PASS", "");
        if crate::config::parse_bool(&sup_http) {
            if !sup_pass.is_empty() {
                h.check(
                    "  SUPERVISOR_HTTP_PASS (if HTTP enabled)",
                    CheckStatus::Ok,
                    &format!("set ({} chars)", sup_pass.len()),
                );
            } else {
                h.check(
                    "  SUPERVISOR_HTTP_PASS (if HTTP enabled)",
                    CheckStatus::Warn,
                    "empty — no authentication!",
                );
            }
        } else {
            h.check(
                "  SUPERVISOR_HTTP (disabled)",
                CheckStatus::Ok,
                "Supervisor web interface disabled",
            );
        }
    } else {
        h.check(
            "valheim.env file",
            CheckStatus::Err,
            &format!("MISSING ({}) — all commands will fail", env_file.display()),
        );
    }

    // docker-compose.yml
    let cf = ["docker-compose.yml", "docker-compose.yaml"]
        .iter()
        .map(|f| config.script_dir.join(f))
        .find(|p| p.exists());
    if let Some(cf) = cf {
        h.check(
            "docker-compose.yml file",
            CheckStatus::Ok,
            &cf.display().to_string(),
        );
        let valid = Command::new("docker")
            .args(["compose", "-f", &cf.to_string_lossy(), "config"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if valid {
            h.check("  docker-compose YAML syntax", CheckStatus::Ok, "valid");
        } else {
            h.check(
                "  docker-compose YAML syntax",
                CheckStatus::Err,
                "INVALID — run docker compose config to see errors",
            );
        }
    } else {
        h.check(
            "docker-compose.yml file",
            CheckStatus::Err,
            "MISSING — required for start/stop/restart/update",
        );
    }

    // odin binary
    let odin_bin = config.script_dir.join("odin");
    let odin_sh = config.script_dir.join("odin.sh");
    if odin_bin.exists() {
        h.check(
            "odin binary",
            CheckStatus::Ok,
            &odin_bin.display().to_string(),
        );
    } else if odin_sh.exists() {
        use std::os::unix::fs::PermissionsExt;
        let exec = fs::metadata(&odin_sh)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
        if exec {
            h.check(
                "odin.sh is executable",
                CheckStatus::Ok,
                &odin_sh.display().to_string(),
            );
        } else {
            h.check(
                "odin.sh is executable",
                CheckStatus::Err,
                &format!("not executable — run: chmod +x {}", odin_sh.display()),
            );
        }
    }
}

// ── Section 5 ─────────────────────────────────────────────────────────────────

fn section_5_volumes(config: &AppConfig, h: &mut Health) {
    println!(
        "\n{} {}",
        "►".cyan().bold(),
        "5/8 · Docker volumes (config/ and data/)".bold()
    );
    hsep();

    let config_dir = config.script_dir.join("config");
    if config_dir.is_dir() {
        let perm = run_cmd("stat", &["-c", "%a", &config_dir.to_string_lossy()]);
        h.check(
            "config/ directory",
            CheckStatus::Ok,
            &format!("present (permissions: {perm})"),
        );
    } else {
        h.check(
            "config/ directory",
            CheckStatus::Warn,
            "absent — will be created on first container start",
        );
    }

    let worlds_dir = config.worlds_local_dir();
    if worlds_dir.is_dir() && !dir_is_empty(&worlds_dir) {
        let fwl = count_ext(&worlds_dir, "fwl");
        let db = count_ext(&worlds_dir, "db");
        h.check(
            "config/worlds_local/ (Valheim world)",
            CheckStatus::Ok,
            &format!("{fwl} .fwl file(s) found"),
        );
        if fwl != db {
            h.check(
                "  .fwl/.db pair integrity",
                CheckStatus::Warn,
                &format!("mismatch: {fwl} .fwl / {db} .db"),
            );
        } else {
            h.check(
                "  .fwl/.db pair integrity",
                CheckStatus::Ok,
                &format!("{fwl} pair(s) detected"),
            );
        }
    } else if worlds_dir.is_dir() {
        h.check(
            "config/worlds_local/ (Valheim world)",
            CheckStatus::Warn,
            "empty — a new world will be generated on start",
        );
    } else {
        h.check(
            "config/worlds_local/ (Valheim world)",
            CheckStatus::Warn,
            "absent — a new world will be generated on start",
        );
    }

    let backups_dir = config.backups_dir();
    if backups_dir.is_dir() {
        let n = count_ext(&backups_dir, "zip");
        h.check(
            "config/backups/",
            CheckStatus::Ok,
            &format!("{n} backup archive(s)"),
        );
    } else {
        h.check(
            "config/backups/",
            CheckStatus::Warn,
            "absent — will be created on first backup",
        );
    }

    let data_dir = config.data_dir();
    if data_dir.is_dir() && !dir_is_empty(&data_dir) {
        let sz = du_sh(&data_dir);
        h.check(
            "data/ (Valheim server installed)",
            CheckStatus::Ok,
            &format!("{sz} — steamcmd already downloaded the server"),
        );
        if data_dir.join("server/valheim_server.x86_64").exists() {
            h.check(
                "  Binary valheim_server.x86_64",
                CheckStatus::Ok,
                "present in data/server/",
            );
        } else {
            h.check(
                "  Binary valheim_server.x86_64",
                CheckStatus::Warn,
                "absent — will be downloaded by steamcmd on first start",
            );
        }
        let mp = df_field(&data_dir, 5);
        if !mp.is_empty() {
            let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
            if mounts
                .lines()
                .any(|l| l.contains(&mp) && l.contains("noexec"))
            {
                h.check(
                    "  Filesystem data/ (noexec check)",
                    CheckStatus::Err,
                    "mounted with 'noexec' — server cannot run!",
                );
            } else {
                h.check(
                    "  Filesystem data/ (noexec check)",
                    CheckStatus::Ok,
                    "executable filesystem (no noexec)",
                );
            }
        }
    } else {
        h.check(
            "data/ (Valheim server installed)",
            CheckStatus::Warn,
            "empty or absent — steamcmd will download ~1 GB on first start",
        );
        let fs_type = df_field(&data_dir, 1);
        if fs_type == "zfs" {
            h.check(
                "  ZFS filesystem detected (steamcmd quirk)",
                CheckStatus::Warn,
                "ZFS without quota → steamcmd may report '250 MB required'. Apply a quota ≤ 2 TB.",
            );
        } else {
            h.check("  Filesystem type", CheckStatus::Ok, &fs_type);
        }
    }

    for (name, dir) in [("config/", &config_dir), ("data/", &data_dir)] {
        if dir.is_dir() {
            use std::os::unix::fs::MetadataExt;
            let writable = fs::metadata(dir)
                .map(|m| {
                    let uid = unsafe { libc::getuid() };
                    let mode = m.mode();
                    uid == 0 || (m.uid() == uid && mode & 0o200 != 0) || (mode & 0o002 != 0)
                })
                .unwrap_or(false);
            if writable {
                h.check(&format!("  Write access to {name}"), CheckStatus::Ok, "");
            } else {
                h.check(
                    &format!("  Write access to {name}"),
                    CheckStatus::Err,
                    "permission denied — run: sudo chown -R $(id -u):$(id -g) <dir>",
                );
            }
        }
    }
}

// ── Section 6 ─────────────────────────────────────────────────────────────────

fn section_6_mods(config: &AppConfig, h: &mut Health) {
    println!("\n{} {}", "►".cyan().bold(), "6/8 · Mods & plugins".bold());
    hsep();

    let mods_list = config.mods_list_file();
    if mods_list.exists() {
        let count = fs::read_to_string(&mods_list)
            .map(|s| {
                s.lines()
                    .filter(|l| !l.trim().starts_with('#') && !l.trim().is_empty())
                    .count()
            })
            .unwrap_or(0);
        h.check(
            "mods_list.txt",
            CheckStatus::Ok,
            &format!("{count} mod entry(ies)"),
        );
    } else {
        h.check(
            "mods_list.txt",
            CheckStatus::Warn,
            "absent — required for install-mods/filter-mods",
        );
    }

    let cache_dir = config.mods_cache_dir();
    if cache_dir.is_dir() {
        let n = count_ext(&cache_dir, "zip");
        let sz = du_sh(&cache_dir);
        h.check(
            "mods_cache/ directory",
            CheckStatus::Ok,
            &format!("{n} archive(s) cached ({sz})"),
        );
    } else {
        h.check(
            "mods_cache/ directory",
            CheckStatus::Warn,
            "absent — will be created by install-mods",
        );
    }

    let plugins_dir = config.plugins_dir();
    if plugins_dir.is_dir() {
        let n = fs::read_dir(&plugins_dir).map(|rd| rd.count()).unwrap_or(0);
        if n > 0 {
            h.check(
                "config/bepinex/plugins/",
                CheckStatus::Ok,
                &format!("{n} item(s) installed"),
            );
        } else {
            h.check(
                "config/bepinex/plugins/",
                CheckStatus::Warn,
                "empty — run install-mods if mods are required",
            );
        }
    } else {
        h.check(
            "config/bepinex/plugins/",
            CheckStatus::Warn,
            "absent — will be created by install-mods",
        );
    }
}

// ── Section 7 ─────────────────────────────────────────────────────────────────

fn section_7_network(h: &mut Health) {
    println!("\n{} {}", "►".cyan().bold(), "7/8 · Network & ports".bold());
    hsep();

    for (port, proto, label) in [
        (2456u16, "udp", "Port 2456/udp (main game)"),
        (2457u16, "udp", "Port 2457/udp (Steam queries)"),
        (2458u16, "udp", "Port 2458/udp (crossplay / RPC mods)"),
        (9001u16, "tcp", "Port 9001/tcp (Supervisor HTTP)"),
    ] {
        let flag = if proto == "udp" { "-ulnp" } else { "-tlnp" };
        let out = run_cmd("ss", &[flag]);
        let search = format!(":{port}");
        let status = out.lines().find(|l| l.contains(&search));
        match status {
            None => h.check(label, CheckStatus::Ok, "available"),
            Some(line) => {
                let lower = line.to_lowercase();
                if lower.contains("valheim") || lower.contains("docker") {
                    h.check(label, CheckStatus::Ok, "used by the Valheim container");
                } else {
                    let info: String = line
                        .split_whitespace()
                        .last()
                        .unwrap_or("")
                        .chars()
                        .take(40)
                        .collect();
                    h.check(
                        label,
                        CheckStatus::Warn,
                        &format!("IN USE by another process ({info})"),
                    );
                }
            }
        }
    }

    let internet = Command::new("curl")
        .args(["-sf", "--max-time", "5", "https://thunderstore.io/"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if internet {
        h.check(
            "Internet connectivity",
            CheckStatus::Ok,
            "access to thunderstore.io",
        );
    } else {
        h.check(
            "Internet connectivity",
            CheckStatus::Err,
            "no response — steamcmd and Thunderstore unreachable",
        );
    }

    let dns = Command::new("getent")
        .args(["hosts", "github.com"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if dns {
        h.check("DNS resolution", CheckStatus::Ok, "github.com resolved");
    } else {
        h.check(
            "DNS resolution",
            CheckStatus::Warn,
            "DNS resolution failing — updates impossible",
        );
    }
}

// ── Section 8 ─────────────────────────────────────────────────────────────────

fn section_8_steamcmd(config: &AppConfig, h: &mut Health) {
    println!(
        "\n{} {}",
        "►".cyan().bold(),
        "8/8 · SteamCMD — known fixes".bold()
    );
    hsep();

    let data_dir = config.data_dir();
    let target = if data_dir.exists() {
        data_dir.clone()
    } else {
        config.script_dir.clone()
    };

    // ZFS quota
    let fs_type = df_type_for(&target);
    if fs_type == "zfs" {
        let vol = df_field(&target, 0);
        let quota = run_cmd("zfs", &["get", "-H", "-o", "value", "quota", &vol]);
        if quota.trim() == "none" || quota.trim().is_empty() {
            h.check(
                "ZFS quota (steamcmd overflow bug)",
                CheckStatus::Err,
                &format!("No quota on {vol} → apply: zfs set quota=1TB {vol}"),
            );
        } else {
            h.check(
                "ZFS quota (steamcmd overflow bug)",
                CheckStatus::Ok,
                &format!("quota={quota} on {vol}"),
            );
        }
    } else {
        h.check(
            "ZFS overflow (non-ZFS filesystem)",
            CheckStatus::Ok,
            &format!("filesystem {fs_type} — no ZFS risk"),
        );
    }

    // noexec
    let mp = df_field(&target, 5);
    let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
    let noexec = !mp.is_empty()
        && mounts
            .lines()
            .any(|l| l.contains(&mp) && l.contains("noexec"));
    if noexec {
        h.check(
            "noexec flag on data/ (OMV/NAS bug)",
            CheckStatus::Err,
            "Mounted noexec — remove noexec from fstab.",
        );
    } else {
        h.check(
            "noexec flag on data/ (OMV/NAS bug)",
            CheckStatus::Ok,
            "no noexec detected",
        );
    }

    // Free space for steamcmd
    let df_out = run_cmd("df", &["-k", &target.to_string_lossy()]);
    let avail_kb: u64 = df_out
        .lines()
        .last()
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let avail_mb = avail_kb / 1024;
    if avail_mb >= 1000 {
        h.check(
            "Free space for steamcmd (≥ 250 MB required)",
            CheckStatus::Ok,
            &format!("{avail_mb} MB available"),
        );
    } else if avail_mb >= 250 {
        h.check(
            "Free space for steamcmd (≥ 250 MB required)",
            CheckStatus::Warn,
            &format!("{avail_mb} MB — barely enough"),
        );
    } else {
        h.check(
            "Free space for steamcmd (≥ 250 MB required)",
            CheckStatus::Err,
            &format!("{avail_mb} MB — INSUFFICIENT"),
        );
    }

    // PUID/PGID vs data/ owner
    if data_dir.is_dir() {
        let owner = run_cmd("stat", &["-c", "%U:%G", &data_dir.to_string_lossy()]);
        let puid = env_get(&config.env_file(), "PUID", "0");
        let pgid = env_get(&config.env_file(), "PGID", "0");
        h.check(
            "data/ owner vs PUID/PGID",
            CheckStatus::Ok,
            &format!("data/ owner={owner}  PUID={puid}  PGID={pgid}"),
        );
    } else {
        h.check(
            "data/ owner (will be created)",
            CheckStatus::Ok,
            "will be created on first 'start'",
        );
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn count_ext(dir: &Path, ext: &str) -> usize {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with(&format!(".{ext}"))
                })
                .count()
        })
        .unwrap_or(0)
}

fn du_sh(path: &Path) -> String {
    run_cmd("du", &["-sh", &path.to_string_lossy()])
        .split_whitespace()
        .next()
        .unwrap_or("?")
        .to_string()
}

fn df_field(path: &Path, field: usize) -> String {
    let p = if path.exists() {
        path.to_string_lossy().to_string()
    } else {
        ".".to_string()
    };
    let out = run_cmd("df", &["-T", &p]);
    out.lines()
        .last()
        .and_then(|l| l.split_whitespace().nth(field))
        .unwrap_or("")
        .to_string()
}

fn df_type_for(path: &Path) -> String {
    df_field(path, 1)
}
