# Cryptainer — IPC Command Reference

All commands are invoked from the React frontend via `@tauri-apps/api/core`'s `invoke()` function.

```typescript
import { invoke } from '@tauri-apps/api/core';
```

Commands are registered in [`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) and implemented in [`src-tauri/src/commands.rs`](../src-tauri/src/commands.rs).

---

## `create_container`

Create a new encrypted container with v2 per-file encryption.

**Signature**

```rust
async fn create_container(app: AppHandle, input: CreateContainerInput, pool: State<SqlitePool>) -> Result<ContainerMeta>
```

**Input**

```typescript
interface CreateContainerInput {
  name:       string;          // Container name (1-256 chars, unique)
  kdf_params: KdfParams;       // Key derivation parameters
  hint?:      string;          // Password hint (optional, max 256)
  tags?:      string;          // Comma-separated tags (optional, max 1024)
  password:   string;          // Encryption password (min 8 chars)
  files:      FileInput[];     // Files to encrypt
}

interface FileInput {
  name: string;     // Filename
  mime: string;     // MIME type
  data: number[];   // File bytes as number[]
}

interface KdfParams {
  kdf:         'argon2id';
  memory_kb?:  number;   // 65536 (default)
  iterations:  number;   // 2 (default)
  parallelism?: number;  // 1 (default)
}
```

**Output**

```typescript
interface ContainerMeta {
  id:             string;   // UUID v4
  name:           string;
  algo:           'AES-GCM-256' | 'AES-GCM-128';
  kdf_params:     KdfParams;
  hint?:          string;
  tags?:          string;
  file_count:     number;
  total_size:     number;   // bytes (sum of all plaintext file sizes)
  blob_path:      string;   // absolute path to .enc file
  blob_sha256:    string;   // hex SHA-256 of the encrypted blob
  created_at:     string;   // ISO-8601
  modified_at:    string;
  format_version: number;   // 2
}
```

**Errors**

| Error | Cause |
|---|---|
| `"Password must be at least 8 characters"` | Password too short |
| `"Container name must be 1-256 characters"` | Name empty or too long |
| `"Tags too long (max 1024)"` | Tags exceed limit |
| `"Hint too long (max 256)"` | Hint exceeds limit |
| `"A container with this name already exists"` | Duplicate name |
| `"Encryption failed: ..."` | AES-GCM encryption error |
| `"Database error: ..."` | SQLite insert failed |
| `"IO error: ..."` | Blob file write failed |

**Frontend usage**

```typescript
const meta = await invoke<ContainerMeta>('create_container', { input });
```

---

## `unlock_container`

Unlock a container and return its file listing. Handles both v1 (legacy) and v2 (current) formats. v1 containers are automatically migrated to v2 on unlock.

**Signature**

```rust
async fn unlock_container(
    container_id: String,
    password: String,
    pool: State<SqlitePool>,
    sessions: State<SessionStore>,
    sessions_v2: State<SessionStoreV2>,
) -> Result<Vec<VaultFileMeta>>
```

**Input**

```typescript
interface UnlockContainerInput {
  containerId: string;   // Container UUID
  password:    string;   // Decryption password
}
```

**Output**

```typescript
interface VaultFileMeta {
  id:   string;   // File UUID
  name: string;   // Filename
  mime: string;   // MIME type
  size: number;   // File size in bytes
}
```

Returns file list **without file data**. File data is fetched separately via `get_file_data` (lazy decryption).

**Errors**

| Error | Cause |
|---|---|
| `"Password must be at least 8 characters"` | Password too short |
| `"Container not found: ..."` | Container doesn't exist |
| `"Integrity check failed — container may be corrupted or tampered with"` | SHA-256 mismatch |
| `"Blob too short to be valid"` | Corrupted blob |
| `"Decryption failed — wrong password or corrupted data"` | Wrong password or tampered blob |
| `"Database error: ..."` | SQLite query failed |

**Frontend usage**

```typescript
const files = await invoke<VaultFileMeta[]>('unlock_container', {
  containerId: id,
  password,
});
```

---

## `get_file_data`

Fetch the decrypted bytes of a specific file in an unlocked container. V2: checks the LRU cache first; on miss, seeks to the file's offset in the blob, decrypts, verifies SHA-256, and caches the result.

**Signature**

```rust
async fn get_file_data(
    container_id: String,
    file_id: String,
    sessions: State<SessionStore>,
    sessions_v2: State<SessionStoreV2>,
) -> Result<Vec<u8>>
```

**Input**

```typescript
{
  containerId: string;   // Container UUID
  fileId:      string;   // File UUID
}
```

**Output**

```typescript
number[]  // File bytes as number[] — convert to Uint8Array on frontend
```

**Errors**

| Error | Cause |
|---|---|
| `"Session not active — container must be unlocked first"` | Container not unlocked |
| `"File not found: ..."` | File ID doesn't exist in container |
| `"Integrity check failed — container may be corrupted or tampered with"` | File SHA-256 mismatch after decryption |
| `"Decryption failed — wrong password or corrupted data"` | GCM tag verification failed |

**Frontend usage**

```typescript
const bytes = await invoke<number[]>('get_file_data', {
  containerId: id,
  fileId: file.id,
});
const data = new Uint8Array(bytes);
```

---

## `release_file_data`

Explicitly release a file's decrypted data from the LRU cache, zeroizing its contents in memory. Call this when a preview component unmounts to free memory proactively.

**Signature**

```rust
async fn release_file_data(
    container_id: String,
    file_id: String,
    sessions_v2: State<SessionStoreV2>,
) -> Result<()>
```

**Input**

```typescript
{
  containerId: string;   // Container UUID
  fileId:      string;   // File UUID
}
```

**Output**

Void (nothing returned).

**Errors**

None — always succeeds (idempotent).

**Frontend usage**

```typescript
await invoke('release_file_data', {
  containerId: id,
  fileId: file.id,
});
```

---

## `save_edits`

Save changes to an unlocked container: add files, remove files, and re-encrypt. Supports both v1 and v2 formats internally.

**Signature**

```rust
async fn save_edits(
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    pool: State<SqlitePool>,
    sessions: State<SessionStore>,
    sessions_v2: State<SessionStoreV2>,
) -> Result<ContainerMeta>
```

**Input**

```typescript
{
  containerId:      string;      // Container UUID
  password:         string;      // Password (must match original)
  filesToAdd:       FileInput[]; // New files to encrypt
  fileIdsToRemove:  string[];    // UUIDs of files to remove
}
```

**Output**

```typescript
ContainerMeta  // Updated container metadata
```

**Errors**

| Error | Cause |
|---|---|
| `"Session not active — container must be unlocked first"` | Container not unlocked |
| `"Decryption failed — wrong password or corrupted data"` | Wrong password (verification failed) |
| `"Integrity check failed — container may be corrupted or tampered with"` | Blob read error |
| `"Encryption failed: ..."` | Re-encryption failure |

**Frontend usage**

```typescript
const updated = await invoke<ContainerMeta>('save_edits', {
  containerId: id,
  password,
  filesToAdd: newFiles,
  fileIdsToRemove: removedIds,
});
```

---

## `list_containers`

List all container metadata (no sensitive data, no blobs loaded).

**Signature**

```rust
async fn list_containers(pool: State<SqlitePool>) -> Result<Vec<ContainerMeta>>
```

**Input**

None.

**Output**

```typescript
ContainerMeta[]  // Array of container metadata, ordered by created_at DESC
```

**Errors**

| Error | Cause |
|---|---|
| `"Database error: ..."` | SQLite query failed |

**Frontend usage**

```typescript
const containers = await invoke<ContainerMeta[]>('list_containers');
```

---

## `delete_container`

Delete a container: removes the database row, then the encrypted blob file. Locks the session if active. If the blob file is missing (manual cleanup), the DB row is still removed.

**Signature**

```rust
async fn delete_container(
    container_id: String,
    pool: State<SqlitePool>,
    sessions: State<SessionStore>,
    sessions_v2: State<SessionStoreV2>,
) -> Result<()>
```

**Input**

```typescript
{
  containerId: string;   // Container UUID
}
```

**Output**

Void.

**Errors**

| Error | Cause |
|---|---|
| `"Container not found: ..."` | Container doesn't exist |
| `"Database error: ..."` | SQLite deletion failed |
| `"IO error: ..."` | Blob file permissions issue |

**Frontend usage**

```typescript
await invoke('delete_container', { containerId: id });
```

---

## `lock_container`

Lock a container: removes the session from the in-memory store, zeroizing all key material and cached file data.

**Signature**

```rust
async fn lock_container(
    container_id: String,
    sessions: State<SessionStore>,
    sessions_v2: State<SessionStoreV2>,
) -> Result<()>
```

**Input**

```typescript
{
  containerId: string;   // Container UUID
}
```

**Output**

Void.

**Errors**

None — always succeeds.

**Frontend usage**

```typescript
await invoke('lock_container', { containerId: id });
```

---

## `export_container`

Export a container to a portable `.ctnr` file. Refuses to overwrite an existing destination file.

**Signature**

```rust
async fn export_container(
    container_id: String,
    dest_path: String,
    pool: State<SqlitePool>,
) -> Result<()>
```

**Input**

```typescript
{
  containerId: string;   // Container UUID
  destPath:    string;   // Destination file path (absolute)
}
```

**Output**

Void.

**Errors**

| Error | Cause |
|---|---|
| `"Destination file already exists"` | Refusing to overwrite |
| `"Container not found: ..."` | Container doesn't exist |
| `"IO error: ..."` | File read/write failure |

**Frontend usage**

```typescript
await invoke('export_container', {
  containerId: id,
  destPath: path,
});
```

---

## `import_container`

Import a `.ctnr` file into the vault. Verifies blob integrity, checks for duplicate container IDs, and cleans up orphaned blob files if the DB insert fails.

**Signature**

```rust
async fn import_container(
    src_path: String,
    app: AppHandle,
    pool: State<SqlitePool>,
) -> Result<ContainerMeta>
```

**Input**

```typescript
{
  srcPath: string;   // Path to .ctnr file (absolute)
}
```

**Output**

```typescript
ContainerMeta  // Imported container metadata
```

**Errors**

| Error | Cause |
|---|---|
| `"File too short"` | Truncated or invalid file |
| `"Not a .ctnr file"` | Invalid magic bytes |
| `"Unsupported version: ..."` | Unknown format version |
| `"Truncated header"` | Header data incomplete |
| `"A container with this ID already exists in the vault"` | Duplicate container |
| `"Integrity check failed — container may be corrupted or tampered with"` | Blob SHA-256 mismatch |
| `"Database error: ..."` | SQLite insert failed |

**Frontend usage**

```typescript
const meta = await invoke<ContainerMeta>('import_container', { srcPath: path });
```

---

## TypeScript Types (src/types/vault.ts)

```typescript
export interface KdfParams {
  kdf: 'argon2id';
  memory_kb?: number;     // 16384 | 65536 | 131072 | 262144
  iterations: number;     // 1 | 2 | 3 | 4
  parallelism?: number;   // 1 | 2
}

export interface ContainerMeta {
  id:             string;
  name:           string;
  algo:           'AES-GCM-256' | 'AES-GCM-128';
  kdf_params:     KdfParams;
  hint?:          string;
  tags?:          string;
  file_count:     number;
  total_size:     number;
  blob_path:      string;
  blob_sha256:    string;
  created_at:     string;
  modified_at:    string;
  format_version: number;   // 1=v1, 2=v2
}

export interface VaultFileMeta {
  id:   string;
  name: string;
  mime: string;
  size: number;
}

export interface FileInput {
  name: string;
  mime: string;
  data: number[];   // Uint8Array serialized as number[] for IPC
}

export interface CreateContainerInput {
  name:       string;
  kdf_params: KdfParams;
  hint?:      string;
  tags?:      string;
  password:   string;
  files:      FileInput[];
}

export const SECURITY_PRESETS = [
  { label: 'Fast (low-end device)',  params: { kdf: 'argon2id', memory_kb: 16384,  iterations: 1, parallelism: 1 } },
  { label: 'Standard (recommended)', params: { kdf: 'argon2id', memory_kb: 65536,  iterations: 2, parallelism: 1 } },
  { label: 'High security',          params: { kdf: 'argon2id', memory_kb: 131072, iterations: 3, parallelism: 1 } },
  { label: 'Paranoid',               params: { kdf: 'argon2id', memory_kb: 262144, iterations: 4, parallelism: 2 } },
];
```

## Error Handling

All commands return errors as strings via Tauri's `serde::Serialize` on `CryptoError`. Use try/catch on the frontend:

```typescript
try {
  const result = await invoke('command_name', { ...args });
} catch (error) {
  // error is a string describing the error
  console.error('Command failed:', error);
}
```

### Common Error Variants

| Error String | Source |
|---|---|
| `"Encryption failed: ..."` | AES-GCM encryption |
| `"Decryption failed — wrong password or corrupted data"` | GCM tag verification |
| `"Database error: ..."` | sqlx query failure |
| `"Container not found: ..."` | UUID not in DB |
| `"Session not active — container must be unlocked first"` | get_file_data or save_edits on locked container |
| `"Integrity check failed — container may be corrupted or tampered with"` | SHA-256 mismatch |
| `"Not a .ctnr file"` | Invalid export format |
| `"Password must be at least 8 characters"` | Minimum length validation |

---

*Last updated: 2026-06-19*
