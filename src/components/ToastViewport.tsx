import { t } from '../lib/i18n';
import { useAppStore } from '../store/useAppStore';
import { Button } from './Button';

export function ToastViewport() {
  const toasts = useAppStore((state) => state.toasts);
  const dismissToast = useAppStore((state) => state.dismissToast);
  const language = useAppStore((state) => state.settings.language);

  if (!toasts.length) {
    return null;
  }

  return (
    <div className="toast-viewport">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast toast-${toast.tone}`}>
          <div>
            <strong>{toast.title}</strong>
            <p>{toast.message}</p>
          </div>
          <Button variant="ghost" onClick={() => dismissToast(toast.id)}>
            {t(language, 'close')}
          </Button>
        </div>
      ))}
    </div>
  );
}
