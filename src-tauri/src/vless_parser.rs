use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use url::Host;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{NetworkType, ProfileInput, SecurityType},
};

pub fn parse_vless_uri(uri: &str) -> AppResult<ProfileInput> {
    let compact = normalize_uri(uri);
    if !compact.to_ascii_lowercase().starts_with("vless://") {
        return Err(AppError::validation("URI must start with vless://"));
    }

    let remainder = &compact["vless://".len()..];
    let (without_fragment, fragment) = split_once(remainder, '#');
    let (authority_part, query) = split_once(without_fragment, '?');
    let authority_part = authority_part.trim_end_matches('/');
    let (userinfo, host_port) = authority_part
        .rsplit_once('@')
        .ok_or_else(|| AppError::validation("VLESS URI is missing host section"))?;

    let uuid = userinfo.trim();
    if uuid.is_empty() {
        return Err(AppError::validation("VLESS URI is missing UUID"));
    }
    Uuid::parse_str(uuid)?;

    let (host, port) = parse_host_and_port(host_port)?;
    Host::parse(host).map_err(|_| AppError::validation("Server host is invalid"))?;

    if port == 0 {
        return Err(AppError::validation("Port must be between 1 and 65535"));
    }

    let params = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect::<HashMap<String, String>>();

    let network_type = parse_network_type(params.get("type").map(String::as_str))?;
    let security_type = parse_security_type(&params, &network_type)?;

    match (&network_type, &security_type) {
        (NetworkType::Raw | NetworkType::Tcp, SecurityType::None | SecurityType::Tls | SecurityType::Reality)
        | (NetworkType::Ws, SecurityType::None | SecurityType::Tls | SecurityType::Reality)
        | (NetworkType::Grpc, SecurityType::None | SecurityType::Tls | SecurityType::Reality)
        | (NetworkType::Xhttp, SecurityType::None | SecurityType::Tls | SecurityType::Reality)
        | (NetworkType::Httpupgrade, SecurityType::None | SecurityType::Tls | SecurityType::Reality)
        | (NetworkType::Kcp, SecurityType::None | SecurityType::Tls) => {}
        _ => {
            return Err(AppError::validation("Unsupported VLESS mode or security combination"))
        }
    }

    let fragment_name = if fragment.is_empty() {
        None
    } else {
        Some(percent_decode_str(fragment).decode_utf8_lossy().to_string())
    };
    let query_name = first_non_empty(&[
        params.get("remark").cloned(),
        params.get("remarks").cloned(),
        params.get("description").cloned(),
    ]);
    let name = first_non_empty(&[fragment_name, query_name]).unwrap_or_else(|| host.to_string());
    let is_grpc = matches!(network_type, NetworkType::Grpc);
    let is_xhttp = matches!(network_type, NetworkType::Xhttp);

    let input = ProfileInput {
        id: None,
        name,
        server_address: host.to_string(),
        port,
        uuid: uuid.to_string(),
        network_type,
        security_type,
        flow: params.get("flow").cloned(),
        sni: first_non_empty(&[
            params.get("sni").cloned(),
            params.get("serverName").cloned(),
        ]),
        fingerprint: first_non_empty(&[
            params.get("fp").cloned(),
            params.get("fingerprint").cloned(),
        ]),
        public_key: first_non_empty(&[
            params.get("pbk").cloned(),
            params.get("publicKey").cloned(),
        ]),
        short_id: first_non_empty(&[
            params.get("sid").cloned(),
            params.get("shortId").cloned(),
        ]),
        spider_x: first_non_empty(&[
            params.get("spx").cloned(),
            params.get("spiderX").cloned(),
        ]),
        path: params.get("path").cloned(),
        host_header: first_non_empty(&[
            params.get("authority").cloned(),
            params.get("host").cloned(),
        ]),
        service_name: first_non_empty(&[
            params.get("serviceName").cloned(),
            params.get("service_name").cloned(),
            params.get("path").filter(|_| is_grpc).cloned(),
        ]),
        xhttp_mode: first_non_empty(&[
            params.get("mode").filter(|_| is_xhttp).cloned(),
            params.get("xhttpMode").cloned(),
        ]),
        transport_header_type: first_non_empty(&[
            params.get("headerType").cloned(),
            params.get("header").cloned(),
        ]),
        seed: params.get("seed").cloned(),
        alpn: params
            .get("alpn")
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        allow_insecure: matches!(
            params.get("allowInsecure").map(String::as_str),
            Some("1") | Some("true") | Some("yes")
        ),
        remark: first_non_empty(&[
            params.get("remark").cloned(),
            params.get("description").cloned(),
        ]),
        source: None,
        source_label: None,
        subscription_id: None,
    };

    validate_imported_profile(&input)?;
    Ok(input)
}

fn normalize_uri(raw: &str) -> String {
    raw.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .chars()
        .filter(|char| !char.is_whitespace())
        .collect()
}

fn split_once<'a>(value: &'a str, needle: char) -> (&'a str, &'a str) {
    match value.split_once(needle) {
        Some((left, right)) => (left, right),
        None => (value, ""),
    }
}

fn parse_host_and_port(value: &str) -> AppResult<(&str, u16)> {
    if value.starts_with('[') {
        let closing = value
            .find(']')
            .ok_or_else(|| AppError::validation("IPv6 host is missing closing bracket"))?;
        let host = &value[1..closing];
        let port = value[closing + 1..]
            .strip_prefix(':')
            .ok_or_else(|| AppError::validation("VLESS URI is missing port"))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| AppError::validation("Port must be between 1 and 65535"))?;
        return Ok((host, port));
    }

    let (host, port) = value
        .rsplit_once(':')
        .ok_or_else(|| AppError::validation("VLESS URI is missing port"))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| AppError::validation("Port must be between 1 and 65535"))?;
    Ok((host, port))
}

fn parse_network_type(value: Option<&str>) -> AppResult<NetworkType> {
    match value.unwrap_or("tcp").to_ascii_lowercase().as_str() {
        "raw" | "tcp" => Ok(NetworkType::Raw),
        "ws" => Ok(NetworkType::Ws),
        "grpc" | "gun" => Ok(NetworkType::Grpc),
        "xhttp" | "splithttp" => Ok(NetworkType::Xhttp),
        "httpupgrade" => Ok(NetworkType::Httpupgrade),
        "kcp" | "mkcp" => Ok(NetworkType::Kcp),
        other => Err(AppError::validation(format!(
            "Unsupported transport type '{}'",
            other
        ))),
    }
}

fn parse_security_type(params: &HashMap<String, String>, network_type: &NetworkType) -> AppResult<SecurityType> {
    let explicit = params.get("security").map(|value| value.to_ascii_lowercase());
    let inferred = if params.contains_key("pbk") || params.contains_key("publicKey") {
        Some("reality".to_string())
    } else if matches!(
        network_type,
        NetworkType::Ws | NetworkType::Grpc | NetworkType::Xhttp | NetworkType::Httpupgrade
    ) {
        Some("tls".to_string())
    } else {
        None
    };

    match explicit.or(inferred).unwrap_or_else(|| "none".to_string()).as_str() {
        "none" => Ok(SecurityType::None),
        "reality" => Ok(SecurityType::Reality),
        "tls" => Ok(SecurityType::Tls),
        other => Err(AppError::validation(format!(
            "Unsupported security type '{}'",
            other
        ))),
    }
}

fn validate_imported_profile(profile: &ProfileInput) -> AppResult<()> {
    match (&profile.network_type, &profile.security_type) {
        (NetworkType::Raw | NetworkType::Tcp | NetworkType::Ws | NetworkType::Grpc | NetworkType::Xhttp | NetworkType::Httpupgrade, SecurityType::Reality) => {
            if profile.public_key.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::validation("Reality profile is missing public key"));
            }
            if profile.sni.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::validation("Reality profile is missing SNI"));
            }
            if matches!(profile.network_type, NetworkType::Grpc)
                && profile.service_name.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(AppError::validation("gRPC profile is missing serviceName"));
            }
            if matches!(profile.network_type, NetworkType::Xhttp | NetworkType::Httpupgrade)
                && profile.path.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(AppError::validation("HTTP path-based transport is missing path"));
            }
        }
        (NetworkType::Raw | NetworkType::Tcp, SecurityType::None | SecurityType::Tls) => {}
        (NetworkType::Ws, SecurityType::None | SecurityType::Tls) => {}
        (NetworkType::Grpc, SecurityType::None | SecurityType::Tls) => {
            if profile.service_name.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::validation("gRPC profile is missing serviceName"));
            }
        }
        (NetworkType::Xhttp | NetworkType::Httpupgrade, SecurityType::None | SecurityType::Tls) => {
            if profile.path.as_deref().unwrap_or("").trim().is_empty() {
                return Err(AppError::validation("HTTP path-based transport is missing path"));
            }
        }
        (NetworkType::Kcp, SecurityType::None | SecurityType::Tls) => {}
        _ => {
            return Err(AppError::validation("Unsupported VLESS mode or security combination"))
        }
    }

    Ok(())
}

fn first_non_empty(values: &[Option<String>]) -> Option<String> {
    values
        .iter()
        .flatten()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(|value| value.to_string())
}
