import React, { useEffect, useState } from 'react';
import './ImagePreview.css';

interface ImagePreviewProps {
  data: Uint8Array;
  name: string;
}

export const ImagePreview: React.FC<ImagePreviewProps> = ({ data, name }) => {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const blobData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([blobData]);
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);

    return () => {
      URL.revokeObjectURL(objectUrl);
    };
  }, [data]);

  if (!url) return <div className="preview-loading">Loading image...</div>;

  return (
    <div className="image-preview">
      <img src={url} alt={name} />
    </div>
  );
};
