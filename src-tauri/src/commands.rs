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
// Note: AsyncMutex and Arc will be used in future features

use crate::{
    crypto::{self, KdfParams},
    storage,
    vault::{ContainerMeta, ContainerPayload, VaultFile},
    session::{Session, SessionStore},
    export,
    error::CryptoError,
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

    // Scope the mutex lock to ensure it's dropped before await points
    let (total_size, file_count, plaintext) = {
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
        
        (total_size, file_count, plaintext)
    }; // MutexGuard dropped here

    // Re-encrypt with same password (new random salt + nonce)
    let blob = crypto::encrypt(&plaintext, &password, &meta.kdf_params)?;
    let blob_sha256 = crypto::sha256_hex(&blob);

    // Atomic write: tmp → rename
    let blob_path = PathBuf::from(&meta.blob_path);
    let tmp_path = blob_path.with_extension("enc.tmp");
    std::fs::write(&tmp_path, &blob)?;
    std::fs::rename(&tmp_path, &blob_path)?;

    // Update DB
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
