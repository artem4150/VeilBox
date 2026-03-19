use std::{
    path::PathBuf,
    process::Child,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager};
use tokio::sync::{Mutex, RwLock};

use crate::{
    error::{AppError, AppResult},
    log_manager::LogManager,
    models::{ConnectionStatusPayload, LogLevel, LogSource, RuntimeSessionState},
    profile_store::ProfileStore,
    settings_store::SettingsStore,
};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root_dir: PathBuf,
    pub profiles_file: PathBuf,
    pub subscriptions_file: PathBuf,
    pub settings_file: PathBuf,
    pub runtime_file: PathBuf,
    pub app_log_file: PathBuf,
    pub connection_log_file: PathBuf,
    pub temp_config_file: PathBuf,
    pub sidecar_path: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let root_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::storage("Unable to resolve app data directory", Some(error.to_string())))?;

        let logs_dir = root_dir.join("logs");
        let runtime_dir = root_dir.join("runtime");
        std::fs::create_dir_all(&logs_dir)?;
        std::fs::create_dir_all(&runtime_dir)?;

        let workspace_sidecar_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("bin")
            .join("xray.exe");
        let resource_sidecar_path = app
            .path()
            .resolve("bin/xray.exe", tauri::path::BaseDirectory::Resource)
            .ok()
            .filter(|path| path.exists());

        let sidecar_path = if cfg!(debug_assertions) && workspace_sidecar_path.exists() {
            workspace_sidecar_path
        } else {
            resource_sidecar_path.unwrap_or(workspace_sidecar_path)
        };

        Ok(Self {
            root_dir: root_dir.clone(),
            profiles_file: root_dir.join("profiles.json"),
            subscriptions_file: root_dir.join("subscriptions.json"),
            settings_file: root_dir.join("settings.json"),
            runtime_file: runtime_dir.join("session.json"),
            app_log_file: logs_dir.join("app.jsonl"),
            connection_log_file: logs_dir.join("connection.jsonl"),
            temp_config_file: runtime_dir.join("xray-active.json"),
            sidecar_path,
        })
    }
}

#[derive(Debug, Default)]
pub struct RuntimeStateStore {
    path: RwLock<Option<PathBuf>>,
    inner: RwLock<RuntimeSessionState>,
}

impl RuntimeStateStore {
    pub async fn load(path: PathBuf) -> AppResult<Self> {
        let state = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str::<RuntimeSessionState>(&content).unwrap_or_default()
        } else {
            RuntimeSessionState::default()
        };

        Ok(Self {
            path: RwLock::new(Some(path)),
            inner: RwLock::new(state),
        })
    }

    async fn persist(&self) -> AppResult<()> {
        let path = self.path.read().await.clone().ok_or_else(|| {
            AppError::internal("Runtime state path is not initialized", None)
        })?;
        let value = self.inner.read().await.clone();
        let json = serde_json::to_string_pretty(&value)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    pub async fn snapshot(&self) -> RuntimeSessionState {
        self.inner.read().await.clone()
    }

    pub async fn mark_connected(
        &self,
        profile_id: String,
        http_port: u16,
        socks_port: u16,
        proxy_string: Option<String>,
        winhttp_dump: Option<String>,
    ) -> AppResult<()> {
        {
            let mut state = self.inner.write().await;
            state.was_connected = true;
            state.last_profile_id = Some(profile_id);
            state.last_http_proxy_port = Some(http_port);
            state.last_socks_proxy_port = Some(socks_port);
            state.last_proxy_string = proxy_string;
            state.last_winhttp_dump = winhttp_dump;
        }
        self.persist().await
    }

    pub async fn clear(&self) -> AppResult<()> {
        {
            let mut state = self.inner.write().await;
            *state = RuntimeSessionState::default();
        }
        self.persist().await
    }
}

#[derive(Debug)]
pub struct ManagedSession {
    pub id: u64,
    pub profile_id: String,
    pub connected_at: DateTime<Utc>,
    pub http_port: u16,
    pub socks_port: u16,
    pub config_path: PathBuf,
    pub stop_requested: Arc<AtomicBool>,
    pub child: Arc<Mutex<Child>>,
}

#[derive(Debug, Default)]
pub struct ConnectionRuntime {
    pub status: RwLock<ConnectionStatusPayload>,
    pub session: Mutex<Option<ManagedSession>>,
    pub op_lock: Mutex<()>,
    pub desired_profile_id: RwLock<Option<String>>,
    pub session_counter: AtomicU64,
}

impl ConnectionRuntime {
    pub fn next_session_id(&self) -> u64 {
        self.session_counter.fetch_add(1, Ordering::Relaxed) + 1
    }
}

pub struct AppState {
    pub paths: Arc<AppPaths>,
    pub profile_store: Arc<ProfileStore>,
    pub subscription_store: Arc<crate::subscription_store::SubscriptionStore>,
    pub settings_store: Arc<SettingsStore>,
    pub runtime_state: Arc<RuntimeStateStore>,
    pub log_manager: Arc<LogManager>,
    pub connection: Arc<ConnectionRuntime>,
}

impl AppState {
    pub async fn is_logging_enabled(&self) -> bool {
        self.settings_store.get().await.debug_logging
    }

    pub async fn log_if_enabled(
        &self,
        source: LogSource,
        level: LogLevel,
        message: impl AsRef<str>,
    ) -> AppResult<()> {
        if !self.is_logging_enabled().await {
            return Ok(());
        }
        self.log_manager.log(source, level, message).await
    }
}
