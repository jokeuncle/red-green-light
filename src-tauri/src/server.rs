use crate::events::{apply, IncomingEvent};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json,
    },
    routing::{get, post},
    Router,
};
use futures_util::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::WatchStream;

pub const DEFAULT_PORT: u16 = 7878;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", post(post_events))
        .route("/state", get(get_state))
        .route("/state/stream", get(get_state_stream))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn post_events(
    State(state): State<Arc<AppState>>,
    Json(ev): Json<IncomingEvent>,
) -> impl IntoResponse {
    apply(&state, ev).await;
    Json(serde_json::json!({"ok": true}))
}

async fn get_state(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.snapshot().await)
}

async fn get_state_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.snapshot_watch();
    let stream = WatchStream::new(rx).filter_map(|snap| async move {
        match serde_json::to_string(&snap) {
            Ok(json) => Some(Ok(Event::default().data(json))),
            Err(e) => {
                // The Snapshot type derives Serialize over owned strings and
                // enums — encoding shouldn't fail. If it does, log loudly
                // and skip the event rather than emit "{}", which the
                // client would deserialize into a snapshot with a missing
                // `global` field and crash the React state update.
                tracing::error!("failed to serialize snapshot: {e}");
                None
            }
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

pub async fn serve(state: Arc<AppState>, port: u16) -> std::io::Result<()> {
    let app = router(state).layer(tower_http::cors::CorsLayer::permissive());
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await
}
