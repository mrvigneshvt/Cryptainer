import React from 'react';
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
    const audioData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    return (
      <div className="audio-preview">
        <audio controls>
          <source src={URL.createObjectURL(new Blob([audioData], { type: mime }))} type={mime} />
          Your browser does not support the audio tag.
        </audio>
      </div>
    );
  }
  
  // PDF
  if (mime === 'application/pdf') {
    const pdfData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const url = URL.createObjectURL(new Blob([pdfData], { type: 'application/pdf' }));
    return (
      <div className="pdf-preview">
        <iframe src={url} title={name} />
      </div>
    );
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
