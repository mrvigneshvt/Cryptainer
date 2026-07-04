import React from 'react';
import { createPortal } from 'react-dom';
import { ProgressBar } from './ProgressBar';
import type { ProgressPayload } from '../../hooks/useTauriProgress';
import './ProgressOverlay.css';

export interface ProgressOverlayProps {
  open: boolean;
  progress: ProgressPayload | null;
  fallbackMessage?: string;
}

export const ProgressOverlay: React.FC<ProgressOverlayProps> = ({
  open,
  progress,
  fallbackMessage,
}) => {
  if (!open) return null;

  return createPortal(
    <div className="progress-overlay" role="dialog" aria-modal="true">
      <div className="progress-overlay-card">
        <ProgressBar
          operation={progress?.operation ?? 'idle'}
          current={progress?.current ?? 0}
          total={progress?.total ?? 0}
          fileName={progress?.file_name ?? undefined}
          bytesProcessed={progress?.bytes_processed}
          bytesTotal={progress?.bytes_total}
          message={progress?.message ?? fallbackMessage}
          indeterminate={!progress || progress.total === 0}
        />
      </div>
    </div>,
    document.body,
  );
};
