# Cryptainer — Architecture Documentation

## Project Overview

**Cryptainer** is an offline encrypted container manager built with:
- **Frontend**: React 18 + TypeScript + Vite
- **Backend**: Rust (Tauri v2)
- **Cryptography**: AES-256-GCM + Argon2id
- **Storage**: SQLite via sqlx
- **State Management**: Zustand

## Project Structure

```
cryptainer/
├── docs/
│   ├── PROGRESS.md          ← Build progress tracking
│   ├── ARCHITECTURE.md      ← This file
│   ├── CRYPTO.md            ← Cryptographic documentation
│   └── API.md               ← IPC command documentation
│
├── src/                     ← React frontend
│   ├── components/
│   ├── hooks/
│   ├── store/
│   ├── types/
│   ├── utils/
│   ├── styles/
│   ├── App.tsx
│   └── main.tsx
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── error.rs
│   │   ├── crypto.rs
│   │   ├── vault.rs
│   │   ├── storage.rs
│   │   ├── export.rs
│   │   ├── session.rs
│   │   └── commands.rs
│   ├── migrations/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
└── tests/
```

## Dependencies

### Rust Dependencies
- `tauri = "2"` - Tauri v2 framework
- `tauri-plugin-dialog = "2"` - File dialogs
- `tauri-plugin-fs = "2"` - File system access
- `tauri-plugin-shell = "2"` - Shell commands
- `aes-gcm = "0.10"` - AES-256-GCM encryption
- `argon2 = "0.5"` - Argon2id key derivation
- `sqlx = "0.7"` - SQLite ORM
- `uuid = "1"` - UUID generation
- `chrono = "0.4"` - Date/time handling
- `thiserror = "1"` - Error handling
- `zeroize = "1.7"` - Secure memory wiping

### Frontend Dependencies
- `@tauri-apps/api = "^2"` - Tauri JavaScript API
- `react = "^18.2.0"` - React framework
- `zustand = "^4.4.0"` - State management
- `vite = "^5.0.0"` - Build tool

## Component Tree (Frontend)

### To be built in Phase 1:
- **App** - Root component
- **VaultGrid** - Container grid display
- **VaultToolbar** - Toolbar with actions
- **ContainerCard** - Individual container display
- **CreateWizard** - Multi-step creation wizard
- **ContainerModal** - Container management modal
- **Preview Components** - File preview system
- **UI Components** - Button, Input, Modal, etc.

## Module Graph (Rust Backend)

### To be built in Phase 1:
- **error** - Unified error types
- **crypto** - Encryption/decryption primitives
- **storage** - SQLite operations
- **vault** - Data structures
- **session** - In-memory session management
- **export** - .ctnr format serialization
- **commands** - Tauri IPC handlers

## Data Flow

### Create Container
1. Frontend: User selects files and configures encryption
2. IPC: `create_container` command invoked
3. Backend: Files encrypted with AES-256-GCM
4. Backend: Encrypted blob written to disk
5. Backend: Metadata inserted into SQLite
6. Frontend: Container appears in vault grid

### Unlock Container
1. Frontend: User enters password
2. IPC: `unlock_container` command invoked
3. Backend: Blob loaded from disk, integrity checked
4. Backend: Decryption with password-derived key
5. Backend: Session stored in memory (with zeroization)
6. Frontend: File list displayed

## SQLite Schema

### containers Table
```sql
CREATE TABLE containers (
    id           TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    algo         TEXT NOT NULL DEFAULT 'AES-GCM-256',
    kdf          TEXT NOT NULL DEFAULT 'argon2id',
    kdf_params   TEXT NOT NULL,
    hint         TEXT,
    tags         TEXT,
    file_count   INTEGER NOT NULL DEFAULT 0,
    total_size   INTEGER NOT NULL DEFAULT 0,
    blob_path    TEXT NOT NULL UNIQUE,
    blob_sha256  TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    modified_at  TEXT NOT NULL
);
```

## Theme System

CSS custom properties defined in `global.css`:
- Dark theme (default) with accent color `#4de0c0`
- Supports multiple themes via CSS variable swapping
- Font: Inter for UI, JetBrains Mono for code

## Security Considerations

- Key material wrapped in `Zeroizing<>` for automatic memory wiping
- Encrypted blobs stored separately from metadata database
- GCM authentication tags prevent tampering
- Argon2id memory-hard KDF resists brute-force
- Sessions stored only in memory, cleared on lock/close

## Current Status

**Phase 1: Project Scaffold Complete**
- ✅ Tauri v2 project initialized
- ✅ React + TypeScript configured
- ✅ Theme system implemented
- ✅ Dependencies installed
- ⏳ Pending: Rust modules (crypto, storage, etc.)
- ⏳ Pending: Frontend components
- ⏳ Pending: IPC commands

## Last Updated
2026-03-14 - Step 1.1 Complete
