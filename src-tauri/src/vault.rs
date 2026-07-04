//! Vault module — container data types and high-level operations.
//!
//! This module defines the ContainerMeta struct (what lives in SQLite)
//! and the ContainerPayload struct (what is encrypted inside the blob).

use crate::crypto::{self, KdfParams, SALT_LEN};
use crate::error::CryptoError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

/// Decrypt a file's data section (bytes already read from the blob at the
/// file's offset), handling both whole-file (`chunks: None`) and chunked
/// (`chunks: Some`) layouts. Chunk offsets are relative to `section` start.
pub fn decrypt_file(
    section: &[u8],
    fm: &FileMetadata,
    key: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    match &fm.chunks {
        None => crypto::decrypt_section(section, key, &fm.data_nonce),
        Some(chunks) => {
            let mut out = Vec::with_capacity(fm.size as usize);
            for c in chunks {
                let start = c.offset as usize;
                // Checked arithmetic: a hostile/corrupt chunk with an absurd
                // offset/size (near usize::MAX) must not wrap `end` to a small
                // value that slips past the bounds guard below. Overflow ==
                // corruption. With checked_add, end >= start always holds, so
                // the slice `section[start..end]` can never have start > end.
                let end = start
                    .checked_add(c.size as usize)
                    .and_then(|v| v.checked_add(16))
                    .ok_or(CryptoError::IntegrityFailure)?;
                if end > section.len() {
                    return Err(CryptoError::IntegrityFailure);
                }
                out.extend_from_slice(&crypto::decrypt_section(&section[start..end], key, &c.nonce)?);
            }
            Ok(out)
        }
    }
}

/// On-disk encrypted length of a file's data section (ciphertext + GCM tags).
/// Whole-file layout: plaintext size + one 16-byte tag.
/// Chunked layout: sum over chunks of (chunk size + 16-byte tag).
pub fn file_encrypted_len(fm: &FileMetadata) -> usize {
    match &fm.chunks {
        Some(chunks) => chunks.iter().map(|c| c.size as usize + 16).sum(),
        None => fm.size as usize + 16,
    }
}

/// A single audit log event returned by `list_audit_events`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub ts: String,
    pub action: String,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub details: Option<String>,
}

/// Per-file download result returned by the `download_files` command.
/// Per-file errors surface via `error` without aborting the batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub file_id: String,
    pub written_path: Option<String>,
    pub bytes: u64,
    pub error: Option<String>,
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

pub const ENCRYPT_CHUNK_SIZE: usize = 2 * 1024 * 1024;

/// Read `path` in `chunk_size` blocks, sealing each block with AES-256-GCM
/// under its own random nonce. Returns the concatenated ciphertext (chunk
/// ciphertexts laid end to end), the per-chunk metadata (offsets relative to
/// the concatenation start), the SHA-256 of the whole plaintext, and the
/// total plaintext length. `progress` is called with cumulative plaintext
/// bytes processed after each chunk.
pub fn encrypt_file_chunked(
    path: &std::path::Path,
    key: &[u8; 32],
    chunk_size: usize,
    progress: &mut dyn FnMut(u64),
) -> Result<(Vec<u8>, Vec<ChunkMetadata>, String, u64), CryptoError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut out: Vec<u8> = Vec::new();
    let mut chunks: Vec<ChunkMetadata> = Vec::new();
    let mut buf = vec![0u8; chunk_size];
    let mut total: u64 = 0;

    loop {
        // Fill buf up to chunk_size or EOF (handle short reads).
        let mut filled = 0;
        while filled < buf.len() {
            match file.read(&mut buf[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        if filled == 0 {
            break;
        }
        hasher.update(&buf[..filled]);
        let (ct, nonce) = crypto::encrypt_section(&buf[..filled], key)?;
        chunks.push(ChunkMetadata { offset: out.len() as u64, nonce, size: filled as u64 });
        out.extend_from_slice(&ct);
        total += filled as u64;
        progress(total);
    }

    let sha = hex::encode(hasher.finalize());
    Ok((out, chunks, sha, total))
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
    fn file_encrypted_len_whole_vs_chunked() {
        let base = FileMetadata {
            id: "x".into(), name: "a".into(), mime: "application/octet-stream".into(),
            size: 100, offset: 0, data_nonce: [0u8; 12], sha256: String::new(), chunks: None,
        };
        // whole file: plaintext + one 16-byte tag
        assert_eq!(file_encrypted_len(&base), 100 + 16);

        let chunked = FileMetadata {
            chunks: Some(vec![
                ChunkMetadata { offset: 0,  nonce: [0u8; 12], size: 60 },
                ChunkMetadata { offset: 76, nonce: [0u8; 12], size: 40 },
            ]),
            ..base
        };
        // two chunks: (60+16) + (40+16)
        assert_eq!(file_encrypted_len(&chunked), 76 + 56);
    }

    #[test]
    fn decrypt_file_handles_chunked_and_whole() {
        let key = [7u8; 32];

        // whole-file
        let (ct_whole, nonce) = crypto::encrypt_section(b"hello world", &key).unwrap();
        let fm_whole = FileMetadata {
            id: "w".into(), name: "w".into(), mime: "".into(), size: 11, offset: 0,
            data_nonce: nonce, sha256: String::new(), chunks: None,
        };
        assert_eq!(decrypt_file(&ct_whole, &fm_whole, &key).unwrap(), b"hello world");

        // chunked: two chunks "hello" + " world" concatenated on disk
        let (c0, n0) = crypto::encrypt_section(b"hello", &key).unwrap();
        let (c1, n1) = crypto::encrypt_section(b" world", &key).unwrap();
        let mut section = Vec::new();
        section.extend_from_slice(&c0);
        section.extend_from_slice(&c1);
        let fm_chunked = FileMetadata {
            chunks: Some(vec![
                ChunkMetadata { offset: 0, nonce: n0, size: 5 },
                ChunkMetadata { offset: c0.len() as u64, nonce: n1, size: 6 },
            ]),
            ..fm_whole.clone()
        };
        assert_eq!(decrypt_file(&section, &fm_chunked, &key).unwrap(), b"hello world");
    }

    #[test]
    fn decrypt_file_rejects_malformed_chunk_metadata() {
        let key = [7u8; 32];
        let base = FileMetadata {
            id: "m".into(), name: "m".into(), mime: "".into(), size: 0, offset: 0,
            data_nonce: [0u8; 12], sha256: String::new(), chunks: None,
        };

        // Absurd size (u64::MAX) with a short section: the size + 16 arithmetic
        // would overflow. Must return IntegrityFailure, NOT panic.
        let fm_overflow = FileMetadata {
            chunks: Some(vec![ChunkMetadata { offset: 0, nonce: [0u8; 12], size: u64::MAX }]),
            ..base.clone()
        };
        let short_section = vec![0u8; 8];
        assert!(matches!(
            decrypt_file(&short_section, &fm_overflow, &key),
            Err(CryptoError::IntegrityFailure)
        ));

        // Offset past the end of the section. Must return IntegrityFailure.
        let fm_past_end = FileMetadata {
            chunks: Some(vec![ChunkMetadata { offset: 1000, nonce: [0u8; 12], size: 5 }]),
            ..base.clone()
        };
        assert!(matches!(
            decrypt_file(&short_section, &fm_past_end, &key),
            Err(CryptoError::IntegrityFailure)
        ));
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

    #[test]
    fn encrypt_file_chunked_roundtrips_and_reports_progress() {
        use std::io::Write;
        let key = [3u8; 32];
        // 5000 bytes, chunk size 2048 -> 3 chunks (2048, 2048, 904)
        let plaintext: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        let dir = std::env::temp_dir().join(format!("ctnr_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blob.bin");
        std::fs::File::create(&path).unwrap().write_all(&plaintext).unwrap();

        let mut ticks: Vec<u64> = Vec::new();
        let (ct, chunks, sha, len) =
            encrypt_file_chunked(&path, &key, 2048, &mut |done| ticks.push(done)).unwrap();

        assert_eq!(len, 5000);
        assert_eq!(chunks.len(), 3);
        assert_eq!(ticks, vec![2048, 4096, 5000]); // progress after each chunk
        assert_eq!(sha, crypto::sha256_hex(&plaintext));

        // Reconstruct a FileMetadata and roundtrip through decrypt_file
        let fm = FileMetadata {
            id: "t".into(), name: "t".into(), mime: "".into(), size: len, offset: 0,
            data_nonce: [0u8; 12], sha256: sha, chunks: Some(chunks),
        };
        assert_eq!(decrypt_file(&ct, &fm, &key).unwrap(), plaintext);

        std::fs::remove_dir_all(&dir).ok();
    }
}
