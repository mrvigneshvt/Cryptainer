import React, { useState } from 'react';
import { DropZone } from '../../UI';
import { formatBytes } from '../../../utils/format';
import './Step1Files.css';

interface Step1FilesProps {
  onNext: (data: { name: string; files: File[] }) => void;
}

export const Step1Files: React.FC<Step1FilesProps> = ({ onNext }) => {
  const [name, setName] = useState('');
  const [files, setFiles] = useState<File[]>([]);
  const [error, setError] = useState<string | null>(null);

  const handleNext = () => {
    if (!name.trim()) {
      setError('Please enter a container name');
      return;
    }
    if (files.length === 0) {
      setError('Please select at least one file');
      return;
    }
    onNext({ name: name.trim(), files });
  };

  return (
    <div className="step1-files">
      <div className="step1-header">
        <h2 className="step1-title">Create New Container</h2>
        <p className="step1-subtitle">Step 1 of 2 — Choose files to encrypt</p>
      </div>

      <div className="step1-field">
        <label className="step1-label">Container Name</label>
        <input
          type="text"
          value={name}
          onChange={e => setName(e.target.value)}
          placeholder="My Encrypted Files"
          className="step1-input"
          autoFocus
        />
      </div>

      <div className="step1-field">
        <label className="step1-label">Files to Encrypt</label>
        <DropZone onFilesSelected={setFiles} existingFiles={files} />

        {files.length > 0 && (
          <div className="step1-files-summary">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            <span>{files.length} file{files.length !== 1 ? 's' : ''} selected · {formatBytes(files.reduce((sum, f) => sum + f.size, 0))}</span>
          </div>
        )}
      </div>

      {error && <div className="step1-error">{error}</div>}

      <div className="step1-actions">
        <button className="step1-btn step1-btn-primary" onClick={handleNext}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="9 18 15 12 9 6" />
          </svg>
          Next: Security Settings
        </button>
      </div>
    </div>
  );
};
