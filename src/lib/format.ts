import { t } from './i18n';
import type { AppLanguage, Profile, ProfileLatency } from '../types';

export function formatTimestamp(value: string | null, language: AppLanguage = 'ru') {
  if (!value) {
    return t(language, 'unavailable');
  }

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function maskSecret(value: string | null, visible = 4, language: AppLanguage = 'ru') {
  if (!value) {
    return t(language, 'notSet');
  }

  if (value.length <= visible * 2) {
    return `${value.slice(0, 2)}***`;
  }

  return `${value.slice(0, visible)}****${value.slice(-visible)}`;
}

export function profileSubtitle(profile: Profile) {
  return `${profile.serverAddress}:${profile.port} / ${profile.networkType.toUpperCase()} / ${profile.securityType.toUpperCase()}`;
}

export function statusLabel(state: string, language: AppLanguage = 'ru') {
  switch (state) {
    case 'connected':
      return t(language, 'connected');
    case 'connecting':
      return t(language, 'connecting');
    case 'error':
      return t(language, 'error');
    default:
      return t(language, 'disconnected');
  }
}

export function formatLatency(latency?: ProfileLatency | null, language: AppLanguage = 'ru') {
  if (!latency) {
    return t(language, 'checking');
  }

  switch (latency.status) {
    case 'ok':
      return `${latency.latencyMs ?? 0} ms`;
    case 'timeout':
      return t(language, 'timeout');
    default:
      return t(language, 'unavailable');
  }
}
