import React, { useState } from 'react';
import { Input, Button } from '../../UI';
import type { ContainerMeta } from '../../../types/vault';
import './LockView.css';

interface LockViewProps {
  container: ContainerMeta;
  onUnlock: (password: string) => void;
  isLoading?: boolean;
}

export const LockView: React.FC<LockViewProps> = ({ 
  container, 
  onUnlock,
  isLoading = false 
}) => {
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);

  const handleUnlock = () => {
    if (!password) {
      setError('Please enter your password');
      return;
    }
    setError(null);
    onUnlock(password);
  };

  return (
    <div className="lock-view">
      <div className="lock-icon">🔒</div>
      <h2>{container.name}</h2>
      <p className="lock-subtitle">
        {container.file_count} files · {formatBytes(container.total_size)}
      </p>

      {container.hint && (
        <div className="password-hint">
          <span>Hint: {container.hint}</span>
        </div>
      )}

      <div className="unlock-form">
        <Input
          type="password"
          value={password}
          onChange={setPassword}
          placeholder="Enter password to unlock"
        />

        {error && <div className="unlock-error">{error}</div>}

        <Button 
          variant="primary" 
          onClick={handleUnlock}
          loading={isLoading}
        >
          Unlock Container
        </Button>
      </div>
    </div>
  );
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}
