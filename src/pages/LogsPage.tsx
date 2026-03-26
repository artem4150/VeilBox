import { useMemo } from 'react';
import { LogViewer } from '../features/logs/LogViewer';
import { useAppStore } from '../store/useAppStore';

export function LogsPage() {
  const logs = useAppStore((state) => state.logs);
  const clearLogs = useAppStore((state) => state.clearLogs);

  const appText = useMemo(
    () =>
      logs.app
        .map((entry) => `[${entry.timestamp}] ${entry.level} ${entry.source}: ${entry.message}`)
        .join('\n'),
    [logs.app],
  );

  const connectionText = useMemo(
    () =>
      logs.connection
        .map((entry) => `[${entry.timestamp}] ${entry.level} ${entry.source}: ${entry.message}`)
        .join('\n'),
    [logs.connection],
  );

  const copy = async (text: string) => {
    await navigator.clipboard.writeText(text);
  };

  return (
    <div className="page logs-page-minimal">
      <div className="page-header">
        <div>
          <span className="eyebrow">Logs</span>
          <h1>Diagnostics and logs</h1>
          <p>Application events, connection logs and masked Xray output.</p>
        </div>
      </div>

      <div className="logs-text-layout">
        <section className="logs-section">
          <div className="logs-section-heading">
            <h2>Connection logs</h2>
            <p>Connect/disconnect flow, system proxy, TUN and Xray output.</p>
          </div>

          <LogViewer
            title="Latest connection session"
            entries={logs.connection}
            onCopy={() => void copy(connectionText)}
            onClear={() => void clearLogs()}
          />
        </section>

        <section className="logs-section">
          <div className="logs-section-heading">
            <h2>Application logs</h2>
            <p>Storage, tray, state recovery and general runtime events.</p>
          </div>

          <LogViewer title="Runtime log stream" entries={logs.app} onCopy={() => void copy(appText)} />
        </section>
      </div>
    </div>
  );
}
