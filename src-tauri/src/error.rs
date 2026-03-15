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
    Database(String),

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
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl From<sqlx::Error> for CryptoError {
    fn from(e: sqlx::Error) -> Self {
        CryptoError::Database(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CryptoError>;
