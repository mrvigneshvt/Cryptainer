import React from 'react';
import './Button.css';

interface ButtonProps {
  children: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  loading?: boolean;
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md';
  type?: 'button' | 'submit';
}

export const Button: React.FC<ButtonProps> = ({
  children,
  onClick,
  disabled = false,
  loading = false,
  variant = 'primary',
  size = 'md',
  type = 'button',
}) => {
  return (
    <button
      type={type}
      className={`btn btn-${variant} btn-${size} ${loading ? 'btn-loading' : ''}`}
      onClick={onClick}
      disabled={disabled || loading}
    >
      {loading && <span className="btn-spinner" />}
      {children}
    </button>
  );
};
