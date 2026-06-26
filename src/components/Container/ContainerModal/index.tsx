import React, { useState, useEffect, useRef } from 'react';
import { Modal } from '../../UI';
import { LockView } from './LockView';
import { OpenView } from './OpenView';
import { EditView } from './EditView';
import { DownloadView } from './DownloadView';
import { PreviewRouter } from '../../Preview';
import { useVaultStore } from '../../../store/vaultStore';
import type { ContainerMeta, VaultFileMeta } from '../../../types/vault';
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
  const [previewFile, setPreviewFile] = useState<VaultFileMeta | null>(null);
  const [previewData, setPreviewData] = useState<Uint8Array | null>(null);
  const viewRef = useRef(view);
  useEffect(() => { viewRef.current = view; });
  const previewFileRef = useRef(previewFile);
  useEffect(() => { previewFileRef.current = previewFile; });
  const { unlockContainer, lockContainer, getFileData, saveContainerEdits, releaseFileData } = useVaultStore();

  const handleUnlock = async (password: string) => {
    setIsLoading(true);
    try {
      const fileList = await unlockContainer(container.id, password);
      setFiles(fileList);
      setView('open');
    } catch (e) {
      console.error('Unlock failed:', e);
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
  };

  const handleSaveEdits = async (
    password: string,
    filesToRemove: string[],
    newFiles: File[]
  ) => {
    setIsLoading(true);
    try {
      // Convert new files to FileInput format
      const fileInputs = await Promise.all(
        newFiles.map(async (file) => {
          const arrayBuffer = await file.arrayBuffer();
          return {
            name: file.name,
            mime: file.type || 'application/octet-stream',
            data: Array.from(new Uint8Array(arrayBuffer)),
          };
        })
      );

      await saveContainerEdits(container.id, password, fileInputs, filesToRemove);
      
      // Refresh the file list — this re-derives Argon2 on the Rust side
      const updatedFiles = await unlockContainer(container.id, password);
      setFiles(updatedFiles);
      setView('open');
    } catch (e) {
      console.error('Save failed:', e);
    } finally {
      setIsLoading(false);
    }
  };

  const handlePreview = async (file: VaultFileMeta) => {
    setIsLoading(true);
    try {
      const data = await getFileData(container.id, file.id);
      setPreviewFile(file);
      setPreviewData(data);
      setView('preview');
    } catch (e) {
      console.error('Preview failed:', e);
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
        {view === 'locked' && (
          <LockView
            container={container}
            onUnlock={handleUnlock}
            isLoading={isLoading}
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
    </Modal>
  );
};
