import React, { useEffect, useState } from 'react';
import './VideoPreview.css';

interface VideoPreviewProps {
  data: Uint8Array;
  name: string;
  mime: string;
}

export const VideoPreview: React.FC<VideoPreviewProps> = ({ data, mime }) => {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const blobData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([blobData], { type: mime });
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);

    return () => {
      URL.revokeObjectURL(objectUrl);
    };
  }, [data, mime]);

  if (!url) return <div className="preview-loading">Loading video...</div>;

  return (
    <div className="video-preview">
      <video controls src={url}>
        Your browser does not support the video tag.
      </video>
    </div>
  );
};
