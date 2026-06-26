import React, { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useAuditStore } from '../../store/auditStore';
import './ActivityLogPanel.css';

interface ActivityLogPanelProps {
  open: boolean;
  onClose: () => void;
}

function relativeTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function actionIcon(action: string) {
  switch (action) {
    case 'create':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="16" /><line x1="8" y1="12" x2="16" y2="12" />
        </svg>
      );
    case 'unlock':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 9.9-1" />
        </svg>
      );
    case 'lock':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="11" width="18" height="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" />
        </svg>
      );
    case 'edit':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" /><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
      );
    case 'delete':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        </svg>
      );
    case 'export':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      );
    case 'import':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" />
        </svg>
      );
    case 'download':
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      );
    default:
      return (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
      );
  }
}

export const ActivityLogPanel: React.FC<ActivityLogPanelProps> = ({ open, onClose }) => {
  const { events, loading, loadAuditEvents } = useAuditStore();
  const panelRef = useRef<HTMLDivElement>(null);
  const [bodyOverflow, setBodyOverflow] = useState(false);

  useEffect(() => {
    if (open) {
      loadAuditEvents();
      document.body.style.overflow = 'hidden';
      setBodyOverflow(true);
    }
    return () => {
      if (bodyOverflow) {
        document.body.style.overflow = '';
        setBodyOverflow(false);
      }
    };
  }, [open]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    if (open) {
      document.addEventListener('keydown', handleEscape);
    }
    return () => document.removeEventListener('keydown', handleEscape);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <>
      <div className="audit-overlay" onClick={onClose} />
      <div className="audit-panel" ref={panelRef}>
        <div className="audit-header">
          <h3 className="audit-title">Activity Log</h3>
          <span className="audit-count">{events.length}</span>
          <button className="audit-close" onClick={onClose} aria-label="Close activity log">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <div className="audit-list">
          {loading && events.length === 0 && (
            <div className="audit-loading">Loading...</div>
          )}
          {!loading && events.length === 0 && (
            <div className="audit-empty">
              <p>No activity yet</p>
            </div>
          )}
          {events.map(event => (
            <div key={event.id} className="audit-event">
              <div className="audit-event-icon">
                {actionIcon(event.action)}
              </div>
              <div className="audit-event-body">
                <div className="audit-event-top">
                  <span className="audit-event-action">{capitalize(event.action)}</span>
                  {event.container_name && (
                    <span className="audit-event-container">{event.container_name}</span>
                  )}
                </div>
                <div className="audit-event-bottom">
                  <span className="audit-event-ts" title={event.ts}>
                    {relativeTime(event.ts)}
                  </span>
                  {event.details && event.details !== 'null' && (
                    <span className="audit-event-details">{event.details}</span>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </>,
    document.body
  );
};

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
