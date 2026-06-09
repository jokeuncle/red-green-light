#!/usr/bin/env bash
# Remove all Red Green Light hook entries from ~/.claude/settings.json.
# A backup is written next to the file before any change.
set -euo pipefail

SETTINGS="${CLAUDE_SETTINGS:-$HOME/.claude/settings.json}"
RGL_URL_PREFIX="http://127.0.0.1:7878/events"

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required. Install with: brew install jq" >&2
  exit 1
fi

if [ ! -f "$SETTINGS" ]; then
  echo "nothing to do: $SETTINGS does not exist."
  exit 0
fi

ts=$(date +%Y%m%d-%H%M%S)
backup="${SETTINGS}.bak-${ts}"
cp "$SETTINGS" "$backup"
echo "backup: $backup"

tmp=$(mktemp)
jq --arg url "$RGL_URL_PREFIX" '
  if .hooks == null then .
  else
    .hooks |= with_entries(
      .value |= map(
        .hooks |= map(select((.url // "") != $url))
        | select((.hooks // []) | length > 0)
      )
      | select((.value | length) > 0)
    )
    | if (.hooks | length) == 0 then del(.hooks) else . end
  end
' "$SETTINGS" > "$tmp"

mv "$tmp" "$SETTINGS"
echo "removed Red Green Light hooks from $SETTINGS"
