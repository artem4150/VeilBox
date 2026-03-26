import { useEffect, useMemo, useState } from 'react';
import {
  Bot,
  Bug,
  Cable,
  Home,
  Laptop,
  MoonStar,
  Route,
  Shield,
  ShieldOff,
  Sun,
  Tv,
  Wifi,
} from 'lucide-react';
import { TextArea, TextInput } from '../components/Field';
import { backend } from '../lib/tauri';
import { useAppStore } from '../store/useAppStore';
import type { AppLanguage, NetworkInterfaceInfo, SplitTunnelMode } from '../types';

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
const GEO_TUN_PRESETS = [
  { key: 'geosite:private', label: { ru: 'Geo private', en: 'Geo private' }, domains: ['geosite:private'], ips: ['geoip:private'] },
  { key: 'geosite:category-ads-all', label: { ru: 'Geo ads', en: 'Geo ads' }, domains: ['geosite:category-ads-all'], ips: [] },
  { key: 'geosite:netflix', label: { ru: 'Geo Netflix', en: 'Geo Netflix' }, domains: ['geosite:netflix'], ips: [] },
  { key: 'geosite:google', label: { ru: 'Geo Google', en: 'Geo Google' }, domains: ['geosite:google'], ips: ['geoip:google'] },
  { key: 'geosite:telegram', label: { ru: 'Geo Telegram', en: 'Geo Telegram' }, domains: ['geosite:telegram'], ips: ['geoip:telegram'] },
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

function tx(language: AppLanguage, ru: string, en: string) {
  return language === 'ru' ? ru : en;
}

export function SettingsPage() {
  const settings = useAppStore((state) => state.settings);
  const saveSettings = useAppStore((state) => state.saveSettings);
  const language = settings.language;

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
        setInterfacesError(
          tx(
            language,
            'Не удалось получить список сетевых адаптеров Windows.',
            'Could not load Windows network adapters.',
          ),
        );
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
  }, [language]);

  const routingModes = useMemo<Array<{
    value: SplitTunnelMode;
    label: string;
    description: string;
    icon: typeof Shield;
  }>>(() => {
    if (settings.connectionMode === 'systemProxy') {
      return [
        {
          value: 'disabled',
          label: tx(language, 'Весь трафик', 'All traffic'),
          description: tx(language, 'Все через proxy', 'Everything through proxy'),
          icon: Shield,
        },
        {
          value: 'bypassListed',
          label: tx(language, 'Обходить список', 'Bypass listed'),
          description: tx(language, 'Список идет напрямую', 'Listed routes go direct'),
          icon: ShieldOff,
        },
        {
          value: 'proxyListed',
          label: tx(language, 'Только список', 'Only listed'),
          description: tx(language, 'Только список через proxy', 'Only listed routes via proxy'),
          icon: Route,
        },
      ];
    }

    return [
      {
        value: 'disabled',
        label: tx(language, 'Полный туннель', 'Full tunnel'),
        description: tx(language, 'Все через VPN', 'Everything through VPN'),
        icon: Shield,
      },
      {
        value: 'bypassListed',
        label: tx(language, 'Обходить список', 'Bypass listed'),
        description: tx(language, 'Список идет напрямую', 'Listed routes go direct'),
        icon: ShieldOff,
      },
      {
        value: 'proxyListed',
        label: tx(language, 'Только список', 'Only listed'),
        description: tx(language, 'Только список через VPN', 'Only listed routes via VPN'),
        icon: Route,
      },
    ];
  }, [language, settings.connectionMode]);

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
      ? tx(
          language,
          'Домены и поддерживаемые IPv4 диапазоны.',
          'Domains and supported IPv4 ranges.',
        )
      : tx(
          language,
          'Домены, IPv4 и IPv6 CIDR.',
          'Domains, IPv4 and IPv6 CIDR.',
        );

  const settingSwitches = [
    {
      key: 'launchAtStartup' as const,
      icon: Laptop,
      title: tx(language, 'Автозапуск', 'Launch at startup'),
      hint: tx(language, 'Старт вместе с Windows', 'Start with Windows'),
    },
    {
      key: 'minimizeToTray' as const,
      icon: Wifi,
      title: tx(language, 'Сворачивать в трей', 'Minimize to tray'),
      hint: tx(language, 'Закрытие скрывает окно', 'Close hides the window'),
    },
    {
      key: 'autoReconnect' as const,
      icon: Cable,
      title: tx(language, 'Автопереподключение', 'Auto reconnect'),
      hint: tx(language, 'Восстановление после падения Xray', 'Recover after Xray crash'),
    },
    {
      key: 'debugLogging' as const,
      icon: Bug,
      title: tx(language, 'Debug logs', 'Debug logs'),
      hint: tx(language, 'Подробные локальные логи', 'Verbose local logs'),
    },
  ];

  return (
    <div className="page settings-page-minimal">
      <div className="page-header">
        <div>
          <span className="eyebrow">{tx(language, 'Настройки', 'Settings')}</span>
          <h1>{tx(language, 'Поведение приложения', 'App behavior')}</h1>
          <p>
            {tx(
              language,
              'Тема, режим подключения, раздельное туннелирование и системное поведение.',
              'Theme, connection mode, split tunneling and system behavior.',
            )}
          </p>
        </div>
      </div>

      <div className="settings-minimal-layout">
        <section className="settings-minimal-section">
          <div className="settings-minimal-heading">
            <h2>{tx(language, 'Общие', 'General')}</h2>
            <p>{tx(language, 'Быстрые системные параметры.', 'System-wide behavior toggles.')}</p>
          </div>

          <div className="settings-toggle-list">
            {settingSwitches.map(({ key, icon: Icon, title, hint }) => (
              <button
                key={key}
                type="button"
                className={`settings-toggle-item${settings[key] ? ' is-active' : ''}`}
                onClick={() => void saveSettings({ [key]: !settings[key] })}
              >
                <div className="settings-toggle-icon">
                  <Icon size={17} />
                </div>
                <div className="settings-toggle-copy">
                  <strong>{title}</strong>
                  <span>{hint}</span>
                </div>
                <span className={`settings-toggle-switch${settings[key] ? ' is-on' : ''}`}>
                  <span className="settings-toggle-knob" />
                </span>
              </button>
            ))}
          </div>
        </section>

        <section className="settings-minimal-section">
          <div className="settings-minimal-heading">
            <h2>{tx(language, 'Подключение', 'Connection')}</h2>
            <p>
              {tx(
                language,
                'Выбери режим и правила маршрутизации.',
                'Choose a route mode and traffic rules.',
              )}
            </p>
          </div>

          <div className="settings-choice-row">
            {[
              {
                value: 'systemProxy' as const,
                icon: Wifi,
                title: 'System proxy',
                hint: tx(language, 'Стабильнее', 'Most compatible'),
              },
              {
                value: 'tun' as const,
                icon: Route,
                title: 'TUN',
                hint: tx(language, 'Глубже в систему', 'System-wide capture'),
              },
            ].map((mode) => (
              <button
                key={mode.value}
                type="button"
                className={`settings-choice-chip${
                  settings.connectionMode === mode.value ? ' is-active' : ''
                }`}
                onClick={() =>
                  void saveSettings({
                    connectionMode: mode.value,
                  })
                }
              >
                <mode.icon size={16} />
                <div>
                  <strong>{mode.title}</strong>
                  <span>{mode.hint}</span>
                </div>
              </button>
            ))}
          </div>

          {settings.connectionMode === 'tun' ? (
            <div className="settings-inline-grid">
              <label className="settings-inline-field">
                <span>{tx(language, 'Имя TUN', 'TUN name')}</span>
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
              </label>

              <label className="settings-inline-field">
                <span>{tx(language, 'Исходящий адаптер', 'Outbound adapter')}</span>
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
                    {loadingInterfaces
                      ? tx(language, 'Загрузка адаптеров...', 'Loading adapters...')
                      : tx(language, 'Автоматически', 'Automatic')}
                  </option>
                  {settings.tunOutboundInterface && !hasSelectedInterface ? (
                    <option value={settings.tunOutboundInterface}>
                      {settings.tunOutboundInterface}
                    </option>
                  ) : null}
                  {networkInterfaces.map((networkInterface) => (
                    <option key={networkInterface.name} value={networkInterface.name}>
                      {networkInterface.name} ({networkInterface.status})
                    </option>
                  ))}
                </select>
                {interfacesError ? <em>{interfacesError}</em> : null}
              </label>

              <button
                type="button"
                className={`settings-toggle-item settings-toggle-inline${
                  settings.tunDisableIpv6 ? ' is-active' : ''
                }`}
                onClick={() => void saveSettings({ tunDisableIpv6: !settings.tunDisableIpv6 })}
              >
                <div className="settings-toggle-icon">
                  <ShieldOff size={17} />
                </div>
                <div className="settings-toggle-copy">
                  <strong>{tx(language, 'Отключить IPv6', 'Disable IPv6')}</strong>
                  <span>{tx(language, 'Для более жесткого маршрута', 'For stricter routing')}</span>
                </div>
                <span className={`settings-toggle-switch${settings.tunDisableIpv6 ? ' is-on' : ''}`}>
                  <span className="settings-toggle-knob" />
                </span>
              </button>
            </div>
          ) : null}

          <div className="settings-routing-block">
            <div className="settings-routing-heading">
              <h3>{tx(language, 'Раздельное туннелирование', 'Split tunneling')}</h3>
              <p>
                {settings.connectionMode === 'systemProxy'
                  ? tx(
                      language,
                      'Для system proxy доступны полный режим, обход списка и только список через proxy.',
                      'System proxy supports full mode, bypass list and only listed via proxy.',
                    )
                  : tx(
                      language,
                      'Для TUN можно отправлять через VPN только выбранный список.',
                      'TUN can route only selected items through VPN.',
                    )}
              </p>
            </div>

            <div className="settings-choice-row">
              {routingModes.map((mode) => (
                <button
                  key={mode.value}
                  type="button"
                  className={`settings-choice-chip${
                    settings.splitTunnelMode === mode.value ? ' is-active' : ''
                  }`}
                  onClick={() => void saveSettings({ splitTunnelMode: mode.value })}
                >
                  <mode.icon size={16} />
                  <div>
                    <strong>{mode.label}</strong>
                    <span>{mode.description}</span>
                  </div>
                </button>
              ))}
            </div>

            {settings.splitTunnelMode !== 'disabled' ? (
              <>
                <div className="settings-preset-row">
                  <button
                    type="button"
                    className="settings-preset-chip"
                    onClick={() => void applyPreset(LOCAL_NETWORK_DOMAINS, LOCAL_NETWORK_IPS)}
                  >
                    <Home size={15} />
                    <span>{tx(language, 'Локальные сети', 'Local networks')}</span>
                  </button>
                  <button
                    type="button"
                    className="settings-preset-chip"
                    onClick={() => void applyPreset(STREAMING_DOMAINS, [])}
                  >
                    <Tv size={15} />
                    <span>{tx(language, 'Стриминг', 'Streaming')}</span>
                  </button>
                  <button
                    type="button"
                    className="settings-preset-chip"
                    onClick={() => void applyPreset(AI_DOMAINS, [])}
                  >
                    <Bot size={15} />
                    <span>{tx(language, 'AI сервисы', 'AI services')}</span>
                  </button>
                  {settings.connectionMode === 'tun'
                    ? GEO_TUN_PRESETS.map((preset) => (
                        <button
                          key={preset.key}
                          type="button"
                          className="settings-preset-chip"
                          onClick={() => void applyPreset(preset.domains, preset.ips)}
                        >
                          <Route size={15} />
                          <span>{preset.label[language]}</span>
                        </button>
                      ))
                    : null}
                  <button
                    type="button"
                    className="settings-preset-chip is-danger"
                    onClick={() => void clearSplitRules()}
                  >
                    <span>{tx(language, 'Очистить', 'Clear')}</span>
                  </button>
                </div>

                <div className="settings-inline-grid">
                  <label className="settings-inline-field settings-inline-field-wide">
                    <span>{tx(language, 'Домены', 'Domains')}</span>
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
                    <em>{splitHint}</em>
                  </label>

                  <label className="settings-inline-field settings-inline-field-wide">
                    <span>IP / CIDR</span>
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
                    <em>
                      {settings.connectionMode === 'systemProxy'
                        ? tx(language, 'Поддерживаются IPv4 диапазоны.', 'Supported IPv4 ranges.')
                        : tx(language, 'Поддерживаются IPv4, IPv6 и geoip: пресеты.', 'IPv4, IPv6 and geoip: presets are supported.')}
                    </em>
                  </label>
                </div>
                {settings.connectionMode === 'tun' ? (
                  <p className="settings-inline-note">
                    {tx(
                      language,
                      'Geo-пресеты используют geosite:/geoip: категории совместимых наборов правил Xray.',
                      'Geo presets use geosite:/geoip: categories from compatible Xray rule sets.',
                    )}
                  </p>
                ) : (
                  <p className="settings-inline-note">
                    {tx(
                      language,
                      'В System Proxy режиме для Only listed via proxy используется локальный PAC-файл.',
                      'System Proxy uses a local PAC file for only listed via proxy mode.',
                    )}
                  </p>
                )}
              </>
            ) : null}
          </div>
        </section>

        <section className="settings-minimal-section">
          <div className="settings-minimal-heading">
            <h2>{tx(language, 'Оформление', 'Appearance')}</h2>
            <p>{tx(language, 'Тема и внешний вид приложения.', 'Theme and interface appearance.')}</p>
          </div>

          <div className="settings-choice-row">
            {[
              {
                value: 'light' as const,
                icon: Sun,
                title: tx(language, 'Светлая', 'Light'),
                hint: tx(language, 'Белый интерфейс', 'Bright interface'),
              },
              {
                value: 'dark' as const,
                icon: MoonStar,
                title: tx(language, 'Темная', 'Dark'),
                hint: tx(language, 'Темный интерфейс', 'Dark interface'),
              },
              {
                value: 'system' as const,
                icon: Laptop,
                title: tx(language, 'Системная', 'System'),
                hint: tx(language, 'Следовать Windows', 'Follow Windows'),
              },
            ].map((theme) => (
              <button
                key={theme.value}
                type="button"
                className={`settings-choice-chip${settings.theme === theme.value ? ' is-active' : ''}`}
                onClick={() => void saveSettings({ theme: theme.value })}
              >
                <theme.icon size={16} />
                <div>
                  <strong>{theme.title}</strong>
                  <span>{theme.hint}</span>
                </div>
              </button>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
