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
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::error::{CryptoError, Result};

/// Argon2id parameters.
/// These represent the "Standard" security level.
/// See docs/CRYPTO.md for full parameter rationale.
pub const ARGON2_MEMORY_KB: u32 = 65536; // 64 MB
pub const ARGON2_ITERATIONS: u32 = 2;
pub const ARGON2_PARALLELISM: u32 = 1;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32; // AES-256

/// Supported KDF configurations selectable by the user.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KdfParams {
    pub kdf: String,            // "argon2id" | "pbkdf2"
    pub memory_kb: Option<u32>, // Argon2id only
    pub iterations: u32,
    pub parallelism: Option<u32>, // Argon2id only
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
pub fn derive_key(
    password: &str,
    salt: &[u8],
    params: &KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    match params.kdf.as_str() {
        "argon2id" => {
            let m = params.memory_kb.unwrap_or(ARGON2_MEMORY_KB);
            let t = params.iterations;
            let p = params.parallelism.unwrap_or(ARGON2_PARALLELISM);

            let argon2_params = Params::new(m, t, p, Some(KEY_LEN))
                .map_err(|e| CryptoError::KeyDerivation(e.to_string()))?;

            let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

            let mut key = Zeroizing::new([0u8; KEY_LEN]);
            argon2
                .hash_password_into(password.as_bytes(), salt, key.as_mut())
                .map_err(|e| CryptoError::KeyDerivation(e.to_string()))?;

            Ok(key)
        }
        _ => Err(CryptoError::KeyDerivation(format!(
            "Unknown KDF: {}",
            params.kdf
        ))),
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
        return Err(CryptoError::InvalidFormat(
            "Blob too short to be valid".into(),
        ));
    }

    let salt = &blob[..SALT_LEN];
    let nonce_raw = &blob[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt, params)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key.as_ref()));
    let nonce = Nonce::from_slice(nonce_raw);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decryption)
}

/// Encrypt a section (file, chunk, or metadata) with AES-256-GCM using a provided key.
///
/// Generates a random 12-byte nonce and returns `(ciphertext_with_tag, nonce)`.
/// The nonce must be stored alongside the ciphertext for later decryption.
pub fn encrypt_section(plaintext: &[u8], key: &[u8; KEY_LEN]) -> Result<(Vec<u8>, [u8; NONCE_LEN])> {
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    Ok((ciphertext, nonce_bytes))
}

/// Decrypt a section encrypted by `encrypt_section`.
///
/// Verifies the GCM authentication tag. Returns `CryptoError::Decryption` on
/// wrong key, wrong nonce, or tampered ciphertext.
pub fn decrypt_section(
    ciphertext: &[u8],
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

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
        KdfParams {
            kdf: "argon2id".into(),
            memory_kb: Some(8192),
            iterations: 1,
            parallelism: Some(1),
        }
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

    #[test]
    fn section_roundtrip() {
        let key = [42u8; KEY_LEN];
        let plaintext = b"per-file encrypted data";
        let (ciphertext, nonce) = encrypt_section(plaintext, &key).unwrap();
        let recovered = decrypt_section(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(plaintext.as_ref(), recovered.as_slice());
    }

    #[test]
    fn section_wrong_nonce_fails() {
        let key = [42u8; KEY_LEN];
        let (ciphertext, _nonce) = encrypt_section(b"secret", &key).unwrap();
        let wrong_nonce = [0u8; NONCE_LEN];
        let result = decrypt_section(&ciphertext, &key, &wrong_nonce);
        assert!(matches!(result, Err(CryptoError::Decryption)));
    }

    #[test]
    fn section_tampered_ciphertext_fails() {
        let key = [42u8; KEY_LEN];
        let (mut ciphertext, nonce) = encrypt_section(b"secret", &key).unwrap();
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xFF;
        let result = decrypt_section(&ciphertext, &key, &nonce);
        assert!(matches!(result, Err(CryptoError::Decryption)));
    }

    #[test]
    fn section_unique_nonces_per_call() {
        let key = [42u8; KEY_LEN];
        let (_, n1) = encrypt_section(b"data", &key).unwrap();
        let (_, n2) = encrypt_section(b"data", &key).unwrap();
        assert_ne!(n1, n2);
    }
}
