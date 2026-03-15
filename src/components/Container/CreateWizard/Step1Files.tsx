import React, { useState } from 'react';
import { DropZone, Input, Button } from '../../UI';
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
      <h2>Create New Container</h2>
      <p className="step-subtitle">Step 1 of 2: Select files to encrypt</p>

      <Input
        label="Container Name"
        value={name}
        onChange={setName}
        placeholder="My Encrypted Files"
      />

      <div className="dropzone-section">
        <label className="section-label">Files to Encrypt</label>
        <DropZone onFilesSelected={setFiles} existingFiles={files} />
      </div>

      {error && <div className="step-error">{error}</div>}

      <div className="step-actions">
        <Button variant="primary" onClick={handleNext}>
          Next: Configure Security →
        </Button>
      </div>
    </div>
  );
};
