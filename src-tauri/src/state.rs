use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{watch, Mutex};

/// One of the three semantic states a session can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Idle,
    Working,
    Waiting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LightColor {
    Green,
    Yellow,
    Red,
}

#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub id: String,
    pub source: String,
    pub state: SessionState,
    pub cwd: Option<String>,
    pub last_event: Option<String>,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub global: LightColor,
    pub sessions: Vec<Session>,
}

pub struct AppState {
    inner: Arc<Mutex<Inner>>,
    color_tx: watch::Sender<LightColor>,
    snapshot_tx: watch::Sender<Snapshot>,
}

struct Inner {
    sessions: HashMap<String, Session>,
}

impl AppState {
    pub fn new() -> Self {
        let initial = Snapshot {
            global: LightColor::Green,
            sessions: vec![],
        };
        let (color_tx, _) = watch::channel(LightColor::Green);
        let (snapshot_tx, _) = watch::channel(initial);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                sessions: HashMap::new(),
            })),
            color_tx,
            snapshot_tx,
        }
    }

    pub fn color_watch(&self) -> watch::Receiver<LightColor> {
        self.color_tx.subscribe()
    }

    pub fn snapshot_watch(&self) -> watch::Receiver<Snapshot> {
        self.snapshot_tx.subscribe()
    }

    /// Apply a state mutation under the lock and recompute the global color
    /// + broadcast a fresh snapshot.
    pub async fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut HashMap<String, Session>),
    {
        let mut g = self.inner.lock().await;
        f(&mut g.sessions);
        let snap = build_snapshot(&g.sessions);
        let _ = self.color_tx.send(snap.global);
        let _ = self.snapshot_tx.send(snap);
    }

    pub async fn snapshot(&self) -> Snapshot {
        let g = self.inner.lock().await;
        build_snapshot(&g.sessions)
    }

    /// Drop sessions that haven't reported in `idle_after_secs` and downgrade
    /// stuck-working sessions to idle (defensive: a hook might never fire
    /// Stop, e.g. if the CC process is killed). Called periodically.
    pub async fn sweep(&self, idle_after_secs: u64) {
        let now = now_secs();
        let mut g = self.inner.lock().await;
        let before = g.sessions.len();
        g.sessions.retain(|_, s| now.saturating_sub(s.updated_at) < 3600);
        for s in g.sessions.values_mut() {
            if s.state != SessionState::Idle
                && now.saturating_sub(s.updated_at) >= idle_after_secs
            {
                s.state = SessionState::Idle;
                s.last_event = Some("(timeout)".into());
            }
        }
        let after = g.sessions.len();
        if before != after || true {
            let snap = build_snapshot(&g.sessions);
            let _ = self.color_tx.send(snap.global);
            let _ = self.snapshot_tx.send(snap);
        }
    }
}

fn build_snapshot(sessions: &HashMap<String, Session>) -> Snapshot {
    let mut any_waiting = false;
    let mut any_working = false;
    for s in sessions.values() {
        match s.state {
            SessionState::Waiting => any_waiting = true,
            SessionState::Working => any_working = true,
            SessionState::Idle => {}
        }
    }
    let global = if any_waiting {
        LightColor::Red
    } else if any_working {
        LightColor::Yellow
    } else {
        LightColor::Green
    };
    let mut list: Vec<Session> = sessions.values().cloned().collect();
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Snapshot {
        global,
        sessions: list,
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
