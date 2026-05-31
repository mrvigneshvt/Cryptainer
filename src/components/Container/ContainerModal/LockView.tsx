import React, { useState } from 'react';
import { Input, Button } from '../../UI';
import type { ContainerMeta } from '../../../types/vault';
import { formatBytes } from '../../../utils/format';
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

