import React from 'react';
import type { VaultFileMeta, ContainerMeta } from '../../../types/vault';
import { formatBytes } from '../../../utils/format';
import './OpenView.css';

interface OpenViewProps {
  container: ContainerMeta;
  files: VaultFileMeta[];
  onEdit: () => void;
  onLock: () => void;
  onDownload: () => void;
  onPreview: (file: VaultFileMeta) => void;
}

function FileSvg({ mime }: { mime: string }) {
  if (mime.startsWith('image/')) {
    return (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <polyline points="21 15 16 10 5 21" />
      </svg>
    );
  }
  if (mime.startsWith('video/')) {
    return (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polygon points="23 7 16 12 23 17 23 7" />
        <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
      </svg>
    );
  }
  if (mime.startsWith('audio/')) {
    return (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
    );
  }
  if (mime.startsWith('text/')) {
    return (
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10 9 9 9 8 9" />
      </svg>
    );
  }
  // Default file icon
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
    </svg>
  );
}

export const OpenView: React.FC<OpenViewProps> = ({
  container,
  files,
  onEdit,
  onLock,
  onDownload,
  onPreview,
}) => {
  return (
    <div className="open-view">
      <div className="open-header">
        <div>
          <h2 className="open-title">{container.name}</h2>
          <p className="open-subtitle">
            {files.length} file{files.length !== 1 ? 's' : ''} · {formatBytes(container.total_size)}
          </p>
        </div>
        <div className="open-actions">
          <button className="open-btn open-btn-secondary" onClick={onEdit}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
              <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
            Edit
          </button>
          <button className="open-btn open-btn-secondary" onClick={onDownload}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            Download
          </button>
          <button className="open-btn open-btn-secondary" onClick={onLock}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
              <path d="M7 11V7a5 5 0 0 1 10 0v4" />
            </svg>
            Lock
          </button>
        </div>
      </div>

      <div className="open-files">
        {files.length === 0 ? (
          <div className="open-empty">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1" strokeLinecap="round" strokeLinejoin="round" className="open-empty-icon">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            <p>No files in this container</p>
          </div>
        ) : (
          <div className="open-grid">
            {files.map(file => (
              <div
                key={file.id}
                className="open-file-card"
                onClick={() => onPreview(file)}
                role="button"
                tabIndex={0}
                onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onPreview(file); } }}
              >
                <div className="open-file-icon">
                  <FileSvg mime={file.mime} />
                </div>
                <div className="open-file-info">
                  <span className="open-file-name" title={file.name}>{file.name}</span>
                  <span className="open-file-size">{formatBytes(file.size)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
