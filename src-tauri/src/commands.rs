//! All Tauri IPC commands exposed to the React frontend.
//!
//! Commands receive JSON-serializable arguments from the frontend and
//! return JSON-serializable results (or CryptoError strings on failure).
//!
//! SECURITY: Raw key material never crosses this boundary.
//! Only plaintext metadata and already-encrypted blobs are transferred.

use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;
use chrono::Utc;
use std::path::PathBuf;
use rand::RngCore;
// Note: AsyncMutex and Arc will be used in future features

use crate::{
    crypto::{self, KdfParams},
    storage,
    vault::{self, ContainerMeta, ContainerPayload, VaultFile, ContainerMetadataV2, FileMetadata},
    session::{SessionStore, SessionStoreV2, SessionV2},
    export,
    error::CryptoError,
    crypto::SALT_LEN,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInput {
    pub path: String,   // absolute path to the source file on disk
    pub name: String,
    pub mime: String,
    pub size: u64,      // plaintext size in bytes (from the frontend's file picker)
}

/// Progress payload emitted via Tauri events during crypto operations.
/// Frontend listens to "cryptainer://progress" and renders a ProgressBar.
#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub operation: String,
    pub current: u64,
    pub total: u64,
    pub file_name: Option<String>,
    pub bytes_processed: u64,
    pub bytes_total: u64,
    pub message: String,
}

/// Emit a progress event to the frontend.
/// Uses the Tauri v2 `Emitter` trait on `AppHandle`.
pub fn emit_progress(app: &AppHandle, payload: ProgressPayload) {
    let _ = app.emit("cryptainer://progress", payload);
}

/// Helper: validate password minimum length.
///
/// Counts Unicode characters (not bytes) to match the frontend's length check,
/// so a multibyte password isn't accepted on one side and rejected on the other.
fn validate_password(password: &str) -> std::result::Result<(), CryptoError> {
    if password.chars().count() < 8 {
        return Err(CryptoError::InvalidFormat(
            "Password must be at least 8 characters".into(),
        ));
    }
    Ok(())
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Create and encrypt a new container (v2 per-file encryption).
/// Encrypts each file individually, builds a v2 blob, writes to disk,
/// inserts metadata into SQLite.
#[tauri::command]
pub async fn create_container(
    app: AppHandle,
    input: CreateContainerInput,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    validate_password(&input.password)?;
    if input.name.trim().is_empty() || input.name.len() > 256 {
        return Err(CryptoError::InvalidFormat(
            "Container name must be 1-256 characters".into(),
        ));
    }
    if let Some(ref t) = input.tags {
        if t.len() > 1024 {
            return Err(CryptoError::InvalidFormat("Tags too long (max 1024)".into()));
        }
    }
    if let Some(ref h) = input.hint {
        if h.len() > 256 {
            return Err(CryptoError::InvalidFormat("Hint too long (max 256)".into()));
        }
    }

    let password = zeroize::Zeroizing::new(input.password);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let existing = storage::get_container_by_name(&pool, &input.name).await?;
    if existing.is_some() {
        return Err(CryptoError::InvalidFormat(
            "A container with this name already exists".into(),
        ));
    }

    // 1. Derive key
    emit_progress(&app, ProgressPayload {
        operation: "derive-key".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: 0,
        bytes_total: 0,
        message: "Deriving encryption key…".into(),
    });
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(&password, &salt, &input.kdf_params)?;

    // 2. Encrypt each file individually (streamed from disk), collect metadata.
    // Every file is chunked (`chunks: Some`, `data_nonce: [0;12]` unused); byte
    // progress is emitted per chunk so large files show smooth movement.
    let total_size: u64 = input.files.iter().map(|f| f.size).sum();
    let total_files = input.files.len() as u64;
    let mut files_meta: Vec<FileMetadata> = Vec::with_capacity(input.files.len());
    let mut encrypted_files: Vec<Vec<u8>> = Vec::with_capacity(input.files.len());
    let mut cumulative_bytes: u64 = 0;

    for (i, f) in input.files.iter().enumerate() {
        let base = cumulative_bytes;
        let mut emit = |done_in_file: u64| {
            emit_progress(&app, ProgressPayload {
                operation: "encrypt".into(),
                current: i as u64,
                total: total_files,
                file_name: Some(f.name.clone()),
                bytes_processed: base + done_in_file,
                bytes_total: total_size,
                message: format!("Encrypting {} ({} / {})", f.name, i + 1, total_files),
            });
        };
        emit(0); // show the file starting before the first chunk
        let (encrypted, chunks, sha256, plaintext_len) = vault::encrypt_file_chunked(
            std::path::Path::new(&f.path), &key, vault::ENCRYPT_CHUNK_SIZE, &mut emit,
        )?;
        files_meta.push(FileMetadata {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: plaintext_len,
            offset: 0,
            data_nonce: [0u8; 12], // unused for chunked files
            sha256,
            chunks: Some(chunks),
        });
        encrypted_files.push(encrypted);
        cumulative_bytes += plaintext_len;
    }

    // Final encrypt progress (all files done)
    emit_progress(&app, ProgressPayload {
        operation: "encrypt".into(),
        current: total_files,
        total: total_files,
        file_name: None,
        bytes_processed: total_size,
        bytes_total: total_size,
        message: "Encryption complete".into(),
    });

    // 3. Compute metadata offsets iteratively (shared helper, converges in 2-3 passes)
    let mut metadata_v2 = ContainerMetadataV2 { version: 2, files: files_meta };
    let (encrypted_metadata, metadata_nonce) = vault::compute_v2_layout(&key, &mut metadata_v2.files, &encrypted_files)?;

    // 4. Assemble blob
    let enc_meta_len = encrypted_metadata.len();
    let metadata_section_len = 4 + crypto::NONCE_LEN + enc_meta_len;
    let file_data_len: usize = encrypted_files.iter().map(|e| e.len()).sum();
    let blob_total_len = SALT_LEN + metadata_section_len + file_data_len;

    let mut blob = Vec::with_capacity(blob_total_len);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta_len as u32).to_le_bytes());
    blob.extend_from_slice(&metadata_nonce);
    blob.extend_from_slice(&encrypted_metadata);
    for ef in &encrypted_files {
        blob.extend_from_slice(ef);
    }

    let blob_sha256 = crypto::sha256_hex(&blob);

    // 6. Emit write-blob progress (indeterminate — writing to disk)
    emit_progress(&app, ProgressPayload {
        operation: "write-blob".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: blob.len() as u64,
        bytes_total: blob.len() as u64,
        message: "Writing encrypted blob to disk…".into(),
    });

    // 7. Write blob to disk
    let blobs_dir = app.path().app_data_dir()
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?
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
        file_count: encrypted_files.len() as u32,
        total_size,
        blob_path: blob_path.to_str()
            .ok_or_else(|| CryptoError::InvalidFormat("Non-UTF-8 blob path".into()))?
            .to_string(),
        blob_sha256,
        created_at: now.clone(),
        modified_at: now,
        format_version: 2,
    };

    storage::insert_container(&pool, &meta).await.inspect_err(|_| {
        let _ = std::fs::remove_file(&blob_path);
    })?;

    record_audit(&pool, "create", Some(&meta.id), Some(&meta.name), None);
    drop(password);
    Ok(meta)
}

/// Unlock (decrypt) a container. Handles both v1 (legacy) and v2 (per-file).
/// V1 containers are automatically migrated to v2 on unlock.
#[tauri::command]
pub async fn unlock_container(
    app: AppHandle,
    container_id: String,
    password: String,
    pool: State<'_, sqlx::SqlitePool>,
    _sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<Vec<serde_json::Value>, CryptoError> {
    validate_password(&password)?;
    let password = zeroize::Zeroizing::new(password);

    let meta = storage::get_container(&pool, &container_id).await?;

    // Emit read-blob progress (indeterminate — reading from disk)
    emit_progress(&app, ProgressPayload {
        operation: "read-blob".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: 0,
        bytes_total: 0,
        message: "Reading encrypted container…".into(),
    });
    let blob = std::fs::read(&meta.blob_path)?;

    let actual_sha256 = crypto::sha256_hex(&blob);
    if actual_sha256 != meta.blob_sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    // Emit derive-key progress (indeterminate)
    emit_progress(&app, ProgressPayload {
        operation: "derive-key".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: blob.len() as u64,
        bytes_total: blob.len() as u64,
        message: "Deriving decryption key…".into(),
    });

    if meta.format_version == 2 {
        let result = unlock_v2(&container_id, &password, &meta, &blob, sessions_v2);
        if result.is_ok() {
            record_audit(&pool, "unlock", Some(&container_id), Some(&meta.name), None);
        }
        return result;
    }

    // v1 detected — auto-migrate to v2
    emit_progress(&app, ProgressPayload {
        operation: "migrate".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: 0,
        bytes_total: 0,
        message: "Migrating v1 container to v2…".into(),
    });
    let plaintext = crypto::decrypt(&blob, &password, &meta.kdf_params)?;
    let payload: ContainerPayload = serde_json::from_slice(&plaintext)?;

    let (new_blob, file_list) = convert_v1_to_v2(&payload, &password, &meta.kdf_params)?;

    // Atomic write — never delete old blob on failure
    atomic_write_blob(&meta.blob_path, &new_blob)?;

    // Update DB format_version (blob_sha256 will be updated after unlock_v2 reads it)
    storage::update_container_format_version(&pool, &container_id, 2).await?;

    // Continue with v2 unlock from the new blob
    let blob_sha256 = crypto::sha256_hex(&new_blob);
    storage::update_container_blob(&pool, &container_id, payload.files.len() as u32,
        payload.files.iter().map(|f| f.size).sum(), &blob_sha256).await?;

    let session = build_v2_session(&password, &meta, &new_blob)?;
    record_audit(&pool, "unlock", Some(&container_id), Some(&meta.name), None);
    sessions_v2.set(container_id, session);

    Ok(file_list)
}

fn build_v2_session(
    password: &str,
    meta: &ContainerMeta,
    blob: &[u8],
) -> std::result::Result<SessionV2, CryptoError> {
    if blob.len() < SALT_LEN {
        return Err(CryptoError::InvalidFormat("blob too short for salt".into()));
    }
    let salt: [u8; SALT_LEN] = blob[..SALT_LEN].try_into()
        .map_err(|_| CryptoError::InvalidFormat("blob too short for salt".into()))?;
    let key = crypto::derive_key(password, &salt, &meta.kdf_params)?;

    let meta_len_offset = SALT_LEN;
    let end = meta_len_offset + 4;
    if blob.len() < end {
        return Err(CryptoError::InvalidFormat("blob too short for metadata length".into()));
    }
    let meta_len = u32::from_le_bytes(
        blob[meta_len_offset..end].try_into()
            .map_err(|_| CryptoError::InvalidFormat("blob too short for metadata length".into()))?
    ) as usize;

    let meta_nonce_offset = meta_len_offset + 4;
    let end = meta_nonce_offset + crypto::NONCE_LEN;
    if blob.len() < end {
        return Err(CryptoError::InvalidFormat("blob too short for metadata nonce".into()));
    }
    let meta_nonce: [u8; crypto::NONCE_LEN] = blob[meta_nonce_offset..end]
        .try_into()
        .map_err(|_| CryptoError::InvalidFormat("blob too short for metadata nonce".into()))?;

    let meta_ct_offset = meta_nonce_offset + crypto::NONCE_LEN;
    let meta_ct_end = meta_ct_offset + meta_len;
    if meta_ct_end > blob.len() {
        return Err(CryptoError::InvalidFormat("metadata section exceeds blob".into()));
    }
    let metadata_ciphertext = &blob[meta_ct_offset..meta_ct_end];

    let metadata_plaintext = crypto::decrypt_section(metadata_ciphertext, &key, &meta_nonce)?;
    let metadata: ContainerMetadataV2 = serde_json::from_slice(&metadata_plaintext)?;

    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key.as_ref());
    Ok(SessionV2::new(
        zeroize::Zeroizing::new(key_arr),
        salt,
        metadata,
        meta.blob_path.clone(),
        crate::session::DEFAULT_MAX_CACHE_BYTES,
    ))
}

/// Convert a v1 container payload into a v2-format blob.
fn convert_v1_to_v2(
    payload: &ContainerPayload,
    password: &str,
    kdf_params: &KdfParams,
) -> std::result::Result<(Vec<u8>, Vec<serde_json::Value>), CryptoError> {
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(password, &salt, kdf_params)?;

    let mut files_meta: Vec<FileMetadata> = Vec::with_capacity(payload.files.len());
    let mut encrypted_files: Vec<Vec<u8>> = Vec::with_capacity(payload.files.len());

    let file_list: Vec<serde_json::Value> = payload.files.iter().map(|f| {
        serde_json::json!({ "id": f.id, "name": f.name, "mime": f.mime, "size": f.size })
    }).collect();

    for f in &payload.files {
        let sha256 = crypto::sha256_hex(&f.data);
        let (encrypted, nonce) = crypto::encrypt_section(&f.data, &key)?;
        files_meta.push(FileMetadata {
            id: f.id.clone(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.size,
            offset: 0,
            data_nonce: nonce,
            sha256,
            chunks: None,
        });
        encrypted_files.push(encrypted);
    }

    // Use shared iterative layout helper (same as create_container / save_edits_v2)
    let (enc_meta, meta_nonce) = vault::compute_v2_layout(&key, &mut files_meta, &encrypted_files)?;

    let enc_meta_len = enc_meta.len();
    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta_len;
    let file_data_len: usize = encrypted_files.iter().map(|e| e.len()).sum();
    let mut blob = Vec::with_capacity(SALT_LEN + meta_section_len + file_data_len);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta_len as u32).to_le_bytes());
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_files {
        blob.extend_from_slice(ef);
    }

    Ok((blob, file_list))
}

fn unlock_v2(
    container_id: &str,
    password: &str,
    meta: &ContainerMeta,
    blob: &[u8],
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<Vec<serde_json::Value>, CryptoError> {
    if blob.len() < SALT_LEN + 4 + crypto::NONCE_LEN + 16 {
        return Err(CryptoError::InvalidFormat("v2 blob too short".into()));
    }

    let salt: [u8; SALT_LEN] = blob[..SALT_LEN].try_into().unwrap();
    let key = crypto::derive_key(password, &salt, &meta.kdf_params)?;

    // Read metadata section: [salt][metadata_len: u32][metadata_nonce][metadata_ciphertext]
    let meta_len_offset = SALT_LEN;
    let meta_len = u32::from_le_bytes(
        blob[meta_len_offset..meta_len_offset + 4].try_into().unwrap()
    ) as usize;

    let meta_nonce_offset = meta_len_offset + 4;
    let meta_nonce: [u8; crypto::NONCE_LEN] = blob[meta_nonce_offset..meta_nonce_offset + crypto::NONCE_LEN]
        .try_into().unwrap();

    let meta_ct_offset = meta_nonce_offset + crypto::NONCE_LEN;
    let meta_ct_end = meta_ct_offset + meta_len;
    if meta_ct_end > blob.len() {
        return Err(CryptoError::InvalidFormat("v2 metadata section exceeds blob".into()));
    }
    let metadata_ciphertext = &blob[meta_ct_offset..meta_ct_end];

    let metadata_plaintext = crypto::decrypt_section(metadata_ciphertext, &key, &meta_nonce)?;
    let metadata: ContainerMetadataV2 = serde_json::from_slice(&metadata_plaintext)?;

    let file_list: Vec<serde_json::Value> = metadata.files.iter().map(|f| {
        serde_json::json!({ "id": f.id, "name": f.name, "mime": f.mime, "size": f.size })
    }).collect();

    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key.as_ref());
    let session = SessionV2::new(
        zeroize::Zeroizing::new(key_arr),
        salt,
        metadata,
        meta.blob_path.clone(),
        crate::session::DEFAULT_MAX_CACHE_BYTES,
    );
    sessions_v2.set(container_id.to_string(), session);

    Ok(file_list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KdfParams;

    fn dummy_meta() -> ContainerMeta {
        ContainerMeta {
            id: "test".into(),
            name: "test".into(),
            algo: "AES-GCM-256".into(),
            kdf_params: KdfParams::argon2id_standard(),
            hint: None,
            tags: None,
            file_count: 0,
            total_size: 0,
            blob_path: "/tmp/test.enc".into(),
            blob_sha256: "0".repeat(64),
            created_at: "2024-01-01T00:00:00Z".into(),
            modified_at: "2024-01-01T00:00:00Z".into(),
            format_version: 2,
        }
    }

    /// A truncated blob shorter than even the salt should fail with an error,
    /// NOT panic with an index-out-of-bounds or try_into unwrap.
    #[test]
    fn build_v2_session_truncated_blob_returns_err() {
        let blob = vec![0u8; 4]; // too short for salt
        let meta = dummy_meta();
        let result = build_v2_session("password", &meta, &blob);
        assert!(result.is_err(), "truncated blob should return Err, got Ok");
    }

    /// A blob that has the salt but is too short for the metadata header
    /// (salt + 4 bytes for metadata_len) should also return Err.
    #[test]
    fn build_v2_session_short_header_returns_err() {
        let blob = vec![0u8; SALT_LEN + 2]; // has salt, but not enough for metadata_len
        let meta = dummy_meta();
        let result = build_v2_session("password", &meta, &blob);
        assert!(result.is_err(), "short-header blob should return Err, got Ok");
    }

    /// A blob that has salt + metadata_len but not metadata_nonce should return Err.
    #[test]
    fn build_v2_session_short_nonce_returns_err() {
        let blob = vec![0u8; SALT_LEN + 4]; // salt + meta_len, but no nonce
        let meta = dummy_meta();
        let result = build_v2_session("password", &meta, &blob);
        assert!(result.is_err(), "short nonce blob should return Err, got Ok");
    }
}

/// Fetch the data bytes of a specific file in an unlocked container.
/// V2: checks cache first, then seeks + decrypts from blob on miss.
/// V1: reads from in-memory session payload.
#[tauri::command]
pub async fn get_file_data(
    container_id: String,
    file_id: String,
    sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<Vec<u8>, CryptoError> {
    // Try v2 first
    if let Some(data) = get_file_data_v2(&container_id, &file_id, &sessions_v2)? {
        return Ok(data);
    }
    // Fallback to v1
    get_file_data_v1(&container_id, &file_id, &sessions)
}

fn get_file_data_v1(
    container_id: &str,
    file_id: &str,
    sessions: &SessionStore,
) -> std::result::Result<Vec<u8>, CryptoError> {
    let store = sessions.0.lock().unwrap();
    let session = store.get(container_id)
        .ok_or(CryptoError::SessionInactive)?;
    let file = session.payload.files.iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| CryptoError::NotFound(file_id.to_string()))?;
    Ok(file.data.clone())
}

fn get_file_data_v2(
    container_id: &str,
    file_id: &str,
    sessions_v2: &SessionStoreV2,
) -> std::result::Result<Option<Vec<u8>>, CryptoError> {
    // Check v2 session and cache first
    let (blob_path, key_arr, fm, prior_sum) = {
        let store = sessions_v2.0.lock().unwrap();
        let session = match store.get(container_id) {
            Some(s) => s,
            None => return Ok(None), // No v2 session, fall through to v1
        };

        // Check cache
        if let Some(cached) = session.cache_get(file_id) {
            return Ok(Some(cached));
        }

        // Find file by id AND its index for prior-sum calculation (read-side recovery)
        let idx = session.metadata.files.iter()
            .position(|f| f.id == file_id)
            .ok_or_else(|| CryptoError::NotFound(file_id.to_string()))?;
        let fm = session.metadata.files[idx].clone();

        // Compute cumulative encrypted size of prior files for read-side recovery
        let prior_sum: usize = session.metadata.files.iter()
            .take(idx)
            .map(vault::file_encrypted_len)
            .sum();

        let mut key = [0u8; 32];
        key.copy_from_slice(session.key.as_ref());
        (session.blob_path.clone(), key, fm, prior_sum)
    };

    // Open blob and read metadata_len from header (read-side recovery)
    let mut file = std::fs::File::open(&blob_path)?;
    use std::io::{Read, Seek, SeekFrom};

    let mut meta_len_buf = [0u8; 4];
    file.seek(SeekFrom::Start(SALT_LEN as u64))?;
    file.read_exact(&mut meta_len_buf)?;
    let metadata_len = u32::from_le_bytes(meta_len_buf) as usize;

    // Compute actual offset: ignores stored fm.offset and derives from
    // blob header metadata_len + cumulative prior-file sums (read-side recovery)
    let actual_offset = (SALT_LEN + 4 + crypto::NONCE_LEN + metadata_len + prior_sum) as u64;

    // Seek + read from blob at the recovered offset
    let enc_len = vault::file_encrypted_len(&fm);
    let mut encrypted = vec![0u8; enc_len];
    file.seek(SeekFrom::Start(actual_offset))?;
    file.read_exact(&mut encrypted)?;

    let plaintext = vault::decrypt_file(&encrypted, &fm, &key_arr)?;

    // Verify SHA-256
    let hash = crypto::sha256_hex(&plaintext);
    if hash != fm.sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    // Insert into cache
    {
        sessions_v2.get_mut(container_id, |session| {
            session.cache_put(file_id.to_string(), plaintext.clone());
        });
    }

    Ok(Some(plaintext))
}

/// Explicitly release a file from the v2 session cache, zeroizing its data.
#[tauri::command]
pub async fn release_file_data(
    container_id: String,
    file_id: String,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<(), CryptoError> {
    sessions_v2.get_mut(&container_id, |session| {
        session.release_file_data(&file_id);
    });
    Ok(())
}

/// Save edits to an unlocked container (add/remove files) and re-encrypt.
/// Supports both v1 (legacy single-encryption) and v2 (per-file encryption).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn save_edits(
    app: AppHandle,
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;

    if meta.format_version == 2 {
        save_edits_v2(app, container_id, password, files_to_add, file_ids_to_remove, meta, pool, sessions_v2).await
    } else {
        save_edits_v1(container_id, password, files_to_add, file_ids_to_remove, meta, pool, sessions).await
    }
}

/// V1 save_edits — legacy flow for containers encrypted as a single blob.
async fn save_edits_v1(
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    meta: ContainerMeta,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    // Verify password matches the original unlock password
    let (stored_key, stored_salt) = {
        let store = sessions.0.lock().unwrap();
        let session = store.get(&container_id)
            .ok_or(CryptoError::SessionInactive)?;
        (session.key.clone(), session.salt)
    };
    let verification_key = crypto::derive_key(&password, &stored_salt, &meta.kdf_params)?;
    if verification_key.as_ref() != stored_key.as_slice() {
        return Err(CryptoError::Decryption);
    }
    drop(verification_key);

    let password = zeroize::Zeroizing::new(password);

    let (total_size, file_count, plaintext) = {
        let mut store = sessions.0.lock().unwrap();
        let session = store.get_mut(&container_id)
            .ok_or(CryptoError::SessionInactive)?;

        let mut modified_payload = session.payload.clone();
        modified_payload.files.retain(|f| !file_ids_to_remove.contains(&f.id));
        for f in &files_to_add {
            // FileInput now carries a path, not bytes — read the source file.
            // (Legacy v1 flow keeps whole-file behavior; no chunking here.)
            let bytes = std::fs::read(&f.path)?;
            modified_payload.files.push(VaultFile {
                id: Uuid::new_v4().to_string(),
                name: f.name.clone(),
                mime: f.mime.clone(),
                size: bytes.len() as u64,
                data: bytes,
            });
        }

        let total_size: u64 = modified_payload.files.iter().map(|f| f.size).sum();
        let file_count = modified_payload.files.len() as u32;
        let plaintext = serde_json::to_vec(&modified_payload)?;

        session.payload = modified_payload;

        (total_size, file_count, plaintext)
    };

    let blob = crypto::encrypt(&plaintext, &password, &meta.kdf_params)?;
    let blob_sha256 = crypto::sha256_hex(&blob);

    atomic_write_blob(&meta.blob_path, &blob)?;
    storage::update_container_blob(&pool, &container_id, file_count, total_size, &blob_sha256).await?;

    drop(password);
    let updated_meta = storage::get_container(&pool, &container_id).await?;
    let removed_json = serde_json::json!({ "removed": file_ids_to_remove }).to_string();
    record_audit(&pool, "edit", Some(&container_id), Some(&meta.name), Some(&removed_json));
    Ok(updated_meta)
}

/// V2 save_edits — per-file encryption flow. Copies retained files verbatim; only encrypts new files.
#[allow(clippy::too_many_arguments)]
async fn save_edits_v2(
    app: AppHandle,
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    meta: ContainerMeta,
    pool: State<'_, sqlx::SqlitePool>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    // Emit derive-key verify progress (indeterminate)
    emit_progress(&app, ProgressPayload {
        operation: "derive-key".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: 0,
        bytes_total: 0,
        message: "Verifying encryption key…".into(),
    });

    // Lock session to extract key, salt, and metadata
    let (key_arr, salt, old_metadata) = {
        let store = sessions_v2.0.lock().unwrap();
        let session = store.get(&container_id)
            .ok_or(CryptoError::SessionInactive)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(session.key.as_ref());
        (zeroize::Zeroizing::new(key), session.salt, session.metadata.clone())
    };

    // Verify password
    let verification_key = crypto::derive_key(&password, &salt, &meta.kdf_params)?;
    if verification_key.as_ref() != key_arr.as_ref() {
        return Err(CryptoError::Decryption);
    }
    drop(verification_key);

    // Read existing blob
    let blob = std::fs::read(&meta.blob_path)?;

    // Read metadata_len from blob header for read-side recovery
    let end = SALT_LEN + 4;
    if blob.len() < end {
        return Err(CryptoError::InvalidFormat("save_edits_v2 blob too short for header".into()));
    }
    let meta_len_bytes: [u8; 4] = blob[SALT_LEN..end].try_into()
        .map_err(|_| CryptoError::InvalidFormat("save_edits_v2 blob too short for header".into()))?;
    let blob_metadata_len = u32::from_le_bytes(meta_len_bytes) as usize;

    // Compute total plaintext bytes for progress bar (retained + new)
    let retained_bytes: u64 = old_metadata.files.iter()
        .filter(|fm| !file_ids_to_remove.contains(&fm.id))
        .map(|fm| fm.size)
        .sum();
    let total_add_bytes: u64 = files_to_add.iter().map(|f| f.size).sum();
    let total_bytes = retained_bytes + total_add_bytes;
    let retained_count = old_metadata.files.iter().filter(|fm| !file_ids_to_remove.contains(&fm.id)).count();
    let total_ops = (retained_count + files_to_add.len()) as u64;

    // Decrypt remaining files (not removed), re-encrypt with new nonces.
    // Uses read-side offset recovery (ignores stored fm.offset, computes from
    // blob header metadata_len + cumulative `file_encrypted_len` sums) so that
    // containers created with the buggy two-pass code — and chunked files from
    // the streaming create_container — remain readable.
    let mut new_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_parts: Vec<Vec<u8>> = Vec::new();
    let mut total_size: u64 = 0;
    let mut prior_sum: usize = 0; // cumulative encrypted size of prior retained files
    let mut progress_idx: u64 = 0;

    for fm in &old_metadata.files {
        // On-disk encrypted length handles both whole-file and chunked layouts.
        // Retained files created by the streaming `create_container` are chunked,
        // so `fm.size + 16` would mis-slice them.
        let enc_len = vault::file_encrypted_len(fm);
        if file_ids_to_remove.contains(&fm.id) {
            prior_sum += enc_len;
            continue;
        }
        emit_progress(&app, ProgressPayload {
            operation: "encrypt".into(),
            current: progress_idx,
            total: total_ops,
            file_name: Some(fm.name.clone()),
            bytes_processed: total_size,
            bytes_total: total_bytes,  // total plaintext bytes (retained + new)
            message: format!("Saving {} ({} / {})", fm.name, progress_idx + 1, total_ops),
        });
        // Read-side recovery: compute actual offset from blob header metadata_len
        let actual_offset = SALT_LEN + 4 + crypto::NONCE_LEN + blob_metadata_len + prior_sum;
        if actual_offset + enc_len > blob.len() {
            return Err(CryptoError::IntegrityFailure);
        }
        // Copy ciphertext as-is — no decrypt/re-encrypt needed.
        // Plaintext unchanged, so original data_nonce + sha256 + chunks remain valid.
        // Offset will be recomputed by compute_v2_layout below.
        let enc_slice = blob[actual_offset..actual_offset + enc_len].to_vec();

        new_meta.push(FileMetadata {
            id: fm.id.clone(),
            name: fm.name.clone(),
            mime: fm.mime.clone(),
            size: fm.size,
            offset: 0, // set by compute_v2_layout below
            data_nonce: fm.data_nonce,
            sha256: fm.sha256.clone(),
            chunks: fm.chunks.clone(),
        });
        total_size += fm.size;
        encrypted_parts.push(enc_slice);
        prior_sum += enc_len;
        progress_idx += 1;
    }

    // Encrypt new files by streaming from disk in chunks (chunked layout +
    // per-chunk byte progress). Progress reports cumulative added bytes against
    // the total plaintext size (retained + new).
    let mut add_done: u64 = 0;
    let retained_size = total_size; // snapshot before loop — total_size mutates
    for f in &files_to_add {
        let base = add_done;
        let mut emit = |done_in_file: u64| {
            emit_progress(&app, ProgressPayload {
                operation: "encrypt".into(),
                current: progress_idx,
                total: total_ops,
                file_name: Some(f.name.clone()),
                bytes_processed: retained_size + base + done_in_file, // retained + cumulative new
                bytes_total: total_bytes,                         // total plaintext (retained + new)
                message: format!("Encrypting {} ({} / {})", f.name, progress_idx + 1, total_ops),
            });
        };
        emit(0);
        let (enc, chunks, sha256, plaintext_len) = vault::encrypt_file_chunked(
            std::path::Path::new(&f.path), &key_arr, vault::ENCRYPT_CHUNK_SIZE, &mut emit,
        )?;
        new_meta.push(FileMetadata {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: plaintext_len,
            offset: 0,
            data_nonce: [0u8; 12],
            sha256,
            chunks: Some(chunks),
        });
        total_size += plaintext_len;
        add_done += plaintext_len;
        encrypted_parts.push(enc);
        progress_idx += 1;
    }

    // Compute offsets iteratively using shared helper
    let mut metadata_v2 = ContainerMetadataV2 { version: 2, files: new_meta };
    let (enc_meta, meta_nonce) = vault::compute_v2_layout(&key_arr, &mut metadata_v2.files, &encrypted_parts)?;

    let enc_meta_len = enc_meta.len();
    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta_len;
    let file_data_len: usize = encrypted_parts.iter().map(|e| e.len()).sum();

    // Assemble v2 blob
    let mut new_blob = Vec::with_capacity(SALT_LEN + meta_section_len + file_data_len);
    new_blob.extend_from_slice(&salt);
    new_blob.extend_from_slice(&(enc_meta_len as u32).to_le_bytes());
    new_blob.extend_from_slice(&meta_nonce);
    new_blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_parts {
        new_blob.extend_from_slice(ef);
    }

    let blob_sha256 = crypto::sha256_hex(&new_blob);

    // Emit write-blob progress
    emit_progress(&app, ProgressPayload {
        operation: "write-blob".into(),
        current: 0,
        total: 0,
        file_name: None,
        bytes_processed: new_blob.len() as u64,
        bytes_total: new_blob.len() as u64,
        message: "Writing updated blob to disk…".into(),
    });

    // Atomic write
    atomic_write_blob(&meta.blob_path, &new_blob)?;

    // Update DB
    let file_count = metadata_v2.files.len() as u32;
    storage::update_container_blob(&pool, &container_id, file_count, total_size, &blob_sha256).await?;

    // Update session metadata
    {
        let mut store = sessions_v2.0.lock().unwrap();
        if let Some(session) = store.get_mut(&container_id) {
            session.metadata = metadata_v2;
        }
    }

    let updated_meta = storage::get_container(&pool, &container_id).await?;
    record_audit(&pool, "edit", Some(&container_id), Some(&updated_meta.name), None);
    Ok(updated_meta)
}

/// Atomic blob write: tmp → rename.
fn atomic_write_blob(path: &str, data: &[u8]) -> std::result::Result<(), CryptoError> {
    let blob_path = PathBuf::from(path);
    let tmp_path = blob_path.with_extension("enc.tmp");
    std::fs::write(&tmp_path, data)?;
    std::fs::rename(&tmp_path, &blob_path)?;
    Ok(())
}

/// Return the platform-appropriate download directory.
/// On mobile (Android/iOS) this is the public Downloads folder;
/// on desktop it is the user's download dir. Creates the directory if needed.
#[tauri::command]
pub async fn get_download_dir(app: tauri::AppHandle) -> std::result::Result<String, CryptoError> {
    let dir = app.path().download_dir()
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.to_string_lossy().into_owned();
    Ok(path)
}

/// List all containers (metadata only — no blobs, no keys).
#[tauri::command]
pub async fn list_containers(
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<Vec<ContainerMeta>, CryptoError> {
    storage::list_containers(&pool).await
}

/// Delete a container — removes the DB row, then the blob file.
///
/// The DB row is removed first so a container is never left undeletable: if
/// the blob file is already gone (manual cleanup, a prior partial failure, or
/// a DB restored on another machine), an `fs::remove_file` error must not block
/// removal of the entry. We therefore tolerate `NotFound` on the blob and only
/// propagate real filesystem errors (e.g. permissions) after the row is gone.
#[tauri::command]
pub async fn delete_container(
    container_id: String,
    pool: State<'_, sqlx::SqlitePool>,
    sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<(), CryptoError> {
    let meta = storage::get_container(&pool, &container_id).await?;
    storage::delete_container(&pool, &container_id).await?;
    match std::fs::remove_file(&meta.blob_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    sessions.lock(&container_id);
    sessions_v2.lock(&container_id);
    record_audit(&pool, "delete", Some(&container_id), Some(&meta.name), None);
    Ok(())
}

/// Lock a container session — wipes decrypted data and key from memory.
#[tauri::command]
pub async fn lock_container(
    container_id: String,
    sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<(), CryptoError> {
    sessions.lock(&container_id);
    sessions_v2.lock(&container_id);
    record_audit(&pool, "lock", Some(&container_id), None, None);
    Ok(())
}

/// Export a container to a .ctnr file.
/// Checks that the destination path does not already exist to avoid overwriting.
#[tauri::command]
pub async fn export_container(
    app: AppHandle,
    container_id: String,
    dest_path: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<(), CryptoError> {
    // Refuse to overwrite existing files
    if std::fs::metadata(&dest_path).is_ok() {
        return Err(CryptoError::InvalidFormat(
            "Destination file already exists".into(),
        ));
    }
    let meta = storage::get_container(&pool, &container_id).await?;

    emit_progress(&app, ProgressPayload {
        operation: "export".into(),
        current: 0, total: 0,
        file_name: Some(meta.name.clone()),
        bytes_processed: 0, bytes_total: 0,
        message: "Reading container for export\u{2026}".into(),
    });

    let blob = std::fs::read(&meta.blob_path)?;
    let ctnr_bytes = export::serialize(&meta, &blob)?;

    emit_progress(&app, ProgressPayload {
        operation: "export".into(),
        current: 0, total: 0,
        file_name: Some(meta.name.clone()),
        bytes_processed: 0,
        bytes_total: 0,
        message: "Writing export file\u{2026}".into(),
    });

    std::fs::write(&dest_path, ctnr_bytes)?;
    let details = serde_json::json!({ "dest_path": &dest_path }).to_string();
    record_audit(&pool, "export", Some(&container_id), Some(&meta.name), Some(&details));
    Ok(())
}

/// Import a .ctnr file into the vault.
/// Cleans up orphaned blob files if the database insert fails.
#[tauri::command]
pub async fn import_container(
    src_path: String,
    app: AppHandle,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<ContainerMeta, CryptoError> {
    let bytes = std::fs::read(&src_path)?;

    emit_progress(&app, ProgressPayload {
        operation: "import".into(),
        current: 0, total: 0,
        file_name: None,
        bytes_processed: 0, bytes_total: 0,
        message: "Reading import file\u{2026}".into(),
    });
    let (header, blob) = export::deserialize(&bytes)?;

    // Verify blob integrity
    let actual_sha256 = crypto::sha256_hex(&blob);
    if actual_sha256 != header.blob_sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    // Check for duplicate ID before writing the blob, so a re-import fails
    // cleanly with a friendly message instead of a cryptic UNIQUE-constraint
    // SQL error — and without leaving an orphaned blob on disk.
    let existing = storage::get_container(&pool, &header.id).await;
    if existing.is_ok() {
        return Err(CryptoError::InvalidFormat(
            "A container with this ID already exists in the vault".into(),
        ));
    }

    // Write blob to local blobs dir
    let blobs_dir = app.path().app_data_dir()
        .map_err(|e| CryptoError::Io(std::io::Error::other(e.to_string())))?
        .join("blobs");
    std::fs::create_dir_all(&blobs_dir)?;
    let blob_path = blobs_dir.join(format!("{}.enc", header.id));

    emit_progress(&app, ProgressPayload {
        operation: "import".into(),
        current: 0, total: 0,
        file_name: Some(header.name.clone()),
        bytes_processed: 0,
        bytes_total: 0,
        message: "Writing imported container\u{2026}".into(),
    });
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
        blob_path:   blob_path.to_str()
            .ok_or_else(|| CryptoError::InvalidFormat("Non-UTF-8 blob path".into()))?
            .to_string(),
        blob_sha256: header.blob_sha256,
        created_at:  header.created_at,
        modified_at: header.modified_at,
        format_version: header.format_version,
    };

    // Insert DB — clean up blob on failure to avoid orphaned files
    storage::insert_container(&pool, &meta).await.inspect_err(|_| {
        let _ = std::fs::remove_file(&blob_path);
    })?;

    let details = serde_json::json!({ "src_path": &src_path }).to_string();
    record_audit(&pool, "import", Some(&meta.id), Some(&meta.name), Some(&details));
    Ok(meta)
}

/// Resolve a destination filename that doesn't collide with existing files.
/// `name.ext` → `name (1).ext` → `name (2).ext` … until the path is unused.
/// Uses the `std::path::Path::extension()` definition of "extension" —
/// only the part after the LAST dot. Files without an extension get
/// ` (N)` appended before the whole name: `README` → `README (1)`.
fn resolve_collision_path(dest_dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let stem = std::path::Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let candidate = dest_dir.join(name);
    if !candidate.exists() {
        return candidate;
    }

    for n in 1..1000 {
        let new_name = format!("{} ({}){}", stem, n, ext);
        let candidate = dest_dir.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    // Fallback: append timestamp (should never happen)
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dest_dir.join(format!("{}_{}{}", stem, ts, ext))
}

/// Download files from an unlocked container to a chosen directory.
///
/// Decrypts each requested file via the v2 session (benefiting from WI-0288's
/// read-side recovery and LRU cache), writes it to `<dest_dir>/<original_name>`,
/// and auto-renames on collision (`name.ext` → `name (1).ext` …).
///
/// Per-file errors surface via the `error` field in `DownloadResult`; a single
/// failed file NEVER aborts the whole batch.
#[tauri::command]
pub async fn download_files(
    app: AppHandle,
    container_id: String,
    file_ids: Vec<String>,
    dest_dir: String,
    sessions_v2: State<'_, SessionStoreV2>,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<Vec<vault::DownloadResult>, CryptoError> {
    let dest = std::path::PathBuf::from(&dest_dir);
    std::fs::create_dir_all(&dest)?;

    let total_files = file_ids.len() as u64;

    // Pre-fetch all file metadata in a single lock — zero lock acquisitions in loop
    let file_metas: Vec<(String, String, u64)> = {
        let store = sessions_v2.0.lock().unwrap();
        store.get(&container_id).map(|s| {
            file_ids.iter().filter_map(|id| {
                s.metadata.files.iter()
                    .find(|fm| fm.id == *id)
                    .map(|fm| (fm.id.clone(), fm.name.clone(), fm.size))
            }).collect()
        }).unwrap_or_default()
    };
    let total_bytes: u64 = file_metas.iter().map(|(_, _, sz)| sz).sum();

    let mut results = Vec::with_capacity(file_ids.len());
    let mut cumulative_bytes: u64 = 0;

    for (i, (fid, fname, _fsize)) in file_metas.iter().enumerate() {
        // Emit decrypt progress before processing this file
        emit_progress(&app, ProgressPayload {
            operation: "decrypt".into(),
            current: i as u64,
            total: total_files,
            file_name: Some(fname.clone()),
            bytes_processed: cumulative_bytes,
            bytes_total: total_bytes,
            message: format!("Decrypting {} ({} / {})", fname, i + 1, total_files),
        });
        match get_file_data_v2(&container_id, fid, &sessions_v2) {
            Ok(Some(data)) => {
                cumulative_bytes += data.len() as u64;
                let write_path = resolve_collision_path(&dest, fname);
                match std::fs::write(&write_path, &data) {
                    Ok(()) => {
                        let path_str = write_path.to_string_lossy().into_owned();
                        results.push(vault::DownloadResult {
                            file_id: fid.clone(),
                            written_path: Some(path_str),
                            bytes: data.len() as u64,
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(vault::DownloadResult {
                            file_id: fid.clone(),
                            written_path: None,
                            bytes: 0,
                            error: Some(format!("Write error: {}", e)),
                        });
                    }
                }
            }
            Ok(None) => {
                results.push(vault::DownloadResult {
                    file_id: fid.clone(),
                    written_path: None,
                    bytes: 0,
                    error: Some("No v2 session — container may be locked".into()),
                });
            }
            Err(e) => {
                results.push(vault::DownloadResult {
                    file_id: fid.clone(),
                    written_path: None,
                    bytes: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    // Audit container name from DB
    let container_name = storage::get_container(&pool, &container_id)
        .await
        .map(|m| m.name)
        .unwrap_or_default();
    let details = serde_json::json!({"files": file_ids.len(), "dest_dir": &dest_dir }).to_string();
    record_audit(&pool, "download", Some(&container_id), Some(&container_name), Some(&details));

    // Final progress: download complete
    emit_progress(&app, ProgressPayload {
        operation: "decrypt".into(),
        current: total_files,
        total: total_files,
        file_name: None,
        bytes_processed: total_bytes,
        bytes_total: total_bytes,
        message: "Download complete".into(),
    });

    Ok(results)
}

/// Best-effort audit logging helper. Never propagates errors to the caller.
fn record_audit(
    pool: &sqlx::SqlitePool,
    action: &str,
    container_id: Option<&str>,
    container_name: Option<&str>,
    details: Option<&str>,
) {
    tauri::async_runtime::spawn({
        let pool = pool.clone();
        let action = action.to_string();
        let container_id = container_id.map(|s| s.to_string());
        let container_name = container_name.map(|s| s.to_string());
        let details = details.map(|s| s.to_string());
        async move {
            if let Err(e) = storage::insert_audit_event(
                &pool, &action,
                container_id.as_deref(),
                container_name.as_deref(),
                details.as_deref(),
            ).await {
                eprintln!("audit log failed: {e}");
            }
        }
    });
}

/// List audit events, newest first. Defaults to 200, max 1000.
#[tauri::command]
pub async fn list_audit_events(
    limit: Option<u32>,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<Vec<vault::AuditEvent>, CryptoError> {
    let limit = limit.unwrap_or(200).min(1000);
    storage::list_audit_events(&pool, limit).await
}
