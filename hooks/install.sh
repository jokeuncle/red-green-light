#!/usr/bin/env bash
# Idempotently merge Red Green Light hooks into ~/.claude/settings.json.
# A backup is written next to the file before any change.
set -euo pipefail

SETTINGS="${CLAUDE_SETTINGS:-$HOME/.claude/settings.json}"
SNIPPET_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SNIPPET="$SNIPPET_DIR/claude-hooks.json"

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required. Install with: brew install jq" >&2
  exit 1
fi

if [ ! -f "$SNIPPET" ]; then
  echo "error: hook snippet not found at $SNIPPET" >&2
  exit 1
fi

mkdir -p "$(dirname "$SETTINGS")"
if [ ! -f "$SETTINGS" ]; then
  echo "{}" > "$SETTINGS"
  echo "created empty $SETTINGS"
fi

ts=$(date +%Y%m%d-%H%M%S)
backup="${SETTINGS}.bak-${ts}"
cp "$SETTINGS" "$backup"
echo "backup: $backup"

tmp=$(mktemp)
# Merge snippet hooks into settings. unique_by(tojson) makes the merge
# idempotent: re-running install.sh never duplicates entries.
jq --slurpfile add "$SNIPPET" '
  ($add[0].hooks) as $snip
  | .hooks //= {}
  | reduce ($snip | keys[]) as $ev (
      .;
      .hooks[$ev] = ((.hooks[$ev] // []) + $snip[$ev] | unique_by(tojson))
    )
' "$SETTINGS" > "$tmp"

mv "$tmp" "$SETTINGS"
echo "merged Red Green Light hooks into $SETTINGS"
echo
echo "next steps:"
# Prefer launching the installed .app if it exists (the root install.sh path);
# fall back to `pnpm tauri dev` for contributors running this on its own.
if [ -d "/Applications/Red Green Light.app" ]; then
  echo "  1. Launch the app: open -a 'Red Green Light'"
else
  echo "  1. Start the app: cd $(cd "$SNIPPET_DIR/.." && pwd) && pnpm tauri dev"
fi
echo "  2. Open a Claude Code session — the menu-bar light turns yellow when it works,"
echo "     red when it waits for input, and green when idle."
