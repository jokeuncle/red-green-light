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

    /// Periodic maintenance:
    ///   1. Drop sessions whose last event was more than `retain_secs` ago.
    ///   2. Downgrade *Working* sessions that have been silent for
    ///      `idle_after_secs` to Idle. This is defensive — `Stop` should
    ///      arrive, but a killed CC process won't deliver it.
    ///
    /// Waiting sessions are NEVER auto-downgraded. A waiting session is the
    /// whole point of the red light: it means the user actually needs to do
    /// something. Silently flipping it to green would hide a real signal.
    pub async fn sweep(&self, idle_after_secs: u64, retain_secs: u64) {
        let now = now_secs();
        let mut g = self.inner.lock().await;
        let before = g.sessions.len();
        g.sessions
            .retain(|_, s| now.saturating_sub(s.updated_at) < retain_secs);
        let mut downgraded = false;
        for s in g.sessions.values_mut() {
            if s.state == SessionState::Working
                && now.saturating_sub(s.updated_at) >= idle_after_secs
            {
                s.state = SessionState::Idle;
                s.last_event = Some("(timeout)".into());
                downgraded = true;
            }
        }
        let after = g.sessions.len();
        if before != after || downgraded {
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
