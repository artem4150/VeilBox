export type ThemeMode = 'dark' | 'light' | 'system';
export type AppLanguage = 'ru' | 'en';
export type ConnectionMode = 'systemProxy' | 'tun';
export type SplitTunnelMode = 'disabled' | 'bypassListed' | 'proxyListed';
export type ProfileSource = 'manual' | 'subscription';
export type NetworkType = 'raw' | 'tcp' | 'ws' | 'grpc' | 'xhttp' | 'httpupgrade' | 'kcp';
export type SecurityType = 'none' | 'reality' | 'tls';
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';
export type LogSource = 'app' | 'connection' | 'xray-stdout' | 'xray-stderr';

export interface Profile {
  id: string;
  name: string;
  serverAddress: string;
  port: number;
  uuid: string;
  networkType: NetworkType;
  securityType: SecurityType;
  flow: string | null;
  sni: string | null;
  fingerprint: string | null;
  publicKey: string | null;
  shortId: string | null;
  spiderX: string | null;
  path: string | null;
  hostHeader: string | null;
  serviceName: string | null;
  xhttpMode: string | null;
  transportHeaderType: string | null;
  seed: string | null;
  alpn: string[];
  allowInsecure: boolean;
  remark: string | null;
  source: ProfileSource;
  sourceLabel: string | null;
  subscriptionId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Subscription {
  id: string;
  name: string;
  url: string;
  createdAt: string;
  updatedAt: string;
}

export interface Settings {
  launchAtStartup: boolean;
  minimizeToTray: boolean;
  autoReconnect: boolean;
  theme: ThemeMode;
  language: AppLanguage;
  debugLogging: boolean;
  connectionMode: ConnectionMode;
  tunInterfaceName: string;
  tunOutboundInterface: string | null;
  splitTunnelMode: SplitTunnelMode;
  splitTunnelDomains: string[];
  splitTunnelIps: string[];
  lastSelectedProfileId: string | null;
}

export interface ConnectionStatus {
  state: ConnectionState;
  activeProfileId: string | null;
  message: string | null;
  connectedAt: string | null;
  localHttpProxyPort: number | null;
  localSocksProxyPort: number | null;
  restartCount: number;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  source: LogSource;
  message: string;
}

export interface LogsResponse {
  app: LogEntry[];
  connection: LogEntry[];
}

export interface ProfileLatency {
  profileId: string;
  latencyMs: number | null;
  status: 'ok' | 'timeout' | 'error';
  checkedAt: string;
  message: string | null;
}

export interface ProfileCountry {
  profileId: string;
  countryCode: string | null;
  countryName: string | null;
}

export interface NetworkInterfaceInfo {
  name: string;
  status: string;
  description: string | null;
}

export interface AboutInfo {
  appVersion: string;
  tauriVersion: string;
  xrayVersion: string | null;
  platform: string;
}

export interface SubscriptionImportResult {
  subscription: Subscription;
  profiles: Profile[];
}

export interface TestConnectionResult {
  profileId: string;
  success: boolean;
  message: string;
  durationMs: number | null;
}

export interface BackendError {
  code: string;
  message: string;
  details?: string | null;
}

export interface ToastMessage {
  id: string;
  title: string;
  message: string;
  tone: 'info' | 'error' | 'success';
}

export interface ManualProfileDraft {
  name: string;
  serverAddress: string;
  port: number;
  uuid: string;
  networkType: NetworkType;
  securityType: SecurityType;
  flow: string;
  sni: string;
  fingerprint: string;
  publicKey: string;
  shortId: string;
  spiderX: string;
  path: string;
  hostHeader: string;
  serviceName: string;
  xhttpMode: string;
  transportHeaderType: string;
  seed: string;
  alpn: string[];
  allowInsecure: boolean;
  remark: string;
}
