# Changelog

All notable changes to Cryptainer are documented here.

## [Unreleased]

### Added
- Comprehensive documentation rewrite (IDEA.md, ARCHITECTURE.md, API.md, CRYPTO.md, SETUP.md, CHANGELOG.md)

### Fixed
- Auto-lock reset race condition: `updateActivity` now calls `setIsLocked(false)` before `setLastActivity(Date.now())` to prevent immediate re-trigger when activity events fire while the lock state is still set

## 0.1.0 — 2026-06-19

### v2 Per-File Encryption Format

- **Per-file encryption**: Each file in a container is now encrypted individually using `encrypt_section`/`decrypt_section`, enabling lazy decryption and selective file access.
- **Lazy decryption**: `unlock_container` only decrypts the metadata section to list files. Individual files are decrypted on demand via `get_file_data`.
- **LRU cache**: Decrypted file data is cached in memory (50 MB default) with LRU eviction and automatic zeroization on eviction/drop.
- **`release_file_data` IPC**: Explicitly release a file from cache, zeroizing its contents. Called by preview components on unmount.
- **`lock_container` IPC**: Lock a container, wiping all key material and cached file data from memory.
- **V1 → V2 migration**: Containers created with the v1 format are automatically migrated to v2 on first unlock.
- **Video chunking**: Video files larger than 2 MB are chunked for streaming-friendly playback.
- **Chunk metadata** in `FileMetadata.chunks` for per-chunk nonce and offset tracking.

### Frontend

- **Full component tree**: VaultGrid, ContainerModal (Lock→Open→Edit→Preview), CreateWizard (2-step), Settings screen.
- **Preview system**: Image, Video, Audio, PDF (iframe), Text (syntax-highlighted with Prism.js), Hex dump viewers.
- **Search & filter**: Search containers by name/tags, filter by tag, sort by name/date/size/file count.
- **Auto-lock**: Configurable inactivity timeout (default 5 min). Resets on user activity (mousedown, keydown, touchstart, scroll).
- **Settings**: Auto-lock timeout configuration, saved to localStorage.
- **Import/Export UI**: File dialogs for `.ctnr` import/export, batch import.

### Backend

- **AES-256-GCM** with Argon2id KDF (4 security presets: Fast/Standard/High/Paranoid).
- **SQLite storage** via sqlx with migrations.
- **Session management**: `SessionStore` (v1) and `SessionStoreV2` (v2 with LRU cache).
- **`.ctnr` export format**: Self-describing binary format with plaintext header + encrypted blob.
- **10 IPC commands**: `create_container`, `unlock_container`, `get_file_data`, `release_file_data`, `save_edits`, `list_containers`, `delete_container`, `lock_container`, `export_container`, `import_container`.
- **Unit tests** for crypto, session, vault, export modules.
- **Integration tests** for v2 format roundtrip, v1→v2 migration, tamper detection.

### Project Scaffold

- Tauri v2 project with React 18 + TypeScript + Vite.
- Zustand state management.
- CSS custom property theme system (dark theme default).
- Linux/macOS/Windows build support.

---

## v1 (Pre-0.1.0) — Legacy

Initial scaffold implementing single-blob encryption:

- Entire container payload encrypted as one AES-256-GCM blob.
- 8 IPC commands (no `release_file_data` or `lock_container`).
- No per-file lazy decryption — entire container decrypted on unlock.
- No LRU cache — all file data held in memory for session duration.
- No video chunking.
- No auto-lock.
- Minimal frontend scaffold.

*Note: v1 containers are automatically migrated to v2 on unlock.*
