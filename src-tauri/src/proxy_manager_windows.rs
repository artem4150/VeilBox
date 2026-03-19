use std::{
    fs,
    net::Ipv4Addr,
    os::windows::process::CommandExt,
    path::PathBuf,
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
}

pub fn set_proxy(port: u16, settings_snapshot: &Settings) -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(
            INTERNET_SETTINGS,
            winreg::enums::KEY_SET_VALUE | winreg::enums::KEY_QUERY_VALUE,
        )
        .map_err(|error| {
            AppError::proxy(
                "Не удалось открыть настройки Windows Internet Settings",
                Some(error.to_string()),
            )
        })?;

    settings
        .set_value("ProxyEnable", &1u32)
        .map_err(|error| AppError::proxy("Не удалось включить системный proxy", Some(error.to_string())))?;
    settings
        .set_value(
            "ProxyServer",
            &format!("127.0.0.1:{port}"),
        )
        .map_err(|error| AppError::proxy("Не удалось записать адрес proxy", Some(error.to_string())))?;
    settings
        .set_value("ProxyOverride", &build_proxy_override(settings_snapshot)?)
        .map_err(|error| AppError::proxy("Не удалось записать список исключений proxy", Some(error.to_string())))?;

    refresh_internet_settings()
}

pub fn clear_proxy() -> AppResult<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey_with_flags(
            INTERNET_SETTINGS,
            winreg::enums::KEY_SET_VALUE | winreg::enums::KEY_QUERY_VALUE,
        )
        .map_err(|error| {
            AppError::proxy(
                "Не удалось открыть настройки Windows Internet Settings",
                Some(error.to_string()),
            )
        })?;

    settings
        .set_value("ProxyEnable", &0u32)
        .map_err(|error| AppError::proxy("Не удалось выключить системный proxy", Some(error.to_string())))?;
    let _ = settings.delete_value("ProxyServer");
    let _ = settings.delete_value("ProxyOverride");

    refresh_internet_settings()
}

pub fn capture_winhttp_dump() -> AppResult<String> {
    run_netsh(["winhttp", "dump"])
}

pub fn apply_winhttp_proxy(port: u16, settings_snapshot: &Settings) -> AppResult<()> {
    if !matches!(settings_snapshot.connection_mode, ConnectionMode::SystemProxy) {
        return Ok(());
    }

    let proxy_server = format!("127.0.0.1:{port}");
    let bypass_list = format!("bypass-list={}", build_proxy_override(settings_snapshot)?);

    let _ = run_netsh(["winhttp", "set", "proxy", &proxy_server, &bypass_list])?;
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
    Ok(state.enabled
        && state
            .server
            .unwrap_or_default()
            .contains(&format!("127.0.0.1:{port}")))
}

pub fn best_effort_cleanup(
    known_proxy_string: Option<String>,
    previous_winhttp_dump: Option<String>,
) -> AppResult<bool> {
    let mut cleaned = false;
    let state = get_proxy_state()?;
    if !state.enabled {
        // continue to WinHTTP cleanup
    } else {
        let server = state.server.unwrap_or_default();
        let looks_like_ours = known_proxy_string
            .map(|known| server == known)
            .unwrap_or_else(|| server.contains("127.0.0.1:"));

        if looks_like_ours {
            clear_proxy()?;
            cleaned = true;
        }
    }

    if let Ok(dump) = capture_winhttp_dump() {
        if dump.contains("127.0.0.1:") {
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
                "Не удалось прочитать состояние системного proxy",
                Some(error.to_string()),
            )
        })?;

    let enabled = settings.get_value::<u32, _>("ProxyEnable").unwrap_or_default() == 1;
    let server = settings.get_value::<String, _>("ProxyServer").ok();
    Ok(ProxyState {
        enabled,
        server,
    })
}

fn build_proxy_override(settings: &Settings) -> AppResult<String> {
    let mut entries = vec!["<local>".to_string()];

    if !matches!(settings.connection_mode, ConnectionMode::SystemProxy) {
        return Ok(entries.join(";"));
    }

    match settings.split_tunnel_mode {
        SplitTunnelMode::Disabled => {}
        SplitTunnelMode::BypassListed => {
            for domain in &settings.split_tunnel_domains {
                entries.extend(domain_to_proxy_override(domain)?);
            }
            for ip in &settings.split_tunnel_ips {
                entries.extend(ip_to_proxy_override(ip)?);
            }
        }
        SplitTunnelMode::ProxyListed => {
            return Err(AppError::validation(
                "В режиме системного proxy поддерживается только полный proxy или обходить список.",
            ));
        }
    }

    entries.sort();
    entries.dedup();
    Ok(entries.join(";"))
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
            "Запись '{}' не поддерживается в режиме системного proxy. Используй домен, full: или keyword:.",
            trimmed
        )));
    }

    Ok(domain_patterns(trimmed))
}

fn domain_patterns(domain: &str) -> Vec<String> {
    let normalized = domain.trim().trim_start_matches("*.").trim_matches('.');
    if normalized.is_empty() {
        return Vec::new();
    }

    vec![normalized.to_string(), format!("*.{normalized}")]
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
            "Запись '{}' не поддерживается в режиме системного proxy. Используй IPv4-адрес или CIDR.",
            trimmed
        )));
    };

    let ip = ip_text.parse::<Ipv4Addr>().map_err(|_| {
        AppError::validation(format!(
            "Запись '{}' не поддерживается в режиме системного proxy. Допустим только IPv4.",
            trimmed
        ))
    })?;
    let prefix = prefix_text.parse::<u8>().map_err(|_| {
        AppError::validation(format!(
            "Некорректный CIDR-префикс в записи '{}'.",
            trimmed
        ))
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
                    "CIDR '{}' не может быть представлен через системный proxy. Поддерживаются /8, /16, /24, /32 и локальная сеть 172.16.0.0/12.",
                    trimmed
                )));
            }
            Ok((16u8..=31u8).map(|octet| format!("172.{octet}.*")).collect())
        }
        _ => Err(AppError::validation(format!(
            "CIDR '{}' не поддерживается в режиме системного proxy. Поддерживаются только /8, /12, /16, /24 и /32.",
            trimmed
        ))),
    }
}

fn refresh_internet_settings() -> AppResult<()> {
    unsafe {
        if InternetSetOptionW(null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, null_mut(), 0) == 0 {
            return Err(AppError::proxy(
                "Не удалось уведомить Windows об изменении настроек интернета",
                None,
            ));
        }
        if InternetSetOptionW(null_mut(), INTERNET_OPTION_REFRESH, null_mut(), 0) == 0 {
            return Err(AppError::proxy(
                "Не удалось обновить настройки интернета в Windows",
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
