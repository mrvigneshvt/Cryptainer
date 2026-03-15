import React, { useState } from 'react';
import './Input.css';

interface InputProps {
  label?: string;
  type?: 'text' | 'password';
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  error?: string;
  disabled?: boolean;
}

export const Input: React.FC<InputProps> = ({
  label,
  type = 'text',
  value,
  onChange,
  placeholder,
  error,
  disabled = false,
}) => {
  const [showPassword, setShowPassword] = useState(false);
  const inputType = type === 'password' && showPassword ? 'text' : type;

  return (
    <div className="input-wrapper">
      {label && <label className="input-label">{label}</label>}
      <div className="input-container">
        <input
          type={inputType}
          className={`input-field ${error ? 'input-error' : ''}`}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          disabled={disabled}
        />
        {type === 'password' && (
          <button
            type="button"
            className="input-toggle"
            onClick={() => setShowPassword(!showPassword)}
            tabIndex={-1}
          >
            {showPassword ? '👁️' : '👁️‍🗨️'}
          </button>
        )}
      </div>
      {error && <span className="input-error-msg">{error}</span>}
    </div>
  );
};
