use std::{ffi::OsStr, os::windows::process::CommandExt, process::Command};

use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::{
    elevation_manager,
    error::{AppError, AppResult},
    models::{
        ConnectionState, ConnectionStatusPayload, LogLevel, LogSource, Profile,
        TestConnectionResult,
    },
    state::AppState,
    tun_route_manager,
};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const AWG_START_TIMEOUT_MS: u64 = 12_000;

pub async fn connect_profile(
    app: &AppHandle,
    state: &AppState,
    profile: &Profile,
) -> AppResult<ConnectionStatusPayload> {
    if !crate::network_interface_manager::is_elevated() {
        elevation_manager::relaunch_as_administrator_for_profile(&profile.id)?;
        app.exit(0);
        return Err(AppError::state(
            "Relaunching VeilBox as Administrator for AmneziaWG.",
        ));
    }

    let _guard = state.connection.op_lock.lock().await;
    let current_status = state.connection.status.read().await.clone();
    if matches!(
        current_status.state,
        ConnectionState::Connected | ConnectionState::Connecting
    ) {
        return Err(AppError::state("A connection operation is already in progress"));
    }

    let config = profile
        .amnezia_config
        .clone()
        .ok_or_else(|| AppError::validation("Amnezia config is missing from the selected profile"))?;

    if !state.paths.amnezia_sidecar_path.exists() {
        return Err(AppError::process(
            "amneziawg.exe was not found. Place it into src-tauri/bin/amneziawg.exe before running.",
            Some(state.paths.amnezia_sidecar_path.display().to_string()),
        ));
    }

    {
        let mut desired = state.connection.desired_profile_id.write().await;
        *desired = Some(profile.id.clone());
    }

    set_status(
        app,
        state,
        ConnectionStatusPayload {
            state: ConnectionState::Connecting,
            active_profile_id: Some(profile.id.clone()),
            message: Some(format!("Starting AmneziaWG for {}...", profile.name)),
            connected_at: None,
            local_http_proxy_port: None,
            local_socks_proxy_port: None,
            restart_count: 0,
        },
    )
    .await?;

    let _ = cleanup_tunnel_service(state).await;
    tokio::fs::write(&state.paths.amnezia_config_file, &config.raw_config).await?;

    spawn_awg_command(
        state,
        ["/installtunnelservice", state.paths.amnezia_config_file.to_string_lossy().as_ref()],
    )?;

    let tunnel_name = tunnel_name(state)?;
    if let Err(error) =
        tun_route_manager::wait_for_interface(&tunnel_name, AWG_START_TIMEOUT_MS)
    {
        let _ = cleanup_tunnel_service(state).await;
        return Err(error);
    }

    let connected_at = Utc::now();
    state
        .runtime_state
        .mark_connected(profile.id.clone(), 0, 0, None, None)
        .await?;

    let connected = ConnectionStatusPayload {
        state: ConnectionState::Connected,
        active_profile_id: Some(profile.id.clone()),
        message: Some(format!("Connected via {} (AmneziaWG)", profile.name)),
        connected_at: Some(connected_at),
        local_http_proxy_port: None,
        local_socks_proxy_port: None,
        restart_count: 0,
    };
    {
        let mut session = state.connection.session.lock().await;
        *session = None;
    }
    set_status(app, state, connected.clone()).await?;
    let _ = state
        .log_if_enabled(
            LogSource::Connection,
            LogLevel::Info,
        format!(
            "Connected profile '{}' through AmneziaWG tunnel service '{}'.",
            profile.name, tunnel_name
        ),
        )
        .await;

    Ok(connected)
}

pub async fn disconnect_profile(
    app: &AppHandle,
    state: &AppState,
) -> AppResult<ConnectionStatusPayload> {
    let _guard = state.connection.op_lock.lock().await;

    {
        let mut desired = state.connection.desired_profile_id.write().await;
        *desired = None;
    }

    cleanup_tunnel_service(state).await?;
    let _ = tokio::fs::remove_file(&state.paths.amnezia_config_file).await;
    state.runtime_state.clear().await?;

    let disconnected = ConnectionStatusPayload {
        state: ConnectionState::Disconnected,
        active_profile_id: None,
        message: Some("Disconnected".to_string()),
        connected_at: None,
        local_http_proxy_port: None,
        local_socks_proxy_port: None,
        restart_count: 0,
    };
    set_status(app, state, disconnected.clone()).await?;
    Ok(disconnected)
}

pub async fn test_profile_connection(
    _app: &AppHandle,
    _state: &AppState,
    profile: &Profile,
) -> AppResult<TestConnectionResult> {
    let config = profile
        .amnezia_config
        .as_ref()
        .ok_or_else(|| AppError::validation("Amnezia config is missing from the selected profile"))?;

    Ok(TestConnectionResult {
        profile_id: profile.id.clone(),
        success: true,
        message: format!(
            "AmneziaWG config for '{}' looks structurally valid: endpoint {}:{}.",
            profile.name, config.endpoint_host, config.endpoint_port
        ),
        duration_ms: None,
    })
}

pub async fn cleanup_on_launch(state: &AppState) -> AppResult<()> {
    let _ = cleanup_tunnel_service(state).await;
    let _ = tokio::fs::remove_file(&state.paths.amnezia_config_file).await;
    Ok(())
}

async fn cleanup_tunnel_service(state: &AppState) -> AppResult<()> {
    if !state.paths.amnezia_sidecar_path.exists() {
        return Ok(());
    }

    let tunnel_name = tunnel_name(state)?;
    let output = run_awg_command(state, ["/uninstalltunnelservice", tunnel_name.as_str()]).await?;

    if output.status.success() {
        return Ok(());
    }

    let message = stderr_and_stdout(&output);
    if message.contains("service does not exist")
        || message.contains("not installed")
        || message.contains("was not found")
    {
        return Ok(());
    }

    Err(AppError::process(
        "Failed to stop AmneziaWG tunnel service.",
        Some(message),
    ))
}

async fn run_awg_command<const N: usize>(
    state: &AppState,
    args: [&str; N],
) -> AppResult<std::process::Output> {
    let binary = state.paths.amnezia_sidecar_path.clone();
    let current_dir = binary.parent().map(|path| path.to_path_buf());
    let args = args.map(|value| value.to_string());

    tokio::task::spawn_blocking(move || {
        let mut command = Command::new(binary);
        if let Some(current_dir) = current_dir {
            command.current_dir(current_dir);
        }
        command.creation_flags(CREATE_NO_WINDOW).args(args);
        command.output().map_err(|error| {
            AppError::process(
                "Failed to execute amneziawg.exe.",
                Some(error.to_string()),
            )
        })
    })
    .await
    .map_err(|error| AppError::process("Failed to join AmneziaWG command task.", Some(error.to_string())))?
}

fn spawn_awg_command<const N: usize>(state: &AppState, args: [&str; N]) -> AppResult<()> {
    let binary = state.paths.amnezia_sidecar_path.clone();
    let current_dir = binary.parent().map(|path| path.to_path_buf());
    let args = args.map(|value| value.to_string());

    let mut command = Command::new(binary);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    command
        .creation_flags(CREATE_NO_WINDOW)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            AppError::process(
                "Failed to launch amneziawg.exe.",
                Some(error.to_string()),
            )
        })
}

fn stderr_and_stdout(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => format!("{} | {}", stderr, stdout),
        (false, true) => stderr,
        (true, false) => stdout,
        (true, true) => format!("exit status: {}", output.status),
    }
}

async fn set_status(app: &AppHandle, state: &AppState, status: ConnectionStatusPayload) -> AppResult<()> {
    {
        let mut guard = state.connection.status.write().await;
        *guard = status.clone();
    }
    app.emit("connection-status-changed", status)
        .map_err(|error| AppError::internal("Unable to emit connection status event", Some(error.to_string())))
}

fn tunnel_name(state: &AppState) -> AppResult<String> {
    state
        .paths
        .amnezia_config_file
        .file_stem()
        .and_then(OsStr::to_str)
        .map(|value| value.to_string())
        .ok_or_else(|| AppError::internal("Unable to derive Amnezia tunnel name.", None))
}
