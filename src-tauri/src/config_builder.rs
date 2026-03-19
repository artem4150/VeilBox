use serde::Serialize;
use std::collections::BTreeMap;

use crate::{
    error::{AppError, AppResult},
    models::{ConnectionMode, NetworkType, Profile, SecurityType, Settings, SplitTunnelMode},
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
    stream_settings: Option<StreamSettings>,
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
              "address": tun_addresses,
              "mtu": TUN_MTU,
              "stack": "system",
              "autoRoute": true,
              "strictRoute": true,
              "sniff": true,
            }),
            sniffing: Some(default_sniffing(true)),
        });
    }

    let outbound_sockopt = outbound_sockopt(settings);
    let proxy_outbound = Outbound {
        tag: "proxy".to_string(),
        protocol: "vless".to_string(),
        settings: serde_json::to_value(VlessSettings {
            vnext: vec![Vnext {
                address: profile.server_address.clone(),
                port: profile.port,
                users: vec![VlessUser {
                    id: profile.uuid.clone(),
                    encryption: "none",
                    flow: profile.flow.clone(),
                }],
            }],
        })?,
        stream_settings: Some(build_stream_settings(profile, outbound_sockopt.clone())?),
    };

    let outbounds = vec![
        proxy_outbound,
        Outbound {
            tag: "direct".to_string(),
            protocol: "freedom".to_string(),
            settings: serde_json::json!({}),
            stream_settings: Some(StreamSettings {
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
            }),
        },
        Outbound {
            tag: "dns-out".to_string(),
            protocol: "dns".to_string(),
            settings: serde_json::json!({}),
            stream_settings: Some(StreamSettings {
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
            }),
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
            outbound_tag: "direct".to_string(),
        });
    } else {
        rules.push(RoutingRule {
            kind: "field",
            inbound_tag: None,
            ip: None,
            domain: Some(vec![format!("full:{}", profile.server_address)]),
            port: None,
            outbound_tag: "direct".to_string(),
        });
    }

    rules.push(RoutingRule {
        kind: "field",
        inbound_tag: Some(vec!["tun-in".to_string()]),
        ip: Some(local_bypass_ips(settings)),
        domain: None,
        port: None,
        outbound_tag: "direct".to_string(),
    });

    rules.push(RoutingRule {
        kind: "field",
        inbound_tag: Some(vec!["tun-in".to_string()]),
        ip: None,
        domain: None,
        port: Some("53".to_string()),
        outbound_tag: "dns-out".to_string(),
    });

    match settings.split_tunnel_mode {
        SplitTunnelMode::Disabled => {
            rules.push(rule_for_inbounds(vec!["tun-in"], "proxy"));
        }
        SplitTunnelMode::BypassListed => {
            if !settings.split_tunnel_domains.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: None,
                    domain: Some(normalize_domain_rules(&settings.split_tunnel_domains)),
                    port: None,
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
                    outbound_tag: "direct".to_string(),
                });
            }
            rules.push(rule_for_inbounds(vec!["tun-in"], "proxy"));
        }
        SplitTunnelMode::ProxyListed => {
            if !settings.split_tunnel_domains.is_empty() {
                rules.push(RoutingRule {
                    kind: "field",
                    inbound_tag: Some(vec!["tun-in".to_string()]),
                    ip: None,
                    domain: Some(normalize_domain_rules(&settings.split_tunnel_domains)),
                    port: None,
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
                    outbound_tag: "proxy".to_string(),
                });
            }
            rules.push(rule_for_inbounds(vec!["tun-in"], "direct"));
        }
    }

    rules.push(rule_for_inbounds(vec!["http-in", "socks-in"], "proxy"));

    RoutingSection {
        domain_strategy: "IPIfNonMatch",
        rules,
    }
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
