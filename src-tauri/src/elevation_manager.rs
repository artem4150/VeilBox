use std::{env, process::Command};

use crate::{
    error::{AppError, AppResult},
    models::ConnectionMode,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const ELEVATED_CONNECT_ARG: &str = "--elevated-connect-profile";

pub fn pending_elevated_connect_profile() -> Option<String> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == ELEVATED_CONNECT_ARG {
            return args.next();
        }
    }
    None
}

pub fn relaunch_as_administrator_for_tun(profile_id: &str) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = profile_id;
        Err(AppError::process(
            "Automatic elevation is only available on Windows.",
            None,
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let exe = env::current_exe().map_err(|error| {
            AppError::process(
                "Failed to resolve current executable for elevation.",
                Some(error.to_string()),
            )
        })?;

        let exe_escaped = exe.to_string_lossy().replace('\'', "''");
        let profile_id_escaped = profile_id.replace('\'', "''");
        let script = format!(
            concat!(
                "Start-Sleep -Milliseconds 650; ",
                "Start-Process -FilePath '{exe}' -Verb RunAs -ArgumentList @('{arg}','{profile}')"
            ),
            exe = exe_escaped,
            arg = ELEVATED_CONNECT_ARG,
            profile = profile_id_escaped
        );

        Command::new("powershell.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ])
            .spawn()
            .map_err(|error| {
                AppError::process(
                    "Failed to request administrator relaunch for TUN mode.",
                    Some(error.to_string()),
                )
            })?;

        Ok(())
    }
}

pub fn should_auto_elevate(connection_mode: &ConnectionMode, elevated: bool) -> bool {
    matches!(connection_mode, ConnectionMode::Tun) && !elevated
}
