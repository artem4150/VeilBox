use std::{
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DefaultRouteInfo {
    pub interface_alias: String,
    pub next_hop: String,
}

pub fn discover_primary_ipv4_route(exclude_alias: Option<&str>) -> AppResult<DefaultRouteInfo> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = exclude_alias;
        return Err(AppError::process(
            "TUN route management is only available on Windows.",
            None,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let exclude = exclude_alias.unwrap_or_default().replace('\'', "''");
        let script = format!(
            concat!(
                "$ErrorActionPreference = 'Stop'; ",
                "$route = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' | ",
                "Where-Object {{ $_.NextHop -ne '0.0.0.0' -and $_.InterfaceAlias -ne '{exclude}' }} | ",
                "ForEach-Object {{ ",
                "  $iface = Get-NetIPInterface -AddressFamily IPv4 -InterfaceIndex $_.InterfaceIndex -ErrorAction SilentlyContinue; ",
                "  [PSCustomObject]@{{ ",
                "    InterfaceAlias = $_.InterfaceAlias; ",
                "    NextHop = $_.NextHop; ",
                "    Score = ($_.RouteMetric + $(if ($iface) {{ $iface.InterfaceMetric }} else {{ 0 }})) ",
                "  }} ",
                "}} | Sort-Object Score | Select-Object -First 1; ",
                "if (-not $route) {{ throw 'No active IPv4 default route found.' }}; ",
                "$route | Select-Object InterfaceAlias, NextHop | ConvertTo-Json -Compress"
            ),
            exclude = exclude
        );

        let stdout = run_powershell(&script)?;
        serde_json::from_str::<DefaultRouteInfo>(stdout.trim()).map_err(|error| {
            AppError::process(
                "Failed to parse current Windows default route.",
                Some(error.to_string()),
            )
        })
    }
}

pub fn wait_for_interface(alias: &str, timeout_ms: u64) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (alias, timeout_ms);
        return Err(AppError::process(
            "TUN route management is only available on Windows.",
            None,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let started = Instant::now();
        let escaped = alias.replace('\'', "''");
        let script = format!(
            concat!(
                "$iface = Get-NetIPInterface -AddressFamily IPv4 -InterfaceAlias '{alias}' -ErrorAction SilentlyContinue; ",
                "if ($iface) {{ 'ready' }}"
            ),
            alias = escaped
        );

        while started.elapsed() < Duration::from_millis(timeout_ms) {
            if run_powershell(&script).map(|value| value.trim() == "ready").unwrap_or(false) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(250));
        }

        Err(AppError::process(
            "Timed out waiting for the TUN interface to appear in Windows.",
            Some(alias.to_string()),
        ))
    }
}

pub fn enable_full_tunnel(alias: &str) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = alias;
        return Err(AppError::process(
            "TUN route management is only available on Windows.",
            None,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = alias.replace('\'', "''");
        let script = format!(
            concat!(
                "$ErrorActionPreference = 'Stop'; ",
                "Set-NetIPInterface -AddressFamily IPv4 -InterfaceAlias '{alias}' -AutomaticMetric Disabled -InterfaceMetric 5; ",
                "Get-NetRoute -AddressFamily IPv4 -InterfaceAlias '{alias}' -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | ",
                "Remove-NetRoute -Confirm:$false -ErrorAction SilentlyContinue; ",
                "New-NetRoute -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' -InterfaceAlias '{alias}' -NextHop '0.0.0.0' -RouteMetric 5 -PolicyStore ActiveStore | Out-Null; ",
                "'ok'"
            ),
            alias = escaped
        );
        let _ = run_powershell(&script)?;
        Ok(())
    }
}

pub fn disable_full_tunnel(alias: &str) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = alias;
        return Err(AppError::process(
            "TUN route management is only available on Windows.",
            None,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let escaped = alias.replace('\'', "''");
        let script = format!(
            concat!(
                "Get-NetRoute -AddressFamily IPv4 -InterfaceAlias '{alias}' -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | ",
                "Remove-NetRoute -Confirm:$false -ErrorAction SilentlyContinue; ",
                "'ok'"
            ),
            alias = escaped
        );
        let _ = run_powershell(&script)?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> AppResult<String> {
    let mut command = std::process::Command::new("powershell.exe");
    command
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().map_err(|error| {
        AppError::process(
            "Failed to launch PowerShell for Windows route configuration.",
            Some(error.to_string()),
        )
    })?;

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

    Err(AppError::process(
        "Windows TUN route configuration failed.",
        Some(details),
    ))
}
