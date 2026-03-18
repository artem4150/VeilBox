import { useMemo } from 'react';
import { Panel } from '../components/Panel';
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
    <div className="page">
      <div className="page-header">
        <div>
          <span className="eyebrow">Логи</span>
          <h1>Диагностика и журналы</h1>
          <p>Логи приложения, подключения и вывод Xray с маскированием чувствительных значений.</p>
        </div>
      </div>

      <div className="logs-layout">
        <Panel
          title="Логи подключения"
          description="Сценарий connect/disconnect, системный proxy, TUN и вывод Xray"
        >
          <LogViewer
            title="Последнее подключение"
            entries={logs.connection}
            onCopy={() => void copy(connectionText)}
            onClear={() => void clearLogs()}
          />
        </Panel>
        <Panel
          title="Логи приложения"
          description="Хранилище, tray, восстановление состояния и общие события"
        >
          <LogViewer
            title="Логи приложения"
            entries={logs.app}
            onCopy={() => void copy(appText)}
          />
        </Panel>
      </div>
    </div>
  );
}
