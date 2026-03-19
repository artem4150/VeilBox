import { useEffect, useMemo, useState } from 'react';
import { Button } from '../components/Button';
import { Field, TextArea, TextInput } from '../components/Field';
import { Panel } from '../components/Panel';
import { Toggle } from '../components/Toggle';
import { backend } from '../lib/tauri';
import { useAppStore } from '../store/useAppStore';
import type { NetworkInterfaceInfo, SplitTunnelMode } from '../types';

const LOCAL_NETWORK_DOMAINS = ['localhost', 'local', 'home.arpa'];
const LOCAL_NETWORK_IPS = [
  '127.0.0.0/8',
  '10.0.0.0/8',
  '172.16.0.0/12',
  '192.168.0.0/16',
  '169.254.0.0/16',
];
const STREAMING_DOMAINS = [
  'netflix.com',
  'youtube.com',
  'twitch.tv',
  'spotify.com',
  'disneyplus.com',
  'hulu.com',
];
const AI_DOMAINS = [
  'chatgpt.com',
  'openai.com',
  'oaistatic.com',
  'oaiusercontent.com',
  'claude.ai',
  'anthropic.com',
  'perplexity.ai',
  'gemini.google.com',
  'copilot.microsoft.com',
  'grok.com',
];

function parseRuleList(value: string) {
  return value
    .split(/[\n,;]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function mergeUnique(current: string[], items: string[]) {
  return Array.from(new Set([...current, ...items]));
}

export function SettingsPage() {
  const settings = useAppStore((state) => state.settings);
  const saveSettings = useAppStore((state) => state.saveSettings);
  const [networkInterfaces, setNetworkInterfaces] = useState<NetworkInterfaceInfo[]>([]);
  const [loadingInterfaces, setLoadingInterfaces] = useState(false);
  const [interfacesError, setInterfacesError] = useState<string | null>(null);
  const [tunInterfaceDraft, setTunInterfaceDraft] = useState(settings.tunInterfaceName);
  const [splitDomainsDraft, setSplitDomainsDraft] = useState(
    settings.splitTunnelDomains.join('\n'),
  );
  const [splitIpsDraft, setSplitIpsDraft] = useState(settings.splitTunnelIps.join('\n'));
  const hasSelectedInterface = networkInterfaces.some(
    (networkInterface) => networkInterface.name === settings.tunOutboundInterface,
  );

  useEffect(() => {
    setTunInterfaceDraft(settings.tunInterfaceName);
  }, [settings.tunInterfaceName]);

  useEffect(() => {
    setSplitDomainsDraft(settings.splitTunnelDomains.join('\n'));
  }, [settings.splitTunnelDomains]);

  useEffect(() => {
    setSplitIpsDraft(settings.splitTunnelIps.join('\n'));
  }, [settings.splitTunnelIps]);

  useEffect(() => {
    let active = true;

    async function loadNetworkInterfaces() {
      setLoadingInterfaces(true);
      setInterfacesError(null);
      try {
        const items = await backend.listNetworkInterfaces();
        if (!active) {
          return;
        }
        setNetworkInterfaces(items);
      } catch {
        if (!active) {
          return;
        }
        setNetworkInterfaces([]);
        setInterfacesError('Не удалось получить список сетевых адаптеров Windows.');
      } finally {
        if (active) {
          setLoadingInterfaces(false);
        }
      }
    }

    void loadNetworkInterfaces();

    return () => {
      active = false;
    };
  }, []);

  const routingModes = useMemo<Array<{
    value: SplitTunnelMode;
    label: string;
    description: string;
  }>>(() => {
    if (settings.connectionMode === 'systemProxy') {
      return [
        {
          value: 'disabled',
          label: 'Полный proxy',
          description: 'Весь трафик приложений, которые используют системный proxy, идет через Xray.',
        },
        {
          value: 'bypassListed',
          label: 'Обходить список',
          description:
            'Домены и поддерживаемые IPv4-диапазоны из списка пойдут напрямую, остальное через proxy.',
        },
      ];
    }

    return [
      {
        value: 'disabled',
        label: 'Полный туннель',
        description: 'Весь трафик в TUN-режиме идет через Xray.',
      },
      {
        value: 'bypassListed',
        label: 'Обходить список',
        description: 'Домены и CIDR из списка идут напрямую, остальное через VPN.',
      },
      {
        value: 'proxyListed',
        label: 'Только список через VPN',
        description: 'Через VPN идет только список доменов и CIDR, остальное напрямую.',
      },
    ];
  }, [settings.connectionMode]);

  const applyPreset = async (domains: string[], ips: string[]) => {
    const nextDomains = mergeUnique(settings.splitTunnelDomains, domains);
    const nextIps = mergeUnique(settings.splitTunnelIps, ips);
    setSplitDomainsDraft(nextDomains.join('\n'));
    setSplitIpsDraft(nextIps.join('\n'));
    await saveSettings({
      splitTunnelDomains: nextDomains,
      splitTunnelIps: nextIps,
    });
  };

  const clearSplitRules = async () => {
    setSplitDomainsDraft('');
    setSplitIpsDraft('');
    await saveSettings({
      splitTunnelDomains: [],
      splitTunnelIps: [],
    });
  };

  const splitHint =
    settings.connectionMode === 'systemProxy'
      ? 'В режиме системного proxy поддерживаются домены, full:, keyword: и IPv4 /8, /12, /16, /24, /32.'
      : 'В TUN-режиме поддерживаются домены, full:, keyword:, regexp:, geosite:, ext:, IPv4 и IPv6 CIDR.';

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <span className="eyebrow">Настройки</span>
          <h1>Поведение приложения</h1>
          <p>Автозапуск, трей, переподключение, тема, режим соединения и раздельное туннелирование.</p>
        </div>
      </div>

      <div className="settings-layout">
        <Panel title="Общие" description="Настройки сохраняются локально в AppData">
          <div className="settings-stack">
            <Toggle
              checked={settings.launchAtStartup}
              onChange={(launchAtStartup) => void saveSettings({ launchAtStartup })}
              label="Запускать вместе с Windows"
              description="Регистрирует приложение в автозапуске через плагин Tauri."
            />
            <Toggle
              checked={settings.minimizeToTray}
              onChange={(minimizeToTray) => void saveSettings({ minimizeToTray })}
              label="Сворачивать в трей"
              description="Закрытие окна скрывает приложение вместо полного завершения."
            />
            <Toggle
              checked={settings.autoReconnect}
              onChange={(autoReconnect) => void saveSettings({ autoReconnect })}
              label="Автопереподключение"
              description="Пытается восстановить последнюю сессию, если Xray завершился неожиданно."
            />
            <Toggle
              checked={settings.debugLogging}
              onChange={(debugLogging) => void saveSettings({ debugLogging })}
              label="Подробное логирование"
              description="Сохраняет расширенные диагностические записи в локальные логи."
            />
          </div>
        </Panel>

        <Panel
          title="Режим соединения"
          description="Системный proxy стабильнее. TUN охватывает больше трафика, но требует wintun.dll и запуск от администратора."
        >
          <div className="theme-grid">
            {([
              {
                value: 'systemProxy',
                label: 'Системный proxy',
                description:
                  'Локальный HTTP proxy и настройки Windows Internet Settings.',
              },
              {
                value: 'tun',
                label: 'TUN',
                description:
                  'Перехват системного трафика через Xray и wintun.dll.',
              },
            ] as const).map((mode) => (
              <button
                key={mode.value}
                type="button"
                className={`theme-card${settings.connectionMode === mode.value ? ' theme-card-active' : ''}`}
                onClick={() =>
                  void saveSettings({
                    connectionMode: mode.value,
                    ...(mode.value === 'systemProxy' && settings.splitTunnelMode === 'proxyListed'
                      ? { splitTunnelMode: 'bypassListed' }
                      : {}),
                  })
                }
              >
                <strong>{mode.label}</strong>
                <p>{mode.description}</p>
              </button>
            ))}
          </div>

          <div className="settings-stack" style={{ marginTop: 14 }}>
            {settings.connectionMode === 'tun' ? (
              <>
                <Field
                  label="Имя TUN-интерфейса"
                  hint="По умолчанию xray0. Меняй только если понимаешь, зачем нужен другой адаптер."
                >
                  <TextInput
                    value={tunInterfaceDraft}
                    onChange={(event) => setTunInterfaceDraft(event.target.value)}
                    onBlur={(event) => {
                      const next = event.target.value.trim() || 'xray0';
                      if (next !== settings.tunInterfaceName) {
                        void saveSettings({ tunInterfaceName: next });
                      }
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') {
                        event.currentTarget.blur();
                      }
                    }}
                  />
                </Field>

                <Toggle
                  checked={settings.tunDisableIpv6}
                  onChange={(tunDisableIpv6) => void saveSettings({ tunDisableIpv6 })}
                  label="Отключить IPv6 в TUN"
                  description="Рекомендуется для клиентов, которые могут обходить system proxy или утекают по IPv6. Для более жесткой маршрутизации оставь включенным."
                />

                <Field
                  label="Исходящий интерфейс"
                  hint={
                    interfacesError ??
                    'Необязательно. Выбери активный Ethernet или Wi-Fi, чтобы уменьшить риск route loop в TUN-режиме.'
                  }
                >
                  <select
                    className="select"
                    value={settings.tunOutboundInterface ?? ''}
                    onChange={(event) =>
                      void saveSettings({
                        tunOutboundInterface: event.target.value || null,
                      })
                    }
                  >
                    <option value="">
                      {loadingInterfaces ? 'Загрузка адаптеров...' : 'Автоматически / не задано'}
                    </option>
                    {settings.tunOutboundInterface && !hasSelectedInterface ? (
                      <option value={settings.tunOutboundInterface}>
                        {settings.tunOutboundInterface} (сохранено)
                      </option>
                    ) : null}
                    {networkInterfaces.map((networkInterface) => (
                      <option key={networkInterface.name} value={networkInterface.name}>
                        {networkInterface.name} ({networkInterface.status})
                      </option>
                    ))}
                  </select>
                </Field>
              </>
            ) : null}

            <Field
              label="Раздельное туннелирование"
              hint={
                settings.connectionMode === 'systemProxy'
                  ? 'Для системного proxy доступен только режим обхода списка. Изменения применятся после переподключения.'
                  : 'Изменения применятся после переподключения.'
              }
            >
              <div className="theme-grid">
                {routingModes.map((mode) => (
                  <button
                    key={mode.value}
                    type="button"
                    className={`theme-card${settings.splitTunnelMode === mode.value ? ' theme-card-active' : ''}`}
                    onClick={() => void saveSettings({ splitTunnelMode: mode.value })}
                  >
                    <strong>{mode.label}</strong>
                    <p>{mode.description}</p>
                  </button>
                ))}
              </div>
            </Field>

            {settings.splitTunnelMode !== 'disabled' ? (
              <>
                <Field
                  label="Предустановки"
                  hint="Быстро заполняют списки ниже. Потом их можно вручную отредактировать."
                >
                  <div className="button-row split-preset-row">
                    <Button
                      variant="ghost"
                      type="button"
                      onClick={() => void applyPreset(LOCAL_NETWORK_DOMAINS, LOCAL_NETWORK_IPS)}
                    >
                      Локальные сети
                    </Button>
                    <Button
                      variant="ghost"
                      type="button"
                      onClick={() => void applyPreset(STREAMING_DOMAINS, [])}
                    >
                      Стриминг
                    </Button>
                    <Button
                      variant="ghost"
                      type="button"
                      onClick={() => void applyPreset(AI_DOMAINS, [])}
                    >
                      AI-сервисы
                    </Button>
                    <Button
                      variant="ghost"
                      type="button"
                      className="danger-ghost"
                      onClick={() => void clearSplitRules()}
                    >
                      Очистить списки
                    </Button>
                  </div>
                </Field>

                <Field label="Домены" hint={splitHint}>
                  <TextArea
                    value={splitDomainsDraft}
                    placeholder={'example.com\nfull:login.example.com\nkeyword:openai'}
                    onChange={(event) => setSplitDomainsDraft(event.target.value)}
                    onBlur={() => {
                      const next = parseRuleList(splitDomainsDraft);
                      const current = settings.splitTunnelDomains;
                      if (
                        next.length !== current.length ||
                        next.some((item, index) => item !== current[index])
                      ) {
                        void saveSettings({ splitTunnelDomains: next });
                      }
                    }}
                  />
                </Field>

                <Field
                  label="IP / CIDR"
                  hint={
                    settings.connectionMode === 'systemProxy'
                      ? 'Для системного proxy поддерживаются только IPv4 /8, /12, /16, /24, /32.'
                      : 'Для TUN поддерживаются IPv4 и IPv6. Примеры: 1.1.1.1, 8.8.8.0/24, 2001:4860:4860::8888.'
                  }
                >
                  <TextArea
                    value={splitIpsDraft}
                    placeholder={
                      settings.connectionMode === 'systemProxy'
                        ? '1.1.1.1\n192.168.0.0/16'
                        : '1.1.1.1\n8.8.8.0/24'
                    }
                    onChange={(event) => setSplitIpsDraft(event.target.value)}
                    onBlur={() => {
                      const next = parseRuleList(splitIpsDraft);
                      const current = settings.splitTunnelIps;
                      if (
                        next.length !== current.length ||
                        next.some((item, index) => item !== current[index])
                      ) {
                        void saveSettings({ splitTunnelIps: next });
                      }
                    }}
                  />
                </Field>
              </>
            ) : null}
          </div>
        </Panel>

        <Panel title="Тема" description="Темная, светлая или системная тема Windows">
          <div className="theme-grid">
            {(['dark', 'light', 'system'] as const).map((theme) => (
              <button
                key={theme}
                type="button"
                className={`theme-card${settings.theme === theme ? ' theme-card-active' : ''}`}
                onClick={() => void saveSettings({ theme })}
              >
                <strong>
                  {theme === 'dark' ? 'Темная' : theme === 'light' ? 'Светлая' : 'Системная'}
                </strong>
                <p>
                  {theme === 'system'
                    ? 'Следовать текущей теме Windows.'
                    : theme === 'dark'
                      ? 'Использовать темное оформление приложения.'
                      : 'Использовать светлое оформление приложения.'}
                </p>
              </button>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}
