use std::collections::BTreeMap;

use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        AmneziaConfig, AmneziaParam, NetworkType, ProfileEngine, ProfileInput, ProfileSource,
        SecurityType,
    },
};

#[derive(Default)]
struct ParsedSections {
    interface: BTreeMap<String, String>,
    peer: BTreeMap<String, String>,
    advanced: Vec<AmneziaParam>,
}

pub fn parse_amnezia_config(raw_config: &str, override_name: Option<String>) -> AppResult<ProfileInput> {
    let raw_config = raw_config.trim();
    if raw_config.is_empty() {
        return Err(AppError::validation("Amnezia config is empty"));
    }

    let sections = parse_sections(raw_config)?;
    let endpoint = sections
        .peer
        .get("endpoint")
        .cloned()
        .ok_or_else(|| AppError::validation("Amnezia config must contain Peer.Endpoint"))?;
    let (endpoint_host, endpoint_port) = parse_endpoint(&endpoint)?;

    let interface_addresses = split_csv(
        sections
            .interface
            .get("address")
            .ok_or_else(|| AppError::validation("Amnezia config must contain Interface.Address"))?,
    );
    let dns_servers = split_csv(sections.interface.get("dns").map(String::as_str).unwrap_or_default());
    let allowed_ips = split_csv(
        sections
            .peer
            .get("allowedips")
            .ok_or_else(|| AppError::validation("Amnezia config must contain Peer.AllowedIPs"))?,
    );
    let interface_private_key = sections
        .interface
        .get("privatekey")
        .cloned()
        .ok_or_else(|| AppError::validation("Amnezia config must contain Interface.PrivateKey"))?;
    let peer_public_key = sections
        .peer
        .get("publickey")
        .cloned()
        .ok_or_else(|| AppError::validation("Amnezia config must contain Peer.PublicKey"))?;
    let preshared_key = sections.peer.get("presharedkey").cloned();
    let persistent_keepalive = sections
        .peer
        .get("persistentkeepalive")
        .and_then(|value| value.parse::<u16>().ok());

    let name = override_name
        .and_then(trimmed)
        .unwrap_or_else(|| format!("Amnezia {}", endpoint_host));

    Ok(ProfileInput {
        id: None,
        name,
        engine: ProfileEngine::Amneziawg,
        server_address: endpoint_host.clone(),
        port: endpoint_port,
        uuid: Uuid::nil().to_string(),
        network_type: NetworkType::Raw,
        security_type: SecurityType::None,
        flow: None,
        sni: None,
        fingerprint: None,
        public_key: None,
        short_id: None,
        spider_x: None,
        path: None,
        host_header: None,
        service_name: None,
        xhttp_mode: None,
        transport_header_type: None,
        seed: None,
        alpn: Vec::new(),
        allow_insecure: false,
        remark: Some("Imported AmneziaWG configuration".to_string()),
        source: Some(ProfileSource::Manual),
        source_label: None,
        subscription_id: None,
        amnezia_config: Some(AmneziaConfig {
            raw_config: raw_config.to_string(),
            interface_addresses,
            dns_servers,
            interface_private_key,
            endpoint_host,
            endpoint_port,
            peer_public_key,
            allowed_ips,
            preshared_key,
            persistent_keepalive,
            advanced_params: sections.advanced,
        }),
    })
}

pub fn parse_amnezia_uri(uri: &str) -> AppResult<ProfileInput> {
    let compact = uri.trim().trim_matches('`').trim_matches('"').trim_matches('\'');
    let encoded_source = compact
        .strip_prefix("amnezia ")
        .or_else(|| compact.strip_prefix("Amnezia "))
        .unwrap_or(compact);
    let encoded = encoded_source
        .strip_prefix("vpn://")
        .or_else(|| encoded_source.strip_prefix("VPN://"))
        .ok_or_else(|| AppError::validation("Amnezia URI must start with vpn://"))?;

    let payload = decode_share_payload(encoded)?;
    let root: Value = serde_json::from_slice(&payload)
        .map_err(|error| AppError::validation(format!("Invalid Amnezia share payload: {}", error)))?;

    let default_container = root
        .get("defaultContainer")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string());
    let description = root
        .get("description")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let dns1 = root.get("dns1").and_then(Value::as_str).unwrap_or("1.1.1.1");
    let dns2 = root.get("dns2").and_then(Value::as_str).unwrap_or("1.0.0.1");

    let container = root
        .get("containers")
        .and_then(Value::as_array)
        .and_then(|containers| {
            default_container.as_ref().and_then(|default_name| {
                containers.iter().find(|entry| {
                    entry
                        .get("container")
                        .and_then(Value::as_str)
                        .map(|name| name == default_name)
                        .unwrap_or(false)
                })
            })
        })
        .or_else(|| {
            root.get("containers")
                .and_then(Value::as_array)
                .and_then(|containers| containers.first())
        })
        .ok_or_else(|| AppError::validation("Amnezia URI does not contain any containers"))?;

    let awg = container
        .get("awg")
        .ok_or_else(|| AppError::validation("Only AmneziaWG shares are supported in this build"))?;
    let last_config: Value = awg
        .get("last_config")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation("Amnezia share is missing last_config"))  
        .and_then(|raw| {
            serde_json::from_str(raw)
                .map_err(|error| AppError::validation(format!("Invalid Amnezia last_config: {}", error)))
        })?;
    let config_template = last_config
        .get("config")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation("Amnezia share is missing config template"))?;
    let expanded = config_template
        .replace("$PRIMARY_DNS", dns1)
        .replace("$SECONDARY_DNS", dns2);

    parse_amnezia_config(&expanded, description)
}

fn parse_sections(raw_config: &str) -> AppResult<ParsedSections> {
    let mut sections = ParsedSections::default();
    let mut current_section = String::new();

    for raw_line in raw_config.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| AppError::validation(format!("Invalid config line: {}", line)))?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().to_string();

        match current_section.as_str() {
            "interface" => {
                if is_standard_interface_key(&key) {
                    sections.interface.insert(key, value);
                } else {
                    sections.advanced.push(AmneziaParam { key, value });
                }
            }
            "peer" => {
                if is_standard_peer_key(&key) {
                    sections.peer.insert(key, value);
                } else {
                    sections.advanced.push(AmneziaParam { key, value });
                }
            }
            "" => {
                return Err(AppError::validation(
                    "Amnezia config must start with [Interface] and [Peer] sections",
                ));
            }
            _ => {
                sections.advanced.push(AmneziaParam {
                    key: format!("{}:{}", current_section, key),
                    value,
                });
            }
        }
    }

    if sections.interface.is_empty() || sections.peer.is_empty() {
        return Err(AppError::validation(
            "Amnezia config must contain both [Interface] and [Peer] sections",
        ));
    }

    Ok(sections)
}

fn decode_share_payload(encoded: &str) -> AppResult<Vec<u8>> {
    let compact: String = encoded.chars().filter(|char| !char.is_whitespace()).collect();
    let decoded = general_purpose::URL_SAFE
        .decode(format!("{}{}", compact, "=".repeat((4 - compact.len() % 4) % 4)))
        .map_err(|error| AppError::validation(format!("Invalid Amnezia URI payload: {}", error)))?;

    if decoded.len() < 5 {
        return Err(AppError::validation("Amnezia URI payload is too short"));
    }

    let expected_size = u32::from_be_bytes([decoded[0], decoded[1], decoded[2], decoded[3]]) as usize;
    let inflated = miniz_oxide::inflate::decompress_to_vec_zlib(&decoded[4..])
        .map_err(|_| AppError::validation("Failed to decompress Amnezia URI payload"))?;

    if expected_size != inflated.len() {
        return Err(AppError::validation(
            "Amnezia URI payload length check failed after decompression",
        ));
    }

    Ok(inflated)
}

fn is_standard_interface_key(key: &str) -> bool {
    matches!(key, "privatekey" | "address" | "dns" | "listenport" | "mtu")
}

fn is_standard_peer_key(key: &str) -> bool {
    matches!(
        key,
        "publickey" | "presharedkey" | "allowedips" | "endpoint" | "persistentkeepalive"
    )
}

fn parse_endpoint(endpoint: &str) -> AppResult<(String, u16)> {
    if let Some(rest) = endpoint.strip_prefix('[') {
        let (host, port) = rest
            .split_once("]:")
            .ok_or_else(|| AppError::validation("Invalid Amnezia endpoint"))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| AppError::validation("Invalid Amnezia endpoint port"))?;
        return Ok((host.to_string(), port));
    }

    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| AppError::validation("Invalid Amnezia endpoint"))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| AppError::validation("Invalid Amnezia endpoint port"))?;
    Ok((host.to_string(), port))
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(trimmed)
        .collect()
}

fn trimmed(value: impl AsRef<str>) -> Option<String> {
    let value = value.as_ref().trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
