import React from 'react';
import type { VaultFileMeta, ContainerMeta } from '../../../types/vault';
import { formatBytes } from '../../../utils/format';
import './OpenView.css';

interface OpenViewProps {
  container: ContainerMeta;
  files: VaultFileMeta[];
  onEdit: () => void;
  onLock: () => void;
  onPreview: (file: VaultFileMeta) => void;
}

export const OpenView: React.FC<OpenViewProps> = ({
  container,
  files,
  onEdit,
  onLock,
  onPreview,
}) => {
  return (
    <div className="open-view">
      <div className="open-header">
        <div>
          <h2>{container.name}</h2>
          <p className="open-subtitle">
            {files.length} files · {formatBytes(container.total_size)}
          </p>
        </div>
        <div className="open-actions">
          <button className="btn-secondary" onClick={onEdit}>
            Edit
          </button>
          <button className="btn-secondary" onClick={onLock}>
            Lock
          </button>
        </div>
      </div>

      <div className="file-list-container">
        {files.length === 0 ? (
          <div className="empty-files">
            <p>No files in this container</p>
          </div>
        ) : (
          <div className="files-grid">
            {files.map((file) => (
              <div
                key={file.id}
                className="file-card"
                onClick={() => onPreview(file)}
              >
                <div className="file-icon">{getFileIcon(file.mime)}</div>
                <div className="file-info">
                  <span className="file-name" title={file.name}>
                    {file.name}
                  </span>
                  <span className="file-size">{formatBytes(file.size)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
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
