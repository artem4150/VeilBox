import { Gauge, Info, Logs, Settings } from 'lucide-react';
import { NavLink } from 'react-router-dom';
import { Button } from './Button';
import { t } from '../lib/i18n';
import { useAppStore } from '../store/useAppStore';

interface SidebarProps {
  collapsed: boolean;
}

export function Sidebar({ collapsed }: SidebarProps) {
  const settings = useAppStore((state) => state.settings);
  const saveSettings = useAppStore((state) => state.saveSettings);

  const items = [
    { to: '/', label: t(settings.language, 'navDashboard'), icon: Gauge },
    { to: '/settings', label: t(settings.language, 'navSettings'), icon: Settings },
    { to: '/about', label: t(settings.language, 'navAbout'), icon: Info },
    { to: '/logs', label: t(settings.language, 'navLogs'), icon: Logs }
  ];

  return (
    <div className={`sidebar${collapsed ? ' sidebar-collapsed' : ''}`}>
      <div className="sidebar-brand">
        <div className="sidebar-brand-main">
          <div className="sidebar-logo">
            <img src="/app-icon.png" alt="VailBox icon" className="sidebar-logo-image" />
          </div>
          {!collapsed ? (
            <div>
              <h1>VailBox</h1>
              <p>{t(settings.language, 'sidebarClient')}</p>
            </div>
          ) : null}
        </div>
      </div>

      <nav className="sidebar-nav">
        {items.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `sidebar-link${isActive ? ' sidebar-link-active' : ''}`
            }
            title={label}
          >
            <Icon size={18} />
            {!collapsed ? <span>{label}</span> : null}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        {!collapsed ? (
          <p>
            {settings.connectionMode === 'tun'
              ? 'Sidecar Xray in TUN mode.'
              : t(settings.language, 'sidebarFooter')}
          </p>
        ) : null}
        <Button
          variant="secondary"
          className="sidebar-language-button"
          title={t(settings.language, 'langButton')}
          onClick={() => void saveSettings({ language: settings.language === 'ru' ? 'en' : 'ru' })}
        >
          {collapsed ? settings.language.toUpperCase() : t(settings.language, 'langButton')}
        </Button>
      </div>
    </div>
  );
}
