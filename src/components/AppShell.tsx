import { ReactNode, useState } from 'react';
import { Sidebar } from './Sidebar';

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(true);

  return (
    <div className={`app-shell${sidebarCollapsed ? ' app-shell-collapsed' : ''}`}>
      <aside className="shell-sidebar">
        <Sidebar
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed((value) => !value)}
        />
      </aside>
      <main className="shell-main">{children}</main>
    </div>
  );
}
