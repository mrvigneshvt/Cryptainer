# Cryptainer — Architecture Documentation

## Project Overview

**Cryptainer** is an offline encrypted container manager built with:

- **Frontend**: React 18 + TypeScript + Vite
- **Backend**: Rust (Tauri v2)
- **Cryptography**: AES-256-GCM + Argon2id
- **Storage**: SQLite via sqlx + encrypted blobs on disk
- **State Management**: Zustand

## Project Structure

```
cryptainer/
├── docs/                       # Project documentation
│   ├── ARCHITECTURE.md         # This file
│   ├── IDEA.md                 # Project vision & design philosophy
│   ├── API.md                  # IPC command reference
│   ├── CRYPTO.md               # Cryptographic specifications
│   ├── SETUP.md                # Development setup guide
│   └── CHANGELOG.md            # Release history
│
├── src/                        # React frontend (TypeScript)
│   ├── components/
│   │   ├── Container/          # Container UI: vault grid, modal, wizard
│   │   │   ├── ContainerModal/  # Lock → Open → Edit → Preview views
│   │   │   │   ├── index.tsx    # Modal state machine (locked/open/edit/preview)
│   │   │   │   ├── LockView.tsx # Password entry
│   │   │   │   ├── OpenView.tsx # File list inside unlocked container
│   │   │   │   └── EditView.tsx # Add/remove files, re-encrypt
│   │   │   └── CreateWizard/   # Step-by-step creation flow
│   │   │       ├── index.tsx    # Orchestrator (Step1 → Step2)
│   │   │       ├── Step1Files.tsx  # File selection with DropZone
│   │   │       └── Step2Config.tsx # Password, KDF params, tags, hint
│   │   ├── Preview/            # File preview components
│   │   │   ├── PreviewRouter.tsx  # MIME-based routing
│   │   │   ├── ImagePreview.tsx
│   │   │   ├── TextPreview.tsx    # With Prism.js syntax highlighting
│   │   │   ├── HexPreview.tsx     # Binary file hex dump
│   │   │   ├── VideoPreview.tsx   # Video playback with cleanup
│   │   │   └── ImagePreview.tsx   # Image display with cleanup
│   │   ├── Settings/           # Settings screen
│   │   │   ├── Settings.tsx    # Theme, auto-lock timeout, default security
│   │   │   └── index.ts
│   │   └── UI/                 # Shared UI primitives
│   │       ├── Button.tsx
│   │       ├── Input.tsx
│   │       ├── Modal.tsx
│   │       ├── DropZone.tsx    # File drag-and-drop area
│   │       ├── PasswordStrength.tsx
│   │       └── index.ts
│   ├── hooks/                  # Custom React hooks
│   │   ├── useAutoLock.ts      # Inactivity timeout → lock all containers
│   │   └── useMediaQuery.ts
│   ├── store/
│   │   └── vaultStore.ts       # Zustand store — all IPC wrappers
│   ├── types/
│   │   └── vault.ts            # TypeScript interfaces mirroring Rust structs
│   ├── utils/
│   │   └── format.ts           # formatBytes helper
│   ├── styles/
│   │   └── global.css          # Theme system (CSS custom properties)
│   ├── App.tsx                 # Root component: vault grid, toolbar, modals
│   ├── App.css
│   └── main.tsx                # React entry point
│
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs             # Entry point (desktop)
│   │   ├── lib.rs              # Tauri builder: plugins, state, IPC handler registration
│   │   ├── error.rs            # CryptoError enum — unified error type
│   │   ├── crypto.rs           # AES-256-GCM encrypt/decrypt, Argon2id KDF
│   │   ├── vault.rs            # Data structures: ContainerMeta, ContainerPayload,
│   │   │                       #   ContainerMetadataV2, FileMetadata, ChunkMetadata
│   │   ├── storage.rs          # SQLite CRUD via sqlx
│   │   ├── session.rs          # In-memory session store + LRU cache with zeroization
│   │   ├── export.rs           # .ctnr file format serialization/deserialization
│   │   └── commands.rs         # 10 Tauri IPC commands
│   ├── migrations/             # SQLite migrations
│   │   ├── 001_initial.sql     # Containers table
│   │   └── 002_add_format_version.sql
│   ├── tests/                  # Integration tests
│   │   └── v2_integration.rs   # v2 format roundtrip, migration, tamper detection
│   ├── icons/                  # App icons
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Tauri configuration
│
├── tests/                      # (reserved for e2e tests)
├── package.json                # npm dependencies
├── tsconfig.json               # TypeScript configuration
├── vite.config.ts              # Vite build configuration
└── index.html                  # HTML entry point
```

## Dependencies

### Rust (Cargo.toml)

| Crate | Version | Purpose |
|---|---|---|
| `tauri` | 2 | Application framework |
| `tauri-plugin-dialog` | 2 | File open/save dialogs |
| `tauri-plugin-fs` | 2 | Filesystem access |
| `tauri-plugin-shell` | 2 | Shell integration |
| `aes-gcm` | 0.10 | AES-256-GCM authenticated encryption |
| `argon2` | 0.5 | Argon2id key derivation (memory-hard KDF) |
| `serde` / `serde_json` | 1 | JSON serialization for IPC and persistence |
| `sqlx` | 0.7 | SQLite ORM with migrations |
| `uuid` | 1 | Container and file UUID generation |
| `lru` | 0.12 | Bounded LRU cache for decrypted file data |
| `chrono` | 0.4 | ISO-8601 timestamps |
| `zeroize` | 1.7 | `Zeroizing<>` wrapper — secure memory wiping |
| `secrecy` | 0.8 | Secret string handling (serde support) |
| `rand` | 0.8 | Cryptographically secure random (salt, nonce) |
| `sha2` / `hex` | 0.10 | SHA-256 integrity checksums |
| `thiserror` / `anyhow` | 1 | Error handling |
| `tokio` | 1 | Async runtime (for sqlx) |

### Frontend (package.json)

| Package | Purpose |
|---|---|
| `react` / `react-dom` ^18.2 | UI framework |
| `@tauri-apps/api` ^2 | Tauri IPC (`invoke`) |
| `@tauri-apps/plugin-dialog` ^2 | Native file dialogs |
| `@tauri-apps/plugin-fs` ^2 | File I/O helpers |
| `zustand` ^4.5 | Lightweight state management |
| `react-dropzone` ^15 | Drag-and-drop file upload |
| `prism-react-renderer` ^2.4 | Syntax highlighting |
| `vite` ^5 | Build tool |
| `@vitejs/plugin-react` | React Fast Refresh |

## Module Graph (Rust Backend)

```
main.rs
  └── lib.rs
        ├── error.rs           ← CryptoError enum (used by every module)
        ├── vault.rs           ← Data structures (used by all modules)
        ├── crypto.rs           ← encrypt, decrypt, derive_key, encrypt_section, decrypt_section
        │     └── uses: aes-gcm, argon2, sha2, zeroize, rand
        ├── storage.rs         ← insert_container, list_containers, get_container, etc.
        │     └── uses: sqlx, vault::ContainerMeta
        ├── session.rs         ← SessionStore (v1), SessionStoreV2 (v2 with LRU cache)
        │     └── uses: lru, zeroize, vault::ContainerMetadataV2
        ├── export.rs          ← .ctnr serialize/deserialize
        │     └── uses: vault::ContainerMeta, crypto
        └── commands.rs        ← 10 IPC command handlers
              └── uses: crypto, storage, vault, session, export
```

## Component Tree (Frontend)

```
<App>
  ├── Header (logo + actions: Settings, Import, New Container)
  ├── Toolbar (search input, sort dropdown)
  ├── Tag Filter (if any tags exist)
  ├── Vault Grid
  │   └── ContainerCard[] (name, algo badge, file count, size, date, export/delete)
  ├── <CreateWizard>           (modal)
  │   ├── Step1Files           (file picker + DropZone)
  │   └── Step2Config          (password, KDF preset, tags, hint)
  ├── <ContainerModal>         (modal, state machine)
  │   ├── LockView             (password entry)
  │   ├── OpenView             (file list, edit button, lock button)
  │   │   └── FileRow[] → <PreviewRouter>
  │   ├── EditView             (add/remove files, re-encrypt)
  │   └── <PreviewRouter>      (MIME-based routing)
  │       ├── <ImagePreview>
  │       ├── <TextPreview>
  │       ├── <HexPreview>
  │       ├── <VideoPreview>
  │       ├── <AudioPreview>   (inline in PreviewRouter)
  │       └── <PdfPreview>     (inline in PreviewRouter)
  └── <Settings>               (modal)
```

## Data Flow

### Creating a Container (v2)

```
Frontend                          Rust Backend
─────────                         ───────────
1. User selects files
   & configures password/KDF
         │
2. invoke('create_container') ──→ 3. Validate input
                                   4. Generate salt + derive AES key (Argon2id)
                                   5. For each file:
                                      - Compute SHA-256
                                      - encrypt_section(file_data, key) → ciphertext + nonce
                                      - Build FileMetadata (offset, nonce, sha256)
                                   6. Encrypt metadata section
                                   7. Assemble v2 blob: salt | meta_len | meta_nonce | meta_ct | file_cts
                                   8. Write .enc blob to app_data/blobs/
                                   9. Insert ContainerMeta row into SQLite
         ←── ContainerMeta       10. Return metadata
```

### Unlocking & Reading a File (v2)

```
Frontend                          Rust Backend
─────────                         ───────────
1. Enter password
         │
2. invoke('unlock_container') ──→ 3. Read blob + verify SHA-256 integrity
                                   4. Derive key from password + salt (Argon2id)
                                   5. Decrypt metadata section → ContainerMetadataV2
                                   6. Create SessionV2 (key, salt, metadata, LRU cache)
                                   7. Store session in SessionStoreV2
         ←── file-metadata[]      8. Return file list (no file data)
         │
9. Click on a file
         │
10. invoke('get_file_data') ────→ 11. Look up session → check LRU cache (hit → return)
                                   12. Cache miss → seek to file offset in .enc blob
                                   13. decrypt_section(file_ciphertext, key, file_nonce)
                                   14. Verify SHA-256
                                   15. Store in LRU cache (evict oldest if over 50MB)
         ←── file bytes           16. Return plaintext bytes
         │
17. PreviewRouter renders
    (image/video/text/hex/audio/PDF)
```

### Locking / Releasing Memory

```
Frontend                          Rust Backend
─────────                         ──────────
◉ invoke('release_file_data') ──→ Remove file from LRU cache, CachedFile::drop zeroizes
◉ invoke('lock_container')    ──→ Remove session from store, SessionV2::lock clears cache
◉ Preview component unmounts  ──→ React useEffect cleanup calls releaseFileData
```

## V2 Blob Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Salt (16 bytes) — random, used for Argon2id KDF             │
├─────────────────────────────────────────────────────────────┤
│ Metadata Section Length (4 bytes, u32 LE)                   │
├─────────────────────────────────────────────────────────────┤
│ Metadata Nonce (12 bytes) — random, per-metadata-encryption  │
├─────────────────────────────────────────────────────────────┤
│ Metadata Ciphertext (variable) — encrypted ContainerMetadataV2│
│   Contains: version, file list (id, name, mime, size,        │
│             offset, data_nonce, sha256, optional chunks[])   │
├─────────────────────────────────────────────────────────────┤
│ File 1 Ciphertext (variable) — encrypted with file's nonce   │
│ File 2 Ciphertext (variable)                                  │
│ ...                                                          │
│ File N Ciphertext (variable)                                  │
└─────────────────────────────────────────────────────────────┘
```

## Container State Machine

```
                ┌──────────┐
                │  Locked  │
                └────┬─────┘
                     │ unlock_container()
                     ▼
                ┌──────────┐
                │   Open   │ ◀──── PreviewRouter (file selected)
                └────┬─────┘
                     │ save_edits()
                     ▼
                ┌──────────┐
                │   Edit   │
                └────┬─────┘
                     │ save completes → auto return to Open
                     │
                ┌──────────┐
                │ Preview  │ ← opened from OpenView
                └──────────┘
                     │
                lock_container() or close modal → back to Locked
```

## SQLite Schema

```sql
CREATE TABLE containers (
    id           TEXT PRIMARY KEY NOT NULL,         -- UUID v4
    name         TEXT NOT NULL,
    algo         TEXT NOT NULL DEFAULT 'AES-GCM-256',
    kdf          TEXT NOT NULL DEFAULT 'argon2id',
    kdf_params   TEXT NOT NULL,                     -- JSON: KdfParams
    hint         TEXT,                              -- password hint (plaintext)
    tags         TEXT,                              -- comma-separated tags (plaintext)
    file_count   INTEGER NOT NULL DEFAULT 0,
    total_size   INTEGER NOT NULL DEFAULT 0,
    blob_path    TEXT NOT NULL UNIQUE,              -- path to .enc blob
    blob_sha256  TEXT NOT NULL,                     -- hex SHA-256 of blob
    created_at   TEXT NOT NULL,                     -- ISO-8601
    modified_at  TEXT NOT NULL,                     -- ISO-8601
    format_version INTEGER NOT NULL DEFAULT 1       -- 1=v1, 2=v2 per-file
);

CREATE INDEX idx_containers_name ON containers(name);
CREATE INDEX idx_containers_created_at ON containers(created_at);
```

## Theme System

The UI uses CSS custom properties defined in `src/styles/global.css`:

- **Dark theme** (default) with accent color `#4de0c0`
- Supports multiple themes via CSS variable swapping (prep for light theme)
- Font: Inter for UI, JetBrains Mono for code

## Security Architecture

See [`CRYPTO.md`](./CRYPTO.md) for detailed cryptographic specifications.
See [`IDEA.md`](./IDEA.md) for threat model and design philosophy.

Key points:
- Key material is wrapped in `Zeroizing<>` — automatically wiped on drop
- Encrypted blobs stored separately from metadata (both needed for access)
- GCM authentication tags prevent tampering
- Argon2id memory-hard KDF resists brute-force
- Sessions stored only in memory, cleared on lock/close
- LRU cache zeroizes file data on eviction (not just on lock)

## Version History

| Version | Description |
|---|---|
| v1 (legacy) | Single-encryption: entire container payload encrypted as one blob |
| v2 (current) | Per-file encryption: each file encrypted individually, lazy decryption, LRU cache |

---

*Last updated: 2026-06-19*
