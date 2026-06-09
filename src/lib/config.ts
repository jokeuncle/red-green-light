// Single source of truth for the local HTTP port the floating window talks
// to. The Rust side accepts `RGL_PORT` to override; if you change the port,
// also update `hooks/claude-hooks.json` (or template + regenerate it) so the
// Claude Code hooks POST to the same place.
export const DEFAULT_PORT = 7878;
