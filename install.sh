#!/usr/bin/env bash
# Red Green Light — one-shot installer for developers building from source.
# Builds the Tauri app, drops the .app into /Applications, and wires up
# Claude Code hooks. Idempotent.
#
# Usage:
#   ./install.sh                # build + install everything
#   ./install.sh --skip-hooks   # build + install .app, leave settings.json alone
#   ./install.sh --skip-build   # reuse an existing build artifact
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

SKIP_HOOKS=0
SKIP_BUILD=0
for arg in "$@"; do
  case "$arg" in
    --skip-hooks) SKIP_HOOKS=1 ;;
    --skip-build) SKIP_BUILD=1 ;;
    -h|--help)
      # Print the leading comment block (the file header), stopping at the
      # first non-comment line. Robust to comments being added/removed later.
      awk '/^#!/ {next} /^#/ {print; next} {exit}' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

say() { printf '\033[1;36m▸\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m✗\033[0m %s\n' "$*" >&2; exit 1; }

# Pull in rustup's PATH additions if cargo isn't on PATH yet (covers
# non-interactive shells that don't source .zshrc/.bash_profile).
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
  # shellcheck source=/dev/null
  . "$HOME/.cargo/env"
fi

# ----- prereqs --------------------------------------------------------------
need() { command -v "$1" >/dev/null 2>&1 || die "missing: $1 — $2"; }

say "checking prerequisites"
need node  "install Node 18+ (e.g. brew install node)"
need pnpm  "install pnpm (e.g. npm i -g pnpm)"
need cargo "install Rust (https://rustup.rs)"
need jq    "install jq (e.g. brew install jq)"

# ----- build ----------------------------------------------------------------
APP_NAME="Red Green Light.app"
BUNDLE_DIR="src-tauri/target/release/bundle/macos"
BUNDLE_PATH="$BUNDLE_DIR/$APP_NAME"

if [ "$SKIP_BUILD" -eq 0 ]; then
  say "installing JS deps"
  pnpm install --frozen-lockfile

  say "building release bundle (this can take a few minutes)"
  pnpm tauri build
else
  say "skipping build (--skip-build)"
fi

[ -d "$BUNDLE_PATH" ] || die "expected bundle not found at $BUNDLE_PATH"

# ----- install .app ---------------------------------------------------------
DEST="/Applications/$APP_NAME"
say "installing $APP_NAME to /Applications"
if [ -d "$DEST" ]; then
  # Quit a running instance before overwriting — otherwise rm -rf yanks the
  # binary out from under a live process, which on macOS leaves a half-broken
  # tray icon and an orphaned listener on port 7878.
  if pgrep -x "red-green-light" >/dev/null 2>&1 \
     || pgrep -f "/Applications/$APP_NAME" >/dev/null 2>&1; then
    say "quitting running instance"
    osascript -e 'tell application "Red Green Light" to quit' 2>/dev/null || true
    # Give it a moment to release the port; force-kill if it ignored us.
    for _ in 1 2 3 4 5; do
      pgrep -x "red-green-light" >/dev/null 2>&1 || break
      sleep 0.4
    done
    pkill -x "red-green-light" 2>/dev/null || true
  fi
  rm -rf "$DEST"
fi
cp -R "$BUNDLE_PATH" "/Applications/"

# Strip the macOS quarantine attribute so Gatekeeper doesn't pop "unknown
# developer" on first launch (this is a local build, not signed).
xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true

# ----- hooks ----------------------------------------------------------------
if [ "$SKIP_HOOKS" -eq 0 ]; then
  say "installing Claude Code hooks into ~/.claude/settings.json"
  bash "$SCRIPT_DIR/hooks/install.sh"
else
  say "skipping hooks (--skip-hooks)"
fi

# ----- launch hint ----------------------------------------------------------
say "done"
echo
echo "  Launch:   open -a 'Red Green Light'"
echo "  Uninstall app:    rm -rf '$DEST'"
echo "  Uninstall hooks:  $SCRIPT_DIR/hooks/uninstall.sh"
