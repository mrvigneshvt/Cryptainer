import React, { useEffect, useState } from 'react';
import { useVaultStore } from '../../store/vaultStore';
import './ImagePreview.css';

interface ImagePreviewProps {
  data: Uint8Array;
  name: string;
  containerId: string;
  fileId: string;
}

export const ImagePreview: React.FC<ImagePreviewProps> = ({ data, name, containerId, fileId }) => {
  const [url, setUrl] = useState<string | null>(null);
  const { releaseFileData } = useVaultStore();

  useEffect(() => {
    const blobData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    const blob = new Blob([blobData]);
    const objectUrl = URL.createObjectURL(blob);
    setUrl(objectUrl);

    return () => {
      URL.revokeObjectURL(objectUrl);
      releaseFileData(containerId, fileId);
    };
  }, [data, containerId, fileId]);

  if (!url) return <div className="preview-loading">Loading image...</div>;

  return (
    <div className="image-preview">
      <img src={url} alt={name} />
    </div>
  );
};
