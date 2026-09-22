use serde::Serialize;
use std::collections::BTreeMap;

use crate::{
    error::{AppError, AppResult},
    models::{ConnectionMode, NetworkType, Profile, ProxyProtocol, SecurityType, Settings, SplitTunnelMode},
};

const TUN_MTU: u16 = 1500;
const TUN_IPV4_ADDRESS: &str = "172.19.0.1/30";
const TUN_IPV6_ADDRESS: &str = "fdfe:dcba:9876::1/126";
const LOCAL_BYPASS_IPV4_IPS: &[&str] = &[
    "127.0.0.0/8",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "169.254.0.0/16",
];
const LOCAL_BYPASS_IPV6_IPS: &[&str] = &["::1/128", "fc00::/7", "fe80::/10"];
const DEFAULT_DNS_SERVERS: &[&str] = &["1.1.1.1", "8.8.8.8"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct XrayConfig {
    log: LogSection,
    #[serde(skip_serializing_if = "Option::is_none")]
    dns: Option<DnsSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    routing: Option<RoutingSection>,
    inbounds: Vec<Inbound>,
    outbounds: Vec<Outbound>,
}

#[derive(Debug, Serialize)]
struct LogSection {
    loglevel: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DnsSection {
    servers: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutingSection {
    domain_strategy: &'static str,
    rules: Vec<RoutingRule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutingRule {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    inbound_tag: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ip: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process: Option<Vec<String>>,
    outbound_tag: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Inbound {
    tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    listen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    protocol: String,
    settings: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    sniffing: Option<Sniffing>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Sniffing {
    enabled: bool,
    dest_override: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route_only: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Outbound {
    tag: String,
    protocol: String,
    settings: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VlessSettings {
    vnext: Vec<Vnext>,
}

#[derive(Debug, Serialize)]
struct Vnext {
    address: String,
    port: u16,
    users: Vec<VlessUser>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VlessUser {
    id: String,
    encryption: &'static str,
    flow: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sockopt: Option<SockoptSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls_settings: Option<TlsSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reality_settings: Option<RealitySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ws_settings: Option<WsSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grpc_settings: Option<GrpcSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xhttp_settings: Option<XhttpSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    httpupgrade_settings: Option<HttpUpgradeSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kcp_settings: Option<KcpSettings>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SockoptSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    interface: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TlsSettings {
    server_name: Option<String>,
    allow_insecure: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    alpn: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RealitySettings {
    show: bool,
    server_name: String,
    fingerprint: String,
    public_key: String,
    short_id: String,
    spider_x: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsSettings {
    path: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrpcSettings {
    service_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct XhttpSettings {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HttpUpgradeSettings {
    host: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KcpSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<String>,
    header: KcpHeader,
}

#[derive(Debug, Serialize)]
struct KcpHeader {
    #[serde(rename = "type")]
    kind: String,
}

pub fn build_xray_config(
    profile: &Profile,
    settings: &Settings,
    socks_port: u16,
    http_port: u16,
) -> AppResult<String> {
    let tun_mode = matches!(settings.connection_mode, ConnectionMode::Tun);

    let mut inbounds = vec![
        Inbound {
            tag: "socks-in".to_string(),
            listen: Some("127.0.0.1".to_string()),
            port: Some(socks_port),
            protocol: "socks".to_string(),
            settings: serde_json::json!({ "udp": true }),
            sniffing: Some(default_sniffing(true)),
        },
        Inbound {
            tag: "http-in".to_string(),
            listen: Some("127.0.0.1".to_string()),
            port: Some(http_port),
            protocol: "http".to_string(),
            settings: serde_json::json!({}),
            sniffing: Some(default_sniffing(false)),
        },
    ];

    if tun_mode {
        let tun_addresses = tun_addresses(settings);
        inbounds.push(Inbound {
            tag: "tun-in".to_string(),
            listen: None,
            port: None,
            protocol: "tun".to_string(),
            settings: serde_json::json!({
              "name": settings.tun_interface_name,
              "gateway": tun_addresses,
              "dns": DEFAULT_DNS_SERVERS,
              "mtu": TUN_MTU,
              "autoSystemRoutingTable": if settings.tun_disable_ipv6 { vec!["0.0.0.0/0"] } else { vec!["0.0.0.0/0", "::/0"] },
              "autoOutboundsInterface": settings.tun_outbound_interface.as_deref().unwrap_or("auto"),
            }),
            sniffing: Some(default_sniffing(true)),
        });
    }

    let outbound_sockopt = outbound_sockopt(settings);
    let proxy_outbound = build_proxy_outbound(profile, outbound_sockopt.clone())?;

    let outbounds = vec![
        proxy_outbound,
        Outbound {
            tag: "direct".to_string(),
            protocol: "freedom".to_string(),
            settings: serde_json::json!({}),
            stream_settings: Some(serde_json::to_value(StreamSettings {
                network: None,
                security: None,
                sockopt: outbound_sockopt.clone(),
                tls_settings: None,
                reality_settings: None,
                ws_settings: None,
                grpc_settings: None,
                xhttp_settings: None,
                httpupgrade_settings: None,
                kcp_settings: None,
            })?),
        },
        Outbound {
            tag: "dns-out".to_string(),
            protocol: "dns".to_string(),
            settings: serde_json::json!({}),
            stream_settings: Some(serde_json::to_value(StreamSettings {
                network: None,
                security: None,
                sockopt: outbound_sockopt.clone(),
                tls_settings: None,
                reality_settings: None,
                ws_settings: None,
                grpc_settings: None,
                xhttp_settings: None,
                httpupgrade_settings: None,
                kcp_settings: None,
            })?),
        },
    ];

    let dns = Some(DnsSection {
        servers: DEFAULT_DNS_SERVERS.iter().map(|value| value.to_string()).collect(),
    });

    let routing = Some(if tun_mode {
        build_tun_routing(profile, settings)
    } else {
        build_system_proxy_routing()
    });

    let config = XrayConfig {
        log: LogSection { loglevel: "warning" },
        dns,
        routing,
        inbounds,
        outbounds,
    };

    Ok(serde_json::to_string_pretty(&config)?)
}

fn build_tun_routing(profile: &Profile, settings: &Settings) -> RoutingSection {
    let mut rules = Vec::new();

    if profile
        .server_address
        .chars()
        .all(|char| char.is_ascii_digit() || matches!(char, '.' | ':'))
    {
        rules.push(RoutingRule {
            kind: "field",
            inbound_tag: None,
            ip: Some(vec![profile.server_address.clone()]),
            domain: None,
            port: None,
            process: None,
            outbound_tag: "direct".to_string(),
        });
    } else {
        rules.push(RoutingRule {
            kind: "field",
            inbound_tag: None,
            ip: None,
            domain: Some(vec![format!("full:{}", profile.server_address)]),
            port: None,
            process: None,
            outbound_tag: "direct".to_string(),
        });
    }

    rules.push(RoutingRule {
        kind: "field",
        inbound_tag: Some(vec!["tun-in".to_string()]),
        ip: Some(local_bypass_ips(settings)),
        domain: None,
        port: None,
        process: None,
        outbound_tag: "direct".to_string(),
    });

    rules.push(RoutingRule {
        kind: "field",
        inbound_tag: Some(vec!["tun-in".to_string()]),
        ip: None,
        domain: None,
        port: Some("53".to_string()),
        process: None,
        outbound_tag: "dns-out".to_string(),
    });

    match settings.split_tunnel_mode {
        SplitTunnelMode::Disabled => {
            rules.push(rule_for_inbounds(vec!["tun-in"], "proxy"));
        }
        SplitTunnelMode::BypassListed => {
            if !settings.split_tunnel_processes.is_empty() {
                rules.push(process_rule(settings, "direct"));
            }
            if !settings.split_tunnel_domains.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: None,
                    domain: Some(normalize_domain_rules(&settings.split_tunnel_domains)),
                    port: None,
                    process: None,
                    outbound_tag: "direct".to_string(),
                });
            }
            if !settings.split_tunnel_ips.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: Some(settings.split_tunnel_ips.clone()),
                    domain: None,
                    port: None,
                    process: None,
                    outbound_tag: "direct".to_string(),
                });
            }
            rules.push(rule_for_inbounds(vec!["tun-in"], "proxy"));
        }
        SplitTunnelMode::ProxyListed => {
            if !settings.split_tunnel_processes.is_empty() {
                rules.push(process_rule(settings, "proxy"));
            }
            if !settings.split_tunnel_domains.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: None,
                    domain: Some(normalize_domain_rules(&settings.split_tunnel_domains)),
                    port: None,
                    process: None,
                    outbound_tag: "proxy".to_string(),
                });
            }
            if !settings.split_tunnel_ips.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: Some(settings.split_tunnel_ips.clone()),
                    domain: None,
                    port: None,
                    process: None,
                    outbound_tag: "proxy".to_string(),
                });
            }
            rules.push(rule_for_inbounds(vec!["tun-in"], "direct"));
        }
    }

    rules.push(rule_for_inbounds(vec!["http-in", "socks-in"], "proxy"));

    RoutingSection {
        domain_strategy: "IPOnDemand",
        rules,
    }
}

pub fn build_balanced_xray_config(
    primary: &Profile,
    candidates: &[Profile],
    settings: &Settings,
    socks_port: u16,
    http_port: u16,
    probe_port: u16,
    api_port: u16,
) -> AppResult<String> {
    let mut config: serde_json::Value = serde_json::from_str(&build_xray_config(primary, settings, socks_port, http_port)?)?;
    let mut count = 0usize;
    for candidate in std::iter::once(primary).chain(candidates.iter().filter(|p| p.id != primary.id)) {
        if !matches!(candidate.engine, crate::models::ProfileEngine::Xray) { continue; }
        // Reuse exactly the same outbound validation as a normal profile connection.
        let other_config = match build_xray_config(candidate, settings, socks_port, http_port) {
            Ok(config) => config,
            Err(_) if candidate.id != primary.id => continue,
            Err(error) => return Err(error),
        };
        let other: serde_json::Value = serde_json::from_str(&other_config)?;
        let mut outbound = other["outbounds"][0].clone();
        outbound["tag"] = format!("balanced-{count}").into();
        config["outbounds"].as_array_mut().unwrap().push(outbound);
        count += 1;
    }
    if count < 2 { return Ok(serde_json::to_string_pretty(&config)?); }
    config["inbounds"].as_array_mut().unwrap().push(serde_json::json!({
        "tag": "speed-probe-in", "listen": "127.0.0.1", "port": probe_port,
        "protocol": "http", "settings": {}
    }));
    config["api"] = serde_json::json!({
        "tag": "speed-api", "listen": format!("127.0.0.1:{api_port}"),
        "services": ["RoutingService"]
    });
    for rule in config["routing"]["rules"].as_array_mut().unwrap() {
        if rule["outboundTag"] == "proxy" {
            rule.as_object_mut().unwrap().remove("outboundTag");
            rule["balancerTag"] = "best-server".into();
        }
    }
    // Authentication/session traffic should keep one VPN egress IP even while
    // generic traffic is rebalanced. Only apply this in full-tunnel routing;
    // explicit split-tunnel rules must retain the user's chosen behavior.
    if matches!(settings.split_tunnel_mode, SplitTunnelMode::Disabled) {
        let inbound_tags = if matches!(settings.connection_mode, ConnectionMode::Tun) {
            vec!["tun-in", "http-in", "socks-in"]
        } else {
            vec!["http-in", "socks-in"]
        };
        config["routing"]["rules"].as_array_mut().unwrap().insert(0, serde_json::json!({
            "type": "field",
            "inboundTag": inbound_tags,
            "domain": [
                "domain:chatgpt.com",
                "domain:openai.com",
                "domain:oaistatic.com",
                "domain:oaiusercontent.com"
            ],
            "outboundTag": "proxy"
        }));
    }
    config["routing"]["rules"].as_array_mut().unwrap().insert(0, serde_json::json!({
        "type": "field", "inboundTag": ["speed-probe-in"],
        "balancerTag": "speed-probe"
    }));
    config["routing"]["balancers"] = serde_json::json!([
        {
            "tag": "best-server", "selector": ["balanced-"],
            "fallbackTag": "proxy", "strategy": { "type": "leastPing" }
        },
        {
            "tag": "speed-probe", "selector": ["balanced-"],
            "strategy": { "type": "roundRobin" }
        }
    ]);
    config["observatory"] = serde_json::json!({
        "subjectSelector": ["balanced-"],
        "probeUrl": "https://www.gstatic.com/generate_204",
        "probeInterval": "10s",
        "enableConcurrency": true
    });
    Ok(serde_json::to_string_pretty(&config)?)
}

fn build_proxy_outbound(profile: &Profile, sockopt: Option<SockoptSettings>) -> AppResult<Outbound> {
    let tag = "proxy".to_string();
    match profile.protocol {
        ProxyProtocol::Vless => Ok(Outbound {
            tag,
            protocol: "vless".to_string(),
            settings: serde_json::to_value(VlessSettings {
                vnext: vec![Vnext {
                    address: profile.server_address.clone(), port: profile.port,
                    users: vec![VlessUser { id: profile.uuid.clone(), encryption: "none", flow: profile.flow.clone() }],
                }],
            })?,
            stream_settings: Some(serde_json::to_value(build_stream_settings(profile, sockopt)?)?),
        }),
        ProxyProtocol::Shadowsocks => Ok(Outbound {
            tag,
            protocol: "shadowsocks".to_string(),
            settings: serde_json::json!({
                "address": profile.server_address, "port": profile.port,
                "method": required_profile_value(&profile.method, "Shadowsocks method")?,
                "password": required_profile_value(&profile.password, "Shadowsocks password")?,
            }),
            stream_settings: sockopt.map(|value| serde_json::json!({ "sockopt": value })),
        }),
        ProxyProtocol::Hysteria2 => {
            if !matches!(profile.security_type, SecurityType::Tls) {
                return Err(AppError::validation("Hysteria2 requires TLS"));
            }
            Ok(Outbound {
                tag,
                protocol: "hysteria".to_string(),
                settings: serde_json::json!({ "version": 2, "address": profile.server_address, "port": profile.port }),
                stream_settings: Some(serde_json::json!({
                    "method": "hysteria", "security": "tls",
                    "hysteriaSettings": { "version": 2, "auth": required_profile_value(&profile.password, "Hysteria2 auth")? },
                    "tlsSettings": {
                        "serverName": profile.sni.as_deref().unwrap_or(&profile.server_address),
                        "allowInsecure": profile.allow_insecure,
                        "alpn": profile.alpn,
                    },
                    "sockopt": sockopt,
                    "finalmask": profile.obfs_password.as_ref().map(|password| serde_json::json!({
                        "udp": [{ "type": "salamander", "settings": { "password": password } }]
                    })),
                })),
            })
        }
    }
}

fn required_profile_value<'a>(value: &'a Option<String>, label: &str) -> AppResult<&'a str> {
    value.as_deref().filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::validation(format!("{label} is required")))
}

fn build_system_proxy_routing() -> RoutingSection {
    RoutingSection {
        domain_strategy: "IPIfNonMatch",
        rules: vec![
            RoutingRule {
                kind: "field",
                inbound_tag: Some(vec!["http-in".to_string(), "socks-in".to_string()]),
                ip: None,
                domain: None,
                port: Some("53".to_string()),
                process: None,
                outbound_tag: "dns-out".to_string(),
            },
            rule_for_inbounds(vec!["http-in", "socks-in"], "proxy"),
        ],
    }
}

fn tun_addresses(settings: &Settings) -> Vec<&'static str> {
    let mut addresses = vec![TUN_IPV4_ADDRESS];
    if !settings.tun_disable_ipv6 {
        addresses.push(TUN_IPV6_ADDRESS);
    }
    addresses
}

fn local_bypass_ips(settings: &Settings) -> Vec<String> {
    let mut items = LOCAL_BYPASS_IPV4_IPS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();

    if !settings.tun_disable_ipv6 {
        items.extend(
            LOCAL_BYPASS_IPV6_IPS
                .iter()
                .map(|value| value.to_string()),
        );
    }

    items
}

fn rule_for_inbounds(inbounds: Vec<&str>, outbound_tag: &str) -> RoutingRule {
    RoutingRule {
        kind: "field",
        inbound_tag: Some(inbounds.into_iter().map(|value| value.to_string()).collect()),
        ip: None,
        domain: None,
        port: None,
        process: None,
        outbound_tag: outbound_tag.to_string(),
    }
}

fn process_rule(settings: &Settings, outbound_tag: &str) -> RoutingRule {
    RoutingRule {
        kind: "field",
        inbound_tag: Some(vec!["tun-in".to_string()]),
        ip: None,
        domain: None,
        port: None,
        process: Some(settings.split_tunnel_processes.clone()),
        outbound_tag: outbound_tag.to_string(),
    }
}

fn normalize_domain_rules(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| normalize_domain_rule(item))
        .collect()
}

fn normalize_domain_rule(item: &str) -> String {
    const PASSTHROUGH_PREFIXES: [&str; 6] = [
        "domain:",
        "full:",
        "keyword:",
        "regexp:",
        "geosite:",
        "ext:",
    ];

    if PASSTHROUGH_PREFIXES
        .iter()
        .any(|prefix| item.starts_with(prefix))
    {
        item.to_string()
    } else {
        format!("domain:{item}")
    }
}

fn default_sniffing(include_quic: bool) -> Sniffing {
    let mut dest_override = vec!["http".to_string(), "tls".to_string()];
    if include_quic {
        dest_override.push("quic".to_string());
    }
    Sniffing {
        enabled: true,
        dest_override,
        route_only: Some(true),
    }
}

fn outbound_sockopt(settings: &Settings) -> Option<SockoptSettings> {
    if !matches!(settings.connection_mode, ConnectionMode::Tun) {
        return None;
    }

    Some(SockoptSettings {
        interface: settings.tun_outbound_interface.clone(),
    })
}

fn build_stream_settings(
    profile: &Profile,
    sockopt: Option<SockoptSettings>,
) -> AppResult<StreamSettings> {
    match (&profile.network_type, &profile.security_type) {
        (NetworkType::Raw | NetworkType::Tcp, security) => Ok(StreamSettings {
            network: Some("tcp".to_string()),
            security: Some(security_name(security)),
            sockopt,
            tls_settings: tls_settings_for(profile, security),
            reality_settings: reality_settings_for(profile, security)?,
            ws_settings: None,
            grpc_settings: None,
            xhttp_settings: None,
            httpupgrade_settings: None,
            kcp_settings: None,
        }),
        (NetworkType::Ws, security) => {
            let mut headers = BTreeMap::new();
            if let Some(host) = profile.host_header.clone().filter(|value| !value.is_empty()) {
                headers.insert("Host".to_string(), host);
            }
            Ok(StreamSettings {
                network: Some("ws".to_string()),
                security: Some(security_name(security)),
                sockopt,
                tls_settings: tls_settings_for(profile, security),
                reality_settings: reality_settings_for(profile, security)?,
                ws_settings: Some(WsSettings {
                    path: profile.path.clone().unwrap_or_else(|| "/".to_string()),
                    headers,
                }),
                grpc_settings: None,
                xhttp_settings: None,
                httpupgrade_settings: None,
                kcp_settings: None,
            })
        }
        (NetworkType::Grpc, security) => Ok(StreamSettings {
            network: Some("grpc".to_string()),
            security: Some(security_name(security)),
            sockopt,
            tls_settings: tls_settings_for(profile, security),
            reality_settings: reality_settings_for(profile, security)?,
            ws_settings: None,
            grpc_settings: Some(GrpcSettings {
                service_name: profile
                    .service_name
                    .clone()
                    .ok_or_else(|| AppError::validation("gRPC service name is required"))?,
                authority: profile.host_header.clone(),
            }),
            xhttp_settings: None,
            httpupgrade_settings: None,
            kcp_settings: None,
        }),
        (NetworkType::Xhttp, security) => Ok(StreamSettings {
            network: Some("xhttp".to_string()),
            security: Some(security_name(security)),
            sockopt,
            tls_settings: tls_settings_for(profile, security),
            reality_settings: reality_settings_for(profile, security)?,
            ws_settings: None,
            grpc_settings: None,
            xhttp_settings: Some(XhttpSettings {
                path: profile.path.clone().unwrap_or_else(|| "/".to_string()),
                host: profile.host_header.clone(),
                mode: profile
                    .xhttp_mode
                    .clone()
                    .unwrap_or_else(|| "auto".to_string()),
            }),
            httpupgrade_settings: None,
            kcp_settings: None,
        }),
        (NetworkType::Httpupgrade, security) => Ok(StreamSettings {
            network: Some("httpupgrade".to_string()),
            security: Some(security_name(security)),
            sockopt,
            tls_settings: tls_settings_for(profile, security),
            reality_settings: reality_settings_for(profile, security)?,
            ws_settings: None,
            grpc_settings: None,
            xhttp_settings: None,
            httpupgrade_settings: Some(HttpUpgradeSettings {
                host: profile.host_header.clone().unwrap_or_default(),
                path: profile.path.clone().unwrap_or_else(|| "/".to_string()),
            }),
            kcp_settings: None,
        }),
        (NetworkType::Kcp, SecurityType::None | SecurityType::Tls) => Ok(StreamSettings {
            network: Some("kcp".to_string()),
            security: Some(security_name(&profile.security_type)),
            sockopt,
            tls_settings: tls_settings_for(profile, &profile.security_type),
            reality_settings: None,
            ws_settings: None,
            grpc_settings: None,
            xhttp_settings: None,
            httpupgrade_settings: None,
            kcp_settings: Some(KcpSettings {
                seed: profile.seed.clone(),
                header: KcpHeader {
                    kind: profile
                        .transport_header_type
                        .clone()
                        .unwrap_or_else(|| "none".to_string()),
                },
            }),
        }),
        _ => Err(AppError::validation(
            "Unsupported VLESS mode or security combination for this build",
        )),
    }
}

fn security_name(security: &SecurityType) -> String {
    match security {
        SecurityType::None => "none",
        SecurityType::Tls => "tls",
        SecurityType::Reality => "reality",
    }
    .to_string()
}

fn tls_settings_for(profile: &Profile, security: &SecurityType) -> Option<TlsSettings> {
    match security {
        SecurityType::Tls => Some(TlsSettings {
            server_name: profile.sni.clone(),
            allow_insecure: profile.allow_insecure,
            alpn: profile.alpn.clone(),
            fingerprint: profile.fingerprint.clone().or_else(|| Some("chrome".to_string())),
        }),
        _ => None,
    }
}

fn reality_settings_for(
    profile: &Profile,
    security: &SecurityType,
) -> AppResult<Option<RealitySettings>> {
    match security {
        SecurityType::Reality => Ok(Some(RealitySettings {
            show: false,
            server_name: profile
                .sni
                .clone()
                .ok_or_else(|| AppError::validation("Reality SNI is required"))?,
            fingerprint: profile
                .fingerprint
                .clone()
                .unwrap_or_else(|| "chrome".to_string()),
            public_key: profile
                .public_key
                .clone()
                .ok_or_else(|| AppError::validation("Reality public key is required"))?,
            short_id: profile.short_id.clone().unwrap_or_default(),
            spider_x: profile.spider_x.clone().unwrap_or_else(|| "/".to_string()),
        })),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ProfileEngine, ProfileSource};

    #[test]
    fn tun_geo_and_process_rules_are_accepted_by_bundled_xray() {
        let now = chrono::Utc::now();
        let profile = Profile {
            id: "test".into(), name: "test".into(), engine: ProfileEngine::Xray,
            protocol: ProxyProtocol::Vless, password: None, method: None, obfs_password: None,
            server_address: "example.com".into(), port: 443,
            uuid: "11111111-1111-4111-8111-111111111111".into(),
            network_type: NetworkType::Tcp, security_type: SecurityType::Tls,
            flow: None, sni: Some("example.com".into()), fingerprint: None,
            public_key: None, short_id: None, spider_x: None, path: None,
            host_header: None, service_name: None, xhttp_mode: None,
            transport_header_type: None, seed: None, alpn: vec![],
            allow_insecure: false, remark: None, source: ProfileSource::Manual,
            source_label: None, subscription_id: None, amnezia_config: None,
            created_at: now, updated_at: now,
        };
        let mut settings = Settings::default();
        settings.connection_mode = ConnectionMode::Tun;
        settings.split_tunnel_mode = SplitTunnelMode::ProxyListed;
        settings.split_tunnel_processes = vec!["telegram.exe".into()];
        settings.split_tunnel_ips = vec!["geoip:ru".into()];
        settings.split_tunnel_domains = vec!["geosite:telegram".into()];
        let config = build_xray_config(&profile, &settings, 19340, 19341).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed["inbounds"][2]["settings"]["autoOutboundsInterface"], "auto");
        assert_eq!(parsed["routing"]["rules"][3]["process"][0], "telegram.exe");

        let binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bin/xray.exe");
        if binary.exists() {
            let path = std::env::temp_dir().join(format!("veilbox-config-test-{}.json", std::process::id()));
            let mut alternate = profile.clone();
            alternate.id = "alternate".into();
            let balanced = build_balanced_xray_config(&profile, &[alternate.clone()], &settings, 19340, 19341, 19342, 19343).unwrap();
            let balanced_split: serde_json::Value = serde_json::from_str(&balanced).unwrap();
            assert!(!balanced_split["routing"]["rules"].as_array().unwrap().iter().any(|rule| {
                rule["domain"].as_array().is_some_and(|domains| domains.iter().any(|domain| domain == "domain:chatgpt.com"))
            }));
            let balanced_full = build_balanced_xray_config(&profile, &[alternate], &Settings::default(), 19340, 19341, 19342, 19343).unwrap();
            let balanced_full_json: serde_json::Value = serde_json::from_str(&balanced_full).unwrap();
            assert_eq!(balanced_full_json["routing"]["rules"][0]["balancerTag"], "speed-probe");
            assert_eq!(balanced_full_json["routing"]["rules"][1]["outboundTag"], "proxy");
            assert_eq!(balanced_full_json["routing"]["rules"][1]["domain"][0], "domain:chatgpt.com");
            assert!(balanced_full_json["routing"]["rules"].as_array().unwrap().iter().any(|rule| rule["balancerTag"] == "best-server"));
            assert_eq!(balanced_full_json["api"]["listen"], "127.0.0.1:19343");
            let mut shadowsocks = profile.clone();
            shadowsocks.protocol = ProxyProtocol::Shadowsocks;
            shadowsocks.uuid.clear();
            shadowsocks.method = Some("aes-256-gcm".into());
            shadowsocks.password = Some("secret".into());
            let mut hysteria = profile.clone();
            hysteria.protocol = ProxyProtocol::Hysteria2;
            hysteria.uuid.clear();
            hysteria.password = Some("secret".into());
            hysteria.obfs_password = Some("obfs-secret".into());
            let mut hysteria_plain = hysteria.clone();
            hysteria_plain.obfs_password = None;
            for candidate in [
                config, balanced, balanced_full,
                build_xray_config(&shadowsocks, &settings, 19340, 19341).unwrap(),
                build_xray_config(&hysteria, &settings, 19340, 19341).unwrap(),
                build_xray_config(&hysteria_plain, &settings, 19340, 19341).unwrap(),
            ] {
                std::fs::write(&path, candidate).unwrap();
                let output = std::process::Command::new(&binary).args(["run", "-test", "-c"])
                    .arg(&path).current_dir(path.parent().unwrap()).output().unwrap();
                assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            }
            let _ = std::fs::remove_file(path);
        }
    }
}
