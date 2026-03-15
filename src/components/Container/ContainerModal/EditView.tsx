import React, { useState } from 'react';
import { DropZone, Button, Input } from '../../UI';
import type { VaultFileMeta } from '../../../types/vault';
import './EditView.css';

interface EditViewProps {
  files: VaultFileMeta[];
  onSave: (password: string, filesToRemove: string[], newFiles: File[]) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

export const EditView: React.FC<EditViewProps> = ({
  files,
  onSave,
  onCancel,
  isLoading = false,
}) => {
  const [password, setPassword] = useState('');
  const [filesToRemove, setFilesToRemove] = useState<string[]>([]);
  const [newFiles, setNewFiles] = useState<File[]>([]);
  const [showDropZone, setShowDropZone] = useState(false);

  const toggleRemove = (fileId: string) => {
    setFilesToRemove(prev =>
      prev.includes(fileId)
        ? prev.filter(id => id !== fileId)
        : [...prev, fileId]
    );
  };

  const handleSave = () => {
    if (!password) return;
    onSave(password, filesToRemove, newFiles);
  };

  return (
    <div className="edit-view">
      <h2>Edit Container</h2>
      <p className="edit-subtitle">Remove files or add new ones</p>

      <div className="edit-files-section">
        <h3>Current Files</h3>
        <div className="edit-files-list">
          {files.map((file) => {
            const isMarkedForRemoval = filesToRemove.includes(file.id);
            return (
              <div
                key={file.id}
                className={`edit-file-item ${isMarkedForRemoval ? 'marked-removal' : ''}`}
              >
                <span className="file-icon">{getFileIcon(file.mime)}</span>
                <span className="file-name">{file.name}</span>
                <button
                  type="button"
                  className={`toggle-remove-btn ${isMarkedForRemoval ? 'undo' : ''}`}
                  onClick={() => toggleRemove(file.id)}
                >
                  {isMarkedForRemoval ? 'Undo' : 'Remove'}
                </button>
              </div>
            );
          })}
        </div>
      </div>

      <div className="add-files-section">
        <h3>Add New Files</h3>
        {showDropZone ? (
          <DropZone
            onFilesSelected={(files) => {
              setNewFiles(files);
              if (files.length === 0) setShowDropZone(false);
            }}
            existingFiles={newFiles}
          />
        ) : (
          <button
            type="button"
            className="add-files-btn"
            onClick={() => setShowDropZone(true)}
          >
            + Add Files
          </button>
        )}
      </div>

      <div className="password-section">
        <Input
          type="password"
          value={password}
          onChange={setPassword}
          placeholder="Enter password to save changes"
        />
      </div>

      <div className="edit-actions">
        <Button variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={handleSave}
          loading={isLoading}
          disabled={!password || (filesToRemove.length === 0 && newFiles.length === 0)}
        >
          Save & Re-encrypt
        </Button>
      </div>
    </div>
  );
};

function getFileIcon(mime: string): string {
  if (mime.startsWith('image/')) return '🖼️';
  if (mime.startsWith('video/')) return '🎬';
  if (mime.startsWith('audio/')) return '🎵';
  if (mime.startsWith('text/')) return '📄';
  if (mime.includes('pdf')) return '📑';
  if (mime.includes('zip') || mime.includes('compressed')) return '📦';
  return '📎';
}
