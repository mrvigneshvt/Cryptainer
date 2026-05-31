import React, { useState, useCallback } from 'react';
import { useDropzone } from 'react-dropzone';
import './DropZone.css';
import { formatBytes } from '../../utils/format';

interface FileWithPreview extends File {
  preview?: string;
}

interface DropZoneProps {
  onFilesSelected: (files: File[]) => void;
  existingFiles?: File[];
}

export const DropZone: React.FC<DropZoneProps> = ({ onFilesSelected, existingFiles = [] }) => {
  const [files, setFiles] = useState<FileWithPreview[]>(existingFiles);

  const onDrop = useCallback((acceptedFiles: File[]) => {
    const newFiles = [...files, ...acceptedFiles];
    setFiles(newFiles);
    onFilesSelected(newFiles);
  }, [files, onFilesSelected]);

  const removeFile = (index: number) => {
    const newFiles = files.filter((_, i) => i !== index);
    setFiles(newFiles);
    onFilesSelected(newFiles);
  };

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    noClick: files.length > 0,
  });

  return (
    <div className="dropzone-wrapper">
      <div
        {...getRootProps()}
        className={`dropzone ${isDragActive ? 'dropzone-active' : ''} ${files.length > 0 ? 'dropzone-has-files' : ''}`}
      >
        <input {...getInputProps()} />
        {files.length === 0 ? (
          <div className="dropzone-placeholder">
            <div className="dropzone-icon">📁</div>
            <p>Drag & drop files here, or click to select</p>
            <span className="dropzone-hint">All files will be encrypted</span>
          </div>
        ) : (
          <div className="dropzone-files">
            <p className="dropzone-more">Drag more files or click to add</p>
          </div>
        )}
      </div>

      {files.length > 0 && (
        <div className="file-list">
          <div className="file-count">{files.length} file(s) selected</div>
          {files.map((file, index) => (
            <div key={`${file.name}-${index}`} className="file-item">
              <span className="file-name">{file.name}</span>
              <span className="file-size">{formatBytes(file.size)}</span>
              <button
                className="file-remove"
                onClick={() => removeFile(index)}
                type="button"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};


