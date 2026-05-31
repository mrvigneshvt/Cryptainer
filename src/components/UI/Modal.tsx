import React, { useEffect } from 'react';
import { createPortal } from 'react-dom';
import './Modal.css';

// Track open modal count to prevent body overflow conflicts
// when multiple modals are stacked (e.g., Settings + ContainerModal).
let openModalCount = 0;
function incrementOverflow() {
  openModalCount++;
  document.body.style.overflow = 'hidden';
}
function decrementOverflow() {
  openModalCount--;
  if (openModalCount <= 0) {
    document.body.style.overflow = '';
  }
}

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
}

export const Modal: React.FC<ModalProps> = ({
  open,
  onClose,
  title,
  children,
  size = 'md',
}) => {
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    
    // Only register (and therefore only clean up) when actually open, so the
    // increment/decrement stay paired and the shared counter can't drift below
    // zero — which would re-enable body scroll while another modal is still open.
    if (!open) return;

    document.addEventListener('keydown', handleEscape);
    incrementOverflow();

    return () => {
      document.removeEventListener('keydown', handleEscape);
      decrementOverflow();
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div className="modal-overlay" onClick={onClose}>
      <div className={`modal-content modal-${size}`} onClick={(e) => e.stopPropagation()}>
        {title && (
          <div className="modal-header">
            <h3>{title}</h3>
            <button className="modal-close" onClick={onClose}>×</button>
          </div>
        )}
        <div className="modal-body">{children}</div>
      </div>
    </div>,
    document.body
  );
};
