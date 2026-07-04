import React from 'react';
import './ProgressBar.css';
export interface ProgressBarProps {
  operation: 'encrypt' | 'decrypt' | 'derive-key' | 'write-blob' | 'read-blob' | 'migrate' | 'import' | 'export' | 'idle';
  current: number;
  total: number;
  fileName?: string;
  bytesProcessed?: number;
  bytesTotal?: number;
  message?: string;
  indeterminate?: boolean;
  elapsedMs?: number;
  etaMs?: number;
  throughputBytesPerSec?: number;
  error?: string | null;
  compact?: boolean;
}

const OPERATION_VERB: Record<ProgressBarProps['operation'], string> = {
  encrypt:       'Encrypting',
  decrypt:       'Decrypting',
  'derive-key':  'Deriving key\u2026',
  'write-blob':  'Writing blob\u2026',
  'read-blob':   'Reading blob\u2026',
  migrate:       'Migrating\u2026',
  import:        'Importing',
  export:        'Exporting',
  idle:          '',
};

const OPERATION_ICON: Record<ProgressBarProps['operation'], string> = {
  encrypt:       '\u{1F512}', // 🔒
  decrypt:       '\u{1F4E5}', // 📥
  'derive-key':  '\u{1F511}', // 🔑
  'write-blob':  '\u{1F4BE}', // 💾
  'read-blob':   '\u{1F4C2}', // 📂
  migrate:       '\u{1F504}', // 🔄
  import:        '\u{1F4E5}', // 📥
  export:        '\u{1F4E4}', // 📤
  idle:          '',
};

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const clamped = Math.min(i, units.length - 1);
  const value = bytes / Math.pow(1024, clamped);
  return `${value.toFixed(clamped === 0 ? 0 : 1)} ${units[clamped]}`;
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec === 0) return '0 B/s';
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  const i = Math.floor(Math.log(bytesPerSec) / Math.log(1024));
  const clamped = Math.min(i, units.length - 1);
  const value = bytesPerSec / Math.pow(1024, clamped);
  return `${value.toFixed(clamped === 0 ? 0 : 1)} ${units[clamped]}`;
}

function formatEta(ms: number): string {
  if (ms <= 0) return '';
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) return `~${seconds}s left`;
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 60) return `~${minutes}m left`;
  const hours = Math.floor(minutes / 60);
  const remMins = minutes % 60;
  return `~${hours}h ${remMins}m left`;
}

function middleEllipsis(name: string, maxLen: number = 40): string {
  if (name.length <= maxLen) return name;
  const half = Math.floor((maxLen - 3) / 2);
  return `${name.slice(0, half)}\u2026${name.slice(name.length - half)}`;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  operation,
  current,
  total,
  fileName,
  bytesProcessed,
  bytesTotal,
  message,
  indeterminate,
  elapsedMs: _elapsedMs,
  etaMs,
  throughputBytesPerSec,
  error,
  compact,
}) => {
  const isIndeterminate = indeterminate || total === 0;
  const percent = isIndeterminate ? 0 : total > 0 ? Math.round((current / total) * 100) : 0;
  const hasBytes = bytesProcessed !== undefined && bytesTotal !== undefined;
  const verb = OPERATION_VERB[operation];
  const icon = OPERATION_ICON[operation];

  const className = [
    'progress',
    error ? 'progress-error' : '',
    compact ? 'progress-compact' : '',
  ].filter(Boolean).join(' ');

  return (
    <div className={className} role="progressbar" aria-valuenow={isIndeterminate ? undefined : percent} aria-valuemin={0} aria-valuemax={100}>
      {/* Header row: icon + verb */}
      <div className="progress-header">
        {icon && <span className="progress-icon">{icon}</span>}
        <span className="progress-verb">{verb}</span>
      </div>

      {/* Error message */}
      {error && (
        <div className="progress-error-msg">{error}</div>
      )}

      {/* File name row */}
      {fileName && !error && (
        <div className="progress-file" title={fileName}>
          {middleEllipsis(fileName)}
        </div>
      )}

      {/* Message row (indeterminate or custom) */}
      {message && !error && (
        <div className="progress-message">{message}</div>
      )}

      {/* The bar */}
      <div className="progress-track">
        {isIndeterminate ? (
          <div className="progress-shimmer" />
        ) : (
          <div className="progress-fill" style={{ width: `${percent}%` }} />
        )}
      </div>

      {/* Footer row: count + percentage (determinate only) */}
      {!isIndeterminate && !error && (
        <div className="progress-footer">
          <span className="progress-count">{current} / {total} files</span>
          <span className="progress-percent">{percent}%</span>
        </div>
      )}

      {/* Bytes/speed/ETA row */}
      {hasBytes && !isIndeterminate && !error && (
        <div className="progress-bytes">
          {formatBytes(bytesProcessed)} / {formatBytes(bytesTotal)}
          {throughputBytesPerSec !== undefined && (
            <span className="progress-speed">
              {' \u00B7 '}{formatSpeed(throughputBytesPerSec)}
            </span>
          )}
          {etaMs !== undefined && etaMs > 0 && (
            <span className="progress-eta">
              {' \u00B7 '}{formatEta(etaMs)}
            </span>
          )}
        </div>
      )}
    </div>
  );
};
