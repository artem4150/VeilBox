use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use chrono::Utc;
use regex::Regex;
use tokio::net::{lookup_host, TcpStream};
use tokio::process::Command as TokioCommand;
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::{
    config_builder::build_xray_config,
    models::{ConnectionMode, Profile, ProfileEngine, ProfileLatency, ProfileLatencyStatus, ProxyProtocol, Settings},
};

const LATENCY_TIMEOUT: Duration = Duration::from_millis(1500);
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub async fn measure_profiles(profiles: Vec<Profile>, sidecar_path: PathBuf, config_path: PathBuf) -> Vec<ProfileLatency> {
    let mut results = Vec::with_capacity(profiles.len());
    let mut join_set = JoinSet::new();

    for profile in profiles {
        if join_set.len() >= 5 {
            if let Some(Ok(res)) = join_set.join_next().await {
                results.push(res);
            }
        }
        let sidecar_path = sidecar_path.clone();
        let config_path = config_path.clone();
        join_set.spawn(async move { measure_profile(profile, sidecar_path, config_path).await });
    }

    while let Some(Ok(res)) = join_set.join_next().await {
        results.push(res);
    }

    results
}

async fn measure_profile(profile: Profile, sidecar_path: PathBuf, config_path: PathBuf) -> ProfileLatency {
    if matches!(profile.engine, ProfileEngine::Amneziawg) {
        return measure_amnezia_profile(profile).await;
    }
    if matches!(profile.protocol, ProxyProtocol::Hysteria2) {
        return measure_hysteria_profile(profile, sidecar_path, config_path).await;
    }

    let checked_at = Utc::now();
    let address = format!("{}:{}", profile.server_address, profile.port);

    let resolved = match timeout(LATENCY_TIMEOUT, lookup_host(address)).await {
        Ok(Ok(iter)) => iter.collect::<Vec<_>>(),
        Ok(Err(error)) => {
            return ProfileLatency {
                profile_id: profile.id,
                latency_ms: None,
                status: ProfileLatencyStatus::Error,
                checked_at,
                message: Some(error.to_string()),
            }
        }
        Err(_) => {
            return ProfileLatency {
                profile_id: profile.id,
                latency_ms: None,
                status: ProfileLatencyStatus::Timeout,
                checked_at,
                message: Some("DNS resolution timed out".to_string()),
            }
        }
    };

    for socket in resolved {
        let started = Instant::now();
        match timeout(LATENCY_TIMEOUT, TcpStream::connect(socket)).await {
            Ok(Ok(_)) => {
                return ProfileLatency {
                    profile_id: profile.id,
                    latency_ms: Some(started.elapsed().as_millis()),
                    status: ProfileLatencyStatus::Ok,
                    checked_at,
                    message: None,
                }
            }
            Ok(Err(error)) => {
                return ProfileLatency {
                    profile_id: profile.id,
                    latency_ms: None,
                    status: ProfileLatencyStatus::Error,
                    checked_at,
                    message: Some(error.to_string()),
                }
            }
            Err(_) => continue,
        }
    }

    ProfileLatency {
        profile_id: profile.id,
        latency_ms: None,
        status: ProfileLatencyStatus::Timeout,
        checked_at,
        message: Some("TCP connect timed out".to_string()),
    }
}

// Hysteria2 uses QUIC/UDP. A TCP connect to its server port says nothing about
// whether the proxy works, so probe through a temporary, isolated Xray process.
async fn measure_hysteria_profile(profile: Profile, sidecar_path: PathBuf, config_path: PathBuf) -> ProfileLatency {
    let checked_at = Utc::now();
    let result = probe_hysteria(&profile, &sidecar_path, &config_path).await;
    match result {
        Ok(latency_ms) => ProfileLatency {
            profile_id: profile.id,
            latency_ms: Some(latency_ms),
            status: ProfileLatencyStatus::Ok,
            checked_at,
            message: None,
        },
        Err(message) => ProfileLatency {
            profile_id: profile.id,
            latency_ms: None,
            status: ProfileLatencyStatus::Error,
            checked_at,
            message: Some(message),
        },
    }
}

async fn probe_hysteria(profile: &Profile, sidecar_path: &PathBuf, config_path: &PathBuf) -> Result<u128, String> {
    if !sidecar_path.is_file() {
        return Err("xray.exe is unavailable for a Hysteria2 probe".to_string());
    }
    let socks_port = portpicker::pick_unused_port().ok_or("No free SOCKS port for probe")?;
    let http_port = (0..5)
        .filter_map(|_| portpicker::pick_unused_port())
        .find(|port| *port != socks_port)
        .ok_or("No free HTTP port for probe")?;
    let settings = Settings { connection_mode: ConnectionMode::SystemProxy, ..Settings::default() };
    let config = build_xray_config(profile, &settings, socks_port, http_port)
        .map_err(|error| error.message)?;
    let probe_path = config_path.with_file_name(format!("xray-latency-{}.json", uuid::Uuid::new_v4()));
    tokio::fs::write(&probe_path, config).await.map_err(|error| format!("Cannot write probe config: {error}"))?;

    let mut command = TokioCommand::new(sidecar_path);
    command.arg("run").arg("-c").arg(&probe_path)
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(directory) = sidecar_path.parent() {
        command.current_dir(directory);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = tokio::fs::remove_file(&probe_path).await;
            return Err(format!("Cannot start xray.exe for probe: {error}"));
        }
    };

    let result = async {
        let ready_by = Instant::now() + Duration::from_secs(4);
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!("Xray probe exited before its local port opened ({status})"));
            }
            if TcpStream::connect(("127.0.0.1", http_port)).await.is_ok() {
                break;
            }
            if Instant::now() >= ready_by {
                return Err("Xray probe local port did not become ready".to_string());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{http_port}"))
            .map_err(|error| error.to_string())?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(6))
            .build()
            .map_err(|error| error.to_string())?;
        for url in ["https://www.gstatic.com/generate_204", "https://cp.cloudflare.com/generate_204"] {
            let started = Instant::now();
            if client.get(url).send().await.is_ok() {
                // Even an HTTP 403 is an origin response: TLS and the Hysteria2
                // tunnel worked. This check measures connectivity, not site policy.
                return Ok(started.elapsed().as_millis());
            }
        }
        Err("Hysteria2 proxy did not complete either HTTPS connectivity probe".to_string())
    }.await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    let _ = tokio::fs::remove_file(&probe_path).await;
    result
}

async fn measure_amnezia_profile(profile: Profile) -> ProfileLatency {
    let checked_at = Utc::now();
    let host = profile.server_address.clone();
    let profile_id = profile.id.clone();

    let ping_result = tokio::task::spawn_blocking(move || {
        Command::new("ping.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-n", "1", "-w", "1500", &host])
            .output()
    })
    .await;

    let output = match ping_result {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return ProfileLatency {
                profile_id,
                latency_ms: None,
                status: ProfileLatencyStatus::Error,
                checked_at,
                message: Some(format!("Failed to run ping.exe: {}", error)),
            }
        }
        Err(error) => {
            return ProfileLatency {
                profile_id,
                latency_ms: None,
                status: ProfileLatencyStatus::Error,
                checked_at,
                message: Some(format!("Ping task failed: {}", error)),
            }
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        if let Some(latency_ms) = parse_ping_latency(&stdout) {
            return ProfileLatency {
                profile_id,
                latency_ms: Some(latency_ms),
                status: ProfileLatencyStatus::Ok,
                checked_at,
                message: None,
            };
        }

        return ProfileLatency {
            profile_id,
            latency_ms: Some(0),
            status: ProfileLatencyStatus::Ok,
            checked_at,
            message: Some("Ping succeeded but latency could not be parsed".to_string()),
        };
    }

    ProfileLatency {
        profile_id,
        latency_ms: None,
        status: ProfileLatencyStatus::Timeout,
        checked_at,
        message: Some(format!("Host ping failed: {}", stdout.trim())),
    }
}

fn parse_ping_latency(output: &str) -> Option<u128> {
    let regex = Regex::new(r"(?i)(?:time|время)\s*[=<]?\s*(\d+)\s*(?:ms|мс|мсек)").ok()?;
    regex
        .captures(output)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse::<u128>().ok())
}
