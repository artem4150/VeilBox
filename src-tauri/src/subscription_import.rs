use std::net::IpAddr;
use std::time::Duration;
use std::{collections::HashSet, vec};

use base64::{engine::general_purpose, Engine as _};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    models::{NetworkType, ProfileEngine, ProfileInput, ProfileSource, ProxyProtocol, SecurityType},
    proxy_uri_parser::{parse_proxy_uri, parse_shadowsocks_json},
};

static HREF_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)(?:href|src)\s*=\s*["']([^"'#\s>]+)["']"#).unwrap());
static URL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"https?://[^\s"'<>`]+"#).unwrap());
static PROXY_URI_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)(?:vless|ss|hy2|hysteria2)://[^\s"'<>`]+"#).unwrap());

pub struct ImportedSubscriptionPayload {
    pub name: String,
    pub url: String,
    pub profiles: Vec<ProfileInput>,
}

pub async fn import_subscription_url(url: &str) -> AppResult<ImportedSubscriptionPayload> {
    let parsed = Url::parse(url.trim())
        .map_err(|error| AppError::validation(format!("Invalid subscription URL: {}", error)))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::validation(
            "Subscription URL must use http or https",
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("VeilBox/0.1")
        .build()?;

    let source_label = parsed.host_str().map(|host| host.to_string());
    let name = infer_subscription_name(&parsed);
    let profiles = import_subscription_with_discovery(&client, parsed.clone(), source_label).await?;

    Ok(ImportedSubscriptionPayload {
        name,
        url: parsed.to_string(),
        profiles,
    })
}

async fn import_subscription_with_discovery(
    client: &reqwest::Client,
    entrypoint: Url,
    source_label: Option<String>,
) -> AppResult<Vec<ProfileInput>> {
    const MAX_FETCHES: usize = 10;
    const MAX_DEPTH: usize = 2;

    let mut queue = vec![(entrypoint.clone(), 0usize)];
    let mut visited = HashSet::<String>::new();
    let mut discovered_bodies = Vec::<String>::new();
    let mut fetch_errors = Vec::<String>::new();

    while let Some((current_url, depth)) = queue.pop() {
        let key = normalize_url_key(&current_url);
        if !visited.insert(key) {
            continue;
        }
        if visited.len() > MAX_FETCHES {
            break;
        }

        match fetch_subscription_body(client, &current_url).await {
            Ok(body) => {
                discovered_bodies.push(body.clone());
                let decoded = decode_subscription_body(&body);
                if decoded != body {
                    discovered_bodies.push(decoded.clone());
                }

                if depth >= MAX_DEPTH {
                    continue;
                }

                let mut next_urls = derive_related_subscription_urls(&current_url);
                next_urls.extend(extract_nested_subscription_urls(&body, &current_url));
                for next_url in next_urls {
                    if is_supported_subscription_scheme(&next_url)
                        && !visited.contains(&normalize_url_key(&next_url))
                    {
                        queue.push((next_url, depth + 1));
                    }
                }
            }
            Err(error) => {
                fetch_errors.push(format!("{} ({})", current_url, error.message));
            }
        }
    }

    parse_subscription_payloads(discovered_bodies, fetch_errors, source_label)
}

async fn fetch_subscription_body(client: &reqwest::Client, url: &Url) -> AppResult<String> {
    if let Some(host_str) = url.host_str() {
        let port = url.port_or_known_default().unwrap_or(80);
        if let Ok(addrs) = tokio::net::lookup_host((host_str, port)).await {
            for addr in addrs {
                let ip = addr.ip();
                let is_private = match ip {
                    IpAddr::V4(v4) => {
                        v4.is_loopback()
                            || v4.is_private()
                            || v4.is_multicast()
                            || v4.is_link_local()
                            || v4.is_broadcast()
                            || v4.is_unspecified()
                    }
                    IpAddr::V6(v6) => {
                        v6.is_loopback()
                            || v6.is_multicast()
                            || v6.is_unspecified()
                            || (v6.segments()[0] & 0xfe00) == 0xfc00
                    }
                };
                if is_private {
                    return Err(AppError::validation(format!(
                        "Blocked attempt to fetch from private IP: {}",
                        ip
                    )));
                }
            }
        }
    }

    let mut response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| AppError::process("Failed to send request", Some(e.to_string())))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::process(
            format!("Subscription request failed with status {}", status),
            None,
        ));
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| AppError::process("Failed to read response chunk", Some(e.to_string())))?
    {
        body.extend_from_slice(&chunk);
        if body.len() > 10_000_000 {
            return Err(AppError::validation(
                "Subscription response too large (limit 10MB)",
            ));
        }
    }

    String::from_utf8(body).map_err(|_| AppError::validation("Invalid UTF-8 in subscription response"))
}

fn parse_subscription_payloads(
    bodies: Vec<String>,
    fetch_errors: Vec<String>,
    source_label: Option<String>,
) -> AppResult<Vec<ProfileInput>> {
    let mut unique_links = HashSet::<String>::new();
    let mut imported = Vec::<ProfileInput>::new();

    for body in &bodies {
        let links = extract_proxy_links(body);
        for link in links {
            if !unique_links.insert(link.clone()) {
                continue;
            }
            if let Ok(mut profile) = parse_proxy_uri(&link) {
                profile.source = Some(ProfileSource::Subscription);
                profile.source_label = source_label.clone();
                imported.push(profile);
            }
        }
    }

    for body in &bodies {
        imported.extend(parse_xray_subscription_json(body, source_label.clone()));
    }

    if !imported.is_empty() {
        return Ok(imported);
    }

    let details = if fetch_errors.is_empty() {
        None
    } else {
        Some(fetch_errors.join("; "))
    };
    Err(AppError::new(
        "VALIDATION_ERROR",
        "Subscription does not contain any supported VLESS, Shadowsocks or Hysteria2 entries",
        details,
    ))
}

fn normalize_url_key(url: &Url) -> String {
    let mut normalized = url.clone();
    normalized.set_fragment(None);
    normalized.to_string()
}

fn is_supported_subscription_scheme(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

fn derive_related_subscription_urls(url: &Url) -> Vec<Url> {
    let mut derived = Vec::new();

    let segments = url
        .path_segments()
        .map(|parts| parts.collect::<Vec<_>>())
        .unwrap_or_default();

    if segments.iter().any(|segment| segment.eq_ignore_ascii_case("about")) {
        let filtered = segments
            .iter()
            .copied()
            .filter(|segment| !segment.eq_ignore_ascii_case("about"))
            .collect::<Vec<_>>();

        let mut candidate = url.clone();
        if filtered.is_empty() {
            candidate.set_path("/");
        } else {
            candidate.set_path(&format!("/{}/", filtered.join("/")));
        }
        candidate.set_query(None);
        candidate.set_fragment(None);
        derived.push(candidate);
    }

    if url.host_str() == Some("ultm.app") {
        let path = url.path().trim_matches('/');
        if !path.is_empty() {
            if let Ok(candidate) =
                Url::parse(&format!("https://api.ultm.in/user/subscription/{}?context=1", path))
            {
                derived.push(candidate);
            }
        }
    }

    derived
}

fn extract_nested_subscription_urls(raw: &str, base_url: &Url) -> Vec<Url> {
    let mut urls = Vec::<Url>::new();

    for capture in HREF_REGEX.captures_iter(raw) {
        if let Some(url) = capture
            .get(1)
            .and_then(|value| base_url.join(value.as_str()).ok())
        {
            urls.push(url);
        }
    }

    for capture in URL_REGEX.find_iter(raw) {
        if let Ok(url) = Url::parse(capture.as_str()) {
            urls.push(url);
        }
    }

    let mut seen = HashSet::<String>::new();
    urls.into_iter()
        .filter(|url| {
            let key = normalize_url_key(url);
            seen.insert(key)
        })
        .collect()
}

fn decode_subscription_body(raw: &str) -> String {
    let trimmed = raw.trim();
    if contains_proxy_uri(trimmed) || looks_like_structured_payload(trimmed) {
        return trimmed.to_string();
    }
    if let Ok(decoded_url) = urlencoding::decode(trimmed) {
        let decoded_url = decoded_url.to_string();
        if contains_proxy_uri(&decoded_url) || looks_like_structured_payload(&decoded_url) {
            return decoded_url;
        }
    }

    let compact: String = trimmed.chars().filter(|char| !char.is_whitespace()).collect();
    let engines = [
        &general_purpose::STANDARD,
        &general_purpose::STANDARD_NO_PAD,
        &general_purpose::URL_SAFE,
        &general_purpose::URL_SAFE_NO_PAD,
    ];

    for engine in engines {
        if let Ok(bytes) = engine.decode(compact.as_bytes()) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                if contains_proxy_uri(&decoded) || looks_like_structured_payload(&decoded) {
                    return decoded;
                }
                if let Ok(url_decoded) = urlencoding::decode(&decoded) {
                    let url_decoded = url_decoded.to_string();
                    if contains_proxy_uri(&url_decoded)
                        || looks_like_structured_payload(&url_decoded)
                    {
                        return url_decoded;
                    }
                }
            }
        }
    }

    trimmed.to_string()
}

fn contains_proxy_uri(value: &str) -> bool {
    PROXY_URI_REGEX.is_match(value)
}

fn extract_proxy_links(raw: &str) -> Vec<String> {
    let mut links = HashSet::<String>::new();

    for matched in PROXY_URI_REGEX.find_iter(raw) {
        let link = matched
            .as_str()
            .trim_end_matches(|char| matches!(char, ')' | ']' | '}' | ',' | ';' | '.'))
            .to_string();
        if !link.is_empty() {
            links.insert(link);
        }
    }

    for line in raw.lines() {
        let token = line.trim();
        if ["vless://", "ss://", "hy2://", "hysteria2://"].iter().any(|prefix| token.to_ascii_lowercase().starts_with(prefix)) {
            let link = token
                .trim_matches(|char| matches!(char, '"' | '\'' | '`'))
                .trim_end_matches(|char| matches!(char, ')' | ']' | '}' | ',' | ';' | '.'))
                .to_string();
            if !link.is_empty() {
                links.insert(link);
            }
        }
    }

    links.into_iter().collect()
}

fn infer_subscription_name(url: &Url) -> String {
    if let Some(fragment) = url.fragment().filter(|value| !value.trim().is_empty()) {
        return fragment.trim().to_string();
    }
    if let Some(host) = url.host_str() {
        return host.to_string();
    }
    "Subscription".to_string()
}

fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn looks_like_structured_payload(value: &str) -> bool {
    looks_like_json(value) || value.lines().any(|line| line.trim_start().starts_with("proxies:"))
}

fn parse_xray_subscription_json(raw: &str, source_label: Option<String>) -> Vec<ProfileInput> {
    let value: Value = if let Ok(value) = serde_json::from_str(raw.trim()) {
        value
    } else if let Ok(value) = serde_saphyr::from_str(raw.trim()) {
        value
    } else {
        return Vec::new();
    };

    let items = match value {
        Value::Array(items) => items,
        Value::Object(_) => vec![value],
        _ => return Vec::new(),
    };

    let mut profiles = Vec::new();
    for item in items {
        let standalone = item.get("proxies").and_then(Value::as_array);
        if let Some(entries) = standalone {
            for entry in entries {
                if let Some(mut profile) = parse_shadowsocks_json(entry) {
                    profile.source = Some(ProfileSource::Subscription);
                    profile.source_label = source_label.clone();
                    profiles.push(profile);
                }
            }
        } else if let Some(mut profile) = parse_shadowsocks_json(&item) {
            profile.source = Some(ProfileSource::Subscription);
            profile.source_label = source_label.clone();
            profiles.push(profile);
        }
        let Some(outbounds) = item.get("outbounds").and_then(Value::as_array) else { continue; };
        for outbound in outbounds {
            if !matches!(outbound.get("protocol").and_then(Value::as_str), Some("vless" | "shadowsocks" | "hysteria")) { continue; }
            let mut single = item.clone();
            single["outbounds"] = serde_json::json!([outbound]);
            if let Some(profile) = parse_xray_profile_object(single, source_label.clone()) {
                profiles.push(profile);
            }
        }
    }
    profiles
}

fn parse_xray_profile_object(value: Value, source_label: Option<String>) -> Option<ProfileInput> {
    let remarks = value
        .get("remarks")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .get("remark")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "Imported subscription profile".to_string());

    let outbound = value
        .get("outbounds")
        .and_then(Value::as_array)
        .and_then(|outbounds| {
            outbounds
                .iter()
                .find(|outbound| matches!(outbound.get("protocol").and_then(Value::as_str), Some("vless" | "shadowsocks" | "hysteria")))
        })?;

    if outbound.get("protocol").and_then(Value::as_str) != Some("vless") {
        return parse_other_xray_outbound(outbound, remarks, source_label);
    }

    let vnext = outbound
        .get("settings")
        .and_then(|value| value.get("vnext"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())?;
    let user = vnext
        .get("users")
        .and_then(Value::as_array)
        .and_then(|items| items.first())?;

    let server_address = vnext.get("address")?.as_str()?.trim().to_string();
    let port = vnext.get("port")?.as_u64()? as u16;
    let uuid = user.get("id")?.as_str()?.trim().to_string();
    let flow = user
        .get("flow")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let stream_settings = outbound.get("streamSettings");
    let network_type = parse_network_type_from_json(stream_settings)?;
    let security_type = parse_security_type_from_json(stream_settings)?;

    let sni = stream_settings
        .and_then(|value| {
            value
                .get("realitySettings")
                .and_then(|value| value.get("serverName"))
                .or_else(|| value.get("tlsSettings").and_then(|value| value.get("serverName")))
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let fingerprint = stream_settings
        .and_then(|value| {
            value
                .get("realitySettings")
                .and_then(|value| value.get("fingerprint"))
                .or_else(|| value.get("tlsSettings").and_then(|value| value.get("fingerprint")))
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let public_key = stream_settings
        .and_then(|value| value.get("realitySettings"))
        .and_then(|value| value.get("publicKey"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let short_id = stream_settings
        .and_then(|value| value.get("realitySettings"))
        .and_then(|value| value.get("shortId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let spider_x = stream_settings
        .and_then(|value| value.get("realitySettings"))
        .and_then(|value| value.get("spiderX"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let service_name = stream_settings
        .and_then(|value| value.get("grpcSettings"))
        .and_then(|value| value.get("serviceName"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let authority = stream_settings
        .and_then(|value| value.get("grpcSettings"))
        .and_then(|value| value.get("authority"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let ws_path = stream_settings
        .and_then(|value| value.get("wsSettings"))
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let ws_host = stream_settings
        .and_then(|value| value.get("wsSettings"))
        .and_then(|value| value.get("headers"))
        .and_then(|value| value.get("Host"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let xhttp_path = stream_settings
        .and_then(|value| value.get("xhttpSettings"))
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let xhttp_host = stream_settings
        .and_then(|value| value.get("xhttpSettings"))
        .and_then(|value| value.get("host"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let xhttp_mode = stream_settings
        .and_then(|value| value.get("xhttpSettings"))
        .and_then(|value| value.get("mode"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let resolved_path = if matches!(network_type, NetworkType::Xhttp) {
        ws_path.or(xhttp_path).or(Some("/".to_string()))
    } else {
        ws_path.or(xhttp_path)
    };

    let alpn = stream_settings
        .and_then(|value| value.get("tlsSettings"))
        .and_then(|value| value.get("alpn"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let allow_insecure = stream_settings
        .and_then(|value| value.get("tlsSettings"))
        .and_then(|value| value.get("allowInsecure"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Some(ProfileInput {
        id: None,
        name: remarks,
        engine: ProfileEngine::Xray,
        protocol: Default::default(),
        password: None,
        method: None,
        obfs_password: None,
        server_address,
        port,
        uuid,
        network_type,
        security_type,
        flow,
        sni,
        fingerprint,
        public_key,
        short_id,
        spider_x,
        path: resolved_path,
        host_header: ws_host.or(xhttp_host).or(authority),
        service_name,
        xhttp_mode,
        transport_header_type: None,
        seed: None,
        alpn,
        allow_insecure,
        remark: None,
        source: Some(ProfileSource::Subscription),
        source_label,
        subscription_id: None,
        amnezia_config: None,
    })
}

fn parse_other_xray_outbound(outbound: &Value, name: String, source_label: Option<String>) -> Option<ProfileInput> {
    let settings = outbound.get("settings")?;
    let (protocol, server_address, port, password, method, obfs_password, security_type, sni, allow_insecure, alpn) =
        match outbound.get("protocol")?.as_str()? {
            "shadowsocks" => (
                ProxyProtocol::Shadowsocks,
                settings.get("address")?.as_str()?.to_string(), u16::try_from(settings.get("port")?.as_u64()?).ok()?,
                settings.get("password")?.as_str()?.to_string(),
                Some(settings.get("method")?.as_str()?.to_string()), None, SecurityType::None,
                None, false, Vec::new(),
            ),
            "hysteria" if settings.get("version")?.as_u64()? == 2 => {
                let stream = outbound.get("streamSettings")?;
                let transport = stream.get("hysteriaSettings")?;
                if transport.get("version")?.as_u64()? != 2 || stream.get("security")?.as_str()? != "tls" { return None; }
                let tls = stream.get("tlsSettings");
                (
                    ProxyProtocol::Hysteria2,
                    settings.get("address")?.as_str()?.to_string(), u16::try_from(settings.get("port")?.as_u64()?).ok()?,
                    transport.get("auth")?.as_str()?.to_string(), None,
                    stream.get("finalmask").and_then(|value| value.get("udp")).and_then(Value::as_array)
                        .and_then(|items| items.iter().find(|item| item.get("type").and_then(Value::as_str) == Some("salamander")))
                        .and_then(|item| item.get("settings")).and_then(|value| value.get("password"))
                        .and_then(Value::as_str).map(str::to_string),
                    SecurityType::Tls,
                    tls.and_then(|value| value.get("serverName")).and_then(Value::as_str).map(str::to_string),
                    tls.and_then(|value| value.get("allowInsecure")).and_then(Value::as_bool).unwrap_or(false),
                    tls.and_then(|value| value.get("alpn")).and_then(Value::as_array)
                        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default(),
                )
            }
            _ => return None,
        };
    if port == 0 || password.is_empty() { return None; }
    Some(ProfileInput {
        id: None, name, engine: ProfileEngine::Xray, protocol, password: Some(password), method, obfs_password,
        server_address, port, uuid: String::new(), network_type: NetworkType::Raw,
        security_type, flow: None, sni, fingerprint: None, public_key: None,
        short_id: None, spider_x: None, path: None, host_header: None,
        service_name: None, xhttp_mode: None, transport_header_type: None,
        seed: None, alpn, allow_insecure, remark: None,
        source: Some(ProfileSource::Subscription), source_label, subscription_id: None,
        amnezia_config: None,
    })
}

#[cfg(test)]
mod proxy_protocol_tests {
    use super::*;

    #[test]
    fn decodes_shadowsocks_only_base64_subscription() {
        let body = "ss://YWVzLTI1Ni1nY206c2VjcmV0@example.com:8388#Test\n";
        let encoded = general_purpose::STANDARD.encode(body);
        assert_eq!(decode_subscription_body(&encoded), body);
        let profiles = parse_subscription_payloads(vec![encoded, body.to_string()], vec![], None).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].protocol, ProxyProtocol::Shadowsocks);
    }

    #[test]
    fn decodes_hysteria_only_base64_subscription() {
        let body = "hy2://secret@example.com:443#Test";
        let encoded = general_purpose::STANDARD.encode(body);
        assert_eq!(decode_subscription_body(&encoded), body);
    }

    #[test]
    fn imports_shadowsocks_from_clash_json_subscription() {
        let body = serde_json::json!({"proxies": [{"type":"ss", "name":"Fast", "server":"example.com", "port":8388, "cipher":"aes-256-gcm", "password":"secret"}]}).to_string();
        let profiles = parse_subscription_payloads(vec![body], vec![], None).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].protocol, ProxyProtocol::Shadowsocks);
    }

    #[test]
    fn imports_shadowsocks_from_clash_yaml_subscription() {
        let body = "proxies:\n  - name: Fast SS\n    type: ss\n    server: example.com\n    port: 8388\n    cipher: aes-256-gcm\n    password: secret\n";
        let encoded = general_purpose::STANDARD.encode(body);
        assert_eq!(decode_subscription_body(&encoded), body);
        let profiles = parse_subscription_payloads(vec![body.to_string()], vec![], None).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].protocol, ProxyProtocol::Shadowsocks);
    }

    #[test]
    fn imports_multiple_xray_outbound_protocols() {
        let json = serde_json::json!({
            "remarks": "mixed",
            "outbounds": [
                {"protocol":"shadowsocks", "settings":{"address":"ss.example.com", "port":8388, "method":"aes-256-gcm", "password":"secret"}},
                {"protocol":"hysteria", "settings":{"version":2, "address":"hy.example.com", "port":443},
                    "streamSettings":{"security":"tls", "hysteriaSettings":{"version":2, "auth":"secret"}, "tlsSettings":{"serverName":"hy.example.com"}}}
            ]
        });
        let profiles = parse_xray_subscription_json(&json.to_string(), None);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].protocol, ProxyProtocol::Shadowsocks);
        assert_eq!(profiles[1].protocol, ProxyProtocol::Hysteria2);
    }
}

fn parse_network_type_from_json(stream_settings: Option<&Value>) -> Option<NetworkType> {
    match stream_settings
        .and_then(|value| value.get("network"))
        .and_then(Value::as_str)
        .unwrap_or("tcp")
        .to_ascii_lowercase()
        .as_str()
    {
        "raw" => Some(NetworkType::Raw),
        "tcp" => Some(NetworkType::Tcp),
        "ws" => Some(NetworkType::Ws),
        "grpc" => Some(NetworkType::Grpc),
        "xhttp" => Some(NetworkType::Xhttp),
        "httpupgrade" => Some(NetworkType::Httpupgrade),
        "kcp" => Some(NetworkType::Kcp),
        _ => None,
    }
}

fn parse_security_type_from_json(stream_settings: Option<&Value>) -> Option<SecurityType> {
    match stream_settings
        .and_then(|value| value.get("security"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_ascii_lowercase()
        .as_str()
    {
        "none" => Some(SecurityType::None),
        "tls" => Some(SecurityType::Tls),
        "reality" => Some(SecurityType::Reality),
        _ => None,
    }
}
