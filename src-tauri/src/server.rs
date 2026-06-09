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
    let stream = WatchStream::new(rx).map(|snap| {
        Ok(Event::default()
            .data(serde_json::to_string(&snap).unwrap_or_else(|_| "{}".into())))
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
