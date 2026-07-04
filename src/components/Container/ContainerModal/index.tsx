import React, { useState, useEffect, useRef } from 'react';
import { Modal, ProgressOverlay } from '../../UI';
import { LockView } from './LockView';
import { OpenView } from './OpenView';
import { EditView } from './EditView';
import { DownloadView } from './DownloadView';
import { PreviewRouter } from '../../Preview';
import { useVaultStore } from '../../../store/vaultStore';
import { useTauriProgress } from '../../../hooks/useTauriProgress';
import type { ContainerMeta, FileInput, VaultFileMeta } from '../../../types/vault';
import './index.css';

interface ContainerModalProps {
  container: ContainerMeta;
  onClose: () => void;
}

type ViewState = 'locked' | 'open' | 'edit' | 'preview' | 'download';

export const ContainerModal: React.FC<ContainerModalProps> = ({
  container,
  onClose,
}) => {
  const [view, setView] = useState<ViewState>('locked');
  const [files, setFiles] = useState<VaultFileMeta[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewFile, setPreviewFile] = useState<VaultFileMeta | null>(null);
  const [previewData, setPreviewData] = useState<Uint8Array | null>(null);
  const viewRef = useRef(view);
  useEffect(() => { viewRef.current = view; });
  const previewFileRef = useRef(previewFile);
  useEffect(() => { previewFileRef.current = previewFile; });
  const { unlockContainer, lockContainer, getFileData, saveContainerEdits, releaseFileData } = useVaultStore();
  const { progress } = useTauriProgress(isLoading);
  const fallbackMessage = view === 'edit' ? 'Saving changes\u2026'
    : view === 'preview' ? 'Loading file\u2026'
    : 'Unlocking container\u2026';
  const handleUnlock = async (password: string) => {
    setError(null);
    setIsLoading(true);
    try {
      const fileList = await unlockContainer(container.id, password);
      setFiles(fileList);
      setView('open');
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handleDownload = () => {
    setView('download');
  };

  const handleCloseDownload = () => {
    setView('open');
  };

  const handleLock = async () => {
    // Release cached file data before locking
    if (previewFile) {
      releaseFileData(container.id, previewFile.id);
      setPreviewFile(null);
      setPreviewData(null);
    }
    await lockContainer(container.id);
    setView('locked');
    setFiles([]);
    setError(null);
  };

  const handleSaveEdits = async (
    password: string,
    filesToRemove: string[],
    newFiles: FileInput[]
  ) => {
    setError(null);
    setIsLoading(true);
    try {
      await saveContainerEdits(container.id, password, newFiles, filesToRemove);

      const updatedFiles = await unlockContainer(container.id, password);
      setFiles(updatedFiles);
      setView('open');
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handlePreview = async (file: VaultFileMeta) => {
    setError(null);
    setIsLoading(true);
    try {
      const data = await getFileData(container.id, file.id);
      setPreviewFile(file);
      setPreviewData(data);
      setView('preview');
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handleClosePreview = () => {
    if (previewFile) {
      releaseFileData(container.id, previewFile.id);
    }
    setPreviewFile(null);
    setPreviewData(null);
    setView('open');
  };

  useEffect(() => {
    return () => {
      if (previewFileRef.current) {
        releaseFileData(container.id, previewFileRef.current.id);
      }
      if (viewRef.current !== 'locked') {
        lockContainer(container.id);
      }
    };
  }, []);

  return (
    <Modal open={true} onClose={onClose} size="lg">
      <div className="container-modal">
        {error && view !== 'locked' && (
          <div className="container-modal-error">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <span>{error}</span>
          </div>
        )}
        {view === 'locked' && (
          <LockView
            container={container}
            onUnlock={handleUnlock}
            isLoading={isLoading}
            error={error}
          />
        )}

        {view === 'open' && (
          <OpenView
            container={container}
            files={files}
            onEdit={() => setView('edit')}
            onLock={handleLock}
            onDownload={handleDownload}
            onPreview={handlePreview}
          />
        )}

        {view === 'download' && (
          <DownloadView
            containerId={container.id}
            files={files}
            onClose={handleCloseDownload}
          />
        )}

        {view === 'edit' && (
          <EditView
            files={files}
            onSave={handleSaveEdits}
            onCancel={() => setView('open')}
            isLoading={isLoading}
          />
        )}

        {view === 'preview' && previewFile && previewData && (
          <div className="preview-view">
            <div className="preview-header">
              <h3>{previewFile.name}</h3>
              <button className="preview-close" onClick={handleClosePreview} aria-label="Close preview">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
            <div className="preview-content">
              <PreviewRouter
                mime={previewFile.mime}
                data={previewData}
                name={previewFile.name}
                containerId={container.id}
                fileId={previewFile.id}
              />
            </div>
          </div>
        )}
      </div>
      <ProgressOverlay open={isLoading} progress={progress} fallbackMessage={fallbackMessage} />
    </Modal>
  );
};
