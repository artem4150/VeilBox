use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Raw,
    Tcp,
    Ws,
    Grpc,
    Xhttp,
    Httpupgrade,
    Kcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecurityType {
    None,
    Reality,
    Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProfileSource {
    #[default]
    Manual,
    Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub server_address: String,
    pub port: u16,
    pub uuid: String,
    pub network_type: NetworkType,
    pub security_type: SecurityType,
    pub flow: Option<String>,
    pub sni: Option<String>,
    pub fingerprint: Option<String>,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
    pub spider_x: Option<String>,
    pub path: Option<String>,
    pub host_header: Option<String>,
    pub service_name: Option<String>,
    pub xhttp_mode: Option<String>,
    pub transport_header_type: Option<String>,
    pub seed: Option<String>,
    pub alpn: Vec<String>,
    pub allow_insecure: bool,
    pub remark: Option<String>,
    #[serde(default)]
    pub source: ProfileSource,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub subscription_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub server_address: String,
    pub port: u16,
    pub uuid: String,
    pub network_type: NetworkType,
    pub security_type: SecurityType,
    pub flow: Option<String>,
    pub sni: Option<String>,
    pub fingerprint: Option<String>,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
    pub spider_x: Option<String>,
    pub path: Option<String>,
    pub host_header: Option<String>,
    pub service_name: Option<String>,
    pub xhttp_mode: Option<String>,
    pub transport_header_type: Option<String>,
    pub seed: Option<String>,
    #[serde(default)]
    pub alpn: Vec<String>,
    #[serde(default)]
    pub allow_insecure: bool,
    pub remark: Option<String>,
    #[serde(default)]
    pub source: Option<ProfileSource>,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub subscription_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub launch_at_startup: bool,
    pub minimize_to_tray: bool,
    pub auto_reconnect: bool,
    pub theme: ThemeMode,
    pub language: AppLanguage,
    pub debug_logging: bool,
    pub connection_mode: ConnectionMode,
    pub tun_interface_name: String,
    #[serde(default = "default_true")]
    pub tun_disable_ipv6: bool,
    pub tun_outbound_interface: Option<String>,
    pub split_tunnel_mode: SplitTunnelMode,
    pub split_tunnel_domains: Vec<String>,
    pub split_tunnel_ips: Vec<String>,
    pub last_selected_profile_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            auto_reconnect: true,
            theme: ThemeMode::Light,
            language: AppLanguage::En,
            debug_logging: false,
            connection_mode: ConnectionMode::SystemProxy,
            tun_interface_name: "xray0".to_string(),
            tun_disable_ipv6: true,
            tun_outbound_interface: None,
            split_tunnel_mode: SplitTunnelMode::Disabled,
            split_tunnel_domains: Vec::new(),
            split_tunnel_ips: Vec::new(),
            last_selected_profile_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub launch_at_startup: Option<bool>,
    pub minimize_to_tray: Option<bool>,
    pub auto_reconnect: Option<bool>,
    pub theme: Option<ThemeMode>,
    pub language: Option<AppLanguage>,
    pub debug_logging: Option<bool>,
    pub connection_mode: Option<ConnectionMode>,
    pub tun_interface_name: Option<String>,
    pub tun_disable_ipv6: Option<bool>,
    pub tun_outbound_interface: Option<Option<String>>,
    pub split_tunnel_mode: Option<SplitTunnelMode>,
    pub split_tunnel_domains: Option<Vec<String>>,
    pub split_tunnel_ips: Option<Vec<String>>,
    pub last_selected_profile_id: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionMode {
    SystemProxy,
    Tun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SplitTunnelMode {
    #[default]
    Disabled,
    BypassListed,
    ProxyListed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppLanguage {
    Ru,
    En,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusPayload {
    pub state: ConnectionState,
    pub active_profile_id: Option<String>,
    pub message: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
    pub local_http_proxy_port: Option<u16>,
    pub local_socks_proxy_port: Option<u16>,
    pub restart_count: u32,
}

impl Default for ConnectionStatusPayload {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            active_profile_id: None,
            message: None,
            connected_at: None,
            local_http_proxy_port: None,
            local_socks_proxy_port: None,
            restart_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: LogSource,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogSource {
    App,
    Connection,
    XrayStdout,
    XrayStderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsResponse {
    pub app: Vec<LogEntry>,
    pub connection: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLatency {
    pub profile_id: String,
    pub latency_ms: Option<u128>,
    pub status: ProfileLatencyStatus,
    pub checked_at: DateTime<Utc>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCountry {
    pub profile_id: String,
    pub country_code: Option<String>,
    pub country_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub status: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileLatencyStatus {
    Ok,
    Timeout,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub app_version: String,
    pub tauri_version: String,
    pub xray_version: Option<String>,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionImportResult {
    pub subscription: Subscription,
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResult {
    pub profile_id: String,
    pub success: bool,
    pub message: String,
    pub duration_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResponse {
    pub profiles: Vec<Profile>,
    pub subscriptions: Vec<Subscription>,
    pub settings: Settings,
    pub connection_status: ConnectionStatusPayload,
    pub logs: LogsResponse,
    pub about: AboutInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionState {
    pub was_connected: bool,
    pub last_profile_id: Option<String>,
    pub last_http_proxy_port: Option<u16>,
    pub last_socks_proxy_port: Option<u16>,
    pub last_proxy_string: Option<String>,
    pub last_winhttp_dump: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum LogReadMode {
    Preview,
    Full,
}

impl LogReadMode {
    pub fn from_str(value: &str) -> Self {
        match value {
            "full" => Self::Full,
            _ => Self::Preview,
        }
    }

    pub fn limit(self) -> usize {
        match self {
            Self::Preview => 60,
            Self::Full => 5000,
        }
    }
}
