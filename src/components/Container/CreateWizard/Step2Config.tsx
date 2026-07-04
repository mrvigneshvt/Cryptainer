import React, { useState } from 'react';
import { PasswordStrength } from '../../UI';
import { SECURITY_PRESETS, type KdfParams } from '../../../types/vault';
import './Step2Config.css';

interface Step2ConfigProps {
  onBack: () => void;
  onCreate: (config: {
    password: string;
    kdfParams: KdfParams;
    hint?: string;
  }) => void;
  isLoading?: boolean;
}

export const Step2Config: React.FC<Step2ConfigProps> = ({
  onBack,
  onCreate,
  isLoading = false,
}) => {
  const [selectedPreset, setSelectedPreset] = useState(1); // Standard
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [hint, setHint] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleCreate = () => {
    if (password.length < 4) {
      setError('Password must be at least 4 characters');
      return;
    }
    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }
    onCreate({
      password,
      kdfParams: SECURITY_PRESETS[selectedPreset].params,
      hint: hint.trim() || undefined,
    });
  };

  return (
    <div className="step2-config">
      <div className="step2-header">
        <h2 className="step2-title">Configure Security</h2>
        <p className="step2-subtitle">Step 2 of 2 — Set encryption options</p>
      </div>

      <div className="step2-field">
        <label className="step2-label">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
          Security Level
        </label>
        <div className="preset-grid">
          {SECURITY_PRESETS.map((preset, index) => (
            <button
              key={index}
              type="button"
              className={`preset-card ${selectedPreset === index ? 'active' : ''}`}
              onClick={() => setSelectedPreset(index)}
            >
              <div className="preset-card-top">
                {index === 0 && (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                  </svg>
                )}
                {index === 1 && (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                    <polyline points="9 12 11 14 15 10" />
                  </svg>
                )}
                {index === 2 && (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                    <polyline points="9 11 12 14 16 10" />
                  </svg>
                )}
                {index === 3 && (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                    <line x1="12" y1="8" x2="12" y2="11" />
                    <line x1="12" y1="14" x2="12.01" y2="14" />
                  </svg>
                )}
                <span className="preset-name">{preset.label}</span>
              </div>
              <div className="preset-details">
                <span>{preset.params.memory_kb! / 1024}MB memory</span>
                <span>·</span>
                <span>{preset.params.iterations} iteration{preset.params.iterations !== 1 ? 's' : ''}</span>
                {preset.params.parallelism && preset.params.parallelism > 1 && (
                  <>
                    <span>·</span>
                    <span>{preset.params.parallelism} threads</span>
                  </>
                )}
              </div>
            </button>
          ))}
        </div>
      </div>

      <div className="step2-field">
        <label className="step2-label">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
          Password
        </label>
        <div className="step2-password-inputs">
          <div className="step2-password-wrapper">
            <input
              type={showPassword ? 'text' : 'password'}
              value={password}
              onChange={e => setPassword(e.target.value)}
              placeholder="Enter a strong password"
              className="step2-input"
            />
            <button
              type="button"
              className="step2-password-toggle"
              onClick={() => setShowPassword(!showPassword)}
              aria-label={showPassword ? 'Hide password' : 'Show password'}
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
          <PasswordStrength password={password} />
          <input
            type={showPassword ? 'text' : 'password'}
            value={confirmPassword}
            onChange={e => setConfirmPassword(e.target.value)}
            placeholder="Confirm your password"
            className="step2-input"
          />
        </div>
      </div>

      <div className="step2-field">
        <label className="step2-label">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          Password Hint (Optional)
        </label>
        <input
          type="text"
          value={hint}
          onChange={e => setHint(e.target.value)}
          placeholder="e.g., My usual passphrase"
          className="step2-input"
        />
      </div>

      {error && <div className="step2-error">{error}</div>}

      <div className="step2-actions">
        <button className="step2-btn step2-btn-secondary" onClick={onBack}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="15 18 9 12 15 6" />
          </svg>
          Back
        </button>
        <button
          className="step2-btn step2-btn-primary"
          onClick={handleCreate}
          disabled={isLoading}
        >
          {isLoading ? (
            <><span className="step2-spinner" /> Encrypting\u2026</>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
              Create & Encrypt
            </>
          )}
        </button>
      </div>
    </div>
  );
};
