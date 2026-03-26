import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { create } from 'zustand';
import { backend } from '../lib/tauri';
import type {
  AboutInfo,
  BackendError,
  ConnectionStatus,
  LogsResponse,
  Profile,
  ProfileCountry,
  ProfileLatency,
  Settings,
  Subscription,
  TestConnectionResult,
  ToastMessage,
} from '../types';

interface AppStore {
  initialized: boolean;
  loading: boolean;
  profiles: Profile[];
  subscriptions: Subscription[];
  selectedProfileId: string | null;
  connectionStatus: ConnectionStatus;
  logs: LogsResponse;
  latencies: Record<string, ProfileLatency>;
  profileCountries: Record<string, ProfileCountry>;
  settings: Settings;
  about: AboutInfo | null;
  toasts: ToastMessage[];
  listenersAttached: boolean;
  initialize: () => Promise<void>;
  refreshProfiles: () => Promise<void>;
  refreshSubscriptions: () => Promise<void>;
  refreshConnectionStatus: () => Promise<void>;
  refreshLogs: (mode: 'preview' | 'full') => Promise<void>;
  refreshLatencies: () => Promise<void>;
  refreshProfileCountries: () => Promise<void>;
  saveProfile: (profile: Partial<Profile>) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  duplicateProfile: (id: string) => Promise<void>;
  importProfile: (uri: string) => Promise<void>;
  importProfilesJson: (json: string) => Promise<void>;
  importSubscription: (url: string) => Promise<void>;
  refreshSubscription: (subscriptionId: string) => Promise<void>;
  refreshAllSubscriptions: () => Promise<void>;
  deleteSubscription: (subscriptionId: string) => Promise<void>;
  selectProfile: (profileId: string | null) => Promise<void>;
  saveSettings: (patch: Partial<Settings>) => Promise<void>;
  connect: (profileId: string) => Promise<void>;
  disconnect: () => Promise<void>;
  testProfileConnection: (profileId: string) => Promise<TestConnectionResult | null>;
  clearLogs: () => Promise<void>;
  pushToast: (toast: Omit<ToastMessage, 'id'>) => void;
  dismissToast: (id: string) => void;
}

const defaultConnection: ConnectionStatus = {
  state: 'disconnected',
  activeProfileId: null,
  message: null,
  connectedAt: null,
  localHttpProxyPort: null,
  localSocksProxyPort: null,
  restartCount: 0,
};

const defaultSettings: Settings = {
  launchAtStartup: false,
  minimizeToTray: true,
  autoReconnect: true,
  theme: 'light',
  language: 'en',
  debugLogging: false,
  connectionMode: 'systemProxy',
  tunInterfaceName: 'xray0',
  tunDisableIpv6: true,
  tunOutboundInterface: null,
  splitTunnelMode: 'disabled',
  splitTunnelDomains: [],
  splitTunnelIps: [],
  lastSelectedProfileId: null,
};

const defaultLogs: LogsResponse = {
  app: [],
  connection: [],
};

let cleanupListeners: UnlistenFn[] = [];
const toastTimers = new Map<string, ReturnType<typeof setTimeout>>();

function nextToastId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function mapError(error: unknown): BackendError {
  if (typeof error === 'object' && error && 'message' in error) {
    const record = error as Record<string, unknown>;
    return {
      code: typeof record.code === 'string' ? record.code : 'UNKNOWN',
      message: typeof record.message === 'string' ? record.message : 'Unexpected backend error',
      details: typeof record.details === 'string' ? record.details : null,
    };
  }

  return {
    code: 'UNKNOWN',
    message: error instanceof Error ? error.message : 'Unexpected backend error',
    details: null,
  };
}

export const useAppStore = create<AppStore>((set, get) => ({
  initialized: false,
  loading: false,
  profiles: [],
  subscriptions: [],
  selectedProfileId: null,
  connectionStatus: defaultConnection,
  logs: defaultLogs,
  latencies: {},
  profileCountries: {},
  settings: defaultSettings,
  about: null,
  toasts: [],
  listenersAttached: false,

  pushToast: (toast) => {
    const id = nextToastId();
    const timeoutMs = toast.tone === 'error' ? 7000 : 4500;

    set((state) => ({
      toasts: [...state.toasts, { ...toast, id }],
    }));

    const timer = setTimeout(() => {
      toastTimers.delete(id);
      get().dismissToast(id);
    }, timeoutMs);
    toastTimers.set(id, timer);
  },

  dismissToast: (id) => {
    const timer = toastTimers.get(id);
    if (timer) {
      clearTimeout(timer);
      toastTimers.delete(id);
    }
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    }));
  },

  initialize: async () => {
    if (get().initialized) {
      return;
    }

    set({ loading: true });

    try {
      const [bootstrap, startupEnabled] = await Promise.all([backend.bootstrap(), isEnabled()]);
      const selectedProfileId =
        bootstrap.settings.lastSelectedProfileId ??
        bootstrap.connectionStatus.activeProfileId ??
        bootstrap.profiles[0]?.id ??
        null;

      set({
        initialized: true,
        loading: false,
        profiles: bootstrap.profiles,
        subscriptions: bootstrap.subscriptions,
        selectedProfileId,
        settings: {
          ...bootstrap.settings,
          launchAtStartup: startupEnabled,
        },
        connectionStatus: bootstrap.connectionStatus,
        logs: bootstrap.logs,
        about: bootstrap.about,
      });
      void get().refreshLatencies();
      void get().refreshProfileCountries();

      if (!get().listenersAttached) {
        const connectionListener = await listen<ConnectionStatus>(
          'connection-status-changed',
          (event) => {
            set({ connectionStatus: event.payload });
          },
        );

        const errorListener = await listen<BackendError>('backend-error', (event) => {
          get().pushToast({
            title: 'Backend error',
            message: event.payload.message,
            tone: 'error',
          });
        });

        cleanupListeners = [connectionListener, errorListener];
        set({ listenersAttached: true });
      }
    } catch (error) {
      const payload = mapError(error);
      set({ loading: false });
      get().pushToast({
        title: 'Ошибка инициализации',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  refreshProfiles: async () => {
    const profiles = await backend.listProfiles();
    set((state) => ({
      profiles,
      selectedProfileId:
        profiles.some((profile) => profile.id === state.selectedProfileId)
          ? state.selectedProfileId
          : profiles[0]?.id ?? null,
    }));
    void get().refreshProfileCountries();
  },

  refreshSubscriptions: async () => {
    const subscriptions = await backend.listSubscriptions();
    set({ subscriptions });
  },

  refreshConnectionStatus: async () => {
    const connectionStatus = await backend.connectionStatus();
    set({ connectionStatus });
  },

  refreshLogs: async (mode) => {
    const logs = await backend.getLogs(mode);
    set({ logs });
  },

  refreshLatencies: async () => {
    try {
      const latencies = await backend.getProfileLatencies();
      set({
        latencies: Object.fromEntries(latencies.map((entry) => [entry.profileId, entry])),
      });
    } catch {
      // keep UI functional if latency probing fails
    }
  },

  refreshProfileCountries: async () => {
    try {
      const countries = await backend.getProfileCountries();
      set({
        profileCountries: Object.fromEntries(
          countries.map((entry) => [entry.profileId, entry]),
        ),
      });
    } catch {
      // country lookup is best effort only
    }
  },

  saveProfile: async (profile) => {
    try {
      const savedProfile = await backend.saveProfile(profile);
      await get().refreshProfiles();
      await get().selectProfile(savedProfile.id);
      get().pushToast({
        title: 'Профиль сохранен',
        message: `${savedProfile.name} сохранен локально.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось сохранить профиль',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  deleteProfile: async (id) => {
    try {
      await backend.deleteProfile(id);
      await get().refreshProfiles();
      get().pushToast({
        title: 'Профиль удален',
        message: 'Профиль удален из локального хранилища.',
        tone: 'info',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось удалить профиль',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  duplicateProfile: async (id) => {
    try {
      const duplicated = await backend.duplicateProfile(id);
      await get().refreshProfiles();
      await get().selectProfile(duplicated.id);
      get().pushToast({
        title: 'Профиль дублирован',
        message: `${duplicated.name} успешно создан.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось дублировать профиль',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  importProfile: async (uri) => {
    try {
      const profile = await backend.importVlessUri(uri);
      await get().refreshProfiles();
      await get().selectProfile(profile.id);
      get().pushToast({
        title: 'Профиль импортирован',
        message: `${profile.name} успешно разобран.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Ошибка импорта',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  importProfilesJson: async (json) => {
    try {
      const profiles = await backend.importProfilesJson(json);
      await get().refreshProfiles();
      if (profiles[0]) {
        await get().selectProfile(profiles[0].id);
      }
      get().pushToast({
        title: 'JSON импортирован',
        message:
          profiles.length === 1
            ? `Профиль ${profiles[0].name} добавлен.`
            : `Импортировано профилей: ${profiles.length}.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Ошибка импорта JSON',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  importSubscription: async (url) => {
    try {
      const result = await backend.importSubscription(url);
      await get().refreshProfiles();
      await get().refreshSubscriptions();
      if (result.profiles[0]) {
        await get().selectProfile(result.profiles[0].id);
      }
      get().pushToast({
        title: 'Подписка импортирована',
        message:
          result.profiles.length === 1
            ? 'Импортирован 1 профиль из подписки.'
            : `Импортировано профилей из подписки: ${result.profiles.length}.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Ошибка импорта подписки',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  refreshSubscription: async (subscriptionId) => {
    try {
      const result = await backend.refreshSubscription(subscriptionId);
      await get().refreshProfiles();
      await get().refreshSubscriptions();
      get().pushToast({
        title: 'Подписка обновлена',
        message: `${result.subscription.name}: синхронизировано профилей ${result.profiles.length}.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось обновить подписку',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  refreshAllSubscriptions: async () => {
    try {
      const results = await backend.refreshAllSubscriptions();
      await get().refreshProfiles();
      await get().refreshSubscriptions();
      const totalProfiles = results.reduce((sum, entry) => sum + entry.profiles.length, 0);
      get().pushToast({
        title: 'Подписки обновлены',
        message:
          results.length === 0
            ? 'Нет подписок для обновления.'
            : `Синхронизировано подписок: ${results.length}, обновлено профилей: ${totalProfiles}.`,
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось обновить подписки',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  deleteSubscription: async (subscriptionId) => {
    try {
      await backend.deleteSubscription(subscriptionId);
      await get().refreshProfiles();
      await get().refreshSubscriptions();
      get().pushToast({
        title: 'Подписка удалена',
        message: 'Подписка и импортированные профили удалены.',
        tone: 'info',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось удалить подписку',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  selectProfile: async (profileId) => {
    const currentConnection = get().connectionStatus;
    const shouldReconnect =
      !!profileId &&
      currentConnection.state === 'connected' &&
      currentConnection.activeProfileId !== profileId;

    set({ selectedProfileId: profileId });
    try {
      const settings = await backend.updateSettings({ lastSelectedProfileId: profileId });
      set((state) => ({
        settings: {
          ...state.settings,
          ...settings,
        },
      }));

      if (shouldReconnect && profileId) {
        const disconnectedStatus = await backend.disconnect();
        set({ connectionStatus: disconnectedStatus });

        const reconnectedStatus = await backend.connect(profileId);
        set({ connectionStatus: reconnectedStatus });
        await get().refreshLogs('preview');
      }
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось выбрать профиль',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  saveSettings: async (patch) => {
    try {
      if (typeof patch.launchAtStartup === 'boolean') {
        if (patch.launchAtStartup) {
          await enable();
        } else {
          await disable();
        }
      }

      const settings = await backend.updateSettings(patch);
      set({
        settings: {
          ...settings,
          launchAtStartup: await isEnabled(),
        },
      });
      get().pushToast({
        title: 'Настройки обновлены',
        message: 'Параметры сохранены локально.',
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось сохранить настройки',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  connect: async (profileId) => {
    try {
      const connectionStatus = await backend.connect(profileId);
      set({ connectionStatus });
      await get().refreshLogs('preview');
      get().pushToast({
        title: 'Подключено',
        message: 'Маршрут через Xray успешно активирован.',
        tone: 'success',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Ошибка подключения',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  disconnect: async () => {
    try {
      const connectionStatus = await backend.disconnect();
      set({ connectionStatus });
      await get().refreshLogs('preview');
      get().pushToast({
        title: 'Отключено',
        message: 'Маршрут отключен и системный proxy очищен.',
        tone: 'info',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Ошибка отключения',
        message: payload.message,
        tone: 'error',
      });
    }
  },

  testProfileConnection: async (profileId) => {
    try {
      const result = await backend.testProfileConnection(profileId);
      get().pushToast({
        title: 'Тест подключения пройден',
        message:
          result.durationMs != null
            ? `${result.message} ${result.durationMs} ms.`
            : result.message,
        tone: 'success',
      });
      return result;
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Тест подключения не пройден',
        message: payload.message,
        tone: 'error',
      });
      return null;
    }
  },

  clearLogs: async () => {
    try {
      await backend.clearLogs();
      set({ logs: defaultLogs });
      get().pushToast({
        title: 'Логи очищены',
        message: 'Логи приложения и подключения удалены.',
        tone: 'info',
      });
    } catch (error) {
      const payload = mapError(error);
      get().pushToast({
        title: 'Не удалось очистить логи',
        message: payload.message,
        tone: 'error',
      });
    }
  },
}));

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    cleanupListeners.forEach((unlisten) => unlisten());
    cleanupListeners = [];
  });
}
