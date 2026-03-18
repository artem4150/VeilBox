import { Gauge, Info, Logs, PanelLeftClose, PanelLeftOpen, Settings } from 'lucide-react';
import { NavLink } from 'react-router-dom';
import { Button } from './Button';
import { t } from '../lib/i18n';
import { useAppStore } from '../store/useAppStore';

interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
}

export function Sidebar({ collapsed, onToggle }: SidebarProps) {
  const settings = useAppStore((state) => state.settings);
  const saveSettings = useAppStore((state) => state.saveSettings);

  const items = [
    { to: '/', label: t(settings.language, 'navDashboard'), icon: Gauge },
    { to: '/logs', label: t(settings.language, 'navLogs'), icon: Logs },
    { to: '/settings', label: t(settings.language, 'navSettings'), icon: Settings },
    { to: '/about', label: t(settings.language, 'navAbout'), icon: Info },
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
        <button
          type="button"
          className="sidebar-toggle"
          title={collapsed ? 'Развернуть боковую панель' : 'Свернуть боковую панель'}
          aria-label={collapsed ? 'Развернуть боковую панель' : 'Свернуть боковую панель'}
          onClick={onToggle}
        >
          {collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
        </button>
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
              ? 'Sidecar Xray в режиме TUN.'
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
