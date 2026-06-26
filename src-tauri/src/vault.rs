//! Vault module — container data types and high-level operations.
//!
//! This module defines the ContainerMeta struct (what lives in SQLite)
//! and the ContainerPayload struct (what is encrypted inside the blob).

use crate::crypto::{self, KdfParams, SALT_LEN};
use crate::error::CryptoError;
use serde::{Deserialize, Serialize};

/// All metadata stored in plaintext in SQLite.
/// Never includes encrypted content or key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMeta {
    pub id: String,
    pub name: String,
    pub algo: String, // "AES-GCM-256" | "AES-GCM-128"
    pub kdf_params: KdfParams,
    pub hint: Option<String>,
    pub tags: Option<String>,
    pub file_count: u32,
    pub total_size: u64,     // total bytes of all files before encryption
    pub blob_path: String,   // absolute path to the .enc file on disk
    pub blob_sha256: String, // SHA-256 hex of the raw encrypted blob
    pub created_at: String,  // ISO-8601
    pub modified_at: String, // ISO-8601
    pub format_version: u8,  // 1 = legacy single-encryption, 2 = per-file encryption
}

/// A single file stored inside an encrypted container.
/// The data field holds the raw file bytes.
/// Stored as part of ContainerPayload, which is the plaintext inside the blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFile {
    pub id: String, // UUID v4
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub data: Vec<u8>, // raw file bytes — NEVER written to disk as plaintext
}

/// The plaintext content of a container.
/// This struct is serialized to JSON, then encrypted into the blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPayload {
    pub version: u8,
    pub files: Vec<VaultFile>,
}

/// V2 container metadata — the JSON that gets encrypted into the metadata section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetadataV2 {
    pub version: u8,
    pub files: Vec<FileMetadata>,
}

/// Per-file metadata stored in the encrypted metadata section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub offset: u64,
    pub data_nonce: [u8; 12],
    pub sha256: String,
    pub chunks: Option<Vec<ChunkMetadata>>,
}

/// Per-chunk metadata for video files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub offset: u64,
    pub nonce: [u8; 12],
    pub size: u64,
}

pub const VIDEO_CHUNK_SIZE: u64 = 2 * 1024 * 1024; // 2 MB

/// Iteratively compute the v2 blob layout until the encrypted metadata length stabilises.
///
/// The JSON encoding of `FileMetadata::offset` changes size when offset values change
/// (e.g. `0` vs `12345`), creating a circular dependency: metadata size affects file
/// positions, which affect offset values, which change metadata size again. We resolve
/// this by iterating: encrypt metadata → compute offsets → re-encrypt → compare length
/// → repeat if different. Convergence typically takes 2–3 passes.
///
/// This is the SAME algorithm the integration test `build_v2_blob` uses; production code
/// (`create_container`, `save_edits_v2`, `convert_v1_to_v2`) now calls this shared helper
/// instead of duplicating a broken two-pass variant.
///
/// # Arguments
/// * `key` — AES-256-GCM key
/// * `files_meta` — file metadata; `offset` fields are updated in-place to final values
/// * `encrypted_parts` — pre-encrypted file ciphertexts (same order as `files_meta`)
///
/// # Returns
/// `(encrypted_metadata_bytes, metadata_nonce)` — the final stable encryption
pub fn compute_v2_layout(
    key: &[u8; 32],
    files_meta: &mut [FileMetadata],
    encrypted_parts: &[Vec<u8>],
) -> std::result::Result<(Vec<u8>, [u8; 12]), CryptoError> {
    let mut enc_meta_len = 0usize;
    loop {
        let meta = ContainerMetadataV2 {
            version: 2,
            files: files_meta.to_owned(),
        };
        let meta_json = serde_json::to_vec(&meta)?;
        let (enc_meta, _) = crypto::encrypt_section(&meta_json, key)?;
        let prev_len = enc_meta_len;
        enc_meta_len = enc_meta.len();

        // Compute offsets based on this encrypted metadata length
        let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta_len;
        let mut offset = SALT_LEN + meta_section_len;
        for (i, fm) in files_meta.iter_mut().enumerate() {
            fm.offset = offset as u64;
            offset += encrypted_parts[i].len();
        }

        if enc_meta_len == prev_len {
            break;
        }
    }

    // Final encryption with stable offsets
    let meta = ContainerMetadataV2 {
        version: 2,
        files: files_meta.to_owned(),
    };
    let meta_json = serde_json::to_vec(&meta)?;
    let (enc_meta, meta_nonce) = crypto::encrypt_section(&meta_json, key)?;

    Ok((enc_meta, meta_nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_metadata_v2_roundtrip() {
        let meta = ContainerMetadataV2 {
            version: 2,
            files: vec![FileMetadata {
                id: "test-id".into(),
                name: "photo.jpg".into(),
                mime: "image/jpeg".into(),
                size: 1024,
                offset: 0,
                data_nonce: [1u8; 12],
                sha256: "abc123".into(),
                chunks: None,
            }],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let recovered: ContainerMetadataV2 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.version, 2);
        assert_eq!(recovered.files.len(), 1);
        assert_eq!(recovered.files[0].name, "photo.jpg");
    }

    #[test]
    fn file_metadata_with_chunks() {
        let fm = FileMetadata {
            id: "vid-1".into(),
            name: "clip.mp4".into(),
            mime: "video/mp4".into(),
            size: 5 * 1024 * 1024,
            offset: 4096,
            data_nonce: [0u8; 12],
            sha256: "deadbeef".into(),
            chunks: Some(vec![
                ChunkMetadata { offset: 4096, nonce: [1u8; 12], size: 2 * 1024 * 1024 + 16 },
                ChunkMetadata { offset: 4096 + 2 * 1024 * 1024 + 16, nonce: [2u8; 12], size: 1024 },
            ]),
        };
        let json = serde_json::to_string(&fm).unwrap();
        let recovered: FileMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.chunks.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn container_meta_default_format_version() {
        let meta = ContainerMeta {
            id: "x".into(),
            name: "test".into(),
            algo: "AES-GCM-256".into(),
            kdf_params: crate::crypto::KdfParams::argon2id_standard(),
            hint: None,
            tags: None,
            file_count: 0,
            total_size: 0,
            blob_path: "/tmp/test.enc".into(),
            blob_sha256: "0".repeat(64),
            created_at: "2024-01-01T00:00:00Z".into(),
            modified_at: "2024-01-01T00:00:00Z".into(),
            format_version: 1,
        };
        assert_eq!(meta.format_version, 1);
    }
}
