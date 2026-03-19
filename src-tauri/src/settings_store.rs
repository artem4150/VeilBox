use std::path::PathBuf;
use std::net::IpAddr;

use tokio::sync::RwLock;

use crate::{
    error::{AppError, AppResult},
    models::{ConnectionMode, Settings, SettingsPatch, SplitTunnelMode},
};

pub struct SettingsStore {
    path: PathBuf,
    settings: RwLock<Settings>,
}

impl SettingsStore {
    pub async fn load(path: PathBuf) -> AppResult<Self> {
        let settings = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str::<Settings>(&content).unwrap_or_default()
        } else {
            Settings::default()
        };

        Ok(Self {
            path,
            settings: RwLock::new(settings),
        })
    }

    async fn persist(&self, settings: &Settings) -> AppResult<()> {
        let json = serde_json::to_string_pretty(settings)?;
        tokio::fs::write(&self.path, json).await?;
        Ok(())
    }

    pub async fn get(&self) -> Settings {
        self.settings.read().await.clone()
    }

    pub async fn update(&self, patch: SettingsPatch) -> AppResult<Settings> {
        let mut settings = self.settings.write().await;
        if let Some(value) = patch.launch_at_startup {
            settings.launch_at_startup = value;
        }
        if let Some(value) = patch.minimize_to_tray {
            settings.minimize_to_tray = value;
        }
        if let Some(value) = patch.auto_reconnect {
            settings.auto_reconnect = value;
        }
        if let Some(value) = patch.theme {
            settings.theme = value;
        }
        if let Some(value) = patch.language {
            settings.language = value;
        }
        if let Some(value) = patch.debug_logging {
            settings.debug_logging = value;
        }
        if let Some(value) = patch.connection_mode {
            settings.connection_mode = value;
        }
        if let Some(value) = patch.tun_interface_name {
            let trimmed = value.trim();
            settings.tun_interface_name = if trimmed.is_empty() {
                "xray0".to_string()
            } else {
                trimmed.to_string()
            };
        }
        if let Some(value) = patch.tun_disable_ipv6 {
            settings.tun_disable_ipv6 = value;
        }
        if let Some(value) = patch.tun_outbound_interface {
            settings.tun_outbound_interface = value.and_then(|item| {
                let trimmed = item.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });
        }
        if let Some(value) = patch.split_tunnel_mode {
            settings.split_tunnel_mode = value;
        }
        if let Some(value) = patch.split_tunnel_domains {
            settings.split_tunnel_domains = normalize_domain_entries(value)?;
        }
        if let Some(value) = patch.split_tunnel_ips {
            settings.split_tunnel_ips = normalize_ip_entries(value)?;
        }
        if let Some(value) = patch.last_selected_profile_id {
            settings.last_selected_profile_id = value;
        }

        if matches!(settings.connection_mode, ConnectionMode::SystemProxy)
            && matches!(settings.split_tunnel_mode, SplitTunnelMode::ProxyListed)
        {
            settings.split_tunnel_mode = SplitTunnelMode::BypassListed;
        }

        let snapshot = settings.clone();
        self.persist(&snapshot).await?;
        Ok(snapshot)
    }
}

fn normalize_domain_entries(values: Vec<String>) -> AppResult<Vec<String>> {
    let mut items = Vec::new();

    for raw in values {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().any(char::is_whitespace) {
            return Err(AppError::validation(format!(
                "Split tunnel domain entry contains whitespace: {}",
                trimmed
            )));
        }
        items.push(trimmed.to_string());
    }

    items.sort();
    items.dedup();
    Ok(items)
}

fn normalize_ip_entries(values: Vec<String>) -> AppResult<Vec<String>> {
    let mut items = Vec::new();

    for raw in values {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        validate_ip_or_cidr(trimmed)?;
        items.push(trimmed.to_string());
    }

    items.sort();
    items.dedup();
    Ok(items)
}

fn validate_ip_or_cidr(value: &str) -> AppResult<()> {
    if value.parse::<IpAddr>().is_ok() {
        return Ok(());
    }

    let Some((ip, prefix)) = value.split_once('/') else {
        return Err(AppError::validation(format!(
            "Invalid split tunnel IP or CIDR entry: {}",
            value
        )));
    };

    let ip_addr = ip.parse::<IpAddr>().map_err(|_| {
        AppError::validation(format!(
            "Invalid split tunnel IP or CIDR entry: {}",
            value
        ))
    })?;

    let prefix = prefix.parse::<u8>().map_err(|_| {
        AppError::validation(format!(
            "Invalid split tunnel CIDR prefix: {}",
            value
        ))
    })?;

    let max_prefix = match ip_addr {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };

    if prefix > max_prefix {
        return Err(AppError::validation(format!(
            "Invalid split tunnel CIDR prefix: {}",
            value
        )));
    }

    Ok(())
}
