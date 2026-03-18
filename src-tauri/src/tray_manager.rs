use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use crate::{models::LogLevel, state::AppState, xray_manager};

const TRAY_CONNECT_ID: &str = "tray_connect";
const TRAY_DISCONNECT_ID: &str = "tray_disconnect";
const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_QUIT_ID: &str = "tray_quit";

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let connect = MenuItem::with_id(app, TRAY_CONNECT_ID, "Connect", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(app, TRAY_DISCONNECT_ID, "Disconnect", true, None::<&str>)?;
    let show = MenuItem::with_id(app, TRAY_SHOW_ID, "Show window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&connect, &disconnect, &show, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("VailBox")
        .menu(&menu);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_CONNECT_ID => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let selected = state.settings_store.get().await.last_selected_profile_id;
                    if let Some(profile_id) = selected {
                        let _ = xray_manager::connect_profile(&app_handle, state.inner(), profile_id).await;
                    } else {
                        let _ = app_handle.emit(
                            "backend-error",
                            serde_json::json!({
                                "code": "NO_PROFILE",
                                "message": "No active profile is selected for tray connect.",
                                "details": null
                            }),
                        );
                    }
                });
            }
            TRAY_DISCONNECT_ID => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let _ = xray_manager::disconnect_profile(&app_handle, state.inner()).await;
                });
            }
            TRAY_SHOW_ID => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            TRAY_QUIT_ID => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let _ = state
                        .log_if_enabled(
                            crate::models::LogSource::App,
                            LogLevel::Info,
                            "Quit requested from tray.",
                        )
                        .await;
                    let _ = xray_manager::disconnect_profile(&app_handle, state.inner()).await;
                    app_handle.exit(0);
                });
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
