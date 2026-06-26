import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { ContainerMeta, VaultFileMeta, CreateContainerInput, FileInput, DownloadResult } from '../types/vault';
import { isTauri, MOCK_CONTAINERS } from '../utils/tauri';

interface VaultState {
  containers:   ContainerMeta[];
  loading:      boolean;
  error:        string | null;
  isMock:       boolean;
  // Actions
  loadContainers:    () => Promise<void>;
  createContainer:   (input: CreateContainerInput) => Promise<ContainerMeta>;
  deleteContainer:   (id: string) => Promise<void>;
  unlockContainer:   (id: string, password: string) => Promise<VaultFileMeta[]>;
  lockContainer:     (id: string) => Promise<void>;
  getFileData:       (containerId: string, fileId: string) => Promise<Uint8Array>;
  releaseFileData:   (containerId: string, fileId: string) => Promise<void>;
  getDownloadDir:    () => Promise<string>;
  exportContainer:   (id: string, destPath: string) => Promise<void>;
  importContainer:   (srcPath: string) => Promise<ContainerMeta>;
  downloadFiles:    (containerId: string, fileIds: string[], destDir: string) => Promise<DownloadResult[]>;
  saveContainerEdits: (containerId: string, password: string, filesToAdd: FileInput[], fileIdsToRemove: string[]) => Promise<ContainerMeta>;
  clearError:        () => void;
}

export const useVaultStore = create<VaultState>((set) => ({
  containers: [],
  loading:    false,
  error:      null,
  isMock:     !isTauri(),

  loadContainers: async () => {
    set({ loading: true, error: null });
    try {
      if (!isTauri()) {
        // Browser dev mode — serve mock data after a brief delay so loading state is visible
        await new Promise(r => setTimeout(r, 400));
        set({ containers: MOCK_CONTAINERS as ContainerMeta[], loading: false });
        return;
      }
      const containers = await invoke<ContainerMeta[]>('list_containers');
      set({ containers, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createContainer: async (input) => {
    set({ loading: true, error: null });
    try {
      if (!isTauri()) {
        await new Promise(r => setTimeout(r, 400));
        const now = new Date().toISOString();
        const meta: ContainerMeta = {
          id: 'mock-' + Date.now(),
          name: input.name,
          algo: 'AES-GCM-256',
          kdf_params: input.kdf_params,
          file_count: input.files?.length ?? 0,
          total_size: input.files?.reduce((s, f) => s + f.data.length, 0) ?? 0,
          blob_path: '/mock/' + input.name + '.ctnr',
          blob_sha256: 'mock-sha256-' + Date.now(),
          created_at: now,
          modified_at: now,
          tags: input.tags ?? '',
        };
        set(s => ({ containers: [meta, ...s.containers], loading: false }));
        return meta;
      }
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
      if (!isTauri()) {
        set(s => ({ containers: s.containers.filter(c => c.id !== id) }));
        return;
      }
      await invoke('delete_container', { containerId: id });
      set(s => ({ containers: s.containers.filter(c => c.id !== id) }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  unlockContainer: async (id, password) => {
    try {
      if (!isTauri()) {
        return [];
      }
      return await invoke<VaultFileMeta[]>('unlock_container', { containerId: id, password });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  lockContainer: async (id) => {
    if (!isTauri()) return;
    await invoke('lock_container', { containerId: id });
  },

  getFileData: async (containerId, fileId) => {
    if (!isTauri()) return new Uint8Array(0);
    const bytes = await invoke<number[]>('get_file_data', { containerId, fileId });
    return new Uint8Array(bytes);
  },

  releaseFileData: async (containerId, fileId) => {
    if (!isTauri()) return;
    await invoke('release_file_data', { containerId, fileId });
  },

  getDownloadDir: async () => {
    if (!isTauri()) return '/tmp';
    return await invoke<string>('get_download_dir');
  },

  exportContainer: async (id, destPath) => {
    try {
      if (!isTauri()) return;
      await invoke('export_container', { containerId: id, destPath });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  importContainer: async (srcPath) => {
    try {
      if (!isTauri()) {
        const meta = MOCK_CONTAINERS[0] as ContainerMeta;
        set(s => ({ containers: [meta, ...s.containers] }));
        return meta;
      }
      const meta = await invoke<ContainerMeta>('import_container', { srcPath });
      set(s => ({ containers: [meta, ...s.containers] }));
      return meta;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  downloadFiles: async (containerId, fileIds, destDir) => {
    try {
      if (!isTauri()) return [];
      return await invoke<DownloadResult[]>('download_files', { containerId, fileIds, destDir });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveContainerEdits: async (containerId, password, filesToAdd, fileIdsToRemove) => {
    try {
      if (!isTauri()) {
        return MOCK_CONTAINERS[0] as ContainerMeta;
      }
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
