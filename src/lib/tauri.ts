import { invoke } from '@tauri-apps/api/core';
import type {
  AboutInfo,
  ConnectionStatus,
  LogsResponse,
  NetworkInterfaceInfo,
  Profile,
  ProfileCountry,
  ProfileLatency,
  Settings,
  Subscription,
  SubscriptionImportResult,
  TestConnectionResult,
} from '../types';

export async function tauriInvoke<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args);
}

export const backend = {
  bootstrap: () =>
    tauriInvoke<{
      profiles: Profile[];
      subscriptions: Subscription[];
      settings: Settings;
      connectionStatus: ConnectionStatus;
      logs: LogsResponse;
      about: AboutInfo;
    }>('bootstrap'),
  listProfiles: () => tauriInvoke<Profile[]>('list_profiles'),
  saveProfile: (profile: Partial<Profile>) => tauriInvoke<Profile>('save_profile', { profile }),
  deleteProfile: (id: string) => tauriInvoke<void>('delete_profile', { id }),
  duplicateProfile: (id: string) => tauriInvoke<Profile>('duplicate_profile', { id }),
  importVlessUri: (uri: string) => tauriInvoke<Profile>('import_vless_uri', { uri }),
  importProfilesJson: (json: string) =>
    tauriInvoke<Profile[]>('import_profiles_json', { json }),
  importSubscription: (url: string) =>
    tauriInvoke<SubscriptionImportResult>('import_subscription', { url }),
  listSubscriptions: () => tauriInvoke<Subscription[]>('list_subscriptions'),
  refreshSubscription: (subscriptionId: string) =>
    tauriInvoke<SubscriptionImportResult>('refresh_subscription', { subscriptionId }),
  refreshAllSubscriptions: () =>
    tauriInvoke<SubscriptionImportResult[]>('refresh_all_subscriptions'),
  deleteSubscription: (subscriptionId: string) =>
    tauriInvoke<void>('delete_subscription', { subscriptionId }),
  updateSettings: (patch: Partial<Settings>) => tauriInvoke<Settings>('update_settings', { patch }),
  getSettings: () => tauriInvoke<Settings>('get_settings'),
  connect: (profileId: string) => tauriInvoke<ConnectionStatus>('connect', { profileId }),
  disconnect: () => tauriInvoke<ConnectionStatus>('disconnect'),
  connectionStatus: () => tauriInvoke<ConnectionStatus>('connection_status'),
  testProfileConnection: (profileId: string) =>
    tauriInvoke<TestConnectionResult>('test_profile_connection', { profileId }),
  getLogs: (mode: 'preview' | 'full') => tauriInvoke<LogsResponse>('get_logs', { mode }),
  clearLogs: () => tauriInvoke<void>('clear_logs'),
  getProfileLatencies: () => tauriInvoke<ProfileLatency[]>('get_profile_latencies'),
  getProfileCountries: () => tauriInvoke<ProfileCountry[]>('get_profile_countries'),
  listNetworkInterfaces: () =>
    tauriInvoke<NetworkInterfaceInfo[]>('list_network_interfaces'),
  getAbout: () => tauriInvoke<AboutInfo>('get_about_info'),
};
