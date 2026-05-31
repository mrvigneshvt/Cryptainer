# Cryptainer — API Documentation

## Tauri IPC Commands

This document describes all commands available to the React frontend via Tauri's invoke API.

---

## `create_container`

**Description**: Create and encrypt a new container with the specified files and configuration.

**Input**:
```typescript
{
  name: string;           // Container name
  kdf_params: KdfParams;  // Key derivation parameters
  hint?: string;          // Password hint (optional)
  tags?: string;          // Comma-separated tags (optional)
  password: string;       // Encryption password
  files: FileInput[];     // Files to encrypt
}

interface FileInput {
  name: string;   // Filename
  mime: string;   // MIME type
  data: number[]; // File bytes as number[] for IPC
}
```

**Output**: `ContainerMeta` - Metadata of the created container

**Errors**:
- `CryptoError::Encryption` - Encryption failed
- `CryptoError::Database` - SQLite insert failed
- `CryptoError::Io` - File write failed

**Frontend usage**:
```typescript
const meta = await invoke<ContainerMeta>('create_container', { input });
```

---

## `unlock_container`

**Description**: Unlock (decrypt) a container and store the session in memory. Returns file list without file data.

**Input**:
```typescript
{
  containerId: string;  // Container UUID
  password: string;     // Decryption password
}
```

**Output**: `VaultFileMeta[]` - List of files (metadata only)

**Errors**:
- `CryptoError::NotFound` - Container doesn't exist
- `CryptoError::IntegrityFailure` - Blob hash mismatch
- `CryptoError::Decryption` - Wrong password or corrupted data

**Frontend usage**:
```typescript
const files = await invoke<VaultFileMeta[]>('unlock_container', { 
  containerId: id, 
  password 
});
```

---

## `get_file_data`

**Description**: Fetch the decrypted data bytes of a specific file in an unlocked container.

**Input**:
```typescript
{
  containerId: string;  // Container UUID
  fileId: string;       // File UUID
}
```

**Output**: `number[]` - File data as bytes (convert to Uint8Array)

**Errors**:
- `CryptoError::SessionInactive` - Container not unlocked
- `CryptoError::NotFound` - File not found in container

**Frontend usage**:
```typescript
const bytes = await invoke<number[]>('get_file_data', { containerId, fileId });
const data = new Uint8Array(bytes);
```

---

## `save_edits`

**Description**: Save changes to an unlocked container (add/remove files) and re-encrypt atomically.

**Input**:
```typescript
{
  containerId: string;      // Container UUID
  password: string;         // Password for re-encryption
  filesToAdd: FileInput[];  // New files to add
  fileIdsToRemove: string[]; // IDs of files to remove
}
```

**Output**: `ContainerMeta` - Updated container metadata

**Errors**:
- `CryptoError::SessionInactive` - Container not unlocked
- `CryptoError::Encryption` - Re-encryption failed
- `CryptoError::Database` - Metadata update failed

**Frontend usage**:
```typescript
const updated = await invoke<ContainerMeta>('save_edits', {
  containerId: id,
  password,
  filesToAdd: newFiles,
  fileIdsToRemove: removedIds
});
```

---

## `list_containers`

**Description**: List all containers (metadata only, no sensitive data).

**Input**: None

**Output**: `ContainerMeta[]` - Array of container metadata

**Errors**:
- `CryptoError::Database` - Database query failed

**Frontend usage**:
```typescript
const containers = await invoke<ContainerMeta[]>('list_containers');
```

---

## `delete_container`

**Description**: Delete a container, removing both database entry and blob file.

**Input**:
```typescript
{
  containerId: string;  // Container UUID
}
```

**Output**: `void` - Success returns nothing

**Errors**:
- `CryptoError::NotFound` - Container doesn't exist
- `CryptoError::Database` - Database deletion failed

**Frontend usage**:
```typescript
await invoke('delete_container', { containerId: id });
```

---

## `lock_container`

**Description**: Lock (clear) a container session, wiping decrypted data and key from memory.

**Input**:
```typescript
{
  containerId: string;  // Container UUID
}
```

**Output**: `void` - Success returns nothing

**Errors**: None (always succeeds)

**Frontend usage**:
```typescript
await invoke('lock_container', { containerId: id });
```

---

## `export_container`

**Description**: Export a container to a portable .ctnr file.

**Input**:
```typescript
{
  containerId: string;  // Container UUID
  destPath: string;     // Destination file path
}
```

**Output**: `void` - Success returns nothing

**Errors**:
- `CryptoError::NotFound` - Container doesn't exist
- `CryptoError::Io` - File write failed

**Frontend usage**:
```typescript
await invoke('export_container', { containerId: id, destPath: path });
```

---

## `import_container`

**Description**: Import a .ctnr file into the vault.

**Input**:
```typescript
{
  srcPath: string;  // Source .ctnr file path
}
```

**Output**: `ContainerMeta` - Imported container metadata

**Errors**:
- `CryptoError::InvalidFormat` - Not a valid .ctnr file
- `CryptoError::IntegrityFailure` - Blob hash mismatch
- `CryptoError::Database` - Insert failed

**Frontend usage**:
```typescript
const meta = await invoke<ContainerMeta>('import_container', { srcPath: path });
```

---

## TypeScript Types

### ContainerMeta
```typescript
interface ContainerMeta {
  id: string;
  name: string;
  algo: 'AES-GCM-256' | 'AES-GCM-128';
  kdf_params: KdfParams;
  hint?: string;
  tags?: string;
  file_count: number;
  total_size: number;
  blob_path: string;
  blob_sha256: string;
  created_at: string;
  modified_at: string;
}
```

### VaultFileMeta
```typescript
interface VaultFileMeta {
  id: string;
  name: string;
  mime: string;
  size: number;
}
```

### KdfParams
```typescript
interface KdfParams {
  kdf: 'argon2id' | 'pbkdf2';
  memory_kb?: number;
  iterations: number;
  parallelism?: number;
}
```

---

## Error Handling

All commands return errors as strings to the frontend. Use try/catch:

```typescript
try {
  const result = await invoke('command_name', { ... });
} catch (error) {
  // error is a string describing the CryptoError variant
  console.error('Command failed:', error);
}
```

### Common Error Variants
- `"Encryption failed: ..."`
- `"Decryption failed — wrong password or corrupted data"`
- `"Database error: ..."`
- `"Container not found: ..."`
- `"Session not active — container must be unlocked first"`
- `"Integrity check failed — container may be corrupted or tampered with"`

---

## Last Updated
2026-03-14 - Created (commands to be implemented in Phase 1)
