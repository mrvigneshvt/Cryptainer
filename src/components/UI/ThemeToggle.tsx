import React from 'react';
import { useThemeStore } from '../../store/themeStore';
import './ThemeToggle.css';

export const ThemeToggle: React.FC = () => {
  const theme = useThemeStore((s) => s.theme);
  const toggleTheme = useThemeStore((s) => s.toggleTheme);
  const isCipher = theme === 'cipher';

  return (
    <button
      type="button"
      className={`theme-toggle ${isCipher ? 'is-cipher' : 'is-prism'}`}
      onClick={toggleTheme}
      aria-label={isCipher ? 'Switch to PRISM (light) theme' : 'Switch to CIPHER (dark) theme'}
      title={isCipher ? 'PRISM (light)' : 'CIPHER (dark)'}
    >
      <span className="theme-toggle-track">
        <span className="theme-toggle-thumb">
          {isCipher ? (
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="square" strokeLinejoin="miter" aria-hidden="true">
              <polyline points="4 17 10 11 4 5" />
              <line x1="12" y1="19" x2="20" y2="19" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="4" />
              <line x1="12" y1="2" x2="12" y2="5" />
              <line x1="12" y1="19" x2="12" y2="22" />
              <line x1="2" y1="12" x2="5" y2="12" />
              <line x1="19" y1="12" x2="22" y2="12" />
              <line x1="4.93" y1="4.93" x2="7.05" y2="7.05" />
              <line x1="16.95" y1="16.95" x2="19.07" y2="19.07" />
              <line x1="4.93" y1="19.07" x2="7.05" y2="16.95" />
              <line x1="16.95" y1="7.05" x2="19.07" y2="4.93" />
            </svg>
          )}
        </span>
      </span>
      <span className="theme-toggle-label">
        {isCipher ? 'CIPHER' : 'PRISM'}
      </span>
    </button>
  );
};
