use std::path::PathBuf;

use chrono::Utc;
use tokio::sync::RwLock;
use url::Host;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{NetworkType, Profile, ProfileInput, ProfileSource, SecurityType},
};

pub struct ProfileStore {
    path: PathBuf,
    profiles: RwLock<Vec<Profile>>,
}

impl ProfileStore {
    pub async fn load(path: PathBuf) -> AppResult<Self> {
        let profiles = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str::<Vec<Profile>>(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            path,
            profiles: RwLock::new(profiles),
        })
    }

    async fn persist(&self, profiles: &[Profile]) -> AppResult<()> {
        let json = serde_json::to_string_pretty(profiles)?;
        tokio::fs::write(&self.path, json).await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<Profile> {
        self.profiles.read().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<Profile> {
        self.profiles
            .read()
            .await
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
    }

    pub async fn save(&self, input: ProfileInput) -> AppResult<Profile> {
        validate_profile_input(&input)?;
        let mut profiles = self.profiles.write().await;
        let now = Utc::now();
        let existing = input
            .id
            .as_ref()
            .and_then(|id| profiles.iter().find(|profile| profile.id == *id))
            .cloned();
        let source = input
            .source
            .clone()
            .or_else(|| existing.as_ref().map(|profile| profile.source.clone()))
            .unwrap_or(ProfileSource::Manual);
        let source_label = trim_option(input.source_label.clone())
            .or_else(|| existing.as_ref().and_then(|profile| profile.source_label.clone()));
        let subscription_id = trim_option(input.subscription_id.clone())
            .or_else(|| existing.as_ref().and_then(|profile| profile.subscription_id.clone()));

        let profile = Profile {
            id: input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: input.name.trim().to_string(),
            server_address: input.server_address.trim().to_string(),
            port: input.port,
            uuid: input.uuid.trim().to_string(),
            network_type: input.network_type,
            security_type: input.security_type,
            flow: trim_option(input.flow),
            sni: trim_option(input.sni),
            fingerprint: trim_option(input.fingerprint),
            public_key: trim_option(input.public_key),
            short_id: trim_option(input.short_id),
            spider_x: trim_option(input.spider_x),
            path: trim_option(input.path),
            host_header: trim_option(input.host_header),
            service_name: trim_option(input.service_name),
            xhttp_mode: trim_option(input.xhttp_mode),
            transport_header_type: trim_option(input.transport_header_type),
            seed: trim_option(input.seed),
            alpn: input
                .alpn
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
            allow_insecure: input.allow_insecure,
            remark: trim_option(input.remark),
            source,
            source_label,
            subscription_id,
            created_at: now,
            updated_at: now,
        };

        let saved = if let Some(existing) = profiles.iter_mut().find(|item| item.id == profile.id) {
            let created_at = existing.created_at;
            *existing = Profile {
                created_at,
                updated_at: now,
                ..profile.clone()
            };
            existing.clone()
        } else {
            profiles.push(profile.clone());
            profile
        };

        self.persist(&profiles).await?;
        Ok(saved)
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let mut profiles = self.profiles.write().await;
        let initial_len = profiles.len();
        profiles.retain(|profile| profile.id != id);
        if profiles.len() == initial_len {
            return Err(AppError::not_found("Profile was not found"));
        }
        self.persist(&profiles).await
    }

    pub async fn duplicate(&self, id: &str) -> AppResult<Profile> {
        let original = self
            .get(id)
            .await
            .ok_or_else(|| AppError::not_found("Profile was not found"))?;
        self.save(ProfileInput {
            id: None,
            name: format!("{} Copy", original.name),
            server_address: original.server_address,
            port: original.port,
            uuid: original.uuid,
            network_type: original.network_type,
            security_type: original.security_type,
            flow: original.flow,
            sni: original.sni,
            fingerprint: original.fingerprint,
            public_key: original.public_key,
            short_id: original.short_id,
            spider_x: original.spider_x,
            path: original.path,
            host_header: original.host_header,
            service_name: original.service_name,
            xhttp_mode: original.xhttp_mode,
            transport_header_type: original.transport_header_type,
            seed: original.seed,
            alpn: original.alpn,
            allow_insecure: original.allow_insecure,
            remark: original.remark,
            source: Some(original.source),
            source_label: original.source_label,
            subscription_id: original.subscription_id,
        })
        .await
    }

    pub async fn delete_by_subscription_id(&self, subscription_id: &str) -> AppResult<Vec<Profile>> {
        let mut profiles = self.profiles.write().await;
        let removed = profiles
            .iter()
            .filter(|profile| profile.subscription_id.as_deref() == Some(subscription_id))
            .cloned()
            .collect::<Vec<_>>();
        profiles.retain(|profile| profile.subscription_id.as_deref() != Some(subscription_id));
        self.persist(&profiles).await?;
        Ok(removed)
    }
}

fn trim_option(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_profile_input(input: &ProfileInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("Profile name is required"));
    }
    if input.server_address.trim().is_empty() {
        return Err(AppError::validation("Server address is required"));
    }
    Host::parse(input.server_address.trim())
        .map_err(|_| AppError::validation("Server address is invalid"))?;
    Uuid::parse_str(input.uuid.trim())?;
    if input.port == 0 {
        return Err(AppError::validation("Port must be between 1 and 65535"));
    }

    match (&input.network_type, &input.security_type) {
        (NetworkType::Raw | NetworkType::Tcp | NetworkType::Ws | NetworkType::Grpc | NetworkType::Xhttp | NetworkType::Httpupgrade, SecurityType::Reality) => {
            if trim_option(input.public_key.clone()).is_none() {
                return Err(AppError::validation("Reality public key is required"));
            }
            if trim_option(input.sni.clone()).is_none() {
                return Err(AppError::validation("Reality SNI is required"));
            }
            if matches!(input.network_type, NetworkType::Grpc) && trim_option(input.service_name.clone()).is_none() {
                return Err(AppError::validation("gRPC service name is required"));
            }
            if matches!(input.network_type, NetworkType::Xhttp) && trim_option(input.path.clone()).is_none() {
                return Err(AppError::validation("XHTTP path is required"));
            }
            if matches!(input.network_type, NetworkType::Httpupgrade) && trim_option(input.path.clone()).is_none() {
                return Err(AppError::validation("HTTPUpgrade path is required"));
            }
        }
        (NetworkType::Raw | NetworkType::Tcp, SecurityType::None | SecurityType::Tls) => {}
        (NetworkType::Ws, SecurityType::None | SecurityType::Tls) => {}
        (NetworkType::Grpc, SecurityType::None | SecurityType::Tls) => {
            if trim_option(input.service_name.clone()).is_none() {
                return Err(AppError::validation("gRPC service name is required"));
            }
        }
        (NetworkType::Xhttp, SecurityType::None | SecurityType::Tls) => {
            if trim_option(input.path.clone()).is_none() {
                return Err(AppError::validation("XHTTP path is required"));
            }
        }
        (NetworkType::Httpupgrade, SecurityType::None | SecurityType::Tls) => {
            if trim_option(input.path.clone()).is_none() {
                return Err(AppError::validation("HTTPUpgrade path is required"));
            }
        }
        (NetworkType::Kcp, SecurityType::None | SecurityType::Tls) => {}
        _ => {
            return Err(AppError::validation(
                "Unsupported VLESS mode or security combination for this build",
            ))
        }
    }

    Ok(())
}
