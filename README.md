# odin — Valheim Server Manager

Odin is a modern Rust-powered CLI built for managing Dockerized Valheim dedicated servers with reliability, performance, and simplicity in mind. Designed around type-safe configuration, asynchronous mod downloads via the Thunderstore API, and structured error handling, it delivers a seamless server management experience through a single, dependency-free binary. From server lifecycle automation and BepInEx mod management to cross-platform world synchronization between Windows and Linux using rclone and Tailscale, Odin handles the infrastructure complexity so you can stay focused on the game.

## Table of contents

1. [Why Rust?](#why-rust)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Configuration (`valheim.env`)](#configuration-valheimenvenvironmentvariables)
5. [CLI commands](#cli-commands)
6. [Mod management workflow](#mod-management-workflow)
7. [World sync (Windows → Linux)](#world-sync-windows--linux)
8. [Build & test](#build--test)
9. [Project structure](#project-structure)
10. [Best practices](#best-practices-for-valheim-server-deployment)
11. [Troubleshooting](#troubleshooting)

---

## Why Rust?

- **Type safety**: Compile-time guarantees eliminate entire classes of runtime errors.
- **Performance**: Zero-cost abstractions; odin runs with minimal overhead.
- **Maintainability**: Strong module system and error handling reduce bugs and make refactoring safe.
- **Single binary**: No runtime dependencies beyond Docker and standard Unix tools.

---

## Prerequisites

### System

| Requirement | Minimum | Notes |
|---|---|---|
| Linux kernel | 4.11+ | overlay2 support for Docker |
| CPU cores | 2 (4 recommended) | Valheim idle ≈ 1–2 cores |
| RAM | 4 GB (8 GB recommended) | Valheim idle ≈ 2.8 GB |
| Free disk | 10 GB+ | ~1 GB Docker image + world saves |

### Required binaries

| Binary | Purpose | Install |
|---|---|---|
| `docker` + compose v2 | Container runtime | `apt install docker.io` |
| `wget` | Mod downloads | `apt install wget` |
| `7z` | Mod extraction, world backups | `apt install p7zip-full` |
| `curl` | External IP detection | `apt install curl` |
| `zip` | Project snapshots | `apt install zip` |
| `rclone` | World sync (optional) | `apt install rclone` |
| `ssh` | World sync pre-flight (optional) | `apt install openssh-client` |
| `tailscale` | VPN for sync (optional) | See tailscale.com |

### Rust toolchain (for building from source)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version   # 1.75+ recommended
```

---

## Installation

### Option A — Build from source (recommended)

```bash
# 1. Clone the repository
git clone https://github.com/yourorg/odin-valheim.git
cd odin-valheim

# 2. Build the release binary
cargo build --release

# 3. Place the binary next to your docker-compose.yml
cp target/release/odin /srv/valheim/odin

# 4. Create your configuration (see next section)
cp valheim.env.example /srv/valheim/valheim.env
nano /srv/valheim/valheim.env

# 5. Run the health check
cd /srv/valheim
./odin health
```

### Option B — Direct binary deploy

Copy the pre-built `odin` binary and `valheim.env` to the same directory as your `docker-compose.yml`.

```
/srv/valheim/
├── docker-compose.yml
├── odin              ← binary
├── valheim.env       ← your configuration
├── config/           ← created automatically
│   ├── backups/
│   ├── worlds_local/
│   └── bepinex/
│       └── plugins/
├── data/             ← created automatically (steamcmd)
└── mods_list.txt     ← optional, for mod management
```

> **Note:** `odin` always looks for `valheim.env` in the same directory as the binary, falling back to the working directory.

---

## Configuration (`valheim.env`) — Environment variables

Copy `valheim.env.example` to `valheim.env` and edit it.

### Core server settings

| Variable | Default | Description |
|---|---|---|
| `SERVER_NAME` | `My Server` | Name shown in the Steam server browser |
| `WORLD_NAME` | `Dedicated` | World save file name (no extension) |
| `SERVER_PASS` | *(empty)* | Password — **must be ≥ 5 characters** |
| `SERVER_PUBLIC` | `false` | `true` to list publicly on Steam |
| `TZ` | `Etc/UTC` | Timezone for cron schedules (IANA format) |

### Automatic scheduling (cron, 5-field format)

| Variable | Default | Description |
|---|---|---|
| `UPDATE_CRON` | *(empty)* | Auto-pull latest Docker image |
| `RESTART_CRON` | *(empty)* | Auto-restart container |
| `BACKUPS_CRON` | *(empty)* | Auto-backup world saves |

Cron examples:

```
0 4 * * *      → Daily at 04:00
0 */6 * * *    → Every 6 hours
*/30 * * * *   → Every 30 minutes
0 3 * * 1      → Every Monday at 03:00
```

### Features

| Variable | Default | Description |
|---|---|---|
| `CROSSPLAY` | `false` | Enable Xbox/Game Pass crossplay |
| `SUPERVISOR_HTTP` | `false` | Enable Supervisor web UI (port 9001) |
| `SUPERVISOR_HTTP_PASS` | *(empty)* | Supervisor web password |
| `BEPINEX` | `false` | Enable BepInEx mod loader |
| `VALHEIM_PLUS` | `false` | Enable Valheim+ (mutually exclusive with BepInEx) |

### Container user mapping

| Variable | Default | Description |
|---|---|---|
| `PUID` | `1000` | UID that owns `./data` and `./config` |
| `PGID` | `1000` | GID that owns `./data` and `./config` |

### Windows sync (optional)

| Variable | Default | Description |
|---|---|---|
| `WIN_USER` | *(current user)* | Windows account name |
| `WIN_HOST` | *(empty)* | Windows IP or hostname |
| `WIN_SSH_USER` | `WIN_USER` | SSH login on Windows |
| `WIN_SSH_PORT` | `22` | SSH port on Windows |
| `WIN_SSH_KEY` | *(empty)* | Absolute path to SSH private key on this Linux server |

---

## CLI commands

```
odin <COMMAND> [OPTIONS]
```

**Quick start:** Run `./odin` with no arguments to see the full command guide organized by category.

### Diagnostic

```bash
odin health
```

Runs 8 sections of checks: system resources, required binaries, Docker daemon, config files, volumes, mods, network ports, and steamcmd quirks. **Recommended before first use.**

### Server lifecycle

```bash
odin start            # docker compose up -d
odin stop             # graceful stop (waits up to 2 min for world save)
odin restart          # docker compose restart
odin down             # remove container (config/ and data/ preserved)
odin update           # pull latest image, restart
```

### Monitoring

```bash
odin status           # show full server status (passwords hidden)
odin status-password  # same, with passwords revealed
odin logs             # stream logs (last 50 lines)
odin logs 200         # stream logs (last 200 lines)
odin shell            # open an interactive bash shell in the container
```

### Backup & restore

```bash
odin backup           # trigger a manual backup via Supervisor
odin clear-backups    # delete all files in config/backups/ (interactive)
odin restore-worlds   # interactively select and restore a world backup
odin snapshot         # archive the whole project to ~/valheim-server.bak.zip
```

### Mod management

```bash
odin filter-mods      # query Thunderstore API, classify mods, update mods_list.txt
odin download-mods    # download all mods in mods_list.txt to mods_cache/
odin install-mods     # download + extract mods into config/bepinex/plugins/
odin clear-mods       # full cleanup: stop server, backup worlds, remove mods interactively
```

**Progress reporting**: Mod downloads and installations display a clean single-line progress bar with inline warnings, eliminating verbose output clutter.

### World sync

```bash
odin sync-worlds --help-guide   # print the setup guide
odin sync-worlds                # destructive one-way sync Windows → Linux
```

### Fixes

```bash
odin fix permission   # chown 1000:1000 + chmod 755 on ./data and ./config
```

---

## Mod management workflow

`odin` uses `mods_list.txt` as the source of truth.

### Format of `mods_list.txt`

```
# Lines starting with # are ignored
# Format: Author-ModName-Version
# Version can be omitted — odin always resolves the latest via the API

Azumatt-AzuAutoStore-1.2.3
ValheimModding-Jotunn-2.20.0
# SomeAuthor-ClientOnlyMod-1.0.0*    ← trailing * → skip download entirely
# SomeAuthor-ForceBothMod-1.0.0**    ← trailing ** → force classify as "both"
```

### Typical mod workflow

```bash
# 1. Populate mods_list.txt with Author-Mod-Version entries

# 2. Classify mods (server/client/both) via Thunderstore API
#    This updates mods_list.txt automatically after confirmation
odin filter-mods

# 3. Install filtered mods to plugins/
odin install-mods

# 4. Start the server
odin start
```

### Updating mods

```bash
odin clear-mods      # stops server, backs up worlds, removes old mods (interactive)
# Edit mods_list.txt with new versions
odin install-mods    # downloads and installs fresh
odin start
```

---

## World sync (Windows → Linux)

This feature copies your Valheim save files from a Windows machine to the Linux server via rclone SFTP.

### Setup

```bash
# On Windows: enable OpenSSH Server
# Settings → Apps → Optional features → OpenSSH Server

# On Linux: generate a key pair
ssh-keygen -t ed25519 -f ~/.ssh/valheim_sync

# Copy the public key to Windows
ssh-copy-id -i ~/.ssh/valheim_sync.pub username@windows-ip

# Configure valheim.env
WIN_USER=YourWindowsUsername
WIN_HOST=100.x.x.x          # Tailscale IP recommended
WIN_SSH_USER=YourWindowsUsername
WIN_SSH_PORT=22
WIN_SSH_KEY=/home/youruser/.ssh/valheim_sync
```

### Run the sync

```bash
odin sync-worlds --help-guide   # read this first
odin backup                      # snapshot current server worlds
odin sync-worlds                 # ⚠ destructive — overwrites server worlds
odin start
```

> The sync stops if Valheim.exe is detected running on Windows, or if players are connected to the server.

---

## Build & test

### Build in release mode

```bash
cargo build --release
# Binary: target/release/odin
```

With cross-compilation for a remote server:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# Static binary: target/x86_64-unknown-linux-musl/release/odin
```

### Run unit tests

```bash
# All tests (unit + doc)
cargo test

# Only the CLI parsing tests
cargo test --lib cli

# Only config tests
cargo test --lib config

# Verbose output
cargo test -- --nocapture
```

### Run clippy (lints)

```bash
cargo clippy -- -D warnings
```

### Validate binary in place

Once built, copy the binary next to a valid `valheim.env`:

```bash
cp target/release/odin /srv/valheim/odin
cd /srv/valheim

# Smoke-test: print help
./odin --help

# Smoke-test: run health check
./odin health

# Smoke-test: show status (docker must be running)
./odin status
```

### CI-recommended test matrix

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "==> Formatting check"
cargo fmt --check

echo "==> Clippy"
cargo clippy -- -D warnings

echo "==> Unit tests"
cargo test

echo "==> Release build"
cargo build --release

echo "==> CLI self-test (--help)"
./target/release/odin --help

echo "==> CLI self-test (--version)"
./target/release/odin --version

echo "==> Parse smoke-test: start"
./target/release/odin start --help

echo "==> Parse smoke-test: logs default"
# This will fail without Docker but tests CLI parsing:
./target/release/odin logs --help || true

echo "All checks passed."
```

---

## Project structure

```
odin/
├── Cargo.toml
└── src/
    ├── main.rs           — binary entry point (env load, banner, dispatch)
    ├── lib.rs            — library root (re-exports all modules)
    ├── cli.rs            — clap CLI definition (all commands + FixSub)
    ├── config.rs         — AppConfig (valheim.env → typed struct)
    ├── error.rs          — Error enum + Result alias (thiserror)
    ├── api/
    │   ├── mod.rs
    │   └── thunderstore.rs  — Thunderstore REST API client
    ├── commands/
    │   ├── mod.rs
    │   ├── backups.rs    — clear-backups
    │   ├── docker.rs     — start / stop / restart / down / logs / update / backup / snapshot / shell
    │   ├── fix.rs        — fix permission
    │   ├── health.rs     — health (8-section diagnostic)
    │   ├── mods.rs       — filter-mods / download-mods / install-mods / clear-mods
    │   ├── status.rs     — status / status-password
    │   └── worlds.rs     — restore-worlds / sync-worlds
    └── utils/
        ├── mod.rs
        ├── banner.rs     — print_banner() / print_help()
        ├── display.rs    — info / ok / warn / err / section / separator_n / confirm
        ├── env.rs        — env_get() (reads a key from valheim.env)
        ├── fs.rs         — sudo_run / sudo_rm_rf / sudo_mkdir_p / dir_is_empty / file_mtime_str
        └── net.rs        — internal_ips() / external_ip()
```

---

## Best practices for Valheim server deployment

### Initial setup

1. **Run health check first**: `odin health` validates system resources, dependencies, Docker, and network configuration before any server operations.
2. **Configure `valheim.env` carefully**: Pay special attention to `SERVER_PASS` (≥5 chars), `PUID`/`PGID` (match your user), and timezone for cron schedules.
3. **Test with a small mod set**: Start with 5–10 mods to verify the mod pipeline works before scaling up.

### Ongoing maintenance

**Backups**: Enable `BACKUPS_CRON` in `valheim.env` for automatic world snapshots. Manual backups via `odin backup` are also available.

**Updates**: Use `UPDATE_CRON` for automatic Docker image pulls, or run `odin update` manually. Always backup before updating.

**Mod updates**: Use `odin clear-mods` to cleanly remove old mods and back up worlds before installing new versions. This prevents mod conflicts and corrupted saves.

**Monitoring**: Check `odin logs` regularly for errors. Use `odin status` to verify player counts and server health.

### Performance tuning

- **CPU**: Valheim idles at 1–2 cores; allocate 4+ cores for 10+ concurrent players.
- **RAM**: Idle ≈2.8 GB; add ~200 MB per concurrent player. Monitor with `odin status`.
- **Disk**: Keep 10+ GB free for world saves and backups. Use `odin clear-backups` to prune old snapshots.
- **Network**: Ensure ports 2456–2457 (UDP) are open and forwarded. Test with `odin health`.

### Mod management workflow

1. **Classify mods first**: `odin filter-mods` queries Thunderstore to identify server-side, client-only, and both-side mods.
2. **Install filtered mods**: `odin install-mods` downloads and extracts only server-side and both-side mods.
3. **Verify installation**: Check `config/bepinex/plugins/` is non-empty, then restart the server.
4. **Monitor logs**: `odin logs 100` shows mod load errors immediately after restart.

### World sync (Windows → Linux)

- **Pre-flight checks**: `odin sync-worlds` automatically verifies Valheim.exe is not running on Windows and no players are connected.
- **Backup first**: Always run `odin backup` before syncing to preserve server-side progress.
- **Use Tailscale**: Configure `WIN_HOST` to a Tailscale IP for secure, encrypted cross-network sync.
- **SSH key setup**: Generate a key pair with `ssh-keygen -t ed25519` and copy the public key to Windows via `ssh-copy-id`.

---

## Troubleshooting

### `docker daemon is not running`

```bash
sudo systemctl start docker
sudo systemctl enable docker
```

### `fix permission` — ownership errors on startup

```bash
odin fix permission
# or manually:
sudo chown -R 1000:1000 ./data ./config
sudo chmod -R 755 ./data ./config
```

### SERVER_PASS too short (server refuses to start)

Valheim requires `SERVER_PASS` to be at least 5 characters. Update `valheim.env` then restart:

```bash
odin stop
# edit valheim.env
odin start
```

### ZFS filesystem — steamcmd fails with "250 MB required"

```bash
# Find the ZFS volume name
df -T ./data

# Apply a quota ≤ 2 TB
zfs set quota=500G <pool/dataset>
```

### `odin health` reports `jq` missing

`jq` is **not required** by this Rust version of odin — all Thunderstore API calls are handled natively. The warning is informational only.

### Mods not loading after `install-mods`

1. Confirm `BEPINEX=true` in `valheim.env`.
2. Confirm `config/bepinex/plugins/` is non-empty.
3. Restart the server: `odin restart`.
4. Check logs: `odin logs 100`.

### `sync-worlds` fails — SSH connection refused

- Confirm OpenSSH Server is running on Windows.
- Confirm the Tailscale VPN tunnel is active on both machines.
- Test manually: `ssh -i ~/.ssh/valheim_sync user@windows-ip echo ok`
- Run: `odin sync-worlds --help-guide` for the full setup guide.

---

## License

MIT — see `LICENSE`.