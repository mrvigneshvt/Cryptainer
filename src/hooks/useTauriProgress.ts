import { useState, useEffect, useRef, useCallback } from 'react';
import { isTauri } from '../utils/tauri';
import type { ProgressBarProps } from '../components/UI';

export interface ProgressPayload {
  operation: ProgressBarProps['operation'];
  current: number;
  total: number;
  file_name?: string | null;
  bytes_processed: number;
  bytes_total: number;
  message: string;
}

export interface UseTauriProgressResult {
  progress: ProgressPayload | null;
  isActive: boolean;
}

/**
 * Listen to "cryptainer://progress" events from the Tauri backend.
 *
 * Returns the latest progress payload and whether an operation is active.
 * In browser/mock mode, returns { progress: null, isActive: false }.
 * Auto-unlistens on unmount or when `active` becomes false.
 *
 * @param active - Set to true to start listening, false to stop and reset.
 */
export function useTauriProgress(active: boolean): UseTauriProgressResult {
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  const handleEvent = useCallback((event: { payload: ProgressPayload }) => {
    // Convert snake_case from Rust to camelCase for ProgressBar props
    const p = event.payload;
    setProgress({
      operation: p.operation as ProgressPayload['operation'],
      current: p.current,
      total: p.total,
      file_name: p.file_name,
      bytes_processed: p.bytes_processed,
      bytes_total: p.bytes_total,
      message: p.message,
    });
  }, []);

  useEffect(() => {
    if (!isTauri() || !active) {
      // Clean up listener if active turns false or in mock mode
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      if (!active) {
        setProgress(null);
      }
      return;
    }

    let cancelled = false;

    import('@tauri-apps/api/event').then(({ listen }) => {
      if (cancelled) return;
      listen<ProgressPayload>('cryptainer://progress', handleEvent).then((unlisten) => {
        if (cancelled) {
          unlisten();
          return;
        }
        unlistenRef.current = unlisten;
      });
    }).catch(() => {
      // Not in Tauri environment — silently no-op
    });

    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setProgress(null);
    };
  }, [active, handleEvent]);

  const isActive = progress !== null && progress.operation !== 'idle';

  return { progress, isActive };
}
