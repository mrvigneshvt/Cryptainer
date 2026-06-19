import { create } from 'zustand';

export type ThemeId = 'prism' | 'cipher' | 'terminal';

interface ThemeState {
  theme: ThemeId;
  setTheme: (t: ThemeId) => void;
  toggleTheme: () => void;
}

const STORAGE_KEY = 'cryptainer.theme';

function readInitial(): ThemeId {
  if (typeof window === 'undefined') return 'prism';
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === 'prism' || stored === 'cipher' || stored === 'terminal') return stored;
  } catch {
    // ignore
  }
  return 'prism';
}

function applyTheme(theme: ThemeId) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-theme', theme);
  root.style.colorScheme = theme === 'cipher' || theme === 'terminal' ? 'dark' : 'light';
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // ignore
  }
}

const initial = readInitial();
if (typeof document !== 'undefined') applyTheme(initial);

export const useThemeStore = create<ThemeState>((set, get) => ({
  theme: initial,
  setTheme: (t) => {
    applyTheme(t);
    set({ theme: t });
  },
  toggleTheme: () => {
    const next: ThemeId = get().theme === 'prism' ? 'cipher' : 'prism';
    applyTheme(next);
    set({ theme: next });
  },
}));
