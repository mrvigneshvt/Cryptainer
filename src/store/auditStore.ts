import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { AuditEvent } from '../types/vault';
import { isTauri } from '../utils/tauri';

interface AuditState {
  events: AuditEvent[];
  loading: boolean;
  loadAuditEvents: () => Promise<void>;
}

export const useAuditStore = create<AuditState>((set) => ({
  events: [],
  loading: false,

  loadAuditEvents: async () => {
    set({ loading: true });
    try {
      if (!isTauri()) {
        set({ events: [], loading: false });
        return;
      }
      const events = await invoke<AuditEvent[]>('list_audit_events', { limit: 200 });
      set({ events, loading: false });
    } catch (e) {
      console.error('Failed to load audit events:', e);
      set({ loading: false });
    }
  },
}));
