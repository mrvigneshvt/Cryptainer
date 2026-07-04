import React, { useState } from 'react';
import { DropZone } from '../../UI';
import type { VaultFileMeta } from '../../../types/vault';
import { formatBytes } from '../../../utils/format';
import './EditView.css';

interface EditViewProps {
  files: VaultFileMeta[];
  onSave: (password: string, filesToRemove: string[], newFiles: File[]) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

function FileSvg({ mime }: { mime: string }) {
  if (mime.startsWith('image/')) {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <polyline points="21 15 16 10 5 21" />
      </svg>
    );
  }
  if (mime.startsWith('video/')) {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polygon points="23 7 16 12 23 17 23 7" />
        <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
      </svg>
    );
  }
  if (mime.startsWith('audio/')) {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
    );
  }
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
    </svg>
  );
}

export const EditView: React.FC<EditViewProps> = ({
  files,
  onSave,
  onCancel,
  isLoading = false,
}) => {
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
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

  const hasChanges = filesToRemove.length > 0 || newFiles.length > 0;

  return (
    <div className="edit-view">
      <h2 className="edit-title">Edit Container</h2>
      <p className="edit-subtitle">Remove files or add new ones</p>

      <div className="edit-section">
        <h3 className="edit-section-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          Current Files ({files.length})
        </h3>
        <div className="edit-file-list">
          {files.map(file => {
            const isMarked = filesToRemove.includes(file.id);
            return (
              <div key={file.id} className={`edit-file-item ${isMarked ? 'edit-file-removing' : ''}`}>
                <div className="edit-file-icon">
                  <FileSvg mime={file.mime} />
                </div>
                <span className="edit-file-name">{file.name}</span>
                <span className="edit-file-size">{formatBytes(file.size)}</span>
                <button
                  type="button"
                  className={`edit-file-toggle ${isMarked ? 'edit-file-undo' : ''}`}
                  onClick={() => toggleRemove(file.id)}
                  aria-label={isMarked ? 'Undo remove' : 'Remove file'}
                >
                  {isMarked ? (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <polyline points="1 4 1 10 7 10" />
                      <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
                    </svg>
                  ) : (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <polyline points="3 6 5 6 21 6" />
                      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                    </svg>
                  )}
                </button>
              </div>
            );
          })}
        </div>
      </div>

      <div className="edit-section">
        <h3 className="edit-section-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          Add New Files
        </h3>
        {showDropZone ? (
          <DropZone
            onFilesSelected={files => { setNewFiles(files); if (files.length === 0) setShowDropZone(false); }}
            existingFiles={newFiles}
          />
        ) : (
          <button type="button" className="edit-add-btn" onClick={() => setShowDropZone(true)}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            Add Files
          </button>
        )}
      </div>

      <div className="edit-password-section">
        <div className="edit-password-wrapper">
          <input
            type={showPassword ? 'text' : 'password'}
            value={password}
            onChange={e => setPassword(e.target.value)}
            placeholder="Enter password to save changes"
            className="edit-password-input"
          />
          <button
            type="button"
            className="edit-password-toggle"
            onClick={() => setShowPassword(!showPassword)}
            aria-label={showPassword ? 'Hide password' : 'Show password'}
          >
            {showPassword ? (
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
                <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
                <line x1="1" y1="1" x2="23" y2="23" />
              </svg>
            ) : (
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            )}
          </button>
        </div>
      </div>

      <div className="edit-actions">
        <button className="edit-btn edit-btn-secondary" onClick={onCancel}>
          Cancel
        </button>
        <button
          className="edit-btn edit-btn-primary"
          onClick={handleSave}
          disabled={!password || !hasChanges || isLoading}
        >
          {isLoading ? (
            <><span className="edit-spinner" /> Saving…</>
          ) : (
            <>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
                <polyline points="17 21 17 13 7 13 7 21" />
                <polyline points="7 3 7 8 15 8" />
              </svg>
              Save & Re-encrypt
            </>
          )}
        </button>
      </div>
    </div>
  );
};
