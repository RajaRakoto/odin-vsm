#!/usr/bin/env bash
#
# deploy.sh — Automated release workflow for odin
#
# Automates: version bump verification, build, test, commit, push, crates.io publish
# Usage: ./scripts/deploy.sh [--dry-run] [--skip-publish]
#
# Prerequisites:
#   - Rust toolchain (cargo, rustc)
#   - Git configured with user.name and user.email
#   - Cargo login token configured (~/.cargo/credentials.toml)
#   - All changes committed before running
#

set -Eeuo pipefail
shopt -s inherit_errexit

# ── Configuration ─────────────────────────────────────────────────────────────

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
readonly CARGO_TOML="$PROJECT_ROOT/Cargo.toml"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly CYAN='\033[0;36m'
readonly NC='\033[0m' # No Color

# Flags
DRY_RUN=false
SKIP_PUBLISH=false
VERBOSE=false

# ── Logging ───────────────────────────────────────────────────────────────────

log_info() {
    printf "${CYAN}[INFO]${NC} %s\n" "$*" >&2
}

log_success() {
    printf "${GREEN}[✔]${NC} %s\n" "$*" >&2
}

log_warn() {
    printf "${YELLOW}[!]${NC} %s\n" "$*" >&2
}

log_error() {
    printf "${RED}[✘]${NC} %s\n" "$*" >&2
}

log_section() {
    printf "\n${CYAN}━━━ %s ━━━${NC}\n" "$*" >&2
}

# ── Error handling ────────────────────────────────────────────────────────────

trap 'on_error $? $LINENO' ERR
trap 'on_exit' EXIT

on_error() {
    local exit_code=$1
    local line_no=$2
    log_error "Script failed at line $line_no with exit code $exit_code"
    exit "$exit_code"
}

on_exit() {
    local exit_code=$?
    if [[ $exit_code -eq 0 ]]; then
        log_success "Deployment completed successfully"
    fi
    return "$exit_code"
}

# ── Utilities ─────────────────────────────────────────────────────────────────

usage() {
    cat <<'EOF'
Usage: ./scripts/deploy.sh [OPTIONS]

Automated release workflow: build → test → commit → push → publish

OPTIONS:
  --dry-run           Show what would be done without making changes
  --skip-publish      Skip crates.io publish step
  --verbose           Enable verbose output
  -h, --help          Show this help message

EXAMPLES:
  # Full deployment (build, test, commit, push, publish)
  ./scripts/deploy.sh

  # Test the workflow without making changes
  ./scripts/deploy.sh --dry-run

  # Deploy but skip crates.io publish
  ./scripts/deploy.sh --skip-publish

PREREQUISITES:
  - Cargo.toml version must be bumped before running
  - All changes must be committed (git status clean)
  - Git user.name and user.email configured
  - Cargo login token configured (~/.cargo/credentials.toml)

EXIT CODES:
  0   Success
  1   General error
  2   Version not bumped
  3   Git status not clean
  4   Build failed
  5   Tests failed
  6   Publish failed

EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run)
                DRY_RUN=true
                log_warn "DRY-RUN MODE: No changes will be made"
                shift
                ;;
            --skip-publish)
                SKIP_PUBLISH=true
                log_warn "Skipping crates.io publish"
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            -h | --help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

run_cmd() {
    local cmd=("$@")
    if [[ "$VERBOSE" == true ]]; then
        log_info "Running: ${cmd[*]}"
    fi
    if [[ "$DRY_RUN" == true ]]; then
        log_warn "[DRY-RUN] Would run: ${cmd[*]}"
        return 0
    fi
    "${cmd[@]}"
}

# ── Validation ────────────────────────────────────────────────────────────────

check_prerequisites() {
    log_section "Checking prerequisites"

    # Check required commands
    local required_cmds=("cargo" "git" "grep" "sed")
    for cmd in "${required_cmds[@]}"; do
        if ! command -v "$cmd" &>/dev/null; then
            log_error "Required command not found: $cmd"
            exit 1
        fi
    done
    log_success "All required commands available"

    # Check git config
    local git_name git_email
    git_name=$(git config user.name || true)
    git_email=$(git config user.email || true)

    if [[ -z "$git_name" ]] || [[ -z "$git_email" ]]; then
        log_error "Git user.name or user.email not configured"
        log_info "Run: git config user.name 'Your Name' && git config user.email 'your@email.com'"
        exit 1
    fi
    log_success "Git configured: $git_name <$git_email>"

    # Check cargo login
    if [[ ! -f "$HOME/.cargo/credentials.toml" ]]; then
        log_warn "Cargo credentials not found. You may need to run: cargo login"
    fi
}

get_current_version() {
    grep '^version' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

get_last_tag_version() {
    git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//' || echo "0.0.0"
}

check_version_bumped() {
    log_section "Checking version bump"

    local current_version last_version
    current_version=$(get_current_version)
    last_version=$(get_last_tag_version)

    log_info "Current version: $current_version"
    log_info "Last tag version: $last_version"

    if [[ "$current_version" == "$last_version" ]]; then
        log_error "Version not bumped! Update Cargo.toml before deploying"
        exit 2
    fi

    log_success "Version bumped: $last_version → $current_version"
    echo "$current_version"
}

check_git_clean() {
    log_section "Checking git status"

    if ! git diff-index --quiet HEAD --; then
        log_error "Uncommitted changes detected. Commit all changes before deploying"
        git status --short
        exit 3
    fi

    log_success "Git status clean"
}

# ── Build & Test ──────────────────────────────────────────────────────────────

build_project() {
    log_section "Building project"

    run_cmd cargo clean
    run_cmd cargo build --release

    log_success "Build completed"
}

run_tests() {
    log_section "Running tests"

    run_cmd cargo test

    log_success "All tests passed"
}

run_clippy() {
    log_section "Running clippy linter"

    run_cmd cargo clippy --all-targets --all-features -- -D warnings

    log_success "Clippy checks passed"
}

run_fmt_check() {
    log_section "Checking code formatting"

    run_cmd cargo fmt -- --check

    log_success "Code formatting OK"
}

# ── Git operations ────────────────────────────────────────────────────────────

commit_and_push() {
    local version=$1

    log_section "Committing and pushing"

    if [[ "$DRY_RUN" == true ]]; then
        log_warn "[DRY-RUN] Would commit and push version $version"
        return 0
    fi

    # Verify git is clean before committing
    if ! git diff-index --quiet HEAD --; then
        log_error "Unexpected uncommitted changes before commit"
        exit 3
    fi

    log_info "Creating git tag: v$version"
    git tag "v$version"

    log_info "Pushing to origin master"
    git push origin master
    git push origin "v$version"

    log_success "Committed and pushed"
}

# ── Crates.io publish ─────────────────────────────────────────────────────────

publish_to_crates() {
    local version=$1

    if [[ "$SKIP_PUBLISH" == true ]]; then
        log_warn "Skipping crates.io publish (--skip-publish)"
        return 0
    fi

    log_section "Publishing to crates.io"

    # Dry-run first
    log_info "Running publish dry-run"
    if ! run_cmd cargo publish --dry-run; then
        log_error "Publish dry-run failed. Check your Cargo.toml and credentials"
        exit 6
    fi

    if [[ "$DRY_RUN" == true ]]; then
        log_warn "[DRY-RUN] Would publish version $version to crates.io"
        return 0
    fi

    log_info "Publishing version $version to crates.io"
    if ! run_cmd cargo publish; then
        log_error "Publish failed"
        exit 6
    fi

    log_success "Published to crates.io: https://crates.io/crates/odin/$version"
}

# ── Verification ──────────────────────────────────────────────────────────────

verify_deployment() {
    local version=$1

    log_section "Verifying deployment"

    if [[ "$DRY_RUN" == true ]]; then
        log_warn "[DRY-RUN] Skipping verification"
        return 0
    fi

    # Check git tag exists
    if git rev-parse "v$version" >/dev/null 2>&1; then
        log_success "Git tag v$version created"
    else
        log_error "Git tag v$version not found"
        exit 1
    fi

    # Check GitHub release (if using GitHub Actions)
    log_info "GitHub Actions will create a release automatically"
    log_info "Check: https://github.com/RajaRakoto/odin-vsm/releases/tag/v$version"

    if [[ "$SKIP_PUBLISH" != true ]]; then
        log_info "Crates.io page: https://crates.io/crates/odin/$version"
    fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    parse_args "$@"

    log_info "Starting deployment workflow"
    log_info "Project root: $PROJECT_ROOT"

    check_prerequisites
    check_git_clean
    local version
    version=$(check_version_bumped)

    build_project
    run_tests
    run_clippy
    run_fmt_check

    commit_and_push "$version"
    publish_to_crates "$version"

    verify_deployment "$version"

    log_section "Summary"
    log_success "Version $version deployed successfully"
    if [[ "$DRY_RUN" == true ]]; then
        log_warn "This was a dry-run. No changes were made."
    fi
}

main "$@"
