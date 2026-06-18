//! Integration tests for v2 per-file encryption format.

use cryptainer_lib::crypto;
use cryptainer_lib::crypto::KdfParams;
use cryptainer_lib::vault::{ContainerMetadataV2, FileMetadata, VaultFile, ContainerPayload};
use rand::RngCore;

fn test_params() -> KdfParams {
    KdfParams {
        kdf: "argon2id".into(),
        memory_kb: Some(8192),
        iterations: 1,
        parallelism: Some(1),
    }
}

fn build_v2_blob(
    password: &str,
    params: &KdfParams,
    file_data: &[(&str, &[u8])],
) -> (Vec<u8>, ContainerMetadataV2) {
    let mut salt = [0u8; crypto::SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(password, &salt, params).unwrap();

    // Encrypt all files first
    let mut files_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_files: Vec<Vec<u8>> = Vec::new();
    for (name, data) in file_data {
        let sha256 = crypto::sha256_hex(data);
        let (enc, nonce) = crypto::encrypt_section(data, &*key).unwrap();
        files_meta.push(FileMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            mime: "application/octet-stream".into(),
            size: data.len() as u64,
            offset: 0,
            data_nonce: nonce,
            sha256,
            chunks: None,
        });
        encrypted_files.push(enc);
    }

    // Iterative offset calculation (converges in 2-3 passes)
    let mut enc_meta_len = 0usize;
    let mut metadata = ContainerMetadataV2 { version: 2, files: files_meta };
    loop {
        // Encrypt current metadata to get its size
        let meta_json = serde_json::to_vec(&metadata).unwrap();
        let (enc_meta, _) = crypto::encrypt_section(&meta_json, &*key).unwrap();
        let prev_len = enc_meta_len;
        enc_meta_len = enc_meta.len();

        // Compute offsets based on this encrypted metadata length
        let meta_section_len = 4 + crypto::NONCE_LEN + enc_meta_len;
        let mut offset = crypto::SALT_LEN + meta_section_len;
        for (i, fm) in metadata.files.iter_mut().enumerate() {
            fm.offset = offset as u64;
            offset += encrypted_files[i].len();
        }

        if enc_meta_len == prev_len {
            break;
        }
    }

    // Final encryption with correct offsets
    let meta_json = serde_json::to_vec(&metadata).unwrap();
    let (enc_meta, meta_nonce) = crypto::encrypt_section(&meta_json, &*key).unwrap();

    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_files {
        blob.extend_from_slice(ef);
    }

    (blob, metadata)
}

fn verify_v2_blob(blob: &[u8], password: &str, params: &KdfParams, expected_data: &[(&str, &[u8])]) {
    let salt: [u8; 16] = blob[..16].try_into().unwrap();
    let key = crypto::derive_key(password, &salt, params).unwrap();

    let meta_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;
    let meta_nonce: [u8; 12] = blob[20..32].try_into().unwrap();
    let meta_ct = &blob[32..32 + meta_len];
    let meta_pt = crypto::decrypt_section(meta_ct, &*key, &meta_nonce).unwrap();
    let metadata: ContainerMetadataV2 = serde_json::from_slice(&meta_pt).unwrap();

    assert_eq!(metadata.files.len(), expected_data.len());

    for (name, original) in expected_data {
        let fm = metadata.files.iter().find(|f| f.name == *name)
            .unwrap_or_else(|| panic!("File {} not found in metadata", name));
        let off = fm.offset as usize;
        let enc_len = fm.size as usize + 16;
        assert!(off + enc_len <= blob.len(), "File offset out of bounds");
        let pt = crypto::decrypt_section(&blob[off..off + enc_len], &*key, &fm.data_nonce)
            .unwrap_or_else(|_| panic!("Decryption failed for file {}", name));
        assert_eq!(pt, *original, "Data mismatch for file {}", name);
        let hash = crypto::sha256_hex(&pt);
        assert_eq!(hash, fm.sha256, "SHA-256 mismatch for file {}", name);
    }
}

#[test]
fn v2_single_file_roundtrip() {
    let params = test_params();
    let file_data = vec![("readme.txt", b"Hello v2 format!" as &[u8])];
    let (blob, metadata) = build_v2_blob("test-v2", &params, &file_data);
    assert_eq!(metadata.files.len(), 1);
    verify_v2_blob(&blob, "test-v2", &params, &file_data);
}

#[test]
fn v2_multifile_roundtrip() {
    let params = test_params();
    let file_data: Vec<(&str, &[u8])> = vec![
        ("alpha.bin", &[0u8, 1, 2, 3]),
        ("beta.jpg", &[255, 128, 64, 32]),
        ("gamma.json", b"{\"key\": \"value\"}"),
    ];
    let (blob, metadata) = build_v2_blob("multi-test", &params, &file_data);
    assert_eq!(metadata.files.len(), 3);
    verify_v2_blob(&blob, "multi-test", &params, &file_data);
}

#[test]
fn v1_to_v2_migration_roundtrip() {
    let password = "migration-password";
    let params = test_params();

    let payload = ContainerPayload {
        version: 1,
        files: vec![
            VaultFile {
                id: uuid::Uuid::new_v4().to_string(),
                name: "file1.bin".into(),
                mime: "application/octet-stream".into(),
                size: 5,
                data: vec![1, 2, 3, 4, 5],
            },
            VaultFile {
                id: uuid::Uuid::new_v4().to_string(),
                name: "file2.txt".into(),
                mime: "text/plain".into(),
                size: 11,
                data: b"hello world".to_vec(),
            },
        ],
    };

    let v1_plaintext = serde_json::to_vec(&payload).unwrap();
    let v1_blob = crypto::encrypt(&v1_plaintext, password, &params).unwrap();
    let v1_plaintext2 = crypto::decrypt(&v1_blob, password, &params).unwrap();
    let payload2: ContainerPayload = serde_json::from_slice(&v1_plaintext2).unwrap();
    assert_eq!(payload2.files.len(), 2);

    let file_data: Vec<(&str, &[u8])> = payload2.files.iter()
        .map(|f| (f.name.as_str(), f.data.as_slice()))
        .collect();
    let (v2_blob, _) = build_v2_blob(password, &params, &file_data);

    let expected: Vec<(&str, &[u8])> = vec![
        ("file1.bin", &[1u8, 2, 3, 4, 5] as &[u8]),
        ("file2.txt", b"hello world"),
    ];
    verify_v2_blob(&v2_blob, password, &params, &expected);
}

#[test]
fn v2_tampered_file_rejected() {
    let params = test_params();
    let file_data = vec![("secret.txt", b"top secret data" as &[u8])];
    let (mut blob, _) = build_v2_blob("tamper", &params, &file_data);

    let last = blob.len() - 1;
    blob[last] ^= 0xFF;

    let salt: [u8; 16] = blob[..16].try_into().unwrap();
    let key = crypto::derive_key("tamper", &salt, &params).unwrap();
    let meta_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;
    let meta_nonce: [u8; 12] = blob[20..32].try_into().unwrap();
    let meta_ct = &blob[32..32 + meta_len];
    let meta_pt = crypto::decrypt_section(meta_ct, &*key, &meta_nonce).unwrap();
    let metadata: ContainerMetadataV2 = serde_json::from_slice(&meta_pt).unwrap();

    let fm = &metadata.files[0];
    let off = fm.offset as usize;
    let enc_len = fm.size as usize + 16;
    let result = crypto::decrypt_section(&blob[off..off + enc_len], &*key, &fm.data_nonce);
    assert!(result.is_err(), "Tampered blob should fail decryption");
}

#[test]
fn v2_wrong_password_rejected() {
    let params = test_params();
    let file_data = vec![("doc.txt", b"Some data" as &[u8])];
    let (blob, _) = build_v2_blob("correct", &params, &file_data);

    let salt: [u8; 16] = blob[..16].try_into().unwrap();
    let key = crypto::derive_key("wrong-password", &salt, &params).unwrap();

    let meta_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;
    let meta_nonce: [u8; 12] = blob[20..32].try_into().unwrap();
    let meta_ct = &blob[32..32 + meta_len];
    let result = crypto::decrypt_section(meta_ct, &*key, &meta_nonce);
    assert!(result.is_err(), "Wrong password should fail metadata decryption");
}
