use std::{
    fs,
    net::Ipv4Addr,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    ptr::null_mut,
};

use winapi::um::wininet::{
    InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{ConnectionMode, Settings, SplitTunnelMode},
};

const INTERNET_SETTINGS: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct ProxyState {
    pub enabled: bool,
    pub server: Option<String>,
    pub auto_config_url: Option<String>,
}

pub fn set_proxy(port: u16, settings_snapshot: &Settings, pac_path: &Path) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(
            INTERNET_SETTINGS,
            winreg::enums::KEY_SET_VALUE | winreg::enums::KEY_QUERY_VALUE,
        )
        .map_err(|error| {
            AppError::proxy(
                "Unable to open Windows Internet Settings",
                Some(error.to_string()),
            )
        })?;

    if matches!(settings_snapshot.split_tunnel_mode, SplitTunnelMode::ProxyListed) {
        write_proxy_pac(port, settings_snapshot, pac_path)?;
        settings
            .set_value("ProxyEnable", &0u32)
            .map_err(|error| AppError::proxy("Failed to disable explicit proxy before PAC mode", Some(error.to_string())))?;
        let _ = settings.delete_value("ProxyServer");
        let _ = settings.delete_value("ProxyOverride");
        settings
            .set_value("AutoConfigURL", &pac_url_from_path(pac_path))
            .map_err(|error| AppError::proxy("Failed to enable PAC configuration", Some(error.to_string())))?;
    } else {
        let _ = settings.delete_value("AutoConfigURL");
        let _ = fs::remove_file(pac_path);
        settings
            .set_value("ProxyEnable", &1u32)
            .map_err(|error| AppError::proxy("Failed to enable system proxy", Some(error.to_string())))?;
        settings
            .set_value("ProxyServer", &format!("127.0.0.1:{port}"))
            .map_err(|error| AppError::proxy("Failed to save proxy address", Some(error.to_string())))?;
        settings
            .set_value("ProxyOverride", &build_proxy_override(settings_snapshot)?)
            .map_err(|error| AppError::proxy("Failed to save proxy bypass rules", Some(error.to_string())))?;
    }

    refresh_internet_settings()
}

pub fn clear_proxy(pac_path: Option<&Path>) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(
            INTERNET_SETTINGS,
            winreg::enums::KEY_SET_VALUE | winreg::enums::KEY_QUERY_VALUE,
        )
        .map_err(|error| {
            AppError::proxy(
                "Unable to open Windows Internet Settings",
                Some(error.to_string()),
            )
        })?;

    settings
        .set_value("ProxyEnable", &0u32)
        .map_err(|error| AppError::proxy("Failed to disable system proxy", Some(error.to_string())))?;
    let _ = settings.delete_value("ProxyServer");
    let _ = settings.delete_value("ProxyOverride");
    let _ = settings.delete_value("AutoConfigURL");
    if let Some(path) = pac_path {
        let _ = fs::remove_file(path);
    }

    refresh_internet_settings()
}

pub fn capture_winhttp_dump() -> AppResult<String> {
    run_netsh(["winhttp", "dump"])
}

pub fn apply_winhttp_proxy(port: u16, settings_snapshot: &Settings, pac_path: &Path) -> AppResult<()> {
    if !matches!(settings_snapshot.connection_mode, ConnectionMode::SystemProxy) {
        return Ok(());
    }

    if matches!(settings_snapshot.split_tunnel_mode, SplitTunnelMode::ProxyListed) {
        write_proxy_pac(port, settings_snapshot, pac_path)?;
        let payload = serde_json::json!({
            "Proxy": "",
            "ProxyBypass": "",
            "AutoconfigUrl": pac_url_from_path(pac_path),
            "AutoDetect": false
        })
        .to_string();
        let settings_arg = format!("settings={payload}");
        let _ = run_netsh(["winhttp", "set", "advproxy", "setting-scope=user", &settings_arg])?;
    } else {
        let proxy_server = format!("127.0.0.1:{port}");
        let bypass_list = format!("bypass-list={}", build_proxy_override(settings_snapshot)?);
        let _ = run_netsh(["winhttp", "set", "proxy", &proxy_server, &bypass_list])?;
    }
    Ok(())
}

pub fn restore_winhttp_proxy(previous_dump: Option<&str>) -> AppResult<()> {
    match previous_dump {
        Some(dump) if !dump.trim().is_empty() => apply_winhttp_dump(dump),
        _ => reset_winhttp_proxy(),
    }
}

pub fn verify_proxy(port: u16) -> AppResult<bool> {
    let state = get_proxy_state()?;
    if state.enabled {
        return Ok(state
            .server
            .unwrap_or_default()
            .contains(&format!("127.0.0.1:{port}")));
    }

    Ok(state
        .auto_config_url
        .unwrap_or_default()
        .contains("system-proxy.pac"))
}

pub fn best_effort_cleanup(
    known_proxy_string: Option<String>,
    previous_winhttp_dump: Option<String>,
) -> AppResult<bool> {
    let mut cleaned = false;
    let state = get_proxy_state()?;
    if !state.enabled && state.auto_config_url.is_none() {
        // continue to WinHTTP cleanup
    } else {
        let server = state.server.unwrap_or_default();
        let auto_config_url = state.auto_config_url.unwrap_or_default();
        let looks_like_ours = known_proxy_string
            .map(|known| server == known)
            .unwrap_or_else(|| {
                server.contains("127.0.0.1:")
                    || auto_config_url.contains("system-proxy.pac")
                    || auto_config_url.contains("VailBox")
            });

        if looks_like_ours {
            clear_proxy(None)?;
            cleaned = true;
        }
    }

    if let Ok(dump) = capture_winhttp_dump() {
        if dump.contains("127.0.0.1:") || dump.contains("AutoConfigUrl") {
            restore_winhttp_proxy(previous_winhttp_dump.as_deref())?;
            cleaned = true;
        }
    }

    Ok(cleaned)
}

pub fn get_proxy_state() -> AppResult<ProxyState> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(INTERNET_SETTINGS, winreg::enums::KEY_QUERY_VALUE)
        .map_err(|error| {
            AppError::proxy(
                "Unable to read Windows system proxy state",
                Some(error.to_string()),
            )
        })?;

    let enabled = settings.get_value::<u32, _>("ProxyEnable").unwrap_or_default() == 1;
    let server = settings.get_value::<String, _>("ProxyServer").ok();
    let auto_config_url = settings.get_value::<String, _>("AutoConfigURL").ok();
    Ok(ProxyState {
        enabled,
        server,
        auto_config_url,
    })
}

fn build_proxy_override(settings: &Settings) -> AppResult<String> {
    let mut entries = vec!["<local>".to_string()];

    if !matches!(settings.connection_mode, ConnectionMode::SystemProxy) {
        return Ok(entries.join(";"));
    }

    if matches!(settings.split_tunnel_mode, SplitTunnelMode::BypassListed) {
        for domain in &settings.split_tunnel_domains {
            entries.extend(domain_to_proxy_override(domain)?);
        }
        for ip in &settings.split_tunnel_ips {
            entries.extend(ip_to_proxy_override(ip)?);
        }
    }

    entries.sort();
    entries.dedup();
    Ok(entries.join(";"))
}

fn pac_url_from_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file:///{normalized}")
}

fn write_proxy_pac(port: u16, settings: &Settings, pac_path: &Path) -> AppResult<()> {
    let script = build_proxy_pac_script(port, settings)?;
    fs::write(pac_path, script)
        .map_err(|error| AppError::proxy("Failed to write PAC file for system proxy mode", Some(error.to_string())))
}

fn build_proxy_pac_script(port: u16, settings: &Settings) -> AppResult<String> {
    let proxy = format!("PROXY 127.0.0.1:{port}");
    let mut rules = Vec::new();

    for domain in &settings.split_tunnel_domains {
        rules.extend(domain_to_pac_checks(domain)?);
    }
    for ip in &settings.split_tunnel_ips {
        rules.extend(ip_to_pac_checks(ip)?);
    }

    let rules_block = if rules.is_empty() {
        String::new()
    } else {
        format!("  if ({}) return \"{proxy}\";\n", rules.join(" ||\n      "))
    };

    Ok(format!(
        "function FindProxyForURL(url, host) {{\n  if (isPlainHostName(host) || dnsDomainIs(host, \".local\") || shExpMatch(host, \"localhost*\")) return \"DIRECT\";\n{rules_block}  return \"DIRECT\";\n}}\n"
    ))
}

fn domain_to_proxy_override(value: &str) -> AppResult<Vec<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(domain) = trimmed.strip_prefix("full:") {
        return Ok(vec![domain.trim().to_string()]);
    }
    if let Some(domain) = trimmed.strip_prefix("domain:") {
        return Ok(domain_patterns(domain.trim()));
    }
    if let Some(keyword) = trimmed.strip_prefix("keyword:") {
        let keyword = keyword.trim();
        return Ok(vec![format!("*{keyword}*")]);
    }
    if trimmed.starts_with("regexp:") || trimmed.starts_with("geosite:") || trimmed.starts_with("ext:") {
        return Err(AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy mode. Use plain domain, full: or keyword:.",
            trimmed
        )));
    }

    Ok(domain_patterns(trimmed))
}

fn domain_to_pac_checks(value: &str) -> AppResult<Vec<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(domain) = trimmed.strip_prefix("full:") {
        let domain = domain.trim();
        return Ok(vec![format!("host == \"{domain}\"")]);
    }
    if let Some(domain) = trimmed.strip_prefix("domain:") {
        return Ok(pac_domain_checks(domain.trim()));
    }
    if let Some(keyword) = trimmed.strip_prefix("keyword:") {
        let keyword = keyword.trim();
        return Ok(vec![
            format!("shExpMatch(host, \"*{keyword}*\")"),
            format!("shExpMatch(url, \"*{keyword}*\")"),
        ]);
    }
    if trimmed.starts_with("regexp:") || trimmed.starts_with("geosite:") || trimmed.starts_with("ext:") {
        return Err(AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy PAC mode. Use plain domain, full: or keyword:.",
            trimmed
        )));
    }

    Ok(pac_domain_checks(trimmed))
}

fn domain_patterns(domain: &str) -> Vec<String> {
    let normalized = domain.trim().trim_start_matches("*.").trim_matches('.');
    if normalized.is_empty() {
        return Vec::new();
    }

    vec![normalized.to_string(), format!("*.{normalized}")]
}

fn pac_domain_checks(domain: &str) -> Vec<String> {
    let normalized = domain.trim().trim_start_matches("*.").trim_matches('.');
    if normalized.is_empty() {
        return Vec::new();
    }

    vec![
        format!("host == \"{normalized}\""),
        format!("dnsDomainIs(host, \".{normalized}\")"),
    ]
}

fn ip_to_proxy_override(value: &str) -> AppResult<Vec<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(ip) = trimmed.parse::<Ipv4Addr>() {
        return Ok(vec![ip.to_string()]);
    }

    let Some((ip_text, prefix_text)) = trimmed.split_once('/') else {
        return Err(AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy mode. Use IPv4 address or CIDR.",
            trimmed
        )));
    };

    let ip = ip_text.parse::<Ipv4Addr>().map_err(|_| {
        AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy mode. Only IPv4 is supported.",
            trimmed
        ))
    })?;
    let prefix = prefix_text.parse::<u8>().map_err(|_| {
        AppError::validation(format!("Invalid CIDR prefix in '{}'.", trimmed))
    })?;

    match prefix {
        32 => Ok(vec![ip.to_string()]),
        24 => {
            let [a, b, c, _] = ip.octets();
            Ok(vec![format!("{a}.{b}.{c}.*")])
        }
        16 => {
            let [a, b, _, _] = ip.octets();
            Ok(vec![format!("{a}.{b}.*")])
        }
        8 => {
            let [a, _, _, _] = ip.octets();
            Ok(vec![format!("{a}.*")])
        }
        12 => {
            let [a, b, _, _] = ip.octets();
            if a != 172 || !(16..=31).contains(&b) {
                return Err(AppError::validation(format!(
                    "CIDR '{}' cannot be represented through System Proxy. Supported: /8, /16, /24, /32 and the local network 172.16.0.0/12.",
                    trimmed
                )));
            }
            Ok((16u8..=31u8).map(|octet| format!("172.{octet}.*")).collect())
        }
        _ => Err(AppError::validation(format!(
            "CIDR '{}' is not supported in System Proxy mode. Supported: /8, /12, /16, /24, /32.",
            trimmed
        ))),
    }
}

fn ip_to_pac_checks(value: &str) -> AppResult<Vec<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(ip) = trimmed.parse::<Ipv4Addr>() {
        return Ok(vec![format!("dnsResolve(host) == \"{ip}\"")]);
    }

    let Some((ip_text, prefix_text)) = trimmed.split_once('/') else {
        return Err(AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy PAC mode. Use IPv4 address or CIDR.",
            trimmed
        )));
    };

    let ip = ip_text.parse::<Ipv4Addr>().map_err(|_| {
        AppError::validation(format!(
            "Entry '{}' is not supported in System Proxy PAC mode. Only IPv4 is supported.",
            trimmed
        ))
    })?;
    let prefix = prefix_text.parse::<u8>().map_err(|_| {
        AppError::validation(format!("Invalid CIDR prefix in '{}'.", trimmed))
    })?;

    let mask = match prefix {
        32 => "255.255.255.255".to_string(),
        24 => "255.255.255.0".to_string(),
        16 => "255.255.0.0".to_string(),
        12 => "255.240.0.0".to_string(),
        8 => "255.0.0.0".to_string(),
        _ => {
            return Err(AppError::validation(format!(
                "CIDR '{}' is not supported in System Proxy PAC mode. Supported: /8, /12, /16, /24, /32.",
                trimmed
            )))
        }
    };

    Ok(vec![format!(
        "isInNet(dnsResolve(host), \"{}\", \"{}\")",
        ip, mask
    )])
}

fn refresh_internet_settings() -> AppResult<()> {
    unsafe {
        if InternetSetOptionW(null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, null_mut(), 0) == 0 {
            return Err(AppError::proxy(
                "Unable to notify Windows about Internet Settings changes",
                None,
            ));
        }
        if InternetSetOptionW(null_mut(), INTERNET_OPTION_REFRESH, null_mut(), 0) == 0 {
            return Err(AppError::proxy(
                "Unable to refresh Windows Internet Settings",
                None,
            ));
        }
    }
    Ok(())
}

fn reset_winhttp_proxy() -> AppResult<()> {
    let _ = run_netsh(["winhttp", "reset", "proxy"])?;
    Ok(())
}

fn apply_winhttp_dump(dump: &str) -> AppResult<()> {
    let script_path = write_netsh_script(dump)?;
    let script_path_string = script_path.to_string_lossy().to_string();
    let result = run_netsh(["-f", &script_path_string]);
    let _ = fs::remove_file(script_path);
    result.map(|_| ())
}

fn write_netsh_script(contents: &str) -> AppResult<PathBuf> {
    let path = std::env::temp_dir().join(format!("vailbox-winhttp-{}.txt", Uuid::new_v4()));
    fs::write(&path, contents)
        .map_err(|error| AppError::proxy("Failed to write temporary WinHTTP restore script", Some(error.to_string())))?;
    Ok(path)
}

fn run_netsh<'a>(args: impl IntoIterator<Item = &'a str>) -> AppResult<String> {
    let output = Command::new("netsh")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| AppError::proxy("Failed to launch netsh for Windows proxy configuration", Some(error.to_string())))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status: {}", output.status)
    };

    Err(AppError::proxy(
        "Windows WinHTTP proxy command failed",
        Some(details),
    ))
}
