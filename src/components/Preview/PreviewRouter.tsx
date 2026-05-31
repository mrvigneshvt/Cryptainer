import React, { useEffect, useState } from 'react';
import { ImagePreview } from './ImagePreview';
import { TextPreview } from './TextPreview';
import { HexPreview } from './HexPreview';
import { VideoPreview } from './VideoPreview';
import './PreviewRouter.css';

interface PreviewRouterProps {
  mime: string;
  data: Uint8Array;
  name: string;
}

// ── Audio preview with proper Blob URL cleanup ──────────────────────────────

const AudioPreview: React.FC<{ data: Uint8Array; mime: string }> = ({ data, mime }) => {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const blobData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([blobData], { type: mime });
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);
    return () => { URL.revokeObjectURL(objectUrl); };
  }, [data, mime]);

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

const PdfPreview: React.FC<{ data: Uint8Array; name: string }> = ({ data, name }) => {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const pdfData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([pdfData], { type: 'application/pdf' });
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);
    return () => { URL.revokeObjectURL(objectUrl); };
  }, [data]);

  if (!url) return <div className="preview-loading">Loading PDF...</div>;

  return (
    <div className="pdf-preview">
      <iframe src={url} title={name} />
    </div>
  );
};

// ── Preview Router ───────────────────────────────────────────────────────────

export const PreviewRouter: React.FC<PreviewRouterProps> = ({ mime, data, name }) => {
  // Images
  if (mime.startsWith('image/')) {
    return <ImagePreview data={data} name={name} />;
  }
  
  // Videos
  if (mime.startsWith('video/')) {
    return <VideoPreview data={data} name={name} mime={mime} />;
  }
  
  // Audio
  if (mime.startsWith('audio/')) {
    return <AudioPreview data={data} mime={mime} />;
  }
  
  // PDF
  if (mime === 'application/pdf') {
    return <PdfPreview data={data} name={name} />;
  }
  
  // Text files (including code)
  if (mime.startsWith('text/') || 
      mime === 'application/json' ||
      mime === 'application/javascript' ||
      mime === 'application/typescript') {
    return <TextPreview data={data} name={name} />;
  }
  
  // Binary files - show hex dump
  return <HexPreview data={data} />;
};
