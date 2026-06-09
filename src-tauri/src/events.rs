use crate::state::{now_secs, AppState, Session, SessionState};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct IncomingEvent {
    #[serde(default = "default_source")]
    pub source: String,
    pub session_id: Option<String>,
    pub hook_event_name: Option<String>,
    pub cwd: Option<String>,
    pub tool_name: Option<String>,
}

fn default_source() -> String {
    // Claude Code hook payloads do not carry a `source` field, so anything
    // arriving without one is assumed to be Claude Code. Codex senders are
    // expected to set `source: "codex"` explicitly.
    "claude-code".into()
}

/// Map a Claude Code hook event name to a SessionState transition.
/// Returns Some(state) to set, or None to remove the session.
fn transition(event: &str) -> Option<SessionState> {
    match event {
        "SessionStart" | "UserPromptSubmit" | "PreToolUse" | "PostToolUse"
        | "SubagentStop" | "PreCompact" => Some(SessionState::Working),
        "Notification" => Some(SessionState::Waiting),
        "Stop" | "StopFailure" => Some(SessionState::Idle),
        // SessionEnd is handled separately (remove session).
        _ => None,
    }
}

pub async fn apply(state: &AppState, ev: IncomingEvent) {
    let Some(session_id) = ev.session_id else {
        return;
    };
    let event_name = ev.hook_event_name.clone().unwrap_or_default();

    if event_name == "SessionEnd" {
        state
            .update(|map| {
                map.remove(&session_id);
            })
            .await;
        return;
    }

    let Some(next) = transition(&event_name) else {
        return;
    };

    let now = now_secs();
    state
        .update(|map| {
            let entry = map.entry(session_id.clone()).or_insert(Session {
                id: session_id.clone(),
                source: ev.source.clone(),
                state: next,
                cwd: ev.cwd.clone(),
                last_event: Some(event_name.clone()),
                updated_at: now,
            });
            entry.state = next;
            entry.source = ev.source.clone();
            if ev.cwd.is_some() {
                entry.cwd = ev.cwd.clone();
            }
            entry.last_event = Some(if let Some(tool) = &ev.tool_name {
                format!("{event_name}({tool})")
            } else {
                event_name.clone()
            });
            entry.updated_at = now;
        })
        .await;
}
