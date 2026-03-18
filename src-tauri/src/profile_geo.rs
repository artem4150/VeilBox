use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr},
    time::Duration,
};

use serde::Deserialize;

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
        .user_agent("VailBox/0.1")
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

    let mut cache: HashMap<String, Option<CountryInfo>> = HashMap::new();
    let mut result = Vec::with_capacity(profiles.len());

    for profile in profiles {
        let entry = if let Some(cached) = cache.get(&profile.server_address) {
            cached.clone()
        } else {
            let resolved = lookup_country(&client, &profile.server_address).await;
            cache.insert(profile.server_address.clone(), resolved.clone());
            resolved
        };

        result.push(ProfileCountry {
            profile_id: profile.id,
            country_code: entry.as_ref().map(|item| item.code.clone()),
            country_name: entry.map(|item| item.name),
        });
    }

    result
}

async fn lookup_country(client: &reqwest::Client, host: &str) -> Option<CountryInfo> {
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return None;
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        let local_or_reserved = match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_loopback()
                    || ipv4.is_private()
                    || ipv4.is_link_local()
                    || ipv4.is_multicast()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
                    || ipv6.is_multicast()
                    || ipv6.is_unspecified()
                    || ipv6.is_unique_local()
                    || ipv6.is_unicast_link_local()
                    || ipv6.segments()[0] & 0xffc0 == Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0).segments()[0]
            }
        };

        if local_or_reserved {
            return None;
        }
    }

    let response = client
        .get(format!("https://ipwho.is/{}", host))
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let payload: IpWhoIsResponse = response.json().await.ok()?;
    if !payload.success {
        return None;
    }

    let code = payload.country_code?.trim().to_uppercase();
    if code.len() != 2 {
        return None;
    }

    Some(CountryInfo {
        code,
        name: payload.country.unwrap_or_else(|| "Unknown".to_string()),
    })
}
