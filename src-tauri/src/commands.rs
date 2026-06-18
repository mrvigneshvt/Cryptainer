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
    vault::{ContainerMeta, ContainerPayload, VaultFile, ContainerMetadataV2, FileMetadata},
    session::{Session, SessionStore, SessionStoreV2, SessionV2},
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

    // 3. Compute metadata section and calculate file offsets
    let mut metadata_v2 = ContainerMetadataV2 { version: 2, files: files_meta };
    let metadata_json = serde_json::to_vec(&metadata_v2)?;
    let (encrypted_metadata, _metadata_nonce) = crypto::encrypt_section(&metadata_json, &*key)?;

    // Layout: salt (16) | metadata_len (4) | metadata_nonce (12) | metadata_ciphertext | file1 | file2 | ...
    let metadata_section_len = 4 + crypto::NONCE_LEN + encrypted_metadata.len();
    let mut current_offset = SALT_LEN + metadata_section_len;

    for (i, fm) in metadata_v2.files.iter_mut().enumerate() {
        fm.offset = current_offset as u64;
        current_offset += encrypted_files[i].len();
    }

    // 4. Re-encrypt metadata with correct offsets
    let metadata_json_final = serde_json::to_vec(&metadata_v2)?;
    let (encrypted_metadata_final, metadata_nonce_final) = crypto::encrypt_section(&metadata_json_final, &*key)?;

    // 5. Assemble blob
    let metadata_section_len_final = 4 + crypto::NONCE_LEN + encrypted_metadata_final.len();
    let file_data_len: usize = encrypted_files.iter().map(|e| e.len()).sum();
    let blob_total_len = SALT_LEN + metadata_section_len_final + file_data_len;

    let mut blob = Vec::with_capacity(blob_total_len);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(encrypted_metadata_final.len() as u32).to_le_bytes());
    blob.extend_from_slice(&metadata_nonce_final);
    blob.extend_from_slice(&encrypted_metadata_final);
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
    sessions: State<'_, SessionStore>,
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
        return unlock_v2(&container_id, &password, &meta, &blob, sessions_v2);
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

    let mut metadata_v2 = ContainerMetadataV2 { version: 2, files: files_meta };
    let metadata_json = serde_json::to_vec(&metadata_v2)?;
    let (enc_meta, _meta_nonce) = crypto::encrypt_section(&metadata_json, &*key)?;

    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta.len();
    let mut offset = SALT_LEN + meta_section_len;
    for (i, fm) in metadata_v2.files.iter_mut().enumerate() {
        fm.offset = offset as u64;
        offset += encrypted_files[i].len();
    }

    let metadata_json = serde_json::to_vec(&metadata_v2)?;
    let (enc_meta, meta_nonce) = crypto::encrypt_section(&metadata_json, &*key)?;

    let file_data_len: usize = encrypted_files.iter().map(|e| e.len()).sum();
    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta.len();
    let mut blob = Vec::with_capacity(SALT_LEN + meta_section_len + file_data_len);
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
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
    let (_cache_hit, blob_path, key_arr, file_meta_opt) = {
        let store = sessions_v2.0.lock().unwrap();
        let session = match store.get(container_id) {
            Some(s) => s,
            None => return Ok(None), // No v2 session, fall through to v1
        };

        // Check cache
        if let Some(cached) = session.cache_get(file_id) {
            return Ok(Some(cached));
        }

        // Get file metadata
        let fm = match session.metadata.files.iter().find(|f| f.id == file_id) {
            Some(fm) => fm.clone(),
            None => return Err(CryptoError::NotFound(file_id.to_string())),
        };

        let mut key = [0u8; 32];
        key.copy_from_slice(session.key.as_ref());
        (false, session.blob_path.clone(), key, Some(fm))
    };

    let fm = file_meta_opt.ok_or_else(|| CryptoError::NotFound(file_id.to_string()))?;

    // Seek + read from blob
    let mut file = std::fs::File::open(&blob_path)?;
    let offset = fm.offset;
    let enc_len = fm.size as usize + 16; // +GCM tag
    let mut encrypted = vec![0u8; enc_len];
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(offset))?;
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

    // Decrypt remaining files (not removed), re-encrypt with new nonces
    let mut new_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_parts: Vec<Vec<u8>> = Vec::new();
    let mut total_size: u64 = 0;

    for fm in &old_metadata.files {
        if file_ids_to_remove.contains(&fm.id) {
            continue;
        }
        let offset = fm.offset as usize;
        let enc_len = fm.size as usize + 16;
        if offset + enc_len > blob.len() {
            return Err(CryptoError::IntegrityFailure);
        }
        let plaintext = crypto::decrypt_section(&blob[offset..offset + enc_len], &*key_arr, &fm.data_nonce)?;
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

    // Calculate offsets — first encrypt metadata to get its size
    let mut metadata_v2 = ContainerMetadataV2 { version: 2, files: new_meta };
    let metadata_json = serde_json::to_vec(&metadata_v2)?;
    let (enc_meta, _meta_nonce) = crypto::encrypt_section(&metadata_json, &*key_arr)?;

    // Compute final offsets for each file in metadata
    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta.len();
    let mut offset = SALT_LEN + meta_section_len;
    for (i, fm) in metadata_v2.files.iter_mut().enumerate() {
        fm.offset = offset as u64;
        offset += encrypted_parts[i].len();
    }

    // Re-encrypt metadata with correct offsets
    let metadata_json = serde_json::to_vec(&metadata_v2)?;
    let (enc_meta, meta_nonce) = crypto::encrypt_section(&metadata_json, &*key_arr)?;

    let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta.len();
    let file_data_len: usize = encrypted_parts.iter().map(|e| e.len()).sum();

    // Assemble v2 blob
    let mut new_blob = Vec::with_capacity(SALT_LEN + meta_section_len + file_data_len);
    new_blob.extend_from_slice(&salt);
    new_blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
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
    Ok(())
}

/// Lock a container session — wipes decrypted data and key from memory.
#[tauri::command]
pub async fn lock_container(
    container_id: String,
    sessions: State<'_, SessionStore>,
    sessions_v2: State<'_, SessionStoreV2>,
) -> std::result::Result<(), CryptoError> {
    sessions.lock(&container_id);
    sessions_v2.lock(&container_id);
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

    Ok(meta)
}
