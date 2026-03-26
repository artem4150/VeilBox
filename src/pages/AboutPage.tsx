import { useAppStore } from '../store/useAppStore';

export function AboutPage() {
  const about = useAppStore((state) => state.about);

  return (
    <div className="page about-page-minimal">
      <div className="page-header">
        <div>
          <span className="eyebrow">About</span>
          <h1>Build information</h1>
          <p>Component versions and the current support scope for this release.</p>
        </div>
      </div>

      <div className="about-text-layout">
        <section className="about-section">
          <div className="about-section-heading">
            <h2>Versions</h2>
            <p>Built locally with the Tauri backend.</p>
          </div>

          <div className="about-row">
            <span>App version</span>
            <strong>{about?.appVersion ?? 'Unknown'}</strong>
          </div>
          <div className="about-row">
            <span>Tauri version</span>
            <strong>{about?.tauriVersion ?? 'Unknown'}</strong>
          </div>
          <div className="about-row">
            <span>Xray version</span>
            <strong>{about?.xrayVersion ?? 'Unavailable'}</strong>
          </div>
          <div className="about-row">
            <span>Platform</span>
            <strong>{about?.platform ?? 'windows'}</strong>
          </div>
        </section>

        <section className="about-section">
          <div className="about-section-heading">
            <h2>Support</h2>
            <p>Implemented in the parser, config builder and backend commands.</p>
          </div>

          <div className="about-row">
            <span>VLESS modes</span>
            <strong>RAW / TCP / WS / gRPC / XHTTP / HTTPUpgrade / mKCP</strong>
          </div>
          <div className="about-row">
            <span>Security</span>
            <strong>None / TLS / Reality</strong>
          </div>
          <div className="about-row">
            <span>Import</span>
            <strong>URI, JSON, subscriptions and Ctrl+V from clipboard</strong>
          </div>
          <div className="about-row">
            <span>Connection modes</span>
            <strong>System proxy and TUN</strong>
          </div>
          <div className="about-row">
            <span>Split tunneling</span>
            <strong>Available for TUN and System proxy</strong>
          </div>
        </section>
      </div>
    </div>
  );
}
