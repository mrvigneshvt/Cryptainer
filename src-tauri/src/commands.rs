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

#[derive(serde::Deserialize)]
pub struct FileInput {
    pub name: String,
    pub mime: String,
    pub data: Vec<u8>,  // raw bytes from frontend
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
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(&password, &salt, &input.kdf_params)?;

    // 2. Encrypt each file individually, collect metadata (offsets filled later)
    let total_size: u64 = input.files.iter().map(|f| f.data.len() as u64).sum();
    let mut files_meta: Vec<FileMetadata> = Vec::with_capacity(input.files.len());
    let mut encrypted_files: Vec<Vec<u8>> = Vec::with_capacity(input.files.len());

    for f in &input.files {
        let sha256 = crypto::sha256_hex(&f.data);
        let (encrypted, nonce) = crypto::encrypt_section(&f.data, &*key)?;
        files_meta.push(FileMetadata {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.data.len() as u64,
            offset: 0,
            data_nonce: nonce,
            sha256,
            chunks: None,
        });
        encrypted_files.push(encrypted);
    }

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

    // 6. Write blob to disk
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

    storage::insert_container(&pool, &meta).await.or_else(|e| {
        let _ = std::fs::remove_file(&blob_path);
        Err(e)
    })?;

    record_audit(&pool, "create", Some(&meta.id), Some(&meta.name), None);
    drop(password);
    Ok(meta)
}

/// Unlock (decrypt) a container. Handles both v1 (legacy) and v2 (per-file).
/// V1 containers are automatically migrated to v2 on unlock.
#[tauri::command]
pub async fn unlock_container(
    container_id: String,
    password: String,
    pool: State<'_, sqlx::SqlitePool>,
    _sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<Vec<serde_json::Value>, CryptoError> {
    validate_password(&password)?;
    let password = zeroize::Zeroizing::new(password);

    let meta = storage::get_container(&pool, &container_id).await?;
    let blob = std::fs::read(&meta.blob_path)?;

    let actual_sha256 = crypto::sha256_hex(&blob);
    if actual_sha256 != meta.blob_sha256 {
        return Err(CryptoError::IntegrityFailure);
    }

    if meta.format_version == 2 {
        let result = unlock_v2(&container_id, &password, &meta, &blob, sessions_v2);
        if result.is_ok() {
            record_audit(&pool, "unlock", Some(&container_id), Some(&meta.name), None);
        }
        return result;
    }

    // v1 detected — auto-migrate to v2
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
    let salt: [u8; SALT_LEN] = blob[..SALT_LEN].try_into().unwrap();
    let key = crypto::derive_key(password, &salt, &meta.kdf_params)?;

    let meta_len_offset = SALT_LEN;
    let meta_len = u32::from_le_bytes(
        blob[meta_len_offset..meta_len_offset + 4].try_into().unwrap()
    ) as usize;

    let meta_nonce_offset = meta_len_offset + 4;
    let meta_nonce: [u8; crypto::NONCE_LEN] = blob[meta_nonce_offset..meta_nonce_offset + crypto::NONCE_LEN]
        .try_into().unwrap();

    let meta_ct_offset = meta_nonce_offset + crypto::NONCE_LEN;
    let meta_ct_end = meta_ct_offset + meta_len;
    let metadata_ciphertext = &blob[meta_ct_offset..meta_ct_end];

    let metadata_plaintext = crypto::decrypt_section(metadata_ciphertext, &*key, &meta_nonce)?;
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
        let (encrypted, nonce) = crypto::encrypt_section(&f.data, &*key)?;
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

    let metadata_plaintext = crypto::decrypt_section(metadata_ciphertext, &*key, &meta_nonce)?;
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
            .map(|f| f.size as usize + 16) // plaintext + GCM tag
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
    let enc_len = fm.size as usize + 16; // +GCM tag
    let mut encrypted = vec![0u8; enc_len];
    file.seek(SeekFrom::Start(actual_offset))?;
    file.read_exact(&mut encrypted)?;

    let plaintext = crypto::decrypt_section(&encrypted, &key_arr, &fm.data_nonce)?;

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
#[tauri::command]
pub async fn save_edits(
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
        save_edits_v2(container_id, password, files_to_add, file_ids_to_remove, meta, pool, sessions_v2).await
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
            modified_payload.files.push(VaultFile {
                id: Uuid::new_v4().to_string(),
                name: f.name.clone(),
                mime: f.mime.clone(),
                size: f.data.len() as u64,
                data: f.data.clone(),
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

/// V2 save_edits — per-file encryption flow. Re-encrypts every file.
async fn save_edits_v2(
    container_id: String,
    password: String,
    files_to_add: Vec<FileInput>,
    file_ids_to_remove: Vec<String>,
    meta: ContainerMeta,
    pool: State<'_, sqlx::SqlitePool>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<ContainerMeta, CryptoError> {
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
    if verification_key.as_ref() != &*key_arr {
        return Err(CryptoError::Decryption);
    }
    drop(verification_key);

    // Read existing blob
    let blob = std::fs::read(&meta.blob_path)?;

    // Read metadata_len from blob header for read-side recovery
    let meta_len_bytes: [u8; 4] = blob[SALT_LEN..SALT_LEN + 4].try_into().unwrap();
    let blob_metadata_len = u32::from_le_bytes(meta_len_bytes) as usize;

    // Decrypt remaining files (not removed), re-encrypt with new nonces
    // Uses read-side offset recovery (ignores stored fm.offset, computes from
    // blob header metadata_len + cumulative {size+16} sums) so that containers
    // created with the buggy two-pass code remain readable.
    let mut new_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_parts: Vec<Vec<u8>> = Vec::new();
    let mut total_size: u64 = 0;
    let mut prior_sum: usize = 0; // cumulative encrypted size of prior retained files

    for fm in &old_metadata.files {
        let enc_len = fm.size as usize + 16;
        if file_ids_to_remove.contains(&fm.id) {
            prior_sum += enc_len;
            continue;
        }
        // Read-side recovery: compute actual offset from blob header metadata_len
        let actual_offset = SALT_LEN + 4 + crypto::NONCE_LEN + blob_metadata_len + prior_sum;
        if actual_offset + enc_len > blob.len() {
            return Err(CryptoError::IntegrityFailure);
        }
        let plaintext = crypto::decrypt_section(&blob[actual_offset..actual_offset + enc_len], &key_arr, &fm.data_nonce)?;
        let sha256 = crypto::sha256_hex(&plaintext);
        let (new_enc, new_nonce) = crypto::encrypt_section(&plaintext, &*key_arr)?;

        new_meta.push(FileMetadata {
            id: fm.id.clone(),
            name: fm.name.clone(),
            mime: fm.mime.clone(),
            size: fm.size,
            offset: 0,
            data_nonce: new_nonce,
            sha256,
            chunks: fm.chunks.clone(),
        });
        total_size += fm.size;
        encrypted_parts.push(new_enc);
        prior_sum += enc_len;
    }

    // Encrypt new files
    for f in &files_to_add {
        let sha256 = crypto::sha256_hex(&f.data);
        let (enc, nonce) = crypto::encrypt_section(&f.data, &*key_arr)?;
        new_meta.push(FileMetadata {
            id: Uuid::new_v4().to_string(),
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: f.data.len() as u64,
            offset: 0,
            data_nonce: nonce,
            sha256,
            chunks: None,
        });
        total_size += f.data.len() as u64;
        encrypted_parts.push(enc);
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

/// Export a container to a .ctnr file at the given path.
/// Checks that the destination path does not already exist to avoid overwriting.
#[tauri::command]
pub async fn export_container(
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
    let blob = std::fs::read(&meta.blob_path)?;
    let ctnr_bytes = export::serialize(&meta, &blob)?;
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
        blob_path:   blob_path.to_str()
            .ok_or_else(|| CryptoError::InvalidFormat("Non-UTF-8 blob path".into()))?
            .to_string(),
        blob_sha256: header.blob_sha256,
        created_at:  header.created_at,
        modified_at: header.modified_at,
        format_version: header.format_version,
    };

    // Insert DB — clean up blob on failure to avoid orphaned files
    storage::insert_container(&pool, &meta).await.or_else(|e| {
        let _ = std::fs::remove_file(&blob_path);
        Err(e)
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
    container_id: String,
    file_ids: Vec<String>,
    dest_dir: String,
    sessions_v2: State<'_, SessionStoreV2>,
    pool: State<'_, sqlx::SqlitePool>,
) -> std::result::Result<Vec<vault::DownloadResult>, CryptoError> {
    let dest = std::path::PathBuf::from(&dest_dir);
    std::fs::create_dir_all(&dest)?;

    let mut results = Vec::with_capacity(file_ids.len());

    for file_id in &file_ids {
        match get_file_data_v2(&container_id, file_id, &sessions_v2) {
            Ok(Some(data)) => {
                // Look up the file name from session metadata
                let file_name = {
                    let store = sessions_v2.0.lock().unwrap();
                    store.get(&container_id)
                        .and_then(|session| {
                            session.metadata.files.iter()
                                .find(|f| f.id == *file_id)
                                .map(|f| f.name.clone())
                        })
                        .unwrap_or_else(|| file_id.clone())
                };

                let write_path = resolve_collision_path(&dest, &file_name);
                match std::fs::write(&write_path, &data) {
                    Ok(()) => {
                        let path_str = write_path.to_string_lossy().into_owned();
                        results.push(vault::DownloadResult {
                            file_id: file_id.clone(),
                            written_path: Some(path_str),
                            bytes: data.len() as u64,
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(vault::DownloadResult {
                            file_id: file_id.clone(),
                            written_path: None,
                            bytes: 0,
                            error: Some(format!("Write error: {}", e)),
                        });
                    }
                }
            }
            Ok(None) => {
                results.push(vault::DownloadResult {
                    file_id: file_id.clone(),
                    written_path: None,
                    bytes: 0,
                    error: Some("No v2 session — container may be locked".into()),
                });
            }
            Err(e) => {
                results.push(vault::DownloadResult {
                    file_id: file_id.clone(),
                    written_path: None,
                    bytes: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Audit container name from DB (not from session metadata's first file name)
    let container_name = storage::get_container(&pool, &container_id)
        .await
        .map(|m| m.name)
        .unwrap_or_default();
    let details = serde_json::json!({"files": file_ids.len(), "dest_dir": &dest_dir }).to_string();
    record_audit(&pool, "download", Some(&container_id), Some(&container_name), Some(&details));
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
