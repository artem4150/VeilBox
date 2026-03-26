import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  return (
    <div className="app-shell app-shell-collapsed">
      <aside className="shell-sidebar">
        <Sidebar collapsed />
      </aside>
      <main className="shell-main">{children}</main>
    </div>
  );
}
