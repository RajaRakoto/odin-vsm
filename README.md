# ODIN — Valheim Server Manager

Odin is a modern Rust-powered CLI built for managing Dockerized Valheim dedicated servers with reliability, performance, and simplicity in mind. Designed around type-safe configuration, asynchronous mod downloads via the Thunderstore API, and structured error handling, it delivers a seamless server management experience through a single, dependency-free binary. From server lifecycle automation and BepInEx mod management to cross-platform world synchronization between Windows and Linux using rclone and Tailscale, Odin handles the infrastructure complexity so you can stay focused on the game.

## Features

* 🚀 **Server Lifecycle** — start, stop, restart, update, and manage containers with a single command
* 🧩 **Mod Management** — classify, download, and install mods from Thunderstore with automatic dependency resolution
* 🌍 **World Sync** — seamless cross-platform world synchronization between Windows and Linux via SSH/rclone
* 💾 **Automated Backups** — scheduled world snapshots with restore capabilities and manual backup support
* 🩺 **Health Diagnostics** — comprehensive system, Docker, and configuration validation before first use
* 📊 **Monitoring & Logs** — real-time server status, log streaming, and interactive shell access
* 🛠️ **DLL Patching** — apply and verify assembly patches with toggle control via environment variables
* ⏰ **Scheduled Tasks** — cron-based automation for updates, restarts, and backups
* 🔌 **BepInEx & Valheim+** — full mod loader support with mutually exclusive configuration
* 🎮 **Crossplay Support** — Xbox and Game Pass crossplay enablement
* 🧪 **Type-Safe Config** — compile-time validated configuration with sensible defaults
* 📦 **Single Binary** — zero runtime dependencies beyond Docker and standard Unix tools

## Table of contents

1. [Why Rust?](#-why-rust)
2. [Prerequisites](#-prerequisites)
3. [Installation](#-installation)
4. [Configuration](#️-configuration-valheimenvenvironment-variables)
5. [CLI commands](#-cli-commands)
6. [Mod management](#-mod-management-workflow)
7. [World sync](#-world-sync-windows--linux)
8. [Build & test](#-build--test)
9. [Project structure](#-project-structure)
10. [Best practices](#-best-practices)
11. [Troubleshooting](#-troubleshooting)

---

## ⚡ Why Rust?

Odin leverages Rust's unique strengths to deliver a production-grade server manager that's both reliable and performant:

- **Type safety** — compile-time guarantees eliminate entire classes of runtime errors. Odin's configuration system, API responses, and error handling are all type-checked at build time, preventing silent failures in production.

- **Fearless concurrency** — async/await with Tokio enables safe, efficient parallel operations. Odin downloads mods concurrently from Thunderstore, monitors server health, and syncs worlds across networks without race conditions or deadlocks.

- **Memory safety without garbage collection** — Rust's ownership model guarantees memory safety while keeping Odin lightweight and predictable. No GC pauses means reliable server management even under sustained load.

- **Performance** — zero-cost abstractions and minimal overhead. Odin runs as a single, lean binary that starts instantly and uses negligible CPU/memory, ideal for always-on server orchestration.

- **Single binary** — no runtime dependencies beyond Docker and standard Unix tools. Deploy Odin anywhere: bare metal, containers, or embedded systems. No Python, Node, or JVM required.

- **Rich ecosystem** — Tokio (async runtime), Reqwest (HTTP client), Serde (serialization), Clap (CLI parsing). Odin uses battle-tested libraries that handle complexity so you don't have to.

- **Excellent error handling** — Result types and the `?` operator make error propagation explicit and ergonomic. Server management demands reliability; Odin's error handling is unambiguous and recoverable.

- **Cross-platform compilation** — build once, run on Linux, macOS, or Windows. Odin's world sync bridges Windows and Linux seamlessly, powered by Rust's portable standard library.

---

## 📋 Prerequisites

### System

| Requirement | Minimum | Notes |
|---|---|---|
| Linux kernel | 4.11+ | overlay2 support for Docker |
| CPU cores | 2 (4 recommended) | Valheim idle ≈ 1–2 cores |
| RAM | 4 GB (8 GB recommended) | Valheim idle ≈ 2.8 GB |
| Free disk | 10 GB+ | ~1 GB Docker image + world saves |

### Required binaries

| Binary | Purpose |
|---|---|
| `docker` + compose v2 | Container runtime |
| `7z` | Mod extraction, world backups |
| `zip` | Project snapshots |
| `rclone` | World sync *(optional)* |
| `tailscale` | VPN for sync *(optional)* |

### Rust toolchain *(build from source only)*

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

---

## 🚀 Installation

### Option A — Build from source

```bash
git clone https://github.com/yourorg/odin-valheim.git && cd odin-valheim
cargo build --release
cp target/release/odin /srv/valheim/odin
cp valheim.env.example /srv/valheim/valheim.env
cd /srv/valheim && ./odin health
```

### Option B — Direct binary deploy

```
/srv/valheim/
├── docker-compose.yml
├── odin              ← binary
├── valheim.env       ← your configuration
├── config/           ← created automatically
│   ├── backups/
│   ├── worlds_local/
│   └── bepinex/plugins/
├── data/             ← created automatically (steamcmd)
└── mods_list.txt     ← optional
```

> `odin` looks for `valheim.env` next to the binary, falling back to the working directory.

---

## ⚙️ Configuration (`valheim.env`) — Environment variables

Copy `valheim.env.example` to `valheim.env` and edit it.

### Core server settings

| Variable | Default | Description |
|---|---|---|
| `SERVER_NAME` | `My Server` | Name in the Steam server browser |
| `WORLD_NAME` | `Dedicated` | World save file name |
| `SERVER_PASS` | *(empty)* | **Must be ≥ 5 characters** |
| `SERVER_PUBLIC` | `false` | List publicly on Steam |
| `TZ` | `Etc/UTC` | Timezone for cron schedules |

### ⏰ Automatic scheduling

| Variable | Description |
|---|---|
| `UPDATE_CRON` | Auto-pull latest Docker image |
| `RESTART_CRON` | Auto-restart container |
| `BACKUPS_CRON` | Auto-backup world saves |

```
0 4 * * *      → Daily at 04:00
0 */6 * * *    → Every 6 hours
*/30 * * * *   → Every 30 minutes
```

### Features

| Variable | Default | Description |
|---|---|---|
| `CROSSPLAY` | `false` | Xbox/Game Pass crossplay |
| `SUPERVISOR_HTTP` | `false` | Supervisor web UI on port 9001 |
| `BEPINEX` | `false` | BepInEx mod loader |
| `VALHEIM_PLUS` | `false` | Valheim+ *(mutually exclusive with BepInEx)* |
| `APPLY_DLL_PATCH` | `false` | Enable/disable DLL patching — the hook always runs, the script exits early when `false` |
| `PRE_SERVER_RUN_HOOK` | `/scripts/apply-patch.sh` | Fixed hook — do not change; controls patch execution via `APPLY_DLL_PATCH` |
| `PUID` / `PGID` | `1000` | UID/GID owning `./data` and `./config` |

### Windows sync *(optional)*

| Variable | Description |
|---|---|
| `WIN_HOST` | Windows IP or hostname (Tailscale IP recommended) |
| `WIN_USER` | Windows account name |
| `WIN_SSH_USER` | SSH login on Windows |
| `WIN_SSH_PORT` | SSH port (default: 22) |
| `WIN_SSH_KEY` | Absolute path to SSH private key |

---

## 💻 CLI commands

Run `./odin` with no arguments to see the full command guide.

### 🩺 Diagnostic

```bash
odin health          # system, Docker, config, volumes, ports — run before first use
```

### Server lifecycle

```bash
odin start           # docker compose up -d
odin stop            # graceful stop (waits up to 2 min for world save)
odin restart         # docker compose restart
odin down            # remove container (volumes preserved)
odin update          # pull latest image and restart
```

### Monitoring

```bash
odin status          # full server status (passwords hidden)
odin status-password # same, with passwords revealed
odin logs [N]        # stream logs (default: 50 lines)
odin shell           # interactive shell inside the container
```

### 💾 Backup & restore

```bash
odin backup          # manual backup via Supervisor
odin clear-backups   # delete all backups in config/backups/ (interactive)
odin restore-worlds  # interactively restore a world backup
odin snapshot        # archive project to ~/valheim-server.bak.zip
```

### Mod management

```bash
odin filter-mods     # classify mods via Thunderstore API → mods_list.txt
odin download-mods   # download mods to mods_cache/ (no extraction)
odin install-mods    # download + install mods to config/bepinex/plugins/
odin clear-mods      # stop server, backup worlds, remove mods (interactive)
```

### World sync

```bash
odin sync-worlds --help-guide   # setup guide
odin sync-worlds                # ⚠ destructive — overwrites server worlds
```

### 🩹 DLL Patch

```bash
odin apply-patch     # apply APPLY_DLL_PATCH change from valheim.env (recreates container)
odin verify-patch    # verify the patched DLL is active -- shows MD5 + file sizes
```

> `APPLY_DLL_PATCH` is the only toggle — set it to `true` or `false` in `valheim.env`.
>
> **Important Docker behaviour:** `odin restart` reuses the existing container environment.
> A change to `APPLY_DLL_PATCH` in `valheim.env` is only picked up after the container
> is recreated (`down` + `start`). Run `odin apply-patch` to do this automatically:
> it reads the current value from `valheim.env`, confirms with you, then runs
> `docker compose down` + `docker compose up -d`.
>
> Once the fresh container starts, `PRE_SERVER_RUN_HOOK=/scripts/apply-patch.sh` runs
> before every Valheim startup and applies or skips the patch based on `APPLY_DLL_PATCH`.
>
> Requires `./patches/assembly_valheim.dll` and `./scripts/apply-patch.sh` (both
> mounted read-only in `docker-compose.yaml`).

### Fixes

```bash
odin fix permission  # chown 1000:1000 + chmod 755 on ./data and ./config
```

---

## 📦 Mod management workflow

`odin` uses `mods_list.txt` as the source of truth.

```
# Format: Author-ModName-Version  (version optional — always fetches latest)
Azumatt-AzuAutoStore-1.2.3
ValheimModding-Jotunn-2.20.0
# SomeAuthor-ClientOnlyMod-1.0.0*    ← * → skip entirely
# SomeAuthor-ForceBothMod-1.0.0**    ← ** → force classify as "both"
```

```bash
odin filter-mods   # Step 1 — classify
odin install-mods  # Step 2 — install server-side + both mods
odin start         # Step 3 — launch
```

**Updating mods:**

```bash
odin clear-mods    # stop, backup, remove
# edit mods_list.txt
odin install-mods && odin start
```

---

## 🌍 World sync (Windows → Linux)

Copies Valheim save files from Windows to the Linux server via rclone SFTP.

```bash
# On Windows: enable OpenSSH Server (Settings → Apps → Optional features)

# On Linux:
ssh-keygen -t ed25519 -f ~/.ssh/valheim_sync
ssh-copy-id -i ~/.ssh/valheim_sync.pub user@windows-ip

# In valheim.env:
WIN_HOST=100.x.x.x   # Tailscale IP recommended
WIN_SSH_KEY=/home/youruser/.ssh/valheim_sync
```

```bash
odin sync-worlds --help-guide   # read first
odin backup && odin sync-worlds # ⚠ destructive
odin start
```

> Sync aborts if Valheim.exe is running on Windows or players are connected.

---

## 🔨 Build & test

```bash
cargo build --release                                          # standard build
cargo build --release --target x86_64-unknown-linux-musl      # static binary

cargo test                  # all tests
cargo clippy -- -D warnings # lints
cargo fmt --check           # formatting
```

---

## 📁 Project structure

```
src/
├── main.rs           — entry point (env load, banner, dispatch)
├── cli.rs            — clap CLI (all commands)
├── config.rs         — AppConfig (valheim.env → typed struct)
├── error.rs          — Error enum + Result alias
├── api/
│   └── thunderstore.rs  — Thunderstore REST API client
├── commands/
│   ├── backups.rs    — clear-backups
│   ├── docker.rs     — start / stop / restart / down / logs / update / shell
│   ├── fix.rs        — fix permission
│   ├── health.rs     — health diagnostic
│   ├── mods.rs       — filter / download / install / clear mods
│   ├── patch.rs      — apply-patch / verify-patch
│   ├── status.rs     — status / status-password
│   └── worlds.rs     — restore-worlds / sync-worlds
└── utils/
    ├── banner.rs     — print_banner() / print_help()
    ├── display.rs    — info / ok / warn / err / confirm
    ├── fs.rs         — sudo_run / sudo_rm_rf / sudo_mkdir_p
    └── net.rs        — internal_ips() / external_ip()
```

---

## 🎮 Best practices

- Run `odin health` before first use — validates the full environment.
- Set `SERVER_PASS` to ≥ 5 characters or the server won't start.
- Match `PUID`/`PGID` to the user owning `./data` and `./config` (`id -u && id -g`).
- Enable `BACKUPS_CRON` for automatic world snapshots; always backup before `odin update`.
- Start with a small mod set (5–10) to validate the pipeline before scaling up.
- Use a Tailscale IP for `WIN_HOST` — encrypted, works across networks.

---

## 🔧 Troubleshooting

**Docker daemon not running**
```bash
sudo systemctl start docker && sudo systemctl enable docker
```

**Permission errors on startup**
```bash
odin fix permission
```

**SERVER_PASS too short** — must be ≥ 5 characters; update `valheim.env` then `odin start`.

**ZFS — steamcmd fails with "250 MB required"**
```bash
zfs set quota=500G <pool/dataset>
```

**`odin health` reports `jq` missing** — not required; all API calls are handled natively.

**Mods not loading after `install-mods`** — confirm `BEPINEX=true`, check `config/bepinex/plugins/` is non-empty, then `odin restart`.

**`sync-worlds` SSH refused** — confirm OpenSSH Server is running on Windows and Tailscale is active on both machines.

**DLL patch not taking effect after changing `APPLY_DLL_PATCH`** — `odin restart` does not re-read `valheim.env`. Run `odin apply-patch` to recreate the container with the new value. Use `odin verify-patch` to confirm.

---

## License

MIT — see `LICENSE`.
