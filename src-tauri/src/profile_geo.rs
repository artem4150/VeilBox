use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    time::Duration,
};

use serde::Deserialize;
use tokio::{net::lookup_host, task::JoinSet, time::timeout};

use crate::models::{Profile, ProfileCountry};

#[derive(Clone)]
struct CountryInfo {
    code: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct IpWhoIsResponse {
    success: bool,
    country_code: Option<String>,
    country: Option<String>,
}

pub async fn resolve_profile_countries(profiles: Vec<Profile>) -> Vec<ProfileCountry> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("VeilBox/0.1")
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return profiles
                .into_iter()
                .map(|profile| ProfileCountry {
                    profile_id: profile.id,
                    country_code: None,
                    country_name: None,
                })
                .collect()
        }
    };

    let hosts: HashSet<String> = profiles.iter().map(|profile| profile.server_address.clone()).collect();
    let mut cache: HashMap<String, Option<CountryInfo>> = HashMap::new();
    let mut tasks = JoinSet::new();
    for host in hosts {
        if tasks.len() >= 8 {
            if let Some(Ok((host, country))) = tasks.join_next().await {
                cache.insert(host, country);
            }
        }
        let client = client.clone();
        tasks.spawn(async move {
            let country = lookup_country(&client, &host).await;
            (host, country)
        });
    }
    while let Some(Ok((host, country))) = tasks.join_next().await {
        cache.insert(host, country);
    }

    profiles.into_iter().map(|profile| {
        let entry = cache.get(&profile.server_address).and_then(Option::as_ref);
        ProfileCountry {
            profile_id: profile.id,
            country_code: entry.map(|item| item.code.clone()),
            country_name: entry.map(|item| item.name.clone()),
        }
    }).collect()
}

async fn lookup_country(client: &reqwest::Client, host: &str) -> Option<CountryInfo> {
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return None;
    }

    let addresses: Vec<IpAddr> = if let Ok(ip) = host.parse() {
        vec![ip]
    } else {
        timeout(Duration::from_secs(4), lookup_host((host, 443)))
            .await.ok()?.ok()?
            .map(|socket| socket.ip())
            .collect()
    };

    for ip in addresses.into_iter().filter(|ip| is_public_ip(*ip)).take(3) {
        let response = match client.get(format!("https://ipwho.is/{ip}")).send().await {
            Ok(response) if response.status().is_success() => response,
            _ => continue,
        };
        let payload: IpWhoIsResponse = match response.json().await {
            Ok(payload) => payload,
            Err(_) => continue,
        };
        if !payload.success { continue; }
        let code = payload.country_code?.trim().to_uppercase();
        if code.len() == 2 {
            return Some(CountryInfo { code, name: payload.country.unwrap_or_else(|| "Unknown".to_string()) });
        }
    }
    None
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !(ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_multicast() || ip.is_unspecified()),
        IpAddr::V6(ip) => !(ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() || ip.is_unique_local() || ip.is_unicast_link_local()),
    }
}
