import React, { useState, useCallback, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { ProgressBar } from '../../UI';
import { useTauriProgress } from '../../../hooks/useTauriProgress';
import { useVaultStore } from '../../../store/vaultStore';
import { useMediaQuery } from '../../../hooks/useMediaQuery';
import { formatBytes } from '../../../utils/format';
import type { VaultFileMeta, DownloadResult } from '../../../types/vault';
import './DownloadView.css';

interface DownloadViewProps {
  containerId: string;
  files: VaultFileMeta[];
  onClose: () => void;
}

export const DownloadView: React.FC<DownloadViewProps> = ({
  containerId,
  files,
  onClose,
}) => {
  const { downloadFiles, getDownloadDir } = useVaultStore();
  const { isMobile, isTablet } = useMediaQuery();
  const isSmallScreen = isMobile || isTablet;
  const [selectedIds, setSelectedIds] = useState<Set<string>>(
    new Set(files.map(f => f.id))
  );
  const [destDir, setDestDir] = useState<string | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [results, setResults] = useState<DownloadResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { progress } = useTauriProgress(isDownloading);

  // On mobile, auto-fetch the downloads directory (no folder picker available)
  useEffect(() => {
    if (isSmallScreen && !destDir) {
      getDownloadDir()
        .then(dir => { setDestDir(dir); })
        .catch(e => setError(String(e)));
    }
  }, [isSmallScreen, destDir, getDownloadDir]);

  const handleChooseFolder = useCallback(async () => {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (dir) {
        setDestDir(dir);
        setError(null);
      }
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const toggleFile = (id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAll = () => {
    if (selectedIds.size === files.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(files.map(f => f.id)));
    }
  };

  const doDownload = async (useAll: boolean) => {
    if (!destDir) {
      setError('Please choose a destination folder first');
      return;
    }
    setIsDownloading(true);
    setResults(null);
    setError(null);
    try {
      const ids = useAll ? files.map(f => f.id) : Array.from(selectedIds);
      if (ids.length === 0) {
        setError('No files selected');
        setIsDownloading(false);
        return;
      }
      const res = await downloadFiles(containerId, ids, destDir);
      setResults(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsDownloading(false);
    }
  };

  const successCount = results ? results.filter(r => !r.error).length : 0;
  const failCount = results ? results.filter(r => r.error).length : 0;

  return (
    <div className="download-view">
      <div className="download-header">
        <h3>Download Files</h3>
        <p className="download-subtitle">
          {files.length} file{files.length !== 1 ? 's' : ''} available
        </p>
      </div>

      {/* Folder picker — desktop only; mobile auto-uses Downloads */}
      {isSmallScreen ? (
        <div className="download-folder">
          <label className="download-label">Destination folder</label>
          <div className="download-folder-row">
            <input
              className="download-folder-input"
              type="text"
              readOnly
              value={destDir || 'Detecting…'}
              placeholder="Downloads"
            />
            <span className="download-folder-hint">Saving to Downloads</span>
          </div>
        </div>
      ) : (
        <div className="download-folder">
          <label className="download-label">Destination folder</label>
          <div className="download-folder-row">
            <input
              className="download-folder-input"
              type="text"
              readOnly
              value={destDir || ''}
              placeholder="No folder selected"
            />
            <button className="download-btn download-btn-secondary" onClick={handleChooseFolder}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
              </svg>
              Choose folder
            </button>
          </div>
        </div>
      )}

      {/* File list with checkboxes */}
      <div className="download-files">
        <div className="download-files-header">
          <label className="download-checkbox">
            <input
              type="checkbox"
              checked={selectedIds.size === files.length && files.length > 0}
              onChange={toggleAll}
            />
            <span>Select all</span>
          </label>
          <span className="download-selected-count">
            {selectedIds.size} selected
          </span>
        </div>
        <div className="download-file-list">
          {files.map(file => (
            <label key={file.id} className="download-file-row">
              <input
                type="checkbox"
                checked={selectedIds.has(file.id)}
                onChange={() => toggleFile(file.id)}
              />
              <div className="download-file-icon">
                <FileSvg mime={file.mime} />
              </div>
              <div className="download-file-info">
                <span className="download-file-name" title={file.name}>{file.name}</span>
                <span className="download-file-size">{formatBytes(file.size)}</span>
              </div>
            </label>
          ))}
        </div>
      </div>

      {/* Progress bar during download */}
      {isDownloading && (
        <ProgressBar
          operation={progress?.operation ?? 'decrypt'}
          current={progress?.current ?? 0}
          total={progress?.total ?? files.length}
          fileName={progress?.file_name ?? undefined}
          bytesProcessed={progress?.bytes_processed}
          bytesTotal={progress?.bytes_total}
          message={progress?.message ?? 'Downloading…'}
          indeterminate={!progress || progress.total === 0}
        />
      )}

      {/* Action buttons */}
      <div className="download-actions">
        <button
          className="download-btn download-btn-secondary"
          onClick={onClose}
          disabled={isDownloading}
        >
          Cancel
        </button>
        <div className="download-actions-right">
          <button
            className="download-btn download-btn-primary"
            onClick={() => doDownload(false)}
            disabled={isDownloading || !destDir || selectedIds.size === 0}
          >
            {isDownloading ? 'Downloading…' : `Download Selected (${selectedIds.size})`}
          </button>
          <button
            className="download-btn download-btn-accent"
            onClick={() => doDownload(true)}
            disabled={isDownloading || !destDir}
          >
            {isDownloading ? 'Downloading…' : 'Download All'}
          </button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="download-error">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
          <span>{error}</span>
        </div>
      )}

      {/* Results summary */}
      {results && (
        <div className={`download-results ${failCount > 0 ? 'download-results-partial' : 'download-results-success'}`}>
          <div className="download-results-summary">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              {failCount === 0 ? (
                <>
                  <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                  <polyline points="22 4 12 14.01 9 11.01" />
                </>
              ) : (
                <><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></>
              )}
            </svg>
            <span>
              {successCount} file{successCount !== 1 ? 's' : ''} written to {destDir}
            </span>
          </div>
          {failCount > 0 && (
            <div className="download-results-failures">
              {results.filter(r => r.error).map(r => (
                <div key={r.file_id} className="download-result-failure">
                  <span className="download-failure-id">File {r.file_id}</span>
                  <span className="download-failure-msg">{r.error}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

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
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
    </svg>
  );
}
