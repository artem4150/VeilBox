import { useAppStore } from '../store/useAppStore';

export function AboutPage() {
  const about = useAppStore((state) => state.about);
  const language = useAppStore((state) => state.settings.language);
  const tx = (ru: string, en: string) => (language === 'ru' ? ru : en);

  return (
    <div className="page about-page-minimal">
      <div className="page-header">
        <div>
          <span className="eyebrow">{tx('О программе', 'About')}</span>
          <h1>{tx('Информация о сборке и условиях', 'Build and legal information')}</h1>
          <p>
            {tx(
              'Версии, поддерживаемые режимы, приватность, условия использования и сторонние компоненты.',
              'Versions, supported modes, privacy, usage terms, and third-party components.',
            )}
          </p>
        </div>
      </div>

      <div className="about-text-layout">
        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Версии', 'Versions')}</h2>
            <p>{tx('Сборка приложения и runtime-компонентов.', 'Application and runtime component build info.')}</p>
          </div>

          <div className="about-row">
            <span>{tx('Версия приложения', 'App version')}</span>
            <strong>{about?.appVersion ?? tx('Неизвестно', 'Unknown')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Версия Tauri', 'Tauri version')}</span>
            <strong>{about?.tauriVersion ?? tx('Неизвестно', 'Unknown')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Версия Xray', 'Xray version')}</span>
            <strong>{about?.xrayVersion ?? tx('Недоступно', 'Unavailable')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Платформа', 'Platform')}</span>
            <strong>{about?.platform ?? 'windows'}</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Поддержка', 'Support scope')}</h2>
            <p>{tx('Что поддерживает текущая сборка.', 'What this release currently supports.')}</p>
          </div>

          <div className="about-row">
            <span>{tx('Режимы VLESS', 'VLESS modes')}</span>
            <strong>RAW / TCP / WS / gRPC / XHTTP / HTTPUpgrade / mKCP</strong>
          </div>
          <div className="about-row">
            <span>{tx('Безопасность', 'Security')}</span>
            <strong>None / TLS / Reality</strong>
          </div>
          <div className="about-row">
            <span>{tx('Импорт', 'Import')}</span>
            <strong>{tx('URI, JSON, подписки, Amnezia и Ctrl+V из буфера', 'URI, JSON, subscriptions, Amnezia, and Ctrl+V from clipboard')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Режимы подключения', 'Connection modes')}</span>
            <strong>{tx('System proxy, TUN и AmneziaWG', 'System proxy, TUN, and AmneziaWG')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Раздельное туннелирование', 'Split tunneling')}</span>
            <strong>{tx('Доступно для TUN и System proxy', 'Available for TUN and System proxy')}</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Приватность', 'Privacy')}</h2>
            <p>{tx('Коротко о локальных данных и сетевом поведении.', 'Short summary of local data and network behavior.')}</p>
          </div>

          <div className="about-row">
            <span>{tx('Локальное хранение', 'Local storage')}</span>
            <strong>
              {tx(
                'Профили, настройки, выбранный профиль и локальные логи хранятся в AppData.',
                'Profiles, settings, selected profile, and local logs are stored in AppData.',
              )}
            </strong>
          </div>
          <div className="about-row">
            <span>{tx('Телеметрия', 'Telemetry')}</span>
            <strong>{tx('По умолчанию отсутствует.', 'Not enabled by default.')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Сетевые соединения', 'Network connections')}</span>
            <strong>
              {tx(
                'Приложение подключается только к выбранным VPN-серверам, подпискам и диагностическим endpoint при необходимости.',
                'The app only connects to selected VPN servers, subscription URLs, and optional diagnostic endpoints when needed.',
              )}
            </strong>
          </div>
          <div className="about-row">
            <span>{tx('Полный текст', 'Full text')}</span>
            <strong>PRIVACY.md</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Условия использования', 'Terms')}</h2>
            <p>{tx('Ключевые ограничения ответственности и правила использования.', 'Key usage rules and liability limits.')}</p>
          </div>

          <div className="about-row">
            <span>{tx('Ответственность пользователя', 'User responsibility')}</span>
            <strong>
              {tx(
                'Пользователь сам отвечает за законность использования VPN, серверов и импортируемых конфигов.',
                'The user is responsible for lawful VPN use, servers, and imported configs.',
              )}
            </strong>
          </div>
          <div className="about-row">
            <span>{tx('Гарантии', 'Warranty')}</span>
            <strong>{tx('Приложение предоставляется «как есть».', 'The application is provided "as is".')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Полный текст', 'Full text')}</span>
            <strong>TERMS.md</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Сторонние компоненты', 'Third-party notices')}</h2>
            <p>{tx('Основные bundled runtime-компоненты и их лицензии.', 'Main bundled runtime components and their licenses.')}</p>
          </div>

          <div className="about-row">
            <span>Xray-core</span>
            <strong>MPL-2.0</strong>
          </div>
          <div className="about-row">
            <span>AmneziaWG Windows Client</span>
            <strong>MIT</strong>
          </div>
          <div className="about-row">
            <span>Wintun</span>
            <strong>{tx('Официальные условия поставки Wintun', 'Official Wintun distribution terms')}</strong>
          </div>
          <div className="about-row">
            <span>{tx('Полный список', 'Full notice')}</span>
            <strong>THIRD_PARTY_NOTICES.md</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>{tx('Поддержка и контакты', 'Support and contact')}</h2>
            <p>{tx('Куда отправлять баги, privacy-вопросы и диагностические отчеты.', 'Where to send bugs, privacy requests, and diagnostic reports.')}</p>
          </div>

          <div className="about-row">
            <span>{tx('Трекер проекта', 'Project tracker')}</span>
            <strong>github.com/artem4150/VeilBox/issues</strong>
          </div>
          <div className="about-row">
            <span>{tx('Support guide', 'Support guide')}</span>
            <strong>SUPPORT.md</strong>
          </div>
        </section>
      </div>
    </div>
  );
}
