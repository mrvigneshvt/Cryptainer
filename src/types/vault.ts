// All TypeScript types mirror the Rust structs in vault.rs and commands.rs.
// Keep this file in sync with the Rust side whenever structs change.

export interface KdfParams {
  kdf: 'argon2id';
  memory_kb?: number;
  iterations: number;
  parallelism?: number;
}

export interface ContainerMeta {
  id:          string;
  name:        string;
  algo:        'AES-GCM-256' | 'AES-GCM-128';
  kdf_params:  KdfParams;
  hint?:       string;
  tags?:       string;
  file_count:  number;
  total_size:  number;    // bytes
  blob_path:   string;
  blob_sha256: string;
  created_at:  string;    // ISO-8601
  modified_at: string;
}

export interface VaultFileMeta {
  id:   string;
  name: string;
  mime: string;
  size: number;  // bytes
}

export interface FileInput {
  name: string;
  mime: string;
  data: number[];  // Uint8Array serialized as number[] for IPC
}

export interface CreateContainerInput {
  name:       string;
  kdf_params: KdfParams;
  hint?:      string;
  tags?:      string;
  password:   string;
  files:      FileInput[];
}

export const DEFAULT_KDF_PARAMS: KdfParams = {
  kdf:         'argon2id',
  memory_kb:   65536,
  iterations:  2,
  parallelism: 1,
};

export const SECURITY_PRESETS = [
  { label: 'Fast (low-end device)',  params: { kdf: 'argon2id', memory_kb: 16384,  iterations: 1, parallelism: 1 } as KdfParams },
  { label: 'Standard (recommended)', params: { kdf: 'argon2id', memory_kb: 65536,  iterations: 2, parallelism: 1 } as KdfParams },
  { label: 'High security',          params: { kdf: 'argon2id', memory_kb: 131072, iterations: 3, parallelism: 1 } as KdfParams },
  { label: 'Paranoid',               params: { kdf: 'argon2id', memory_kb: 262144, iterations: 4, parallelism: 2 } as KdfParams },
];
