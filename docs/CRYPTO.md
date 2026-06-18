# Cryptainer — Cryptographic Specifications

## Algorithm Overview

| Operation | Algorithm | Crate |
|---|---|---|
| Encryption | AES-256-GCM | [`aes-gcm`](https://crates.io/crates/aes-gcm) v0.10 |
| Key Derivation | Argon2id | [`argon2`](https://crates.io/crates/argon2) v0.5 |
| Integrity (blob) | SHA-256 | [`sha2`](https://crates.io/crates/sha2) v0.10 |
| Random | OsRng (CSPRNG) | [`rand`](https://crates.io/crates/rand) v0.8 |
| Memory Wiping | Zeroizing | [`zeroize`](https://crates.io/crates/zeroize) v1.7 |

## AES-256-GCM (Authenticated Encryption)

- **Key Size**: 256 bits (32 bytes)
- **Nonce Size**: 96 bits (12 bytes) — randomly generated per encryption operation
- **Tag Size**: 128 bits (16 bytes) — appended automatically by `aes-gcm`
- **Rationale**: GCM provides both confidentiality and authenticated integrity in a single pass. No padding is needed (avoiding padding oracle attacks). Hardware-accelerated via AES-NI on modern CPUs.

### Nonce Discipline

GCM with a 96-bit nonce has a 2³² message limit before the probability of nonce reuse becomes significant. Within that limit, **nonces must never repeat for the same key**. Cryptainer ensures this by:

1. Generating random 12-byte nonces via `OsRng` for every encryption call.
2. Storing the nonce alongside the ciphertext (in the file metadata or blob header).
3. Never reusing a nonce — even for re-encryption during `save_edits`, every file gets a fresh nonce.

## Argon2id (Key Derivation)

- **Algorithm**: Argon2id (winner of the Password Hashing Competition)
- **Salt**: 16 bytes, randomly generated per container
- **Key Output**: 256 bits (32 bytes)
- **Version**: 0x13

### Why Argon2id?

| Variant | Property | Use Case |
|---|---|---|
| Argon2d | Maximizes GPU resistance | Cryptocurrency, proof-of-work |
| Argon2i | Side-channel resistant | Key derivation with secret-independent memory access |
| **Argon2id** | **Hybrid** — i-first, d-later | **Best all-round choice** — side-channel resistant with GPU resistance |

### Security Presets

| Preset | Memory (KB) | Iterations | Parallelism | Unlock Time (approx.) |
|---|---|---|---|---|
| Fast | 16,384 (16 MB) | 1 | 1 | ~0.2s |
| **Standard (default)** | **65,536 (64 MB)** | **2** | **1** | **~1s** |
| High | 131,072 (128 MB) | 3 | 1 | ~2-3s |
| Paranoid | 262,144 (256 MB) | 4 | 2 | ~5-8s |

*Unlock times depend on CPU. Values shown are approximate on modern hardware.*

### Trade-Offs

Higher memory and iteration counts increase brute-force resistance but slow down legitimate unlock operations. The "Standard" preset is recommended for most users — it uses 64 MB of RAM and takes ~1 second on modern hardware, making it a good balance of security and usability.

## Encryption Flow

### V1 (Legacy) — Single-Blob Encryption

Used by containers created in v0.1.0 before the v2 migration. Still supported for read access; automatically migrated to v2 on unlock.

1. Serialize all files + metadata into a JSON `ContainerPayload`
2. Generate random 16-byte salt + 12-byte nonce
3. Derive 32-byte key: `Argon2id(password, salt, params)`
4. Encrypt: `AES-256-GCM(key, nonce, plaintext)`
5. Output blob: `[salt (16)] + [nonce (12)] + [ciphertext + GCM tag]`

### V2 (Current) — Per-File Encryption

Each file is encrypted individually. The metadata section (encrypted separately) contains file offsets, per-file nonces, and SHA-256 hashes.

```
v2 Blob Layout:
┌─────────────────────────────────────────────────┐
│ Salt (16 bytes)                                  │
├─────────────────────────────────────────────────┤
│ Metadata section length (4 bytes, little-endian) │
├─────────────────────────────────────────────────┤
│ Metadata nonce (12 bytes)                        │
├─────────────────────────────────────────────────┤
│ Metadata ciphertext (variable)                   │
│   Contains: ContainerMetadataV2 JSON             │
│   (version, file list with id, name, mime, size, │
│    offset, data_nonce, sha256, optional chunks)  │
├─────────────────────────────────────────────────┤
│ File 1 ciphertext (variable)                     │
│ File 2 ciphertext (variable)                     │
│ ...                                              │
│ File N ciphertext (variable)                     │
└─────────────────────────────────────────────────┘
```

#### Encryption (create_container)

1. Generate random 16-byte salt
2. Derive key: `Argon2id(password, salt, kdf_params)`
3. For each file:
   - Compute SHA-256 of plaintext
   - `encrypt_section(plaintext, key)` → `(ciphertext, nonce)`
   - Build `FileMetadata` with id, name, mime, size, offset (to be filled), nonce, sha256
4. Serialize all `FileMetadata` into `ContainerMetadataV2` JSON
5. `encrypt_section(metadata_json, key)` → `(encrypted_meta, meta_nonce)`
6. Calculate file offsets from encrypted metadata size
7. Re-encrypt metadata with correct offsets
8. Assemble blob: `salt | meta_len | meta_nonce | encrypted_meta | file1 | file2 | ...`

#### Decryption (unlock_container + get_file_data)

1. Read blob, verify SHA-256 against stored checksum
2. Extract salt → `derive_key(password, salt, params)` → key
3. Read metadata section length (4 bytes at offset 16)
4. Read metadata nonce → `decrypt_section(metadata_ct, key, meta_nonce)` → metadata
5. Return file list (metadata only — no file data)
6. On `get_file_data(file_id)`:
   - Look up file metadata (offset, data_nonce, sha256)
   - Seek to offset in blob, read ciphertext
   - `decrypt_section(file_ct, key, file_nonce)` → plaintext
   - Verify SHA-256 of decrypted data
   - Insert into LRU cache

## Key Material Handling

### Memory Lifecycle

```
Password entered ──→ Argon2id KDF ──→ AES Key (Zeroizing<[u8; 32]>)
                                               │
                                               ├── Used for encrypt_section / decrypt_section
                                               │
                                               ▼
                                  Stored in SessionV2.key (Zeroizing<[u8; 32]>)
                                               │
                                               ├── Used for get_file_data, save_edits
                                               │
                                               ▼
                              On lock_container() or SessionV2::drop():
                              Zeroizing wipe → memory zeroed, then freed
```

### Decrypted File Data Lifecycle

```
get_file_data() → decrypted bytes
       │
       ├── Inserted into LRU cache (CachedFile { data: Vec<u8> })
       │     │
       │     ├── On cache eviction (LRU): CachedFile::drop → data.fill(0) → data.clear()
       │     ├── On release_file_data(): same zeroization path
       │     └── On lock(): SessionV2::lock → cache.clear() → every CachedFile::drop zeroizes
       │
       └── Passed to frontend via IPC (as Vec<u8>)
             Frontend creates a Blob URL for preview, revokes on unmount
```

### What NEVER Happens

- Keys are NEVER written to disk
- Plaintext file data is NEVER written to disk (except temporarily in RAM for preview)
- Decrypted data is NEVER swapped to disk accidentally (mlock not yet implemented — see security notes)

## Integrity Verification

### Blob-Level

- `ContainerMeta.blob_sha256` stores SHA-256 of the entire encrypted blob
- Verified before every unlock attempt
- Detects disk corruption, partial writes, and tampering

### Per-File

- `FileMetadata.sha256` stores SHA-256 of each file's plaintext
- Verified after decryption in `get_file_data_v2`
- Catches corruption or tampering of individual file ciphertexts

### GCM Authentication

- Every AES-256-GCM encryption produces a 128-bit authentication tag
- Decryption fails if the tag doesn't match (wrong key or tampered ciphertext)
- The tag is verified before any plaintext is returned (aes-gcm crate behavior)

## .ctnr Export Format

Used for portable container export/import. Wraps the internal blob in a self-describing file.

### Binary Layout

```
| Offset | Size | Field |
|--------|------|-------|
| 0      | 4    | Magic: "CNTR" |
| 4      | 1    | Null byte: 0x00 |
| 5      | 1    | Format version: 0x01 |
| 6      | 4    | Header length (u32 LE) |
| 10     | N    | JSON header (plaintext) |
| 10+N   | M    | Encrypted blob (AES-256-GCM) |
```

### Header Fields (Plaintext JSON)

```json
{
  "id": "uuid-string",
  "name": "Container Name",
  "algo": "AES-GCM-256",
  "kdf": "argon2id",
  "kdf_params": { "kdf": "argon2id", "memory_kb": 65536, "iterations": 2, "parallelism": 1 },
  "hint": "optional password hint",
  "tags": "tag1,tag2",
  "file_count": 3,
  "total_size": 1048576,
  "created_at": "2026-01-01T00:00:00Z",
  "modified_at": "2026-01-01T00:00:00Z",
  "blob_sha256": "abc123...",
  "format_version": 2
}
```

The header is plaintext so the app can display container metadata and verify blob integrity before asking for a password.

## V1 → V2 Migration

When a v1 container (format_version=1) is unlocked, automatic migration occurs:

1. Decrypt the v1 single blob with legacy [`crypto::decrypt`]
2. Deserialize the legacy `ContainerPayload` (all files in one JSON blob)
3. Call `convert_v1_to_v2()`:
   - Generate fresh salt
   - Derive new key
   - Encrypt each file with `encrypt_section`
   - Build v2 blob with metadata section
4. Atomic write (tmp → rename) to replace the blob on disk
5. Update `format_version` to 2 in SQLite
6. Update `blob_sha256` and file counts in SQLite
7. Proceed with normal v2 session creation

Migration is one-way and irreversible. The v1 blob is overwritten atomically (on supported filesystems).

## LRU Cache with Zeroization

The v2 session includes a bounded LRU cache for decrypted file data:

- **Maximum size**: 50 MB by default (`DEFAULT_MAX_CACHE_BYTES`)
- **Policy**: Least-recently-used eviction
- **Zeroization**: On eviction, `CachedFile::drop` fills the data buffer with zeros (`data.fill(0)`) before releasing memory
- **Explicit release**: `release_file_data()` IPC removes a file from cache immediately
- **Automatic release**: On `lock_container()` or session drop, the entire cache is cleared and zeroized

## Security Audit Notes

- All cryptographic operations use well-reviewed, battle-tested crates
- No custom crypto implementations
- All parameters are documented and justified
- Memory safety enforced via Rust's ownership + `zeroize`
- Constant-time operations are NOT yet audited (future enhancement)
- `mlock`/`mprotect` to prevent swapping of key material is NOT yet implemented (future enhancement)

---

*Last updated: 2026-06-19*
