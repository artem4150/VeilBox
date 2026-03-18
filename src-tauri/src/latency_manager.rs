use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::net::{lookup_host, TcpStream};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::models::{Profile, ProfileLatency, ProfileLatencyStatus};

const LATENCY_TIMEOUT: Duration = Duration::from_millis(1500);

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
