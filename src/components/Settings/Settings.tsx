import React, { useState, useEffect } from 'react';
import { Modal, Button } from '../UI';
import { useThemeStore, ThemeId } from '../../store/themeStore';
import { SECURITY_PRESETS } from '../../types/vault';
import './Settings.css';

const THEME_ORDER: { id: ThemeId; label: string }[] = [
  { id: 'prism', label: 'Light' },
  { id: 'cipher', label: 'Dark' },
  { id: 'terminal', label: 'Terminal' },
];

interface SettingsData {
  defaultPreset: number;
  autoLockTimeout: number | null;
}

const DEFAULT_SETTINGS: SettingsData = {
  defaultPreset: 1,
  autoLockTimeout: 5 * 60 * 1000,
};

interface SettingsProps {
  onClose: () => void;
}

export const Settings: React.FC<SettingsProps> = ({ onClose }) => {
  const [settings, setSettings] = useState<SettingsData>(DEFAULT_SETTINGS);
  const [hasChanges, setHasChanges] = useState(false);
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  useEffect(() => {
    const saved = localStorage.getItem('cryptainer_settings');
    if (saved) {
      try {
        setSettings({ ...DEFAULT_SETTINGS, ...JSON.parse(saved) });
      } catch {
        // Ignore parse errors
      }
    }
  }, []);

  const saveSettings = () => {
    localStorage.setItem('cryptainer_settings', JSON.stringify(settings));
    setHasChanges(false);
    onClose();
  };

  const updateSetting = <K extends keyof SettingsData>(key: K, value: SettingsData[K]) => {
    setSettings(prev => ({ ...prev, [key]: value }));
    setHasChanges(true);
  };

  const timeoutOptions = [
    { label: '1 minute', value: 60 * 1000 },
    { label: '5 minutes', value: 5 * 60 * 1000 },
    { label: '15 minutes', value: 15 * 60 * 1000 },
    { label: 'Never', value: null },
  ];

  return (
    <Modal open={true} onClose={onClose} title="Settings" size="md">
      <div className="settings">
        <section className="settings-section">
          <h3>Security</h3>
          
          <div className="setting-item">
            <label>Default Security Preset</label>
            <div className="preset-options">
              {SECURITY_PRESETS.map((preset, index) => (
                <button
                  key={index}
                  type="button"
                  className={`preset-btn ${settings.defaultPreset === index ? 'active' : ''}`}
                  onClick={() => updateSetting('defaultPreset', index)}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </div>

          <div className="setting-item">
            <label>Auto-Lock Timeout</label>
            <select
              className="settings-select"
              value={settings.autoLockTimeout ?? 'null'}
              onChange={(e) => updateSetting('autoLockTimeout', e.target.value === 'null' ? null : Number(e.target.value))}
            >
              {timeoutOptions.map(opt => (
                <option key={opt.label} value={opt.value ?? 'null'}>
                  {opt.label}
                </option>
              ))}
            </select>
            <p className="setting-hint">
              Automatically lock unlocked containers after period of inactivity
            </p>
          </div>
        </section>

        <section className="settings-section">
          <h3>Appearance</h3>
          
          <div className="setting-item">
            <label>Theme</label>
            <div className="theme-options">
              {THEME_ORDER.map(({ id, label }) => (
                <button
                  key={id}
                  type="button"
                  className={`theme-btn ${theme === id ? 'active' : ''}`}
                  onClick={() => setTheme(id)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className="settings-section">
          <h3>About</h3>
          <div className="about-info">
            <p><strong>Cryptainer</strong></p>
            <p>Version 0.1.0</p>
            <p className="about-desc">
              Offline encrypted container manager with AES-256-GCM encryption
            </p>
            <p className="about-author">
              Built by: <a href="https://forked.online" target="_blank" rel="noopener noreferrer">VIXYZ</a>
            </p>
            <a href="mailto:vixyz@forked.online" className="about-contact">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
                <polyline points="22,6 12,13 2,6" />
              </svg>
              Contact
            </a>
          </div>
        </section>

        <div className="settings-actions">
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button 
            variant="primary" 
            onClick={saveSettings}
            disabled={!hasChanges}
          >
            Save Changes
          </Button>
        </div>
      </div>
    </Modal>
  );
};