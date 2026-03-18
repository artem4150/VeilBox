import { Panel } from '../components/Panel';
import { useAppStore } from '../store/useAppStore';

export function AboutPage() {
  const about = useAppStore((state) => state.about);

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <span className="eyebrow">О программе</span>
          <h1>Информация о сборке</h1>
          <p>Версии компонентов и текущий объем поддержки в этой версии клиента.</p>
        </div>
      </div>

      <div className="about-grid">
        <Panel title="Версии" description="Собирается локально через Tauri backend">
          <div className="diagnostic-list">
            <div>
              <span>Версия приложения</span>
              <strong>{about?.appVersion ?? 'Неизвестно'}</strong>
            </div>
            <div>
              <span>Версия Tauri</span>
              <strong>{about?.tauriVersion ?? 'Неизвестно'}</strong>
            </div>
            <div>
              <span>Версия Xray</span>
              <strong>{about?.xrayVersion ?? 'Недоступно'}</strong>
            </div>
            <div>
              <span>Платформа</span>
              <strong>{about?.platform ?? 'windows'}</strong>
            </div>
          </div>
        </Panel>

        <Panel title="Поддержка" description="Реализовано в parser, config builder и backend-командах">
          <div className="support-list">
            <div>VLESS RAW / TCP / WS / gRPC / XHTTP / HTTPUpgrade / mKCP</div>
            <div>Безопасность: None / TLS / Reality</div>
            <div>Импорт через URI, JSON, подписки и Ctrl+V из буфера</div>
            <div>Подключение через System proxy и TUN</div>
            <div>Раздельное туннелирование для TUN и System proxy</div>
          </div>
        </Panel>
      </div>
    </div>
  );
}
