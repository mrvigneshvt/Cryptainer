import React, { useEffect, useState } from 'react';
import { ImagePreview } from './ImagePreview';
import { TextPreview } from './TextPreview';
import { HexPreview } from './HexPreview';
import { VideoPreview } from './VideoPreview';
import { useVaultStore } from '../../store/vaultStore';
import './PreviewRouter.css';

interface PreviewRouterProps {
  mime: string;
  data: Uint8Array;
  name: string;
  containerId: string;
  fileId: string;
}

// ── Audio preview with proper Blob URL cleanup ──────────────────────────────

const AudioPreview: React.FC<{ data: Uint8Array; mime: string; containerId: string; fileId: string }> = ({ data, mime, containerId, fileId }) => {
  const [url, setUrl] = useState<string | null>(null);
  const { releaseFileData } = useVaultStore();

  useEffect(() => {
    const blobData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([blobData], { type: mime });
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);
    return () => {
      URL.revokeObjectURL(objectUrl);
      releaseFileData(containerId, fileId);
    };
  }, [data, mime, containerId, fileId]);

  if (!url) return <div className="preview-loading">Loading audio...</div>;

  return (
    <div className="audio-preview">
      <audio controls>
        <source src={url} type={mime} />
        Your browser does not support the audio tag.
      </audio>
    </div>
  );
};

// ── PDF preview with proper Blob URL cleanup ─────────────────────────────────

const PdfPreview: React.FC<{ data: Uint8Array; name: string; containerId: string; fileId: string }> = ({ data, name, containerId, fileId }) => {
  const [url, setUrl] = useState<string | null>(null);
  const { releaseFileData } = useVaultStore();

  useEffect(() => {
    const pdfData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([pdfData], { type: 'application/pdf' });
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);
    return () => {
      URL.revokeObjectURL(objectUrl);
      releaseFileData(containerId, fileId);
    };
  }, [data, containerId, fileId]);

  if (!url) return <div className="preview-loading">Loading PDF...</div>;

  return (
    <div className="pdf-preview">
      <iframe src={url} title={name} />
    </div>
  );
};

// ── Preview Router ───────────────────────────────────────────────────────────

export const PreviewRouter: React.FC<PreviewRouterProps> = ({ mime, data, name, containerId, fileId }) => {
  // Images
  if (mime.startsWith('image/')) {
    return <ImagePreview data={data} name={name} containerId={containerId} fileId={fileId} />;
  }

  // Videos
  if (mime.startsWith('video/')) {
    return <VideoPreview data={data} name={name} mime={mime} containerId={containerId} fileId={fileId} />;
  }

  // Audio
  if (mime.startsWith('audio/')) {
    return <AudioPreview data={data} mime={mime} containerId={containerId} fileId={fileId} />;
  }

  // PDF
  if (mime === 'application/pdf') {
    return <PdfPreview data={data} name={name} containerId={containerId} fileId={fileId} />;
  }

  // Text files (including code)
  if (mime.startsWith('text/') ||
      mime === 'application/json' ||
      mime === 'application/javascript' ||
      mime === 'application/typescript') {
    return <TextPreview data={data} name={name} containerId={containerId} fileId={fileId} />;
  }

  // Binary files - show hex dump
  return <HexPreview data={data} containerId={containerId} fileId={fileId} />;
};
