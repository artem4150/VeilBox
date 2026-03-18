import type { ButtonHTMLAttributes, ReactNode } from 'react';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  wide?: boolean;
}

export function Button({
  children,
  variant = 'primary',
  wide,
  className,
  ...rest
}: ButtonProps) {
  return (
    <button
      className={`button button-${variant}${wide ? ' button-wide' : ''}${className ? ` ${className}` : ''}`}
      {...rest}
    >
      {children}
    </button>
  );
}
