mod events;
mod server;
mod state;
mod tray;

use crate::state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app_state = Arc::new(AppState::new());
    let port: u16 = std::env::var("RGL_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(server::DEFAULT_PORT);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup({
            let app_state = app_state.clone();
            move |app| {
                // Build tray icon (subscribes internally to color updates).
                let _tray = tray::build(app.handle(), app_state.clone())?;

                // Spawn the HTTP server. If it dies (most likely a bind
                // failure: port already in use), exit the whole app — without
                // the server the tray is just a frozen icon and the user has
                // no way to know it's broken.
                let server_state = app_state.clone();
                let server_app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = server::serve(server_state, port).await {
                        tracing::error!(
                            "HTTP server stopped (port {port}): {e}. Exiting."
                        );
                        server_app_handle.exit(1);
                    }
                });

                // Periodic sweeper: timeout stuck-working sessions, evict
                // very old entries. See AppState::sweep for semantics.
                let sweep_state = app_state.clone();
                tauri::async_runtime::spawn(async move {
                    let mut tick =
                        tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        tick.tick().await;
                        // working-idle after 10 min silence; evict after 1h.
                        sweep_state.sweep(600, 3600).await;
                    }
                });

                // On macOS, hide the dock icon — this is a menu-bar utility.
                #[cfg(target_os = "macos")]
                {
                    let _ = app.set_activation_policy(
                        tauri::ActivationPolicy::Accessory,
                    );
                }

                Ok(())
            }
        })
        .on_window_event(|window, event| {
            // Hide instead of close so the app keeps running in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running red-green-light");
}
