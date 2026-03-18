import { useEffect } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { AppShell } from './components/AppShell';
import { ToastViewport } from './components/ToastViewport';
import { applyTheme } from './lib/theme';
import { AboutPage } from './pages/AboutPage';
import { DashboardPage } from './pages/DashboardPage';
import { LogsPage } from './pages/LogsPage';
import { SettingsPage } from './pages/SettingsPage';
import { useAppStore } from './store/useAppStore';

function AppContent() {
  const location = useLocation();
  const initialize = useAppStore((state) => state.initialize);
  const refreshConnectionStatus = useAppStore((state) => state.refreshConnectionStatus);
  const refreshLogs = useAppStore((state) => state.refreshLogs);
  const refreshLatencies = useAppStore((state) => state.refreshLatencies);
  const theme = useAppStore((state) => state.settings.theme);
  const initialized = useAppStore((state) => state.initialized);
  const importProfile = useAppStore((state) => state.importProfile);
  const importProfilesJson = useAppStore((state) => state.importProfilesJson);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (!initialized) {
      return;
    }

    const statusTimer = window.setInterval(() => {
      void refreshConnectionStatus();
      if (location.pathname === '/' || location.pathname === '/logs') {
        void refreshLogs(location.pathname === '/logs' ? 'full' : 'preview');
      }
    }, 3000);

    const latencyTimer = window.setInterval(() => {
      void refreshLatencies();
    }, 15000);

    return () => {
      window.clearInterval(statusTimer);
      window.clearInterval(latencyTimer);
    };
  }, [initialized, location.pathname, refreshConnectionStatus, refreshLatencies, refreshLogs]);

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const active = document.activeElement as HTMLElement | null;
      if (
        active?.tagName === 'INPUT' ||
        active?.tagName === 'TEXTAREA' ||
        active?.isContentEditable
      ) {
        return;
      }

      const text = event.clipboardData?.getData('text')?.trim();
      if (!text || text.length > 1024 * 1024) {
        return;
      }

      if (text.toLowerCase().startsWith('vless://')) {
        event.preventDefault();
        void importProfile(text);
        return;
      }

      if (text.startsWith('{') || text.startsWith('[')) {
        event.preventDefault();
        void importProfilesJson(text);
      }
    };

    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, [importProfile, importProfilesJson]);

  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/logs" element={<LogsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/about" element={<AboutPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
      <ToastViewport />
    </AppShell>
  );
}

export default function App() {
  return <AppContent />;
}
