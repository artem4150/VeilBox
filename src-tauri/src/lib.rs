mod commands;
mod config_builder;
mod error;
mod log_manager;
mod latency_manager;
mod models;
mod network_interface_manager;
mod profile_geo;
mod profile_store;
mod proxy_manager_windows;
mod settings_store;
mod state;
mod subscription_store;
mod subscription_import;
mod tray_manager;
mod vless_parser;
mod xray_manager;

use std::sync::Arc;

use tauri::{AppHandle, LogicalSize, Manager, Size, WindowEvent};

use crate::{
    error::AppResult,
    log_manager::LogManager,
    profile_store::ProfileStore,
    settings_store::SettingsStore,
    state::{AppPaths, AppState, ConnectionRuntime, RuntimeStateStore},
    subscription_store::SubscriptionStore,
};

fn build_state(app: &AppHandle) -> AppResult<AppState> {
    let paths = Arc::new(AppPaths::resolve(app)?);
    let profile_store = tauri::async_runtime::block_on(ProfileStore::load(paths.profiles_file.clone()))?;
    let subscription_store =
        tauri::async_runtime::block_on(SubscriptionStore::load(paths.subscriptions_file.clone()))?;
    let settings_store = tauri::async_runtime::block_on(SettingsStore::load(paths.settings_file.clone()))?;
    let runtime_state = tauri::async_runtime::block_on(RuntimeStateStore::load(paths.runtime_file.clone()))?;
    let log_manager = LogManager::new(paths.app_log_file.clone(), paths.connection_log_file.clone())?;

    Ok(AppState {
        paths,
        profile_store: Arc::new(profile_store),
        subscription_store: Arc::new(subscription_store),
        settings_store: Arc::new(settings_store),
        runtime_state: Arc::new(runtime_state),
        log_manager,
        connection: Arc::new(ConnectionRuntime::default()),
    })
}

fn apply_adaptive_main_window_size(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return;
    };

    let scale_factor = monitor.scale_factor();
    let physical_size = monitor.size();
    let logical_width = physical_size.width as f64 / scale_factor;
    let logical_height = physical_size.height as f64 / scale_factor;

    let max_fit_width = (logical_width - 24.0).max(520.0);
    let max_fit_height = (logical_height - 48.0).max(480.0);

    let min_width = 760.0_f64.min(max_fit_width);
    let min_height = 620.0_f64.min(max_fit_height);

    let max_start_width = 960.0_f64.min(max_fit_width);
    let max_start_height = 760.0_f64.min(max_fit_height);

    let start_width = (logical_width * 0.45).round().clamp(min_width, max_start_width);
    let start_height = (logical_height * 0.78)
        .round()
        .clamp(min_height, max_start_height);

    let _ = window.set_min_size(Some(Size::Logical(LogicalSize::new(min_width, min_height))));
    let _ = window.set_size(Size::Logical(LogicalSize::new(start_width, start_height)));
    let _ = window.center();
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let state = build_state(app.handle())?;
            app.manage(state);
            tray_manager::build_tray(app.handle())?;
            apply_adaptive_main_window_size(app.handle());
            tauri::async_runtime::block_on(async {
                let app_state = app.state::<AppState>();
                let _ = app_state
                    .log_if_enabled(
                        crate::models::LogSource::App,
                        crate::models::LogLevel::Info,
                        "Application initialized.",
                    )
                    .await;
            });
            tauri::async_runtime::block_on(async {
                xray_manager::cleanup_on_launch(app.handle()).await
            })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let settings = tauri::async_runtime::block_on(state.settings_store.get());
                if settings.minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::import_vless_uri,
            commands::import_profiles_json,
            commands::import_subscription,
            commands::list_subscriptions,
            commands::refresh_subscription,
            commands::refresh_all_subscriptions,
            commands::delete_subscription,
            commands::get_settings,
            commands::update_settings,
            commands::connect,
            commands::disconnect,
            commands::connection_status,
            commands::test_profile_connection,
            commands::get_logs,
            commands::clear_logs,
            commands::get_profile_latencies,
            commands::get_profile_countries,
            commands::list_network_interfaces,
            commands::get_about_info
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| panic!("error while building tauri application: {error}"))
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                let _ = proxy_manager_windows::clear_proxy();
                let session = tauri::async_runtime::block_on(async {
                    state.connection.session.lock().await.take()
                });
                if let Some(s) = session {
                    s.stop_requested.store(true, std::sync::atomic::Ordering::SeqCst);
                    let _ = s.child.blocking_lock().kill();
                }
                let _ = tauri::async_runtime::block_on(state.runtime_state.clear());
            }
        });
}
