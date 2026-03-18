import type { ReactNode } from 'react';

interface PanelProps {
  title: string;
  description?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function Panel({ title, description, action, children, className }: PanelProps) {
  return (
    <section className={`panel${className ? ` ${className}` : ''}`}>
      <div className="panel-header">
        <div>
          <h2>{title}</h2>
          {description ? <p>{description}</p> : null}
        </div>
        {action ? <div>{action}</div> : null}
      </div>
      {children}
    </section>
  );
}
