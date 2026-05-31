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

use crate::error::{CryptoError, Result};
use crate::vault::ContainerMeta;
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 4] = b"CNTR";
pub const NULL: u8 = 0x00;
pub const VERSION: u8 = 0x01;

/// Plaintext header stored at the start of every .ctnr file.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerHeader {
    pub id: String,
    pub name: String,
    pub algo: String,
    pub kdf: String,
    pub kdf_params: serde_json::Value,
    pub hint: Option<String>,
    pub tags: Option<String>,
    pub file_count: u32,
    pub total_size: u64,
    pub created_at: String,
    pub modified_at: String,
    pub blob_sha256: String, // SHA-256 of the blob for integrity check
}

/// Serialize a container into .ctnr binary format.
pub fn serialize(meta: &ContainerMeta, blob: &[u8]) -> Result<Vec<u8>> {
    let header = ContainerHeader {
        id: meta.id.clone(),
        name: meta.name.clone(),
        algo: meta.algo.clone(),
        kdf: meta.kdf_params.kdf.clone(),
        kdf_params: serde_json::to_value(&meta.kdf_params)?,
        hint: meta.hint.clone(),
        tags: meta.tags.clone(),
        file_count: meta.file_count,
        total_size: meta.total_size,
        created_at: meta.created_at.clone(),
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
        return Err(CryptoError::InvalidFormat(format!(
            "Unsupported version: {}. Expected {}",
            data[5], VERSION
        )));
    }

    let header_len = u32::from_le_bytes(
        data[6..10].try_into().expect("data[6..10] is always 4 bytes after len check")
    ) as usize;
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
            id: "test-id".into(),
            name: "Test".into(),
            algo: "AES-GCM-256".into(),
            kdf_params: KdfParams::argon2id_standard(),
            hint: Some("test hint".into()),
            tags: None,
            file_count: 2,
            total_size: 1024,
            blob_path: "/tmp/test.enc".into(),
            blob_sha256: "abc123".into(),
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
