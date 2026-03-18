import type { ThemeMode } from '../types';

export function resolveTheme(theme: ThemeMode) {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }

  return theme;
}

export function applyTheme(theme: ThemeMode) {
  document.documentElement.dataset.theme = resolveTheme(theme);
}
