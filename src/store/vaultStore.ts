import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { ContainerMeta, VaultFileMeta, CreateContainerInput, FileInput } from '../types/vault';

interface VaultState {
  containers:   ContainerMeta[];
  loading:      boolean;
  error:        string | null;
  // Actions
  loadContainers:    () => Promise<void>;
  createContainer:   (input: CreateContainerInput) => Promise<ContainerMeta>;
  deleteContainer:   (id: string) => Promise<void>;
  unlockContainer:   (id: string, password: string) => Promise<VaultFileMeta[]>;
  lockContainer:     (id: string) => Promise<void>;
  getFileData:       (containerId: string, fileId: string) => Promise<Uint8Array>;
  releaseFileData:   (containerId: string, fileId: string) => Promise<void>;
  exportContainer:   (id: string, destPath: string) => Promise<void>;
  importContainer:   (srcPath: string) => Promise<ContainerMeta>;
  saveContainerEdits: (containerId: string, password: string, filesToAdd: FileInput[], fileIdsToRemove: string[]) => Promise<ContainerMeta>;
  clearError:        () => void;
}

export const useVaultStore = create<VaultState>((set) => ({
  containers: [],
  loading:    false,
  error:      null,

  loadContainers: async () => {
    set({ loading: true, error: null });
    try {
      const containers = await invoke<ContainerMeta[]>('list_containers');
      set({ containers, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createContainer: async (input) => {
    set({ loading: true, error: null });
    try {
      const meta = await invoke<ContainerMeta>('create_container', { input });
      set(s => ({ containers: [meta, ...s.containers], loading: false }));
      return meta;
    } catch (e) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  deleteContainer: async (id) => {
    try {
      await invoke('delete_container', { containerId: id });
      set(s => ({ containers: s.containers.filter(c => c.id !== id) }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  unlockContainer: async (id, password) => {
    try {
      return await invoke<VaultFileMeta[]>('unlock_container', { containerId: id, password });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  lockContainer: async (id) => {
    await invoke('lock_container', { containerId: id });
  },

  getFileData: async (containerId, fileId) => {
    const bytes = await invoke<number[]>('get_file_data', { containerId, fileId });
    return new Uint8Array(bytes);
  },

  releaseFileData: async (containerId, fileId) => {
    await invoke('release_file_data', { containerId, fileId });
  },

  exportContainer: async (id, destPath) => {
    try {
      await invoke('export_container', { containerId: id, destPath });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  importContainer: async (srcPath) => {
    try {
      const meta = await invoke<ContainerMeta>('import_container', { srcPath });
      set(s => ({ containers: [meta, ...s.containers] }));
      return meta;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveContainerEdits: async (containerId, password, filesToAdd, fileIdsToRemove) => {
    try {
      const updated = await invoke<ContainerMeta>('save_edits', {
        containerId,
        password,
        filesToAdd,
        fileIdsToRemove,
      });
      set(s => ({
        containers: s.containers.map(c => c.id === containerId ? updated : c),
      }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearError: () => set({ error: null }),
}));
