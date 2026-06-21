/**
 * Detect whether the app is running inside the Tauri webview.
 * In a browser (dev mode), `window.__TAURI__` is undefined.
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI__;
}

import type { ContainerMeta } from '../types/vault';

/** Mock container data for browser dev / UI preview */
export const MOCK_CONTAINERS: ContainerMeta[] = [
  {
    id: 'mock-1',
    name: 'Project Alpha',
    algo: 'AES-GCM-256',
    kdf_params: { kdf: 'argon2id', memory_kb: 65536, iterations: 2, parallelism: 1 },
    file_count: 12,
    total_size: 4_250_000,
    blob_path: '/mock/project-alpha.ctnr',
    blob_sha256: 'a3f5c8e9d2b1a3f5c8e9d2b1a3f5c8e9d2b1a3f5c8e9d2b1a3f5c8e9d2b1a3f5',
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2).toISOString(),
    modified_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2).toISOString(),
    tags: 'work,confidential',
  },
  {
    id: 'mock-2',
    name: 'Personal Docs',
    algo: 'AES-GCM-256',
    kdf_params: { kdf: 'argon2id', memory_kb: 65536, iterations: 2, parallelism: 1 },
    file_count: 5,
    total_size: 890_000,
    blob_path: '/mock/personal-docs.ctnr',
    blob_sha256: 'b7e1d4f9a2c6b7e1d4f9a2c6b7e1d4f9a2c6b7e1d4f9a2c6b7e1d4f9a2c6b7e1',
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 5).toISOString(),
    modified_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 5).toISOString(),
    tags: 'personal',
  },
  {
    id: 'mock-3',
    name: 'Backup 2024',
    algo: 'AES-GCM-256',
    kdf_params: { kdf: 'argon2id', memory_kb: 65536, iterations: 2, parallelism: 1 },
    file_count: 48,
    total_size: 156_000_000,
    blob_path: '/mock/backup-2024.ctnr',
    blob_sha256: 'c2a8e5b1d9f3c2a8e5b1d9f3c2a8e5b1d9f3c2a8e5b1d9f3c2a8e5b1d9f3c2a8',
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(),
    modified_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(),
    tags: 'backup,archive',
  },
];
