//! Vault module — container data types and high-level operations.
//!
//! This module defines the ContainerMeta struct (what lives in SQLite)
//! and the ContainerPayload struct (what is encrypted inside the blob).

use crate::crypto::KdfParams;
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
