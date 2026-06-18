import React, { useEffect, useMemo } from 'react';
import { useVaultStore } from '../../store/vaultStore';
import './HexPreview.css';

interface HexPreviewProps {
  data: Uint8Array;
  containerId: string;
  fileId: string;
}

const BYTES_PER_ROW = 16;
const MAX_PREVIEW_BYTES = 4096; // First 4KB

export const HexPreview: React.FC<HexPreviewProps> = ({ data, containerId, fileId }) => {
  const { releaseFileData } = useVaultStore();

  useEffect(() => {
    return () => { releaseFileData(containerId, fileId); };
  }, [containerId, fileId]);

  const hexRows = useMemo(() => {
    const preview = data.slice(0, MAX_PREVIEW_BYTES);
    const rows: { offset: string; hex: string; ascii: string }[] = [];
    
    for (let i = 0; i < preview.length; i += BYTES_PER_ROW) {
      const rowBytes = preview.slice(i, i + BYTES_PER_ROW);
      
      const offset = i.toString(16).padStart(8, '0');
      
      const hex = Array.from(rowBytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
      
      const ascii = Array.from(rowBytes)
        .map(b => (b >= 32 && b < 127) ? String.fromCharCode(b) : '.')
        .join('');
      
      rows.push({ offset, hex, ascii });
    }
    
    return rows;
  }, [data]);

  const isTruncated = data.length > MAX_PREVIEW_BYTES;

  return (
    <div className="hex-preview">
      <div className="hex-header">
        <span className="hex-offset">Offset</span>
        <span className="hex-bytes">00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F</span>
        <span className="hex-ascii">ASCII</span>
      </div>
      
      <div className="hex-content">
        {hexRows.map((row, index) => (
          <div key={index} className="hex-row">
            <span className="hex-offset">{row.offset}</span>
            <span className="hex-bytes">{row.hex}</span>
            <span className="hex-ascii">{row.ascii}</span>
          </div>
        ))}
      </div>
      
      {isTruncated && (
        <div className="hex-truncated">
          ... ({(data.length - MAX_PREVIEW_BYTES).toLocaleString()} more bytes)
        </div>
      )}
    </div>
  );
};
