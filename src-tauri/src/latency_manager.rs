use std::{
    os::windows::process::CommandExt,
    process::Command,
    time::{Duration, Instant},
};

use chrono::Utc;
use regex::Regex;
use tokio::net::{lookup_host, TcpStream};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::models::{Profile, ProfileEngine, ProfileLatency, ProfileLatencyStatus};

const LATENCY_TIMEOUT: Duration = Duration::from_millis(1500);
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub async fn measure_profiles(profiles: Vec<Profile>) -> Vec<ProfileLatency> {
    let mut results = Vec::with_capacity(profiles.len());
    let mut join_set = JoinSet::new();

    for profile in profiles {
        if join_set.len() >= 5 {
            if let Some(Ok(res)) = join_set.join_next().await {
                results.push(res);
            }
        }
        join_set.spawn(async move { measure_profile(profile).await });
    }

    while let Some(Ok(res)) = join_set.join_next().await {
        results.push(res);
    }

    results
}

async fn measure_profile(profile: Profile) -> ProfileLatency {
    if matches!(profile.engine, ProfileEngine::Amneziawg) {
        return measure_amnezia_profile(profile).await;
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
