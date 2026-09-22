use base64::{engine::general_purpose, Engine as _};
use percent_encoding::percent_decode_str;
use serde_json::Value;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    models::{NetworkType, ProfileEngine, ProfileInput, ProxyProtocol, SecurityType},
    vless_parser::parse_vless_uri,
};

pub fn parse_proxy_uri(raw: &str) -> AppResult<ProfileInput> {
    let uri = raw.trim().trim_matches('`').trim_matches('"');
    let scheme = uri.split_once("://").map(|(scheme, _)| scheme.to_ascii_lowercase());
    match scheme.as_deref() {
        Some("vless") => parse_vless_uri(uri),
        Some("ss") => parse_shadowsocks_uri(uri),
        Some("hy2" | "hysteria2") => parse_hysteria2_uri(uri),
        _ => Err(AppError::validation("Expected vless://, ss://, hy2:// or hysteria2:// URI")),
    }
}

/// Accepts common standalone Shadowsocks and Clash-style JSON entries.
pub fn parse_shadowsocks_json(value: &Value) -> Option<ProfileInput> {
    if value.get("type").and_then(Value::as_str).is_some_and(|kind| kind != "ss" && kind != "shadowsocks") {
        return None;
    }
    if value.get("plugin").is_some() {
        return None;
    }
    let host = value.get("server")?.as_str()?.trim();
    let port_value = value.get("server_port").or_else(|| value.get("port"))?;
    let port = port_value.as_u64()
        .or_else(|| port_value.as_str().and_then(|port| port.parse().ok()))
        .and_then(|port| u16::try_from(port).ok())?;
    let method = value.get("method").or_else(|| value.get("cipher"))?.as_str()?.trim();
    let password = value.get("password")?.as_str()?;
    if host.is_empty() || port == 0 || method.is_empty() || password.is_empty() {
        return None;
    }
    let name = value.get("name").and_then(Value::as_str).filter(|name| !name.trim().is_empty()).unwrap_or(host);
    Some(base_profile(name.to_string(), host.to_string(), port, ProxyProtocol::Shadowsocks,
        Some(password.to_string()), Some(method.to_string()), SecurityType::None, None, false, Vec::new()))
}

fn parse_shadowsocks_uri(uri: &str) -> AppResult<ProfileInput> {
    let (without_fragment, fragment) = uri[5..].split_once('#').unwrap_or((&uri[5..], ""));
    let (authority, query) = without_fragment.split_once('?').unwrap_or((without_fragment, ""));
    if url::form_urlencoded::parse(query.as_bytes()).any(|(key, _)| key == "plugin") {
        return Err(AppError::validation("Shadowsocks plugin links are not supported by this Xray profile"));
    }
    let authority = authority.trim_end_matches('/');
    let decoded_authority = if authority.contains('@') {
        authority.to_string()
    } else {
        decode_base64(authority)?
    };
    let (credentials, endpoint) = decoded_authority.rsplit_once('@')
        .ok_or_else(|| AppError::validation("Shadowsocks URI is missing credentials or server"))?;
    let percent_decoded_credentials = decode_component(credentials);
    let decoded_credentials = if percent_decoded_credentials.contains(':') {
        percent_decoded_credentials
    } else {
        decode_base64(credentials)?
    };
    let (method, password) = decoded_credentials.split_once(':')
        .ok_or_else(|| AppError::validation("Shadowsocks URI must contain method:password"))?;
    if method.is_empty() || password.is_empty() {
        return Err(AppError::validation("Shadowsocks method and password are required"));
    }
    let (host, port) = parse_endpoint(endpoint)?;
    Ok(base_profile(
        (!fragment.is_empty()).then(|| decode_component(fragment)).unwrap_or_else(|| host.clone()),
        host, port, ProxyProtocol::Shadowsocks, Some(password.to_string()), Some(method.to_string()),
        SecurityType::None, None, false, Vec::new(),
    ))
}

fn parse_hysteria2_uri(uri: &str) -> AppResult<ProfileInput> {
    let parsed = Url::parse(uri).map_err(|_| AppError::validation("Invalid Hysteria2 URI"))?;
    let authority = uri.split_once("://").unwrap().1.split(['?', '#']).next().unwrap_or("");
    let (auth, _) = authority.rsplit_once('@')
        .ok_or_else(|| AppError::validation("Hysteria2 URI is missing auth password"))?;
    let password = decode_component(auth);
    if password.is_empty() { return Err(AppError::validation("Hysteria2 auth password is required")); }
    let host = parsed.host_str().ok_or_else(|| AppError::validation("Hysteria2 server is missing"))?.to_string();
    let port = parsed.port().ok_or_else(|| AppError::validation("Hysteria2 port is missing"))?;
    let mut sni = None;
    let mut insecure = false;
    let mut alpn = Vec::new();
    let mut obfs = None;
    let mut obfs_password = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "sni" | "peer" => sni = Some(value.into_owned()),
            "insecure" | "allowInsecure" => insecure = matches!(value.as_ref(), "1" | "true"),
            "alpn" => alpn = value.split(',').map(str::to_string).filter(|item| !item.is_empty()).collect(),
            "obfs" => obfs = Some(value.into_owned()),
            "obfs-password" | "obfsParam" => obfs_password = Some(value.into_owned()),
            _ => {}
        }
    }
    if obfs.as_deref().is_some_and(|value| value != "salamander") {
        return Err(AppError::validation("Only Hysteria2 Salamander obfuscation is supported"));
    }
    if obfs.is_some() && obfs_password.as_deref().unwrap_or("").is_empty() {
        return Err(AppError::validation("Hysteria2 Salamander password is required"));
    }
    if obfs.is_none() && obfs_password.is_some() {
        return Err(AppError::validation("Hysteria2 obfs password requires obfs=salamander"));
    }
    let mut profile = base_profile(
        parsed.fragment().map(decode_component).filter(|name| !name.is_empty()).unwrap_or_else(|| host.clone()),
        host, port, ProxyProtocol::Hysteria2, Some(password), None,
        SecurityType::Tls, sni, insecure, alpn,
    );
    profile.obfs_password = obfs_password;
    Ok(profile)
}

fn parse_endpoint(endpoint: &str) -> AppResult<(String, u16)> {
    let parsed = Url::parse(&format!("https://{endpoint}"))
        .map_err(|_| AppError::validation("Invalid proxy server address"))?;
    let host = parsed.host_str().ok_or_else(|| AppError::validation("Proxy server is missing"))?;
    let port = parsed.port().ok_or_else(|| AppError::validation("Proxy server port is missing"))?;
    Ok((host.to_string(), port))
}

fn decode_component(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().to_string()
}

fn decode_base64(value: &str) -> AppResult<String> {
    let value = value.trim_end_matches('=');
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(value)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(value))
        .map_err(|_| AppError::validation("Invalid base64 proxy credentials"))?;
    String::from_utf8(decoded).map_err(|_| AppError::validation("Proxy credentials are not UTF-8"))
}

#[allow(clippy::too_many_arguments)]
fn base_profile(name: String, server_address: String, port: u16, protocol: ProxyProtocol,
    password: Option<String>, method: Option<String>, security_type: SecurityType,
    sni: Option<String>, allow_insecure: bool, alpn: Vec<String>) -> ProfileInput {
    ProfileInput {
        id: None, name, engine: ProfileEngine::Xray, protocol, password, method, obfs_password: None,
        server_address, port, uuid: String::new(), network_type: NetworkType::Raw,
        security_type, flow: None, sni, fingerprint: None, public_key: None,
        short_id: None, spider_x: None, path: None, host_header: None,
        service_name: None, xhttp_mode: None, transport_header_type: None,
        seed: None, alpn, allow_insecure, remark: None, source: None,
        source_label: None, subscription_id: None, amnezia_config: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ss_sip002_and_hysteria2() {
        let ss = parse_proxy_uri("ss://YWVzLTI1Ni1nY206c2VjcmV0@example.com:8388#Test").unwrap();
        assert_eq!(ss.protocol, ProxyProtocol::Shadowsocks);
        assert_eq!(ss.method.as_deref(), Some("aes-256-gcm"));
        assert_eq!(ss.password.as_deref(), Some("secret"));
        let plain = parse_proxy_uri("ss://aes-256-gcm:pa%40ss%3Aword@example.com:8388#Plain").unwrap();
        assert_eq!(plain.password.as_deref(), Some("pa@ss:word"));
        let legacy = parse_proxy_uri("ss://YWVzLTI1Ni1nY206c2VjcmV0QGV4YW1wbGUuY29tOjgzODg=#Legacy").unwrap();
        assert_eq!(legacy.port, 8388);
        let hy2 = parse_proxy_uri("hy2://secret@example.com:443?sni=cdn.example.com#Fast").unwrap();
        assert_eq!(hy2.protocol, ProxyProtocol::Hysteria2);
        assert_eq!(hy2.sni.as_deref(), Some("cdn.example.com"));
        assert!(parse_proxy_uri("hy2://secret@example.com:443?obfs=salamander").is_err());
        assert_eq!(parse_proxy_uri("hy2://secret@example.com:443?obfs=salamander&obfs-password=mask").unwrap().obfs_password.as_deref(), Some("mask"));
    }

    #[test]
    fn parses_shadowsocks_json_formats() {
        let native = serde_json::json!({"server":"example.com","server_port":8388,"method":"aes-256-gcm","password":"secret"});
        assert_eq!(parse_shadowsocks_json(&native).unwrap().method.as_deref(), Some("aes-256-gcm"));
        let clash = serde_json::json!({"type":"ss","name":"Fast","server":"example.com","port":8388,"cipher":"chacha20-ietf-poly1305","password":"secret"});
        assert_eq!(parse_shadowsocks_json(&clash).unwrap().name, "Fast");
    }
}
