import React, { useState } from 'react';
import type { ContainerMeta } from '../../../types/vault';
import { formatBytes } from '../../../utils/format';
import './LockView.css';

interface LockViewProps {
  container: ContainerMeta;
  onUnlock: (password: string) => void;
  isLoading?: boolean;
  error?: string | null;
}

export const LockView: React.FC<LockViewProps> = ({
  container,
  onUnlock,
  isLoading = false,
  error: serverError,
}) => {
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleUnlock = () => {
    if (!password) {
      setError('Please enter your password');
      return;
    }
    setError(null);
    onUnlock(password);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleUnlock();
  };

  return (
    <div className="lock-view">
      <div className="lock-icon-container">
        <svg className="lock-svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
          <path d="M7 11V7a5 5 0 0 1 10 0v4" />
        </svg>
      </div>

      <h2 className="lock-title">{container.name}</h2>
      <p className="lock-subtitle">
        {container.file_count} file{container.file_count !== 1 ? 's' : ''} · {formatBytes(container.total_size)}
      </p>

      {container.hint && (
        <div className="lock-hint">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          <span>{container.hint}</span>
        </div>
      )}

      <div className="lock-form">
        <div className="lock-password-wrapper">
          <input
            type={showPassword ? 'text' : 'password'}
            value={password}
            onChange={e => { setPassword(e.target.value); setError(null); }}
            onKeyDown={handleKeyDown}
            placeholder="Enter password to unlock"
            className="lock-password-input"
            autoFocus
            disabled={isLoading}
          />
          <button
            type="button"
            className="lock-password-toggle"
            onClick={() => setShowPassword(!showPassword)}
            aria-label={showPassword ? 'Hide password' : 'Show password'}
            tabIndex={-1}
          >
            {showPassword ? (
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                <line x1="1" y1="1" x2="23" y2="23" />
              </svg>
            ) : (
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            )}
          </button>
        </div>

        {/* Password strength indicator */}
        {password.length > 0 && (
          <div className="lock-strength">
            <div className="lock-strength-bars">
              <div className={`lock-strength-bar ${password.length >= 4 ? 'active' : ''}`} />
              <div className={`lock-strength-bar ${password.length >= 8 ? 'active' : ''}`} />
              <div className={`lock-strength-bar ${password.length >= 12 ? 'active' : ''}`} />
            </div>
            <span className="lock-strength-label">
              {password.length < 4 ? 'Weak' : password.length < 8 ? 'Fair' : password.length < 12 ? 'Good' : 'Strong'}
            </span>
          </div>
        )}

        {(error || serverError) && <div className="lock-error">{serverError || error}</div>}

        <button
          className="lock-unlock-btn"
          onClick={handleUnlock}
          disabled={isLoading}
        >
          {isLoading ? (
            <><span className="lock-spinner" /> Unlocking\u2026</>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
              Unlock Container
            </>
          )}
        </button>
      </div>
    </div>
  );
};
