import React, { useState } from 'react';
import { Input, Button, PasswordStrength } from '../../UI';
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
  isLoading = false 
}) => {
  const [selectedPreset, setSelectedPreset] = useState(1); // Standard
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [hint, setHint] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleCreate = () => {
    if (password.length < 8) {
      setError('Password must be at least 8 characters');
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
      <h2>Configure Security</h2>
      <p className="step-subtitle">Step 2 of 2: Set encryption options</p>

      <div className="preset-selector">
        <label className="section-label">Security Level</label>
        <div className="preset-options">
          {SECURITY_PRESETS.map((preset, index) => (
            <button
              key={index}
              type="button"
              className={`preset-btn ${selectedPreset === index ? 'active' : ''}`}
              onClick={() => setSelectedPreset(index)}
            >
              <span className="preset-label">{preset.label}</span>
              <span className="preset-params">
                {preset.params.memory_kb! / 1024}MB · {preset.params.iterations} iter
              </span>
            </button>
          ))}
        </div>
      </div>

      <div className="password-section">
        <Input
          label="Password"
          type="password"
          value={password}
          onChange={setPassword}
          placeholder="Enter a strong password"
        />
        <PasswordStrength password={password} />

        <Input
          label="Confirm Password"
          type="password"
          value={confirmPassword}
          onChange={setConfirmPassword}
          placeholder="Confirm your password"
        />

        <Input
          label="Password Hint (Optional)"
          type="text"
          value={hint}
          onChange={setHint}
          placeholder="e.g., My usual passphrase"
        />
      </div>

      {error && <div className="step-error">{error}</div>}

      <div className="step-actions">
        <Button variant="secondary" onClick={onBack}>
          ← Back
        </Button>
        <Button 
          variant="primary" 
          onClick={handleCreate}
          loading={isLoading}
        >
          Create & Encrypt
        </Button>
      </div>
    </div>
  );
};
