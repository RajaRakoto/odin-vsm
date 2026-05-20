# Deployment Guide — odin

Automated release workflow for publishing odin to GitHub Releases and crates.io.

## Quick Start

```bash
# 1. Bump version in Cargo.toml
vim Cargo.toml
# Change: version = "1.0.2" → version = "1.0.3"

# 2. Commit the version bump
git add Cargo.toml
git commit -m "chore(release): bump version to 1.0.3"
git push origin master

# 3. Run the deployment script
./scripts/deploy.sh

# Done! GitHub Actions builds binaries, script publishes to crates.io
```

## Prerequisites

### Local Setup

```bash
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Git configured
git config user.name "Raja"
git config user.email "raja.rakoto7@gmail.com"

# 3. Cargo login (one-time)
cargo login
# Paste your crates.io API token from https://crates.io/me
```

### GitHub Setup

The repository already has `.github/workflows/release.yml` configured. It:
- Detects version bumps in `Cargo.toml`
- Builds binaries for 4 platforms (linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64)
- Creates a GitHub Release with all binaries attached
- Generates changelog from git commits

No additional GitHub configuration needed.

## Deployment Workflow

### Step 1: Bump Version

Edit `Cargo.toml` and update the version:

```toml
[package]
name = "odin"
version = "1.0.3"  # ← Change this
```

Commit and push:

```bash
git add Cargo.toml
git commit -m "chore(release): bump version to 1.0.3"
git push origin master
```

### Step 2: Run Deploy Script

```bash
./scripts/deploy.sh
```

The script will:

1. **Validate prerequisites**
   - Check required commands (cargo, git, grep, sed)
   - Verify git user.name and user.email configured
   - Check cargo login token exists

2. **Verify version bump**
   - Compare current version with last git tag
   - Fail if version not bumped

3. **Build & Test**
   - `cargo clean && cargo build --release`
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `cargo fmt -- --check`

4. **Commit & Push**
   - Create git tag `v1.0.3`
   - Push to origin master and tag

5. **Publish to crates.io**
   - Run `cargo publish --dry-run` first
   - Run `cargo publish` if dry-run succeeds

6. **Verify**
   - Confirm git tag created
   - Display GitHub Release URL
   - Display crates.io URL

### Step 3: GitHub Actions (Automatic)

Once you push the tag, GitHub Actions automatically:

1. Detects the new version tag
2. Builds binaries for all 4 platforms
3. Creates a GitHub Release with:
   - All 4 binaries attached
   - Auto-generated changelog from commits
   - Download links for each platform

Check progress: https://github.com/RajaRakoto/odin-vsm/actions

## Script Options

### Dry-Run Mode

Test the workflow without making changes:

```bash
./scripts/deploy.sh --dry-run
```

Shows what would be done:
- ✓ Validates all prerequisites
- ✓ Checks version bump
- ✓ Shows build/test commands (doesn't run them)
- ✓ Shows git operations (doesn't execute them)
- ✓ Shows crates.io publish (doesn't execute it)

**Use this before your first deployment to verify everything is set up correctly.**

### Skip Crates.io Publish

Deploy to GitHub but skip crates.io:

```bash
./scripts/deploy.sh --skip-publish
```

Useful if you want to:
- Test the GitHub Actions workflow first
- Publish manually later
- Deploy a pre-release version

### Verbose Mode

Show detailed output:

```bash
./scripts/deploy.sh --verbose
```

Prints every command being executed.

### Combined Options

```bash
./scripts/deploy.sh --dry-run --verbose
```

## Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Deployment completed |
| 1 | General error | Check error message |
| 2 | Version not bumped | Update Cargo.toml version |
| 3 | Git status not clean | Commit all changes first |
| 4 | Build failed | Fix compilation errors |
| 5 | Tests failed | Fix failing tests |
| 6 | Publish failed | Check crates.io credentials |

## Troubleshooting

### "Version not bumped"

```
[✘] Version not bumped! Update Cargo.toml before deploying
```

**Solution:** Edit `Cargo.toml` and change the version number:

```bash
vim Cargo.toml
# Change version = "1.0.2" to version = "1.0.3"
git add Cargo.toml
git commit -m "chore(release): bump version to 1.0.3"
git push origin master
```

### "Uncommitted changes detected"

```
[✘] Uncommitted changes detected. Commit all changes before deploying
```

**Solution:** Commit all changes:

```bash
git status
git add .
git commit -m "your message"
git push origin master
```

### "Cargo credentials not found"

```
[!] Cargo credentials not found. You may need to run: cargo login
```

**Solution:** Log in to crates.io:

```bash
cargo login
# Paste your API token from https://crates.io/me
```

### "Publish dry-run failed"

```
[✘] Publish dry-run failed. Check your Cargo.toml and credentials
```

**Possible causes:**
- Invalid crates.io token: run `cargo login` again
- Missing fields in Cargo.toml: check `repository`, `homepage`, `readme`, `license`
- Package name already taken: use a different name

**Solution:**

```bash
# Verify Cargo.toml has all required fields
grep -E "^(name|version|description|license|repository|homepage)" Cargo.toml

# Re-login to crates.io
cargo login

# Try dry-run manually
cargo publish --dry-run
```

### "Git tag already exists"

```
[✘] fatal: tag 'v1.0.3' already exists
```

**Solution:** The version was already released. Bump to a new version:

```bash
vim Cargo.toml
# Change version to 1.0.4
git add Cargo.toml
git commit -m "chore(release): bump version to 1.0.4"
git push origin master
./scripts/deploy.sh
```

## Manual Workflow (If Script Fails)

If the script encounters an issue, you can run steps manually:

```bash
# 1. Build and test
cargo clean
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check

# 2. Create tag and push
git tag v1.0.3
git push origin master
git push origin v1.0.3

# 3. Publish to crates.io
cargo publish --dry-run
cargo publish
```

## Verification Checklist

After deployment, verify:

- [ ] Git tag created: `git tag | grep v1.0.3`
- [ ] GitHub Release created: https://github.com/RajaRakoto/odin-vsm/releases
- [ ] Binaries attached to release (4 files)
- [ ] Crates.io page updated: https://crates.io/crates/odin
- [ ] `cargo install odin` works with new version

## Release Notes Template

When creating a manual GitHub Release, use this template:

```markdown
## What's changed in v1.0.3

### Features
- Add `odin init` command for interactive setup
- Fetch config files dynamically from GitHub

### Fixes
- Fix timezone detection on macOS
- Improve error messages

### Documentation
- Update README with installation instructions
- Add deployment guide

---

**Full diff:** [v1.0.2...v1.0.3](https://github.com/RajaRakoto/odin-vsm/compare/v1.0.2...v1.0.3)

**Install:**
```bash
cargo install odin
# or download binary from Assets below
```
```

## Automation Tips

### Pre-commit Hook

Prevent accidental commits with wrong version:

```bash
# .git/hooks/pre-commit
#!/bin/bash
if git diff --cached Cargo.toml | grep -q '^+version'; then
    echo "Version change detected. Run ./scripts/deploy.sh after commit."
fi
```

### CI/CD Integration

The deployment script is designed to work in CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Deploy
  if: github.event_name == 'push' && github.ref == 'refs/heads/master'
  run: ./scripts/deploy.sh --skip-publish
```

### Scheduled Releases

Create a cron job for periodic releases:

```bash
# Bump patch version and deploy every Monday at 9 AM
0 9 * * 1 cd /path/to/odin-vsm && \
  sed -i 's/version = "\([0-9]*\.[0-9]*\)\.\([0-9]*\)"/version = "\1.$((\2+1))"/' Cargo.toml && \
  git add Cargo.toml && \
  git commit -m "chore(release): bump patch version" && \
  git push origin master && \
  ./scripts/deploy.sh
```

## Support

For issues or questions:

1. Check the [Troubleshooting](#troubleshooting) section
2. Review script output with `--verbose` flag
3. Run `./scripts/deploy.sh --dry-run` to test
4. Check GitHub Actions logs: https://github.com/RajaRakoto/odin-vsm/actions

## See Also

- [README.md](../README.md) — Project overview
- [.github/workflows/release.yml](../.github/workflows/release.yml) — GitHub Actions workflow
