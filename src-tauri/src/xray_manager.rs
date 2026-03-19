use std::{
    io::{BufRead, BufReader, Read},
    net::{SocketAddr, TcpStream},
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::{
    config_builder::build_xray_config,
    error::{AppError, AppResult},
    models::{ConnectionMode, ConnectionState, ConnectionStatusPayload, LogLevel, LogSource, Profile, TestConnectionResult},
    proxy_manager_windows,
    state::{AppState, ManagedSession},
};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const XRAY_START_TIMEOUT_MS: u64 = 15_000;
const DISCONNECT_TIMEOUT_MS: u64 = 4_000;

pub async fn connection_status(state: &AppState) -> ConnectionStatusPayload {
    state.connection.status.read().await.clone()
}

pub async fn connect_profile(
    app: &AppHandle,
    state: &AppState,
    profile_id: String,
) -> AppResult<ConnectionStatusPayload> {
    let _guard = state.connection.op_lock.lock().await;
    start_session(app, state, profile_id, None).await
}

pub async fn disconnect_profile(
    app: &AppHandle,
    state: &AppState,
) -> AppResult<ConnectionStatusPayload> {
    let _guard = state.connection.op_lock.lock().await;
    disconnect_locked(app, state).await
}

pub async fn test_profile_connection(
    app: &AppHandle,
    state: &AppState,
    profile_id: String,
) -> AppResult<TestConnectionResult> {
    let _guard = state.connection.op_lock.lock().await;
    let current_status = state.connection.status.read().await.clone();
    if matches!(current_status.state, ConnectionState::Connected | ConnectionState::Connecting) {
        return Err(AppError::state("Disconnect the active session before running a profile test"));
    }

    let profile = state
        .profile_store
        .get(&profile_id)
        .await
        .ok_or_else(|| AppError::not_found("Profile was not found"))?;

    let started = std::time::Instant::now();
    run_connection_test(app, state, &profile).await?;

    Ok(TestConnectionResult {
        profile_id,
        success: true,
        message: format!("{} passed local Xray startup validation.", profile.name),
        duration_ms: Some(started.elapsed().as_millis()),
    })
}

pub async fn cleanup_on_launch(app: &AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let runtime_snapshot = state.runtime_state.snapshot().await;
    let cleaned = proxy_manager_windows::best_effort_cleanup(
        runtime_snapshot.last_proxy_string.clone(),
        runtime_snapshot.last_winhttp_dump.clone(),
    )?;
    if cleaned {
        let _ = state
            .log_if_enabled(
                LogSource::App,
                LogLevel::Warn,
                "Detected stale local system proxy on launch and cleared it.",
            )
            .await;
    }

    let _ = tokio::fs::remove_file(&state.paths.temp_config_file).await;
    state.runtime_state.clear().await?;
    set_status(
        app,
        state.inner(),
        ConnectionStatusPayload {
            state: ConnectionState::Disconnected,
            ..Default::default()
        },
    )
    .await?;

    let settings = state.settings_store.get().await;
    if settings.auto_reconnect {
        if let Some(profile_id) = runtime_snapshot.last_profile_id.filter(|_| runtime_snapshot.was_connected) {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                sleep(Duration::from_millis(650)).await;
                let app_state = app_handle.state::<AppState>();
                let _ = connect_profile(&app_handle, app_state.inner(), profile_id).await;
            });
        }
    }

    Ok(())
}

pub async fn xray_version(app: &AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    let sidecar = state.paths.sidecar_path.clone();
    if !sidecar.exists() {
        return None;
    }

    tokio::task::spawn_blocking(move || {
        let output = Command::new(&sidecar)
            .arg("version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        String::from_utf8(output.stdout)
            .ok()
            .and_then(|value| value.lines().next().map(|line| line.trim().to_string()))
    })
    .await
    .ok()
    .flatten()
}

async fn start_session(
    app: &AppHandle,
    state: &AppState,
    profile_id: String,
    restart_count: Option<u32>,
) -> AppResult<ConnectionStatusPayload> {
    let current_status = state.connection.status.read().await.clone();
    if restart_count.is_none()
        && matches!(current_status.state, ConnectionState::Connected | ConnectionState::Connecting)
    {
        return Err(AppError::state("A connection operation is already in progress"));
    }

    let profile = state
        .profile_store
        .get(&profile_id)
        .await
        .ok_or_else(|| AppError::not_found("Profile was not found"))?;

    {
        let mut desired = state.connection.desired_profile_id.write().await;
        *desired = Some(profile.id.clone());
    }

    let connecting_status = ConnectionStatusPayload {
        state: ConnectionState::Connecting,
        active_profile_id: Some(profile.id.clone()),
        message: Some(format!("Starting Xray for {}...", profile.name)),
        connected_at: None,
        local_http_proxy_port: None,
        local_socks_proxy_port: None,
        restart_count: restart_count.unwrap_or(0),
    };
    set_status(app, state, connecting_status).await?;

    let startup = run_xray(app, state, &profile, restart_count.unwrap_or(0)).await;
    match startup {
        Ok(status) => Ok(status),
        Err(error) => {
            let _ = proxy_manager_windows::clear_proxy();
            let _ = state.runtime_state.clear().await;
            let _ = tokio::fs::remove_file(&state.paths.temp_config_file).await;
            {
                let mut desired = state.connection.desired_profile_id.write().await;
                *desired = None;
            }
            let failure_status = ConnectionStatusPayload {
                state: ConnectionState::Error,
                active_profile_id: Some(profile.id.clone()),
                message: Some(error.message.clone()),
                connected_at: None,
                local_http_proxy_port: None,
                local_socks_proxy_port: None,
                restart_count: restart_count.unwrap_or(0),
            };
            let _ = set_status(app, state, failure_status).await;
            let _ = state
                .log_if_enabled(
                    LogSource::Connection,
                    LogLevel::Error,
                    format!("Connection start failed: {}", error.message),
                )
                .await;
            Err(error)
        }
    }
}

async fn run_xray(
    app: &AppHandle,
    state: &AppState,
    profile: &Profile,
    restart_count: u32,
) -> AppResult<ConnectionStatusPayload> {
    let settings = state.settings_store.get().await;
    let tun_mode = matches!(settings.connection_mode, ConnectionMode::Tun);

    if !state.paths.sidecar_path.exists() {
        return Err(AppError::process(
            "xray.exe was not found. Place it into src-tauri/bin/xray.exe before running.",
            Some(state.paths.sidecar_path.display().to_string()),
        ));
    }
    if tun_mode {
        ensure_tun_prerequisites(state)?;
    }

    let (socks_port, http_port) = pick_two_distinct_ports()?;

    let config = build_xray_config(profile, &settings, socks_port, http_port)?;
    tokio::fs::write(&state.paths.temp_config_file, config).await?;

    let mut command = Command::new(&state.paths.sidecar_path);
    if let Some(sidecar_dir) = state.paths.sidecar_path.parent() {
        command.current_dir(sidecar_dir);
    }
    command
        .arg("run")
        .arg("-c")
        .arg(&state.paths.temp_config_file)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| AppError::process("Failed to launch xray.exe", Some(error.to_string())))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stop_requested = Arc::new(AtomicBool::new(false));
    let child = Arc::new(tokio::sync::Mutex::new(child));
    if let Some(stdout) = stdout {
        spawn_pipe_reader(state, stdout, LogSource::XrayStdout);
    }
    if let Some(stderr) = stderr {
        spawn_pipe_reader(state, stderr, LogSource::XrayStderr);
    }

    let readiness_port = if tun_mode { socks_port } else { http_port };
    let readiness_label = if tun_mode {
        "local SOCKS diagnostic port"
    } else {
        "local proxy port"
    };
    if let Err(error) =
        wait_for_local_port(&child, readiness_port, XRAY_START_TIMEOUT_MS, readiness_label).await
    {
        terminate_child(&child).await;
        return Err(error);
    }
    let previous_winhttp_dump = if tun_mode {
        None
    } else {
        proxy_manager_windows::capture_winhttp_dump().ok()
    };

    if tun_mode {
        let _ = proxy_manager_windows::clear_proxy();
    } else {
        if let Err(error) = proxy_manager_windows::set_proxy(http_port, &settings) {
            terminate_child(&child).await;
            return Err(error);
        }
        if let Err(error) = proxy_manager_windows::apply_winhttp_proxy(http_port, &settings) {
            let _ = state
                .log_if_enabled(
                    LogSource::Connection,
                    LogLevel::Warn,
                    format!("Unable to apply WinHTTP proxy compatibility layer: {}", error.message),
                )
                .await;
        }
        if !proxy_manager_windows::verify_proxy(http_port)? {
            terminate_child(&child).await;
            return Err(AppError::proxy(
                "System proxy verification failed after enabling the proxy.",
                None,
            ));
        }
    }

    let session_id = state.connection.next_session_id();
    let connected_at = Utc::now();
    let session = ManagedSession {
        id: session_id,
        profile_id: profile.id.clone(),
        connected_at,
        http_port,
        socks_port,
        config_path: state.paths.temp_config_file.clone(),
        stop_requested: stop_requested.clone(),
        child: child.clone(),
    };

    {
        let mut session_slot = state.connection.session.lock().await;
        *session_slot = Some(session);
    }
    let proxy_string = if tun_mode {
        None
    } else {
        Some(format!("127.0.0.1:{}", http_port))
    };
    state
        .runtime_state
        .mark_connected(
            profile.id.clone(),
            http_port,
            socks_port,
            proxy_string,
            previous_winhttp_dump,
        )
        .await?;

    let connected_status = ConnectionStatusPayload {
        state: ConnectionState::Connected,
        active_profile_id: Some(profile.id.clone()),
        message: Some(if tun_mode {
            format!("Connected via {} in TUN mode", profile.name)
        } else {
            format!("Connected via {}", profile.name)
        }),
        connected_at: Some(connected_at),
        local_http_proxy_port: if tun_mode { None } else { Some(http_port) },
        local_socks_proxy_port: Some(socks_port),
        restart_count,
    };
    set_status(app, state, connected_status.clone()).await?;

    let _ = state
        .log_if_enabled(
            LogSource::Connection,
            LogLevel::Info,
            if tun_mode {
                format!(
                    "Connected profile '{}' in TUN mode. Local SOCKS diagnostic port 127.0.0.1:{socks_port}.",
                    profile.name
                )
            } else {
                format!(
                    "Connected profile '{}' through local proxy 127.0.0.1:{http_port}",
                    profile.name
                )
            },
        )
        .await;

    spawn_session_monitor(app.clone(), session_id, profile.id.clone(), stop_requested, child);

    Ok(connected_status)
}

async fn disconnect_locked(app: &AppHandle, state: &AppState) -> AppResult<ConnectionStatusPayload> {
    {
        let mut desired = state.connection.desired_profile_id.write().await;
        *desired = None;
    }

    let session = {
        let mut session_slot = state.connection.session.lock().await;
        session_slot.take()
    };

    let mut proxy_error = None;
    let runtime_snapshot = state.runtime_state.snapshot().await;

    if let Err(error) = proxy_manager_windows::clear_proxy() {
        proxy_error = Some(error);
    }
    if let Err(error) =
        proxy_manager_windows::restore_winhttp_proxy(runtime_snapshot.last_winhttp_dump.as_deref())
    {
        let _ = state
            .log_if_enabled(
                LogSource::Connection,
                LogLevel::Warn,
                format!("Unable to restore previous WinHTTP proxy state: {}", error.message),
            )
            .await;
    }

    if let Some(session) = session {
        session.stop_requested.store(true, Ordering::SeqCst);
        terminate_child(&session.child).await;
        let _ = tokio::fs::remove_file(&session.config_path).await;
    } else {
        let _ = tokio::fs::remove_file(&state.paths.temp_config_file).await;
    }

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

    let _ = state
        .log_if_enabled(
            LogSource::Connection,
            LogLevel::Info,
            "Disconnected and cleared system proxy.",
        )
        .await;

    if let Some(error) = proxy_error {
        return Err(error);
    }

    Ok(disconnected)
}

fn spawn_pipe_reader(state: &AppState, stream: impl Read + Send + 'static, source: LogSource) {
    let logger = state.log_manager.clone();
    let settings_store = state.settings_store.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let logger = logger.clone();
            let settings_store = settings_store.clone();
            let source = source.clone();
            tauri::async_runtime::spawn(async move {
                if settings_store.get().await.debug_logging {
                    let _ = logger.log(source, LogLevel::Info, line).await;
                }
            });
        }
    });
}

fn spawn_session_monitor(
    app: AppHandle,
    session_id: u64,
    profile_id: String,
    stop_requested: Arc<AtomicBool>,
    child: Arc<tokio::sync::Mutex<std::process::Child>>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;

            if stop_requested.load(Ordering::SeqCst) {
                break;
            }

            let exit_status = {
                let mut child = child.lock().await;
                child.try_wait()
            };

            match exit_status {
                Ok(Some(status)) => {
                    if stop_requested.load(Ordering::SeqCst) {
                        break;
                    }
                    let state = app.state::<AppState>();
                    let _ = state
                        .log_if_enabled(
                            LogSource::Connection,
                            LogLevel::Warn,
                            format!(
                                "Xray exited unexpectedly with status {}. Preparing recovery.",
                                status
                            ),
                        )
                        .await;
                    let _ = proxy_manager_windows::clear_proxy();
                    let runtime_snapshot = state.runtime_state.snapshot().await;
                    let _ = proxy_manager_windows::restore_winhttp_proxy(
                        runtime_snapshot.last_winhttp_dump.as_deref(),
                    );
                    let _ = handle_unexpected_exit(&app, state.inner(), session_id, profile_id.clone()).await;
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    let state = app.state::<AppState>();
                    let _ = state
                        .log_if_enabled(
                            LogSource::Connection,
                            LogLevel::Warn,
                            format!("Unable to poll Xray process status: {}", error),
                        )
                        .await;
                }
            }
        }
    });
}

async fn handle_unexpected_exit(
    app: &AppHandle,
    state: &AppState,
    session_id: u64,
    profile_id: String,
) -> AppResult<()> {
    {
        let mut session = state.connection.session.lock().await;
        if session.as_ref().map(|value| value.id) == Some(session_id) {
            *session = None;
        }
    }

    state.runtime_state.clear().await?;
    let settings = state.settings_store.get().await;
    let desired_profile = state.connection.desired_profile_id.read().await.clone();

    if settings.auto_reconnect && desired_profile.as_deref() == Some(profile_id.as_str()) {
        let mut restart_count = state.connection.status.read().await.restart_count.saturating_add(1);
        let delays = [3u64, 5u64, 8u64];

        for delay_seconds in delays {
            let reconnecting = ConnectionStatusPayload {
                state: ConnectionState::Connecting,
                active_profile_id: Some(profile_id.clone()),
                message: Some(format!("Xray exited unexpectedly. Reconnecting in {}s...", delay_seconds)),
                connected_at: None,
                local_http_proxy_port: None,
                local_socks_proxy_port: None,
                restart_count,
            };
            set_status(app, state, reconnecting).await?;
            sleep(Duration::from_secs(delay_seconds)).await;

            let reconnect = {
                let _guard = state.connection.op_lock.lock().await;
                if state.connection.desired_profile_id.read().await.as_deref() != Some(profile_id.as_str()) {
                    return Ok(());
                }
                start_session(app, state, profile_id.clone(), Some(restart_count)).await
            };

            match reconnect {
                Ok(_) => return Ok(()),
                Err(error) => {
                    let _ = state
                        .log_if_enabled(
                            LogSource::Connection,
                            LogLevel::Warn,
                            format!("Auto reconnect attempt failed: {}", error.message),
                        )
                        .await;
                }
            }

            restart_count = restart_count.saturating_add(1);
        }
    }

    let failed = ConnectionStatusPayload {
        state: ConnectionState::Error,
        active_profile_id: Some(profile_id),
        message: Some("Connection core exited unexpectedly.".to_string()),
        connected_at: None,
        local_http_proxy_port: None,
        local_socks_proxy_port: None,
        restart_count: state.connection.status.read().await.restart_count,
    };
    set_status(app, state, failed).await?;
    Ok(())
}

async fn wait_for_local_port(
    child: &Arc<tokio::sync::Mutex<std::process::Child>>,
    port: u16,
    timeout_ms: u64,
    label: &str,
) -> AppResult<()> {
    let start = std::time::Instant::now();
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    while start.elapsed() < Duration::from_millis(timeout_ms) {
        if TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok() {
            return Ok(());
        }

        {
            let mut child = child.lock().await;
            if let Some(status) = child.try_wait().map_err(|error| {
                AppError::process("Unable to query Xray process status", Some(error.to_string()))
            })? {
                return Err(AppError::process(
                    format!("xray.exe exited before the {label} became ready"),
                    Some(format!("exit status: {status}")),
                ));
            }
        }

        sleep(Duration::from_millis(250)).await;
    }

    Err(AppError::process(
        format!("Timed out waiting for the {label} to become ready"),
        Some(format!("127.0.0.1:{port}")),
    ))
}

fn ensure_tun_prerequisites(state: &AppState) -> AppResult<()> {
    let sidecar_dir = state
        .paths
        .sidecar_path
        .parent()
        .ok_or_else(|| AppError::process("Unable to resolve the xray.exe directory.", None))?;
    let wintun_path = sidecar_dir.join("wintun.dll");

    if !wintun_path.exists() {
        return Err(AppError::process(
            "TUN mode requires wintun.dll next to xray.exe.",
            Some(format!(
                "expected DLL path: {}; sidecar path: {}",
                wintun_path.display(),
                state.paths.sidecar_path.display()
            )),
        ));
    }

    if !crate::network_interface_manager::is_elevated() {
        return Err(AppError::validation(
            "TUN mode requires administrator privileges. Please run VailBox as Administrator.",
        ));
    }

    Ok(())
}

async fn set_status(app: &AppHandle, state: &AppState, status: ConnectionStatusPayload) -> AppResult<()> {
    {
        let mut guard = state.connection.status.write().await;
        *guard = status.clone();
    }
    app.emit("connection-status-changed", status)
        .map_err(|error| AppError::internal("Unable to emit connection status event", Some(error.to_string())))
}

async fn terminate_child(child: &Arc<tokio::sync::Mutex<std::process::Child>>) {
    let started = std::time::Instant::now();
    loop {
        {
            let mut locked = child.lock().await;
            let _ = locked.kill();
            match locked.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(_) => return,
            }
        }

        if started.elapsed() >= Duration::from_millis(DISCONNECT_TIMEOUT_MS) {
            return;
        }

        sleep(Duration::from_millis(120)).await;
    }
}

fn pick_two_distinct_ports() -> AppResult<(u16, u16)> {
    for _ in 0..5 {
        let a = portpicker::pick_unused_port()
            .ok_or_else(|| AppError::process("Unable to reserve a local loopback port", None))?;
        let b = portpicker::pick_unused_port()
            .ok_or_else(|| AppError::process("Unable to reserve a local loopback port", None))?;
        if a != b {
            return Ok((a, b));
        }
    }
    Err(AppError::process(
        "Failed to pick two distinct local ports after multiple attempts",
        None,
    ))
}

async fn run_connection_test(app: &AppHandle, state: &AppState, profile: &Profile) -> AppResult<()> {
    let settings = state.settings_store.get().await;
    let tun_mode = matches!(settings.connection_mode, ConnectionMode::Tun);

    if !state.paths.sidecar_path.exists() {
        return Err(AppError::process(
            "xray.exe was not found. Place it into src-tauri/bin/xray.exe before running.",
            Some(state.paths.sidecar_path.display().to_string()),
        ));
    }
    if tun_mode {
        ensure_tun_prerequisites(state)?;
    }

    let (socks_port, http_port) = pick_two_distinct_ports()?;
    let config = build_xray_config(profile, &settings, socks_port, http_port)?;
    let test_config_path = state
        .paths
        .temp_config_file
        .with_file_name(format!("xray-test-{}.json", profile.id));
    tokio::fs::write(&test_config_path, config).await?;

    let mut command = Command::new(&state.paths.sidecar_path);
    if let Some(sidecar_dir) = state.paths.sidecar_path.parent() {
        command.current_dir(sidecar_dir);
    }
    command
        .arg("run")
        .arg("-c")
        .arg(&test_config_path)
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| AppError::process("Failed to launch xray.exe", Some(error.to_string())))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(tokio::sync::Mutex::new(child));

    if let Some(stdout) = stdout {
        spawn_pipe_reader(state, stdout, LogSource::XrayStdout);
    }
    if let Some(stderr) = stderr {
        spawn_pipe_reader(state, stderr, LogSource::XrayStderr);
    }

    let readiness_port = if tun_mode { socks_port } else { http_port };
    let readiness_label = if tun_mode {
        "local SOCKS diagnostic port"
    } else {
        "local proxy port"
    };

    let result = wait_for_local_port(&child, readiness_port, XRAY_START_TIMEOUT_MS, readiness_label).await;
    terminate_child(&child).await;
    let _ = tokio::fs::remove_file(&test_config_path).await;
    let _ = app;
    result
}
