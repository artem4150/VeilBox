import type { ConnectionState } from '../types';
import { statusLabel } from '../lib/format';
import { useAppStore } from '../store/useAppStore';

export function StatusBadge({ state }: { state: ConnectionState }) {
  const language = useAppStore((store) => store.settings.language);
  return <span className={`status-badge status-${state}`}>{statusLabel(state, language)}</span>;
}
