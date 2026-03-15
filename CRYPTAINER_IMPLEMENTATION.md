# CRYPTAINER — Claude Code Implementation Guide
> Offline Encrypted Container Manager · Tauri v2 + React + TypeScript + Rust

---

## 📋 AGENT INSTRUCTIONS (READ FIRST)

> **This section is mandatory reading before executing any step.**

You are an AI coding agent tasked with building **Cryptainer** end-to-end. Follow these rules without exception:

### Documentation Mandate
- **After completing every single step**, update `/docs/PROGRESS.md` with what was done, what files were created/modified, and any decisions made.
- **Maintain `/docs/ARCHITECTURE.md`** — update it whenever a new module, command, or data structure is introduced. It must always reflect the current state of the codebase, not the planned state.
- **Maintain `/docs/CRYPTO.md`** — every cryptographic decision, parameter, and rationale must be documented here. Never change a crypto parameter without updating this file.
- **Maintain `/docs/API.md`** — every Tauri IPC command must be documented with its input types, output types, error cases, and a usage example.
- **Write inline comments** on every non-trivial function in both Rust and TypeScript. Security-sensitive code must have a comment explaining *why* it is written the way it is, not just *what* it does.
- **Never skip a step.** If a step fails, document the failure in `PROGRESS.md` with the error and the fix before moving on.
- **Run tests after every phase** and record results in `PROGRESS.md`.
- **Never hardcode secrets, paths, or magic numbers** without a named constant and a comment.

### Code Quality Rules
- All Rust code must pass `cargo clippy -- -D warnings` with zero warnings.
- All TypeScript must pass `tsc --noEmit` with zero errors.
- All new Rust modules must have at least one `#[cfg(test)]` block with meaningful unit tests.
- Commit-ready code only — no `TODO`, `FIXME`, `unwrap()` on fallible operations, or `any` types in TypeScript.
- Use `?` operator and `thiserror` for error propagation in Rust. Never panic in production paths.

---

## 🗂️ PROJECT OVERVIEW

| Item | Value |
|---|---|
| **Project Name** | Cryptainer |
| **Binary Name** | `cryptainer` |
| **Frontend** | React 18 + TypeScript + Vite |
| **Backend** | Rust (Tauri v2) |
| **Crypto** | AES-256-GCM + Argon2id (via `aes-gcm` + `argon2` crates) |
| **Storage** | SQLite via `sqlx` |
| **State Management** | Zustand |
| **Platforms** | Windows · macOS · Linux (Phase 1) · Android · iOS (Phase 4) |
| **Export Format** | `.ctnr` (custom binary format) |

---

## 📁 FINAL PROJECT STRUCTURE (Target)

```
cryptainer/
├── docs/
│   ├── PROGRESS.md          ← agent updates after every step
│   ├── ARCHITECTURE.md      ← always current system design
│   ├── CRYPTO.md            ← all crypto decisions documented
│   └── API.md               ← all Tauri IPC commands documented
│
├── src/                     ← React frontend
│   ├── components/
│   │   ├── Vault/
│   │   │   ├── VaultGrid.tsx
│   │   │   ├── VaultList.tsx
│   │   │   ├── VaultToolbar.tsx
│   │   │   └── index.ts
│   │   ├── Container/
│   │   │   ├── ContainerCard.tsx
│   │   │   ├── CreateWizard/
│   │   │   │   ├── Step1Files.tsx
│   │   │   │   ├── Step2Config.tsx
│   │   │   │   └── index.tsx
│   │   │   ├── ContainerModal/
│   │   │   │   ├── LockView.tsx
│   │   │   │   ├── OpenView.tsx
│   │   │   │   ├── EditView.tsx
│   │   │   │   └── index.tsx
│   │   │   └── index.ts
│   │   ├── Preview/
│   │   │   ├── ImagePreview.tsx
│   │   │   ├── TextPreview.tsx
│   │   │   ├── HexPreview.tsx
│   │   │   ├── VideoPreview.tsx
│   │   │   └── PreviewRouter.tsx
│   │   └── UI/
│   │       ├── Button.tsx
│   │       ├── Input.tsx
│   │       ├── Modal.tsx
│   │       ├── DropZone.tsx
│   │       ├── PasswordStrength.tsx
│   │       ├── ProgressBar.tsx
│   │       └── index.ts
│   ├── hooks/
│   │   ├── useVault.ts
│   │   ├── useSession.ts
│   │   └── useExport.ts
│   ├── store/
│   │   └── vaultStore.ts
│   ├── types/
│   │   └── vault.ts
│   ├── utils/
│   │   ├── format.ts
│   │   └── fileIcons.ts
│   ├── App.tsx
│   ├── main.tsx
│   └── styles/
│       └── global.css
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── error.rs          ← unified error type
│   │   ├── crypto.rs         ← Argon2id + AES-256-GCM
│   │   ├── vault.rs          ← container CRUD logic
│   │   ├── storage.rs        ← SQLite via sqlx
│   │   ├── export.rs         ← .ctnr format serializer/deserializer
│   │   ├── session.rs        ← in-memory session + key zeroize
│   │   └── commands.rs       ← all #[tauri::command] handlers
│   ├── migrations/
│   │   └── 001_initial.sql
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── tests/
│   └── crypto_integration.rs
│
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## ⚙️ PHASE 1 — Project Scaffold & Core Desktop

### STEP 1.1 — Initialize Tauri v2 Project

```bash
# Create the Tauri v2 project with React + TypeScript template
npm create tauri-app@latest cryptainer -- --template react-ts
cd cryptainer

# Verify Tauri v2 is installed
npm install
cargo tauri info
```

**After this step, agent must:**
- Confirm `src-tauri/Cargo.toml` shows `tauri = "2"` in dependencies
- Confirm `package.json` shows `@tauri-apps/api` version `^2`
- Create `/docs/PROGRESS.md` with entry: `STEP 1.1 complete — project scaffolded`
- Create `/docs/ARCHITECTURE.md` with initial project description

---

### STEP 1.2 — Configure Tauri App Identity

Edit `src-tauri/tauri.conf.json`:

```json
{
  "productName": "Cryptainer",
  "version": "0.1.0",
  "identifier": "com.cryptainer.app",
  "app": {
    "windows": [
      {
        "title": "Cryptainer",
        "width": 1100,
        "height": 720,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
}
```

**After this step, agent must:**
- Run `cargo tauri dev` and confirm the app window opens
- Update `PROGRESS.md`

---

### STEP 1.3 — Add Tauri Plugins

```bash
# Add required Tauri plugins
cargo add tauri-plugin-dialog --manifest-path src-tauri/Cargo.toml
cargo add tauri-plugin-fs --manifest-path src-tauri/Cargo.toml
cargo add tauri-plugin-shell --manifest-path src-tauri/Cargo.toml
```

Register plugins in `src-tauri/src/main.rs`:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // commands registered here in Step 1.8
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cryptainer");
}
```

Add to `package.json` dependencies:
```bash
npm install @tauri-apps/plugin-dialog @tauri-apps/plugin-fs
```

---

### STEP 1.4 — Add Rust Dependencies

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Cryptography
aes-gcm = "0.10"
argon2 = { version = "0.5", features = ["std"] }
rand = { version = "0.8", features = ["getrandom"] }
zeroize = { version = "1.7", features = ["derive"] }
secrecy = { version = "0.8", features = ["serde"] }
sha2 = "0.10"
hex = "0.4"

# Storage
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "macros", "migrate"] }
tokio = { version = "1", features = ["full"] }

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```

Run `cargo build` to verify all deps resolve before continuing.

**After this step, agent must:**
- Confirm `cargo build` exits with code 0
- Update `PROGRESS.md` and `ARCHITECTURE.md` with dependency list

---

### STEP 1.5 — Unified Error Type

Create `src-tauri/src/error.rs`:

```rust
//! Unified error type for Cryptainer.
//!
//! All fallible operations return CryptoError or map into it.
//! This ensures consistent, descriptive errors bubble up to the IPC layer
//! and are surfaced to the frontend rather than crashing the process.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Decryption failed — wrong password or corrupted data")]
    Decryption,

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid container file: {0}")]
    InvalidFormat(String),

    #[error("Container not found: {0}")]
    NotFound(String),

    #[error("Session not active — container must be unlocked first")]
    SessionInactive,

    #[error("Integrity check failed — container may be corrupted or tampered with")]
    IntegrityFailure,
}

/// Make CryptoError serializable so Tauri can send it to the frontend as a string.
impl serde::Serialize for CryptoError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}

pub type Result<T> = std::result::Result<T, CryptoError>;
```

Add `mod error;` and `pub use error::*;` to `src-tauri/src/lib.rs`.

---

### STEP 1.6 — Cryptography Module

Create `src-tauri/src/crypto.rs`:

```rust
//! Cryptainer cryptography primitives.
//!
//! All encryption uses AES-256-GCM (authenticated encryption).
//! Key derivation uses Argon2id — the current best-practice password KDF,
//! resistant to GPU and ASIC brute-force due to memory hardness.
//!
//! # Security design decisions:
//! - Salt is 16 bytes (128-bit), randomly generated per container
//! - Nonce is 12 bytes (96-bit GCM standard), randomly generated per encryption
//! - Key material is wrapped in Zeroizing<> so it is wiped from memory on drop
//! - Plaintext is never written to disk — only the encrypted blob is persisted

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng as AesOsRng},
    Aes256Gcm, Nonce, Key,
};
use argon2::{Argon2, Params, Version, Algorithm};
use rand::{RngCore, rngs::OsRng};
use sha2::{Sha256, Digest};
use zeroize::Zeroizing;

use crate::error::{CryptoError, Result};

/// Argon2id parameters.
/// These represent the "Standard" security level.
/// See docs/CRYPTO.md for full parameter rationale.
pub const ARGON2_MEMORY_KB:   u32 = 65536; // 64 MB
pub const ARGON2_ITERATIONS:  u32 = 2;
pub const ARGON2_PARALLELISM: u32 = 1;
pub const SALT_LEN:            usize = 16;
pub const NONCE_LEN:           usize = 12;
pub const KEY_LEN:             usize = 32; // AES-256

/// Supported KDF configurations selectable by the user.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KdfParams {
    pub kdf: String,               // "argon2id" | "pbkdf2"
    pub memory_kb: Option<u32>,    // Argon2id only
    pub iterations: u32,
    pub parallelism: Option<u32>,  // Argon2id only
}

impl KdfParams {
    pub fn argon2id_standard() -> Self {
        Self {
            kdf: "argon2id".into(),
            memory_kb: Some(ARGON2_MEMORY_KB),
            iterations: ARGON2_ITERATIONS,
            parallelism: Some(ARGON2_PARALLELISM),
        }
    }
}

/// Derive a 32-byte encryption key from a password and salt using Argon2id.
///
/// The resulting key is wrapped in Zeroizing so it will be wiped
/// from memory automatically when dropped. Never store the raw bytes
/// outside of a Zeroizing wrapper.
pub fn derive_key(password: &str, salt: &[u8], params: &KdfParams) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    match params.kdf.as_str() {
        "argon2id" => {
            let m = params.memory_kb.unwrap_or(ARGON2_MEMORY_KB);
            let t = params.iterations;
            let p = params.parallelism.unwrap_or(ARGON2_PARALLELISM);

            let argon2_params = Params::new(m, t, p, Some(KEY_LEN))
                .map_err(|e| CryptoError::KeyDerivation(e.to_string()))?;

            let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

            let mut key = Zeroizing::new([0u8; KEY_LEN]);
            argon2.hash_password_into(password.as_bytes(), salt, key.as_mut())
                .map_err(|e| CryptoError::KeyDerivation(e.to_string()))?;

            Ok(key)
        }
        _ => Err(CryptoError::KeyDerivation(format!("Unknown KDF: {}", params.kdf))),
    }
}

/// Encrypt plaintext bytes with AES-256-GCM.
///
/// Returns: [salt (16)] + [nonce (12)] + [ciphertext + GCM tag]
/// The GCM authentication tag is appended by the aes-gcm crate automatically.
pub fn encrypt(plaintext: &[u8], password: &str, params: &KdfParams) -> Result<Vec<u8>> {
    // Generate fresh random salt and nonce for every encryption operation.
    // Reusing a nonce with the same key under GCM is catastrophic — always generate fresh.
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt, params)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    // Layout: salt | nonce | ciphertext (includes 16-byte GCM tag at end)
    let mut output = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// Decrypt a blob produced by `encrypt()`.
///
/// Returns the original plaintext on success.
/// Returns CryptoError::Decryption on wrong password or tampered data.
/// The GCM tag verification happens inside aes-gcm::decrypt — if the tag
/// does not match, decryption is aborted and no plaintext is returned.
pub fn decrypt(blob: &[u8], password: &str, params: &KdfParams) -> Result<Vec<u8>> {
    if blob.len() < SALT_LEN + NONCE_LEN + 16 {
        return Err(CryptoError::InvalidFormat("Blob too short to be valid".into()));
    }

    let salt      = &blob[..SALT_LEN];
    let nonce_raw = &blob[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt, params)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
    let nonce = Nonce::from_slice(nonce_raw);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decryption)
}

/// Compute SHA-256 of data and return as a lowercase hex string.
/// Used for the integrity checksum stored in the container header.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> KdfParams {
        // Use minimal params in tests to keep them fast
        KdfParams { kdf: "argon2id".into(), memory_kb: Some(8192), iterations: 1, parallelism: Some(1) }
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"Hello, Cryptainer!";
        let password = "correct-horse-battery-staple";
        let params = test_params();

        let blob = encrypt(plaintext, password, &params).unwrap();
        let recovered = decrypt(&blob, password, &params).unwrap();

        assert_eq!(plaintext.as_ref(), recovered.as_slice());
    }

    #[test]
    fn wrong_password_returns_error() {
        let blob = encrypt(b"secret", "right-password", &test_params()).unwrap();
        let result = decrypt(&blob, "wrong-password", &test_params());
        assert!(matches!(result, Err(crate::error::CryptoError::Decryption)));
    }

    #[test]
    fn tampered_blob_returns_error() {
        let mut blob = encrypt(b"secret", "password", &test_params()).unwrap();
        // Flip a byte in the ciphertext region
        let last = blob.len() - 1;
        blob[last] ^= 0xFF;
        let result = decrypt(&blob, "password", &test_params());
        assert!(matches!(result, Err(crate::error::CryptoError::Decryption)));
    }

    #[test]
    fn sha256_hex_deterministic() {
        let h1 = sha256_hex(b"test data");
        let h2 = sha256_hex(b"test data");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn unique_salts_per_encryption() {
        let params = test_params();
        let blob1 = encrypt(b"same plaintext", "same password", &params).unwrap();
        let blob2 = encrypt(b"same plaintext", "same password", &params).unwrap();
        // Same plaintext + password must produce different ciphertexts (random salt/nonce)
        assert_ne!(blob1, blob2);
    }
}
```

Run `cargo test -p cryptainer-lib 2>&1` or equivalent and confirm all 5 tests pass.

**After this step, agent must:**
- Record test results in `PROGRESS.md`
- Create `docs/CRYPTO.md` with full parameter rationale

---

### STEP 1.7 — Storage Module (SQLite)

Create `src-tauri/migrations/001_initial.sql`:

```sql
-- Cryptainer database schema
-- Containers table stores ONLY metadata.
-- Encrypted blobs are stored as flat files; blob_path points to them.
-- This separation means the DB can be backed up without exposing any secrets.

CREATE TABLE IF NOT EXISTS containers (
    id           TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    algo         TEXT NOT NULL DEFAULT 'AES-GCM-256',
    kdf          TEXT NOT NULL DEFAULT 'argon2id',
    kdf_params   TEXT NOT NULL,          -- JSON: KdfParams struct
    hint         TEXT,                   -- nullable password hint (plaintext)
    tags         TEXT,                   -- nullable comma-separated tags
    file_count   INTEGER NOT NULL DEFAULT 0,
    total_size   INTEGER NOT NULL DEFAULT 0,
    blob_path    TEXT NOT NULL UNIQUE,   -- absolute path to .enc blob on disk
    blob_sha256  TEXT NOT NULL,          -- hex SHA-256 for integrity verification
    created_at   TEXT NOT NULL,
    modified_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_containers_name ON containers(name);
CREATE INDEX IF NOT EXISTS idx_containers_created_at ON containers(created_at);
```

Create `src-tauri/src/storage.rs`:

```rust
//! SQLite storage layer for Cryptainer.
//!
//! Only container metadata is stored in the database.
//! Encrypted blobs live on disk at blob_path.
//! This module handles all DB reads and writes — no crypto happens here.

use sqlx::{SqlitePool, Row};
use chrono::Utc;
use std::path::PathBuf;

use crate::error::{CryptoError, Result};
use crate::vault::ContainerMeta;

/// Initialize SQLite connection pool and run migrations.
pub async fn init_db(app_data_dir: &PathBuf) -> Result<SqlitePool> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("cryptainer.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url).await?;

    // Run embedded migrations from src-tauri/migrations/
    sqlx::migrate!("./migrations").run(&pool).await
        .map_err(|e| CryptoError::Database(sqlx::Error::from(e)))?;

    Ok(pool)
}

/// Insert a new container metadata row.
pub async fn insert_container(pool: &SqlitePool, meta: &ContainerMeta) -> Result<()> {
    let kdf_json = serde_json::to_string(&meta.kdf_params)?;
    sqlx::query!(
        r#"INSERT INTO containers
           (id, name, algo, kdf, kdf_params, hint, tags, file_count, total_size,
            blob_path, blob_sha256, created_at, modified_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        meta.id, meta.name, meta.algo, meta.kdf_params.kdf, kdf_json,
        meta.hint, meta.tags, meta.file_count, meta.total_size,
        meta.blob_path, meta.blob_sha256, meta.created_at, meta.modified_at
    )
    .execute(pool).await?;
    Ok(())
}

/// Fetch all container metadata rows, ordered by created_at descending.
pub async fn list_containers(pool: &SqlitePool) -> Result<Vec<ContainerMeta>> {
    let rows = sqlx::query!(
        r#"SELECT id, name, algo, kdf_params, hint, tags,
                  file_count, total_size, blob_path, blob_sha256,
                  created_at, modified_at
           FROM containers ORDER BY created_at DESC"#
    )
    .fetch_all(pool).await?;

    rows.iter().map(|r| {
        Ok(ContainerMeta {
            id: r.id.clone(),
            name: r.name.clone(),
            algo: r.algo.clone(),
            kdf_params: serde_json::from_str(&r.kdf_params)?,
            hint: r.hint.clone(),
            tags: r.tags.clone(),
            file_count: r.file_count as u32,
            total_size: r.total_size as u64,
            blob_path: r.blob_path.clone(),
            blob_sha256: r.blob_sha256.clone(),
            created_at: r.created_at.clone(),
            modified_at: r.modified_at.clone(),
        })
    }).collect()
}

/// Update file_count, total_size, blob_sha256, and modified_at after a re-encrypt.
pub async fn update_container_blob(
    pool: &SqlitePool, id: &str, file_count: u32,
    total_size: u64, blob_sha256: &str
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query!(
        r#"UPDATE containers
           SET file_count=?, total_size=?, blob_sha256=?, modified_at=?
           WHERE id=?"#,
        file_count, total_size, blob_sha256, now, id
    )
    .execute(pool).await?;
    Ok(())
}

/// Delete a container row by ID. Blob file deletion is handled by vault.rs.
pub async fn delete_container(pool: &SqlitePool, id: &str) -> Result<()> {
    let affected = sqlx::query!("DELETE FROM containers WHERE id=?", id)
        .execute(pool).await?.rows_affected();
    if affected == 0 {
        return Err(CryptoError::NotFound(id.to_string()));
    }
    Ok(())
}

/// Fetch a single container by ID.
pub async fn get_container(pool: &SqlitePool, id: &str) -> Result<ContainerMeta> {
    let r = sqlx::query!(
        r#"SELECT id, name, algo, kdf_params, hint, tags,
                  file_count, total_size, blob_path, blob_sha256,
                  created_at, modified_at
           FROM containers WHERE id=?"#, id
    )
    .fetch_optional(pool).await?
    .ok_or_else(|| CryptoError::NotFound(id.to_string()))?;

    Ok(ContainerMeta {
        id: r.id,
        name: r.name,
        algo: r.algo,
        kdf_params: serde_json::from_str(&r.kdf_params)?,
        hint: r.hint,
        tags: r.tags,
        file_count: r.file_count as u32,
        total_size: r.total_size as u64,
        blob_path: r.blob_path,
        blob_sha256: r.blob_sha256,
        created_at: r.created_at,
        modified_at: r.modified_at,
    })
}
```

---

### STEP 1.8 — Vault & Session Modules

Create `src-tauri/src/vault.rs`:

```rust
//! Vault module — container data types and high-level operations.
//!
//! This module defines the ContainerMeta struct (what lives in SQLite)
//! and the ContainerPayload struct (what is encrypted inside the blob).

use serde::{Deserialize, Serialize};
use crate::crypto::KdfParams;

/// All metadata stored in plaintext in SQLite.
/// Never includes encrypted content or key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMeta {
    pub id:          String,
    pub name:        String,
    pub algo:        String,       // "AES-GCM-256" | "AES-GCM-128"
    pub kdf_params:  KdfParams,
    pub hint:        Option<String>,
    pub tags:        Option<String>,
    pub file_count:  u32,
    pub total_size:  u64,          // total bytes of all files before encryption
    pub blob_path:   String,       // absolute path to the .enc file on disk
    pub blob_sha256: String,       // SHA-256 hex of the raw encrypted blob
    pub created_at:  String,       // ISO-8601
    pub modified_at: String,       // ISO-8601
}

/// A single file stored inside an encrypted container.
/// The data field holds the raw file bytes.
/// Stored as part of ContainerPayload, which is the plaintext inside the blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFile {
    pub id:       String,   // UUID v4
    pub name:     String,
    pub mime:     String,
    pub size:     u64,
    pub data:     Vec<u8>,  // raw file bytes — NEVER written to disk as plaintext
}

/// The plaintext content of a container.
/// This struct is serialized to JSON, then encrypted into the blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPayload {
    pub version:    u8,
    pub files:      Vec<VaultFile>,
}
```

Create `src-tauri/src/session.rs`:

```rust
//! In-memory session management.
//!
//! When a user unlocks a container, the decrypted payload is held here
//! in memory for the duration of the session. The session is cleared:
//!   - When the user manually locks the container
//!   - When the app closes (memory is reclaimed by the OS)
//!   - On future: idle timeout (Phase 3)
//!
//! Key material (the raw AES key) is stored in Zeroizing<> to ensure
//! it is wiped from memory when the session is dropped, not merely
//! marked as free — which could leave key bytes in RAM.

use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;
use crate::vault::ContainerPayload;

/// A single active decrypted session for one container.
pub struct Session {
    /// The decrypted payload held in memory.
    pub payload: ContainerPayload,
    /// The derived AES key, kept for re-encryption during edit mode.
    /// Wrapped in Zeroizing so bytes are wiped on drop.
    pub key: Zeroizing<Vec<u8>>,
}

/// Global session store — keyed by container ID.
pub struct SessionStore(pub Mutex<HashMap<String, Session>>);

impl SessionStore {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    /// Store a new session for a container.
    pub fn set(&self, container_id: String, session: Session) {
        self.0.lock().unwrap().insert(container_id, session);
    }

    /// Check if a session exists for a container.
    pub fn has(&self, container_id: &str) -> bool {
        self.0.lock().unwrap().contains_key(container_id)
    }

    /// Lock (clear) a container session, wiping key material from memory.
    pub fn lock(&self, container_id: &str) {
        self.0.lock().unwrap().remove(container_id);
        // Session drop triggers Zeroizing<> wipe on the key field
    }

    /// Lock all sessions — called on app close or master lock.
    pub fn lock_all(&self) {
        self.0.lock().unwrap().clear();
    }
}
```

---

### STEP 1.9 — Export Module (.ctnr Format)

Create `src-tauri/src/export.rs`:

```rust
//! .ctnr file format — Cryptainer's portable container export format.
//!
//! File layout (binary):
//!   [0..4]   Magic bytes: "CNTR" (ASCII)
//!   [4]      Null byte: 0x00
//!   [5]      Version: u8 (current: 0x01)
//!   [6..10]  Header length: u32 little-endian
//!   [10..10+header_len]  JSON header (plaintext): ContainerHeader
//!   [10+header_len..]   Encrypted blob (AES-256-GCM output from crypto::encrypt)
//!
//! The header is plaintext so the app can display container metadata
//! (name, algorithm, hint) and verify integrity before asking for the password.

use serde::{Deserialize, Serialize};
use crate::error::{CryptoError, Result};
use crate::vault::ContainerMeta;

pub const MAGIC: &[u8; 4] = b"CNTR";
pub const NULL:  u8 = 0x00;
pub const VERSION: u8 = 0x01;

/// Plaintext header stored at the start of every .ctnr file.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerHeader {
    pub id:          String,
    pub name:        String,
    pub algo:        String,
    pub kdf:         String,
    pub kdf_params:  serde_json::Value,
    pub hint:        Option<String>,
    pub tags:        Option<String>,
    pub file_count:  u32,
    pub total_size:  u64,
    pub created_at:  String,
    pub modified_at: String,
    pub blob_sha256: String,   // SHA-256 of the blob for integrity check
}

/// Serialize a container into .ctnr binary format.
pub fn serialize(meta: &ContainerMeta, blob: &[u8]) -> Result<Vec<u8>> {
    let header = ContainerHeader {
        id:          meta.id.clone(),
        name:        meta.name.clone(),
        algo:        meta.algo.clone(),
        kdf:         meta.kdf_params.kdf.clone(),
        kdf_params:  serde_json::to_value(&meta.kdf_params)?,
        hint:        meta.hint.clone(),
        tags:        meta.tags.clone(),
        file_count:  meta.file_count,
        total_size:  meta.total_size,
        created_at:  meta.created_at.clone(),
        modified_at: meta.modified_at.clone(),
        blob_sha256: meta.blob_sha256.clone(),
    };

    let header_json = serde_json::to_vec(&header)?;
    let header_len = header_json.len() as u32;

    let mut out = Vec::with_capacity(10 + header_json.len() + blob.len());
    out.extend_from_slice(MAGIC);
    out.push(NULL);
    out.push(VERSION);
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_json);
    out.extend_from_slice(blob);

    Ok(out)
}

/// Parse a .ctnr binary file into its header and blob.
pub fn deserialize(data: &[u8]) -> Result<(ContainerHeader, Vec<u8>)> {
    // Validate magic bytes
    if data.len() < 10 {
        return Err(CryptoError::InvalidFormat("File too short".into()));
    }
    if &data[0..4] != MAGIC {
        return Err(CryptoError::InvalidFormat("Not a .ctnr file".into()));
    }
    if data[4] != NULL {
        return Err(CryptoError::InvalidFormat("Invalid null byte".into()));
    }
    if data[5] != VERSION {
        return Err(CryptoError::InvalidFormat(
            format!("Unsupported version: {}. Expected {}", data[5], VERSION)
        ));
    }

    let header_len = u32::from_le_bytes(data[6..10].try_into().unwrap()) as usize;
    if data.len() < 10 + header_len {
        return Err(CryptoError::InvalidFormat("Truncated header".into()));
    }

    let header: ContainerHeader = serde_json::from_slice(&data[10..10 + header_len])?;
    let blob = data[10 + header_len..].to_vec();

    Ok((header, blob))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KdfParams;

    fn dummy_meta() -> ContainerMeta {
        ContainerMeta {
            id: "test-id".into(), name: "Test".into(),
            algo: "AES-GCM-256".into(), kdf_params: KdfParams::argon2id_standard(),
            hint: Some("test hint".into()), tags: None,
            file_count: 2, total_size: 1024,
            blob_path: "/tmp/test.enc".into(), blob_sha256: "abc123".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            modified_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let meta = dummy_meta();
        let blob = vec![1u8, 2, 3, 4, 5];
        let bytes = serialize(&meta, &blob).unwrap();
        let (header, recovered_blob) = deserialize(&bytes).unwrap();
        assert_eq!(header.id, meta.id);
        assert_eq!(recovered_blob, blob);
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut bad = vec![0u8; 20];
        bad[0] = b'X';
        assert!(deserialize(&bad).is_err());
    }
}
```

---

### STEP 1.10 — Tauri Commands (IPC Layer)

Create `src-tauri/src/commands.rs`:

```rust
//! All Tauri IPC commands exposed to the React frontend.
//!
//! Commands receive JSON-serializable arguments from the frontend and
//! return JSON-serializable results (or CryptoError strings on failure).
//!
//! SECURITY: Raw key material never crosses this boundary.
//! Only plaintext metadata and already-encrypted blobs are transferred.

use tauri::{AppHandle, Manager, State};
use uuid::Uuid;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    crypto::{self, KdfParams},
    storage,
    vault::{ContainerMeta, ContainerPayload, VaultFile},
    session::{Session, SessionStore},
    export,
    error::{CryptoError, Result},
};

// ── Input types from frontend ─────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateContainerInput {
    pub name:       String,
    pub kdf_params: KdfParams,
    pub hint:       Option<String>,
    pub tags:       Option<String>,
    pub password:   String,
    pub files:      Vec<FileInput>,
}

#[derive(serde::Deserialize)]
pub struct FileInput {
    pub name: String,
    pub mime: String,
    pub data: Vec<u8>,  // raw bytes from frontend
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Create and encrypt a new container.
/// Encrypts all files, writes blob to disk, inserts metadata into SQLite.
#[tauri::command]
pub async fn create_container(
    app: AppHandle,
    input: CreateContainerInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Build the plaintext payload
    let payload = ContainerPayload {
        version: 1,
        files: input.files.iter().map(|f| VaultFile {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.data.len() as u64,
            data: f.data.clone(),
        }).collect(),
    };

    let total_size: u64 = payload.files.iter().map(|f| f.size).sum();
    let plaintext = serde_json::to_vec(&payload)?;

    // Encrypt
    let blob = crypto::encrypt(&plaintext, &input.password, &input.kdf_params)?;
    let blob_sha256 = crypto::sha256_hex(&blob);

    // Write blob to app data dir
    let blobs_dir = app.path().app_data_dir()
        .map_err(|e| CryptoError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
        .join("blobs");
    std::fs::create_dir_all(&blobs_dir)?;
    let blob_path = blobs_dir.join(format!("{}.enc", id));
    std::fs::write(&blob_path, &blob)?;

    let meta = ContainerMeta {
        id: id.clone(),
        name: input.name,
        algo: "AES-GCM-256".into(),
        kdf_params: input.kdf_params,
        hint: input.hint,
        tags: input.tags,
        file_count: payload.files.len() as u32,
        total_size,
        blob_path: blob_path.to_string_lossy().to_string(),
        blob_sha256,
        created_at: now.clone(),
        modified_at: now,
    };

    storage::insert_container(&pool, &meta).await?;
    Ok(meta)
}

/// Unlock (decrypt) a container. Stores the session in memory.
/// Returns the list of files (without data bytes — data fetched separately).
#[tauri::command]
pub async fn unlock_container(
    container_id: String,
    password: String,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<Vec<serde_json::Value>, CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;
    let blob = std::fs::read(&meta.blob_path)?;

    // Integrity check before attempting decryption
    let actual_sha256 = crypto::sha256_hex(&blob);
    if actual_sha256 != meta.blob_sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    let plaintext = crypto::decrypt(&blob, &password, &meta.kdf_params)?;
    let payload: ContainerPayload = serde_json::from_slice(&plaintext)?;

    // Build file list WITHOUT data bytes (data fetched per-file on demand)
    let file_list: Vec<serde_json::Value> = payload.files.iter().map(|f| {
        serde_json::json!({ "id": f.id, "name": f.name, "mime": f.mime, "size": f.size })
    }).collect();

    // Derive key separately to store in session for re-encryption
    let key = {
        let salt = &blob[..crypto::SALT_LEN];
        let key = crypto::derive_key(&password, salt, &meta.kdf_params)?;
        zeroize::Zeroizing::new(key.as_ref().to_vec())
    };

    sessions.set(container_id, Session { payload, key });
    Ok(file_list)
}

/// Fetch the data bytes of a specific file in an unlocked container.
/// Data is never stored in the frontend state — fetched on demand for preview.
#[tauri::command]
pub async fn get_file_data(
    container_id: String,
    file_id: String,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<Vec<u8>, CryptoError> {
    let store = sessions.0.lock().unwrap();
    let session = store.get(&container_id)
        .ok_or(CryptoError::SessionInactive)?;
    let file = session.payload.files.iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| CryptoError::NotFound(file_id.clone()))?;
    Ok(file.data.clone())
}

/// Save edits to an unlocked container (add/remove files) and re-encrypt.
/// Uses atomic write: writes to a .tmp file first, then renames.
#[tauri::command]
pub async fn save_edits(
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;

    let mut store = sessions.0.lock().unwrap();
    let session = store.get_mut(&container_id)
        .ok_or(CryptoError::SessionInactive)?;

    // Remove marked files
    session.payload.files.retain(|f| !file_ids_to_remove.contains(&f.id));

    // Add new files
    for f in &files_to_add {
        session.payload.files.push(VaultFile {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.data.len() as u64,
            data: f.data.clone(),
        });
    }

    let total_size: u64 = session.payload.files.iter().map(|f| f.size).sum();
    let file_count = session.payload.files.len() as u32;
    let plaintext = serde_json::to_vec(&session.payload)?;

    // Re-encrypt with same password (new random salt + nonce)
    let blob = crypto::encrypt(&plaintext, &password, &meta.kdf_params)?;
    let blob_sha256 = crypto::sha256_hex(&blob);

    // Atomic write: tmp → rename
    let blob_path = PathBuf::from(&meta.blob_path);
    let tmp_path = blob_path.with_extension("enc.tmp");
    std::fs::write(&tmp_path, &blob)?;
    std::fs::rename(&tmp_path, &blob_path)?;

    // Update DB
    drop(store); // release lock before async call
    storage::update_container_blob(&pool, &container_id, file_count, total_size, &blob_sha256).await?;
    let updated_meta = storage::get_container(&pool, &container_id).await?;
    Ok(updated_meta)
}

/// List all containers (metadata only — no blobs, no keys).
#[tauri::command]
pub async fn list_containers(
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<Vec<ContainerMeta>, CryptoError> {
    storage::list_containers(&pool).await
}

/// Delete a container — removes DB row and blob file.
#[tauri::command]
pub async fn delete_container(
    container_id: String,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<(), CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;
    storage::delete_container(&pool, &container_id).await?;
    let _ = std::fs::remove_file(&meta.blob_path);
    sessions.lock(&container_id);
    Ok(())
}

/// Lock a container session — wipes decrypted data and key from memory.
#[tauri::command]
pub async fn lock_container(
    container_id: String,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<(), CryptoError> {
    sessions.lock(&container_id);
    Ok(())
}

/// Export a container to a .ctnr file at the given path.
#[tauri::command]
pub async fn export_container(
    container_id: String,
    dest_path: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<(), CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;
    let blob = std::fs::read(&meta.blob_path)?;
    let ctnr_bytes = export::serialize(&meta, &blob)?;
    std::fs::write(&dest_path, ctnr_bytes)?;
    Ok(())
}

/// Import a .ctnr file into the vault.
#[tauri::command]
pub async fn import_container(
    src_path: String,
    app: AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    let bytes = std::fs::read(&src_path)?;
    let (header, blob) = export::deserialize(&bytes)?;

    // Verify blob integrity
    let actual_sha256 = crypto::sha256_hex(&blob);
    if actual_sha256 != header.blob_sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    // Write blob to local blobs dir
    let blobs_dir = app.path().app_data_dir()
        .map_err(|e| CryptoError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
        .join("blobs");
    std::fs::create_dir_all(&blobs_dir)?;
    let blob_path = blobs_dir.join(format!("{}.enc", header.id));
    std::fs::write(&blob_path, &blob)?;

    let kdf_params: KdfParams = serde_json::from_value(header.kdf_params)?;
    let meta = ContainerMeta {
        id:          header.id,
        name:        header.name,
        algo:        header.algo,
        kdf_params,
        hint:        header.hint,
        tags:        header.tags,
        file_count:  header.file_count,
        total_size:  header.total_size,
        blob_path:   blob_path.to_string_lossy().to_string(),
        blob_sha256: header.blob_sha256,
        created_at:  header.created_at,
        modified_at: header.modified_at,
    };
    storage::insert_container(&pool, &meta).await?;
    Ok(meta)
}
```

Register all commands in `main.rs` and wire up the `SessionStore` and `SqlitePool` as Tauri managed state.

---

### STEP 1.11 — TypeScript Types

Create `src/types/vault.ts`:

```typescript
// All TypeScript types mirror the Rust structs in vault.rs and commands.rs.
// Keep this file in sync with the Rust side whenever structs change.

export interface KdfParams {
  kdf: 'argon2id' | 'pbkdf2';
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
```

---

### STEP 1.12 — Zustand Store & IPC Hooks

Create `src/store/vaultStore.ts`:

```typescript
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { ContainerMeta, VaultFileMeta, CreateContainerInput } from '../types/vault';

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
  exportContainer:   (id: string, destPath: string) => Promise<void>;
  importContainer:   (srcPath: string) => Promise<ContainerMeta>;
  clearError:        () => void;
}

export const useVaultStore = create<VaultState>((set, get) => ({
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
    await invoke('delete_container', { containerId: id });
    set(s => ({ containers: s.containers.filter(c => c.id !== id) }));
  },

  unlockContainer: async (id, password) => {
    return invoke<VaultFileMeta[]>('unlock_container', { containerId: id, password });
  },

  lockContainer: async (id) => {
    await invoke('lock_container', { containerId: id });
  },

  getFileData: async (containerId, fileId) => {
    const bytes = await invoke<number[]>('get_file_data', { containerId, fileId });
    return new Uint8Array(bytes);
  },

  exportContainer: async (id, destPath) => {
    await invoke('export_container', { containerId: id, destPath });
  },

  importContainer: async (srcPath) => {
    const meta = await invoke<ContainerMeta>('import_container', { srcPath });
    set(s => ({ containers: [meta, ...s.containers] }));
    return meta;
  },

  clearError: () => set({ error: null }),
}));
```

---

### STEP 1.13 — Frontend UI Components

Install frontend dependencies:

```bash
npm install zustand react-dropzone prism-react-renderer
npm install -D @types/node
```

#### UI Foundations (`src/components/UI/`)

Build these components in order. Each must be fully typed with no `any`:

**`Button.tsx`** — variants: `primary | secondary | danger | ghost`. Props: `onClick`, `disabled`, `loading` (shows spinner), `size: sm | md`.

**`Input.tsx`** — controlled input with `label`, `error` display, `type` (text/password). Password type gets an eye toggle button.

**`Modal.tsx`** — portal-based overlay. Props: `open`, `onClose`, `title`, `size: sm | md | lg`.

**`DropZone.tsx`** — wraps `react-dropzone`. Shows file count, handles duplicate rejection, emits `File[]` to parent.

**`PasswordStrength.tsx`** — accepts `password: string`, computes a 0–4 strength score, renders a segmented bar. Scoring criteria: length ≥ 8, length ≥ 14, has uppercase, has digit, has symbol.

**`ProgressBar.tsx`** — animated progress bar for encryption/decryption operations.

#### Container Components (`src/components/Container/`)

**`ContainerCard.tsx`** — displays `ContainerMeta`. Shows name, algo badge, file count, size, created date, lock icon. Hover shows "Click to open" affordance. Delete button (X) visible on hover.

**`CreateWizard/Step1Files.tsx`** — file drop zone, file list with remove buttons, container name input. Emits: `{ name: string, files: File[] }`.

**`CreateWizard/Step2Config.tsx`** — security preset selector, password + confirm inputs, hint input, password strength bar. Emits `CreateContainerInput`.

**`ContainerModal/LockView.tsx`** — shield icon, password input, hint display, unlock button.

**`ContainerModal/OpenView.tsx`** — file list with preview buttons, Edit button.

**`ContainerModal/EditView.tsx`** — file list with remove toggle, drop zone for new files, Save & Re-encrypt button, Cancel button.

#### Preview Components (`src/components/Preview/`)

**`PreviewRouter.tsx`** — receives `{ mime: string, data: Uint8Array, name: string }`. Routes to correct previewer based on MIME type.

**`ImagePreview.tsx`** — converts `Uint8Array` to object URL, renders `<img>`. Cleans up URL on unmount.

**`TextPreview.tsx`** — decodes UTF-8, renders in `<pre>` with syntax highlighting via `prism-react-renderer` for code files.

**`HexPreview.tsx`** — renders first 4 KB of any binary file as a hex dump (offset | hex | ascii columns).

#### Vault Screen (`src/components/Vault/`)

**`VaultToolbar.tsx`** — "New Container" button, "Import .ctnr" button (opens native file picker via `@tauri-apps/plugin-dialog`), search input, sort select.

**`VaultGrid.tsx`** — responsive CSS grid of `ContainerCard` components. Empty state with call-to-action.

---

### STEP 1.14 — App Entry Point & Global Styles

`src/App.tsx`:

```typescript
import { useEffect, useState } from 'react';
import { useVaultStore } from './store/vaultStore';
import { VaultGrid } from './components/Vault/VaultGrid';
import { VaultToolbar } from './components/Vault/VaultToolbar';
import { CreateWizard } from './components/Container/CreateWizard';
import { ContainerModal } from './components/Container/ContainerModal';
import type { ContainerMeta } from './types/vault';

export default function App() {
  const { containers, loading, loadContainers } = useVaultStore();
  const [showCreate, setShowCreate]         = useState(false);
  const [activeContainer, setActiveContainer] = useState<ContainerMeta | null>(null);

  useEffect(() => { loadContainers(); }, []);

  return (
    <div className="app">
      <header className="app-header">
        <div className="logo">CRYPTAINER</div>
        <VaultToolbar onNew={() => setShowCreate(true)} />
      </header>
      <main className="app-main">
        {loading
          ? <div className="loading">Loading vault…</div>
          : <VaultGrid containers={containers} onOpen={setActiveContainer} />
        }
      </main>
      {showCreate && (
        <CreateWizard onClose={() => setShowCreate(false)} />
      )}
      {activeContainer && (
        <ContainerModal
          container={activeContainer}
          onClose={() => setActiveContainer(null)}
          onUpdate={setActiveContainer}
        />
      )}
    </div>
  );
}
```

`src/styles/global.css` — apply the dark theme from the prototype:
- Background: `#080b0f`, surface: `#0d1520`, border: `#1a2535`
- Accent: `#4de0c0`
- Font: JetBrains Mono (import from Google Fonts) for UI chrome, system-ui for body
- CSS custom properties for all colors (theme-swappable in Phase 3)
- Smooth scrollbar styling
- Input focus ring using accent color
- Transition: `all 0.15s ease` on interactive elements

---

### STEP 1.15 — Phase 1 Integration Test

```bash
# Run all Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript type check
npx tsc --noEmit

# Clippy — zero warnings required
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

# Run dev build
npm run tauri dev
```

**Manual test checklist (agent must verify each item):**
- [ ] Create a container with 3 mixed-type files (image, text, binary)
- [ ] Encrypted blob file appears in app data dir
- [ ] Unlock container — correct password succeeds
- [ ] Unlock container — wrong password shows inline error, does NOT crash
- [ ] Preview an image file
- [ ] Preview a text file
- [ ] Lock container — session cleared
- [ ] Delete container — DB row and blob file both removed
- [ ] App restarts — containers persist from SQLite

**After this step, agent must:**
- Record all test results in `PROGRESS.md`
- Update `ARCHITECTURE.md` with final Phase 1 component diagram
- Update `API.md` with all 9 IPC commands

---

## ⚙️ PHASE 2 — Export / Import & Edit Mode

### STEP 2.1 — Export UI

In `VaultToolbar.tsx` and `ContainerCard.tsx`, add export button that:
1. Calls `@tauri-apps/plugin-dialog` `save()` with filter `[{ name: 'Cryptainer Export', extensions: ['ctnr'] }]`
2. If user selects a path, calls `exportContainer(id, path)` from the store
3. Shows success toast and error toast on failure

### STEP 2.2 — Import UI

In `VaultToolbar.tsx`, add Import button that:
1. Calls `open()` dialog with filter for `.ctnr` files, `multiple: true`
2. For each selected file, calls `importContainer(path)`
3. If a container with the same ID already exists, shows a conflict dialog: **Replace / Keep Both / Cancel**
   - **Replace**: delete old container first, then import
   - **Keep Both**: generate a new UUID for the imported container before inserting
4. Reloads the vault grid on completion

### STEP 2.3 — Edit Mode (Full Implementation)

`ContainerModal/EditView.tsx`:
- Pending additions shown with `+ NEW` badge
- Pending removals shown with strikethrough + `UNDO` button
- `Save & Re-encrypt` calls `save_edits` Tauri command
- Atomic write already handled in Rust — frontend just awaits and refreshes meta
- Cancel reverts local state, no IPC call needed

### STEP 2.4 — Phase 2 Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
```

**Manual test checklist:**
- [ ] Export a container, verify `.ctnr` file is created
- [ ] Import the `.ctnr` on the same device — container appears in vault
- [ ] Import a `.ctnr` when container ID already exists — conflict dialog appears
- [ ] Edit mode: add a new file, save — unlock again and new file is present
- [ ] Edit mode: remove a file, save — unlock again and file is gone
- [ ] Edit mode: cancel — no changes persisted
- [ ] Tamper with a `.ctnr` file's blob bytes — import succeeds but unlock shows integrity error

---

## ⚙️ PHASE 3 — Polish & Power Features

### STEP 3.1 — Extended Preview Support
- PDF: use WebView's native PDF renderer via a `blob:` URL
- Video: HTML5 `<video>` with `blob:` URL, cleaned up on close
- Audio: HTML5 `<audio>` with waveform display
- Code files (`.rs`, `.ts`, `.py`, `.go`, etc.): Prism.js highlighting in `TextPreview`
- Unknown/binary: `HexPreview` — offset | hex | ascii, virtual list for large files

### STEP 3.2 — Search, Filter & Tags
- Vault search input: filter containers by name in real-time (client-side, no DB query)
- Tag filter: click a tag badge to filter to containers with that tag
- Sort: by name (A–Z), date created (newest), file count, total size
- List view: compact table layout with sortable columns

### STEP 3.3 — Session Auto-Lock
- Settings screen: configure auto-lock timeout (1 min / 5 min / 15 min / never)
- Implement idle timer in `useSession.ts` — resets on any mouse/key event
- On timeout: call `lock_container` for all active sessions, show re-lock toast

### STEP 3.4 — App Settings Screen
- Route: `/settings`
- Fields: default security preset, auto-lock timeout, theme (dark/light/system), app version
- Settings persisted in `app_data_dir/settings.json`

### STEP 3.5 — Container Integrity Badge
- On vault grid load, verify `blob_sha256` for every container (read blob, hash, compare)
- Containers with mismatched hash show a red `INTEGRITY FAILURE` badge
- Do this check lazily (on hover or on open) to avoid blocking app startup

---

## ⚙️ PHASE 4 — Mobile (iOS & Android)

### STEP 4.1 — Tauri Mobile Init

```bash
cargo tauri android init
cargo tauri ios init
```

Resolve any build errors before continuing. Both targets must compile.

### STEP 4.2 — Responsive Layout
- Breakpoints: mobile (< 600px), tablet (600–1024px), desktop (> 1024px)
- Mobile: bottom navigation bar, full-screen modals instead of overlay modals
- Touch targets: minimum 44×44px for all interactive elements
- Swipe left on a container card to reveal delete action

### STEP 4.3 — Mobile File Picker
- Android: SAF (Storage Access Framework) via Tauri FS plugin — no changes needed
- iOS: UIDocumentPicker — test with Files app integration
- Accept file from other apps via Tauri's deep link / share extension (optional enhancement)

### STEP 4.4 — Biometric Unlock (Optional Enhancement)
- Use `tauri-plugin-biometric` when available
- Biometrics unlocks the *session* (re-derives key from stored encrypted password blob)
- Biometric is NOT a replacement for the master password — it is a convenience wrapper
- If biometrics fail 3×, fall back to password entry
- Document this clearly in `CRYPTO.md`

---

## 📐 DOCUMENTATION FILES TO MAINTAIN

The agent must keep these four files current at all times:

### `/docs/PROGRESS.md`
```markdown
# Cryptainer — Build Progress

## [STEP X.Y] — Title
- **Date**: YYYY-MM-DD
- **Status**: ✅ Complete | ⚠️ Partial | ❌ Failed
- **Files created/modified**: list
- **Decisions made**: list
- **Tests run**: list results
- **Notes**: any issues encountered and how they were resolved
```

### `/docs/ARCHITECTURE.md`
Must always contain:
- Current component tree (frontend)
- Current module graph (Rust backend)
- IPC boundary diagram (what crosses the boundary and what doesn't)
- Data flow for: create container, unlock container, edit & save, export, import
- SQLite schema (keep in sync with actual migrations)

### `/docs/CRYPTO.md`
Must always contain:
- Algorithm choices and rationale
- KDF parameter table with security levels
- Encryption flow diagram
- What is and isn't protected
- How key material is handled in memory
- .ctnr file format specification

### `/docs/API.md`
For every Tauri command:
```markdown
## `command_name`
**Description**: …
**Input**: `{ field: Type, … }`
**Output**: `Type`
**Errors**: list of possible CryptoError variants
**Frontend usage**:
    invoke('command_name', { … })
```

---

## ✅ FINAL CHECKLIST BEFORE SHIPPING PHASE 1

- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `tsc --noEmit` — zero TypeScript errors
- [ ] `npm run tauri build` — release binary builds on all three desktop platforms
- [ ] All 9 IPC commands documented in `API.md`
- [ ] `CRYPTO.md` completed with full parameter rationale
- [ ] `ARCHITECTURE.md` reflects actual codebase (not planned state)
- [ ] `PROGRESS.md` has entries for every step
- [ ] No `unwrap()`, `todo!()`, `println!()`, or `dbg!()` left in production Rust code
- [ ] No `console.log`, `any` types, or disabled eslint rules in production TypeScript
- [ ] Encrypted blob files verified to be unreadable without password (manual check)
- [ ] App data directory confirmed to be in OS-appropriate location:
  - macOS: `~/Library/Application Support/com.cryptainer.app/`
  - Windows: `%APPDATA%\com.cryptainer.app\`
  - Linux: `~/.local/share/com.cryptainer.app/`

---

*End of Cryptainer Implementation Guide — v1.0*
*Project: Cryptainer · Stack: Tauri v2 + React + TypeScript + Rust*
