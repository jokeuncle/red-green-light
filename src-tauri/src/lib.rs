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

                // Spawn the HTTP server.
                let server_state = app_state.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = server::serve(server_state, port).await {
                        tracing::error!("server error: {e}");
                    }
                });

                // Periodic sweeper: timeout stuck sessions.
                let sweep_state = app_state.clone();
                tauri::async_runtime::spawn(async move {
                    let mut tick =
                        tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        tick.tick().await;
                        sweep_state.sweep(600).await; // 10 min
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
