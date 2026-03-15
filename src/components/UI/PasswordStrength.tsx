import React from 'react';
import './PasswordStrength.css';

interface PasswordStrengthProps {
  password: string;
}

export const PasswordStrength: React.FC<PasswordStrengthProps> = ({ password }) => {
  const calculateStrength = (pwd: string): number => {
    let score = 0;
    if (pwd.length >= 8) score++;
    if (pwd.length >= 14) score++;
    if (/[A-Z]/.test(pwd)) score++;
    if (/[0-9]/.test(pwd)) score++;
    if (/[^A-Za-z0-9]/.test(pwd)) score++;
    return Math.min(score, 4);
  };

  const strength = calculateStrength(password);
  
  const labels = ['Very Weak', 'Weak', 'Fair', 'Good', 'Strong'];
  const colors = ['#ff4444', '#ff8800', '#ffcc00', '#88cc00', '#44cc44'];

  return (
    <div className="password-strength">
      <div className="strength-bars">
        {[0, 1, 2, 3].map((index) => (
          <div
            key={index}
            className={`strength-bar ${index < strength ? 'active' : ''}`}
            style={{
              backgroundColor: index < strength ? colors[strength] : undefined,
            }}
          />
        ))}
      </div>
      <span className="strength-label" style={{ color: colors[strength] }}>
        {password.length > 0 ? labels[strength] : ''}
      </span>
    </div>
  );
};
