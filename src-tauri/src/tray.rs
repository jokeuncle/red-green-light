use crate::state::{AppState, LightColor};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Runtime,
};

const ICON_GREEN: &[u8] = include_bytes!("../icons/tray-green.png");
const ICON_YELLOW: &[u8] = include_bytes!("../icons/tray-yellow.png");
const ICON_RED: &[u8] = include_bytes!("../icons/tray-red.png");

fn icon_for(c: LightColor) -> Image<'static> {
    let bytes = match c {
        LightColor::Green => ICON_GREEN,
        LightColor::Yellow => ICON_YELLOW,
        LightColor::Red => ICON_RED,
    };
    Image::from_bytes(bytes).expect("embedded tray icon must decode")
}

pub fn build<R: Runtime>(
    app: &AppHandle<R>,
    state: Arc<AppState>,
) -> tauri::Result<TrayIcon<R>> {
    let show_i = MenuItem::with_id(app, "show", "显示悬浮窗", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "隐藏悬浮窗", true, None::<&str>)?;
    let about_i = MenuItem::with_id(app, "about", "Red Green Light", false, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&about_i, &sep, &show_i, &hide_i, &sep, &quit_i],
    )?;

    let tray = TrayIconBuilder::with_id("rgl-main")
        .icon(icon_for(LightColor::Green))
        .icon_as_template(false)
        .tooltip("Red Green Light · idle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "hide" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // single left click toggles the floating window
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let visible = w.is_visible().unwrap_or(false);
                    if visible {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    // Subscribe to color changes and update the tray icon accordingly.
    let app_handle = app.clone();
    let mut rx = state.color_watch();
    tauri::async_runtime::spawn(async move {
        loop {
            let color = *rx.borrow_and_update();
            if let Some(tray) = app_handle.tray_by_id("rgl-main") {
                let _ = tray.set_icon(Some(icon_for(color)));
                let label = match color {
                    LightColor::Green => "Red Green Light · idle",
                    LightColor::Yellow => "Red Green Light · working",
                    LightColor::Red => "Red Green Light · waiting for input",
                };
                let _ = tray.set_tooltip(Some(label));
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    });

    Ok(tray)
}
