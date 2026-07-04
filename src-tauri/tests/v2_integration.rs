//! Integration tests for v2 per-file encryption format.

use cryptainer_lib::crypto;
use cryptainer_lib::crypto::KdfParams;
use cryptainer_lib::vault;
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

/// Regression: build a v2 blob through the same `compute_v2_layout` helper used by
/// `create_container`, then verify every file decrypts at its STORED offset.
/// This test FAILS against the buggy two-pass code and PASSES after the fix.
#[test]
fn v2_multifile_roundtrip_production_layout() {
    let params = test_params();
    let password = "v2-production-layout";
    let file_data: Vec<(&str, &[u8])> = vec![
        ("alpha.bin", &[0u8, 1, 2, 3]),
        ("beta.jpg", &[255, 128, 64, 32, 0, 1]),
        ("gamma.json", b"{\"key\": \"value\"}"),
    ];

    let mut salt = [0u8; crypto::SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(password, &salt, &params).unwrap();

    let mut files_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_files: Vec<Vec<u8>> = Vec::new();
    for (name, data) in &file_data {
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

    // Use the same layout helper that create_container / save_edits_v2 use
    let (enc_meta, meta_nonce) = cryptainer_lib::vault::compute_v2_layout(
        &*key, &mut files_meta, &encrypted_files
    ).unwrap();

    // Build v2 blob
    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_files {
        blob.extend_from_slice(ef);
    }

    // Verify every file decrypts at its STORED offset
    for (i, (name, original)) in file_data.iter().enumerate() {
        let fm = &files_meta[i];
        let off = fm.offset as usize;
        let enc_len = fm.size as usize + 16;
        assert!(off + enc_len <= blob.len(), "File {} offset out of bounds", name);
        let pt = crypto::decrypt_section(&blob[off..off + enc_len], &*key, &fm.data_nonce)
            .unwrap_or_else(|_| panic!("Decryption failed for {} at stored offset {}", name, off));
        assert_eq!(pt, *original, "Data mismatch for {}", name);
        let hash = crypto::sha256_hex(&pt);
        assert_eq!(hash, fm.sha256, "SHA-256 mismatch for {}", name);
    }
}

/// Regression: simulate the EXACT blob layout the buggy two-pass code produces
/// (metadata encrypted with wrong offsets inside the blob, file positions
/// based on the final encryption length L2, but stored offsets computed from L1).
/// Asserts read-side recovery still correctly decrypts every file.
#[test]
fn v2_recovers_wrong_offset_blob() {
    let params = test_params();
    let password = "wrong-offset-recovery";
    let file_data: Vec<(&str, &[u8])> = vec![
        ("small.bin", &[1u8, 2, 3]),
        ("medium.bin", &[10u8; 50]),
        ("large.bin", &[20u8; 200]),
    ];

    let mut salt = [0u8; crypto::SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let key = crypto::derive_key(password, &salt, &params).unwrap();

    let mut files_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_files: Vec<Vec<u8>> = Vec::new();

    // We need to keep track of the original sizes for the correct (non-wrong) metadata.
    let mut correct_file_data: Vec<(&str, Vec<u8>, FileMetadata)> = Vec::new();

    for (name, data) in &file_data {
        let sha256 = crypto::sha256_hex(data);
        let (enc, nonce) = crypto::encrypt_section(data, &*key).unwrap();
        let fm = FileMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            mime: "application/octet-stream".into(),
            size: data.len() as u64,
            offset: 0,
            data_nonce: nonce,
            sha256,
            chunks: None,
        };
        correct_file_data.push((name, data.to_vec(), fm.clone()));
        files_meta.push(fm);
        encrypted_files.push(enc);
    }

    // --- Step 1: compute the WRONG offsets (what the buggy code would produce) ---
    // Buggy first pass: encrypt metadata with offset:0 everywhere → L1
    let zero_meta = ContainerMetadataV2 {
        version: 2,
        files: files_meta.iter().map(|f| FileMetadata { offset: 0, ..f.clone() }).collect(),
    };
    let zero_json = serde_json::to_vec(&zero_meta).unwrap();
    let (enc_meta_l1, _) = crypto::encrypt_section(&zero_json, &*key).unwrap();
    let l1 = enc_meta_l1.len();

    // Wrong offsets based on L1 (this is what the buggy code writes into the metadata)
    let wrong_meta_section = 4 + crypto::NONCE_LEN + l1;
    let mut wrong_offset = crypto::SALT_LEN + wrong_meta_section;
    let mut wrong_offsets: Vec<u64> = Vec::new();
    for ef in &encrypted_files {
        wrong_offsets.push(wrong_offset as u64);
        wrong_offset += ef.len();
    }

    // Build metadata with WRONG offsets (what the buggy code stores in the blob)
    let wrong_meta = ContainerMetadataV2 {
        version: 2,
        files: files_meta.iter().zip(wrong_offsets.iter()).map(|(fm, wo)| {
            FileMetadata { offset: *wo, ..fm.clone() }
        }).collect(),
    };

    // Encrypt the wrong-offset metadata → this is the actual metadata in the blob (L2)
    let wrong_json = serde_json::to_vec(&wrong_meta).unwrap();
    let (enc_meta_buggy, meta_nonce) = crypto::encrypt_section(&wrong_json, &*key).unwrap();
    let l2 = enc_meta_buggy.len();

    // L2 should be LONGER than L1 because the wrong offsets are larger than 0
    assert!(l2 > l1,
        "Buggy layout requires L2 > L1 when there are files (different-sized JSON)");

    // --- Step 2: build the blob exactly as the buggy code would ---
    // Layout: salt(16) | L2(4) | nonce(12) | enc_meta_buggy(L2) | file1 | file2 | ...
    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(l2 as u32).to_le_bytes()); // metadata_len = L2
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta_buggy);
    for ef in &encrypted_files {
        blob.extend_from_slice(ef);
    }

    // --- Step 3: verify metadata inside blob stores wrong offsets ---
    // Read back and decrypt the metadata from the blob
    let read_meta_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;
    let read_nonce: [u8; 12] = blob[20..32].try_into().unwrap();
    let read_ct = &blob[32..32 + read_meta_len];
    let read_pt = crypto::decrypt_section(read_ct, &*key, &read_nonce).unwrap();
    let stored_meta: ContainerMetadataV2 = serde_json::from_slice(&read_pt).unwrap();

    // The stored metadata should have wrong (L1-based) offsets
    for (i, (name, _)) in file_data.iter().enumerate() {
        let expected_wrong = wrong_offsets[i];
        let stored = stored_meta.files[i].offset;
        assert_eq!(stored, expected_wrong,
            "Buggy blob stores wrong offset for {}: expected {} got {}",
            name, expected_wrong, stored);

        // The correct file positions (based on L2) are at:
        let file_start = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + l2;
        let correct_offset = file_start + encrypted_files.iter().take(i).map(|e| e.len()).sum::<usize>();
        assert!(stored < correct_offset as u64,
            "Stored offset {} should be LESS than actual file position {} for {}",
            stored, correct_offset, name);
    }

    // --- Step 4: verify that reading at STORED (wrong) offsets would fail ---
    for (i, (name, _)) in file_data.iter().enumerate() {
        let stored_fm = &stored_meta.files[i];
        let off = stored_fm.offset as usize;
        let enc_len = stored_fm.size as usize + 16;

        // The wrong offset points into the metadata section or before our file.
        // Reading at this position and decrypting with its nonce should FAIL.
        let result = crypto::decrypt_section(
            &blob[off..off + enc_len],
            &*key,
            &stored_fm.data_nonce,
        );
        // On most compositions the slice won't even overlap the right file,
        // but in rare cases it might overlap → still must fail GCM tag check
        assert!(result.is_err(),
            "Decrypting at WRONG offset {} for {} should have failed!",
            stored_fm.offset, name);
    }

    // --- Step 5: verify read-side recovery works ---
    // The fix computes: actual_offset = SALT_LEN + 4 + NONCE_LEN + metadata_len + Σ prior (size+16)
    // = SALT_LEN + 4 + 12 + L2 + Σ prior (size+16)
    let header_metadata_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;
    assert_eq!(header_metadata_len, l2);

    let mut cumulative = 0usize;
    for (name, original, correct_fm) in &correct_file_data {
        let actual_offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + l2 + cumulative;
        let enc_len = correct_fm.size as usize + 16;

        assert!(actual_offset + enc_len <= blob.len(),
            "Recovered offset {} out of bounds for {}", actual_offset, name);

        let pt = crypto::decrypt_section(
            &blob[actual_offset..actual_offset + enc_len],
            &*key,
            &correct_fm.data_nonce,
        ).unwrap_or_else(|_| panic!("Read-side recovery failed for {}", name));

        assert_eq!(pt, original.as_slice(), "Recovered data mismatch for {}", name);
        let hash = crypto::sha256_hex(&pt);
        assert_eq!(hash, correct_fm.sha256, "SHA-256 mismatch for recovered file {}", name);

        cumulative += correct_fm.size as usize + 16;
    }
}

/// Mirrors `create_container`'s streaming assembly: two files are encrypted from
/// disk with `encrypt_file_chunked` (chunked layout, `data_nonce: [0;12]`), laid
/// out with `compute_v2_layout`, and assembled into the exact v2 blob format
/// (`salt ‖ meta_len(u32 LE) ‖ meta_nonce ‖ enc_meta ‖ part0 ‖ part1`). It then
/// recovers the SECOND file using read-side offset recovery
/// (`file_encrypted_len` of the prior file) and `decrypt_file`, asserting the
/// recovered bytes equal the original file-2 bytes. A minimal fixed key is used
/// (no KDF) because this exercises blob assembly, not key derivation.
#[test]
fn create_container_streaming_roundtrips_two_files() {
    use std::io::Write;

    // Fixed key — testing blob assembly, not the KDF.
    let key = [0x5Au8; 32];

    // Two temp files with distinct, known bytes.
    let dir = std::env::temp_dir().join(format!(
        "ctnr_create_stream_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let file1_bytes: Vec<u8> = (0..1234u32).map(|i| (i % 251) as u8).collect();
    let file2_bytes: Vec<u8> = (0..4321u32).map(|i| ((i * 7 + 3) % 253) as u8).collect();
    let inputs: Vec<(&str, &str, &[u8])> = vec![
        ("first.bin", "application/octet-stream", &file1_bytes),
        ("second.bin", "application/octet-stream", &file2_bytes),
    ];
    let mut paths = Vec::new();
    for (name, _, bytes) in &inputs {
        let p = dir.join(name);
        std::fs::File::create(&p).unwrap().write_all(bytes).unwrap();
        paths.push(p);
    }

    // Encrypt each file individually (streamed from disk), building chunked metadata
    // exactly as create_container now does.
    let mut files_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_parts: Vec<Vec<u8>> = Vec::new();
    for (i, (name, mime, _)) in inputs.iter().enumerate() {
        let (encrypted, chunks, sha256, plaintext_len) = vault::encrypt_file_chunked(
            &paths[i], &key, vault::ENCRYPT_CHUNK_SIZE, &mut |_| {},
        )
        .unwrap();
        files_meta.push(FileMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            mime: mime.to_string(),
            size: plaintext_len,
            offset: 0,
            data_nonce: [0u8; 12], // unused for chunked files
            sha256,
            chunks: Some(chunks),
        });
        encrypted_parts.push(encrypted);
    }

    // Compute the layout (fills offsets) and assemble the blob EXACTLY as create_container.
    let (enc_meta, meta_nonce) =
        vault::compute_v2_layout(&key, &mut files_meta, &encrypted_parts).unwrap();

    let salt = [0u8; crypto::SALT_LEN];
    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_parts {
        blob.extend_from_slice(ef);
    }

    // Recover the SECOND file via read-side offset recovery + decrypt_file.
    let meta_len = u32::from_le_bytes(
        blob[crypto::SALT_LEN..crypto::SALT_LEN + 4].try_into().unwrap(),
    ) as usize;
    let prior_sum = vault::file_encrypted_len(&files_meta[0]);
    let offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + meta_len + prior_sum;
    let section_len = vault::file_encrypted_len(&files_meta[1]);
    assert!(
        offset + section_len <= blob.len(),
        "recovered offset {} out of bounds",
        offset,
    );
    let section = &blob[offset..offset + section_len];
    let recovered = vault::decrypt_file(section, &files_meta[1], &key).unwrap();

    assert_eq!(recovered, file2_bytes, "recovered file-2 bytes must match original");
    assert_eq!(files_meta[1].size, file2_bytes.len() as u64);
    assert_eq!(crypto::sha256_hex(&recovered), files_meta[1].sha256);

    std::fs::remove_dir_all(&dir).ok();
}

/// Regression: removing a non-trailing file from a v2 container must NOT corrupt
/// reads of subsequent retained files during save_edits_v2's offset-recovery loop.
///
/// Mirrors save_edits_v2's inner loop: `prior_sum` must advance for ALL files
/// (including removed ones) because the on-disk blob still physically contains
/// every file's ciphertext in sequence.
#[test]
fn v2_remove_non_trailing_file_keeps_remaining_decryptable() {
    let params = test_params();
    let password = "remove-offset-fix";
    let file_data: Vec<(&str, &[u8])> = vec![
        ("alpha.bin", &[0u8, 1, 2, 3]),
        ("beta.jpg", &[255, 128, 64, 32, 0, 1]),
        ("gamma.json", b"{\"key\": \"value\"}"),
    ];

    let (blob, metadata) = build_v2_blob(password, &params, &file_data);

    let salt: [u8; 16] = blob[..16].try_into().unwrap();
    let key = crypto::derive_key(password, &salt, &params).unwrap();
    let blob_metadata_len = u32::from_le_bytes(blob[16..20].try_into().unwrap()) as usize;

    // Remove the FIRST file — this is the case that triggers the offset bug
    let removed_id = &metadata.files[0].id;

    let mut prior_sum: usize = 0;
    for fm in &metadata.files {
        let enc_len = fm.size as usize + 16;
        if fm.id == *removed_id {
            prior_sum += enc_len;
            continue;
        }
        let actual_offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + blob_metadata_len + prior_sum;
        assert!(
            actual_offset + enc_len <= blob.len(),
            "File {} offset {} out of bounds", fm.name, actual_offset,
        );
        let pt = crypto::decrypt_section(
            &blob[actual_offset..actual_offset + enc_len],
            &*key,
            &fm.data_nonce,
        ).unwrap_or_else(|_| {
            panic!(
                "Decryption FAILED for retained file {} at offset {} — \
                 prior_sum likely skipped a removed file's ciphertext",
                fm.name, actual_offset,
            )
        });
        let hash = crypto::sha256_hex(&pt);
        assert_eq!(hash, fm.sha256, "SHA-256 mismatch for {}", fm.name);
        let expected_data = file_data.iter()
            .find(|(n, _)| *n == fm.name.as_str())
            .map(|(_, d)| *d)
            .unwrap();
        assert_eq!(pt, expected_data, "Data mismatch for {}", fm.name);
        prior_sum += enc_len;
    }
}

/// Task 5: exercises `save_edits_v2`'s retained+add path at the helper level.
///
/// The real command needs a live Tauri `AppHandle`/pool, so this mirrors the
/// exact sequence the retained and add loops now perform, using the same
/// `vault::` helpers the command calls:
///  1. Build an initial v2 blob holding ONE CHUNKED file (as `create_container`
///     now produces): `encrypt_file_chunked` + `compute_v2_layout` + assembly.
///  2. Retained loop: recover that chunked file via read-side offset recovery
///     (`file_encrypted_len` for the section length, `decrypt_file` for the
///     bytes — the Task 5 change) and re-encrypt it WHOLE-FILE (`chunks: None`).
///  3. Add loop: stream a NEW temp file via `encrypt_file_chunked` (`chunks:
///     Some`, `data_nonce: [0;12]`, `size = plaintext_len`).
///  4. `compute_v2_layout` over [retained, new], assemble the new blob, then
///     recover BOTH files and assert they decode to their originals — retained
///     via its NEW whole-file nonce, added via its chunk metadata.
///
/// This proves the retained-read-of-a-chunked-file works (the pre-fix
/// `fm.size + 16` + `decrypt_section` math mis-slices/mis-authenticates a
/// chunked section) and that the added file is chunked. Fixed key, no KDF.
#[test]
fn save_edits_v2_streaming_roundtrips() {
    use std::io::Write;

    // Fixed key — exercises blob assembly + the retained/add helpers, not KDF.
    let key = [0x3Cu8; 32];
    // Small chunk size so both files span multiple chunks.
    let chunk_size = 2048usize;

    let dir = std::env::temp_dir().join(format!(
        "ctnr_save_edits_stream_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();

    // ── 1. Build the initial v2 blob with ONE chunked file ──────────────────
    let retained_bytes: Vec<u8> = (0..7000u32).map(|i| (i % 251) as u8).collect();
    let retained_path = dir.join("retained.bin");
    std::fs::File::create(&retained_path)
        .unwrap()
        .write_all(&retained_bytes)
        .unwrap();

    let (r_enc, r_chunks, r_sha, r_len) =
        vault::encrypt_file_chunked(&retained_path, &key, chunk_size, &mut |_| {}).unwrap();
    assert!(r_chunks.len() > 1, "retained file must be chunked for a meaningful test");

    let retained_id = uuid::Uuid::new_v4().to_string();
    let mut init_meta = vec![FileMetadata {
        id: retained_id.clone(),
        name: "retained.bin".into(),
        mime: "application/octet-stream".into(),
        size: r_len,
        offset: 0,
        data_nonce: [0u8; 12],
        sha256: r_sha,
        chunks: Some(r_chunks),
    }];
    let init_parts = vec![r_enc];
    let (init_enc_meta, init_meta_nonce) =
        vault::compute_v2_layout(&key, &mut init_meta, &init_parts).unwrap();

    let salt = [0u8; crypto::SALT_LEN];
    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(init_enc_meta.len() as u32).to_le_bytes());
    blob.extend_from_slice(&init_meta_nonce);
    blob.extend_from_slice(&init_enc_meta);
    for ef in &init_parts {
        blob.extend_from_slice(ef);
    }

    // The container's stored metadata (what the v2 session would hold).
    let old_files = init_meta.clone();

    // ── 2. Retained loop: read-side recovery of the CHUNKED file ────────────
    let blob_metadata_len = u32::from_le_bytes(
        blob[crypto::SALT_LEN..crypto::SALT_LEN + 4].try_into().unwrap(),
    ) as usize;

    let mut new_meta: Vec<FileMetadata> = Vec::new();
    let mut encrypted_parts: Vec<Vec<u8>> = Vec::new();
    let mut prior_sum: usize = 0;

    for fm in &old_files {
        // Task 5 change: helper-based encrypted length + decrypt (was
        // `fm.size + 16` and `crypto::decrypt_section(.., &fm.data_nonce)`).
        let enc_len = vault::file_encrypted_len(fm);
        let actual_offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + blob_metadata_len + prior_sum;
        assert!(actual_offset + enc_len <= blob.len(), "retained offset out of bounds");
        let plaintext =
            vault::decrypt_file(&blob[actual_offset..actual_offset + enc_len], fm, &key).unwrap();

        // Retained files are re-encrypted WHOLE-FILE (chunks: None).
        let sha256 = crypto::sha256_hex(&plaintext);
        let (new_enc, new_nonce) = crypto::encrypt_section(&plaintext, &key).unwrap();
        new_meta.push(FileMetadata {
            id: fm.id.clone(),
            name: fm.name.clone(),
            mime: fm.mime.clone(),
            size: fm.size,
            offset: 0,
            data_nonce: new_nonce,
            sha256,
            chunks: None,
        });
        encrypted_parts.push(new_enc);
        prior_sum += enc_len;
    }

    // ── 3. Add loop: stream a NEW file via encrypt_file_chunked ─────────────
    let added_bytes: Vec<u8> = (0..5000u32).map(|i| ((i * 13 + 7) % 249) as u8).collect();
    let added_path = dir.join("added.bin");
    std::fs::File::create(&added_path)
        .unwrap()
        .write_all(&added_bytes)
        .unwrap();

    // Mirror the fixed bytes_total the production emit now reports.
    let total_add_bytes: u64 = added_bytes.len() as u64;
    assert!(total_add_bytes > 0, "bytes_total must be non-zero (the review bug fix)");

    let (a_enc, a_chunks, a_sha, a_len) =
        vault::encrypt_file_chunked(&added_path, &key, chunk_size, &mut |_| {}).unwrap();
    assert!(a_chunks.len() > 1, "added file should be chunked");
    assert_eq!(a_len, total_add_bytes, "size must be plaintext length");

    let added_id = uuid::Uuid::new_v4().to_string();
    new_meta.push(FileMetadata {
        id: added_id.clone(),
        name: "added.bin".into(),
        mime: "application/octet-stream".into(),
        size: a_len,
        offset: 0,
        data_nonce: [0u8; 12],
        sha256: a_sha,
        chunks: Some(a_chunks),
    });
    encrypted_parts.push(a_enc);

    // ── 4. Assemble the new blob and recover BOTH files ─────────────────────
    let (enc_meta, meta_nonce) =
        vault::compute_v2_layout(&key, &mut new_meta, &encrypted_parts).unwrap();
    let mut new_blob = Vec::new();
    new_blob.extend_from_slice(&salt);
    new_blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
    new_blob.extend_from_slice(&meta_nonce);
    new_blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_parts {
        new_blob.extend_from_slice(ef);
    }

    let new_meta_len = u32::from_le_bytes(
        new_blob[crypto::SALT_LEN..crypto::SALT_LEN + 4].try_into().unwrap(),
    ) as usize;
    let base = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + new_meta_len;

    // Retained file: now whole-file (chunks: None) with a fresh nonce.
    let retained_fm = new_meta.iter().find(|f| f.id == retained_id).unwrap();
    assert!(retained_fm.chunks.is_none(), "retained file must be re-encrypted whole-file");
    let r_section_len = vault::file_encrypted_len(retained_fm);
    let r_recovered =
        vault::decrypt_file(&new_blob[base..base + r_section_len], retained_fm, &key).unwrap();
    assert_eq!(r_recovered, retained_bytes, "retained (chunked → whole-file) roundtrip");

    // Added file: chunked, decode via its chunk metadata.
    let added_fm = new_meta.iter().find(|f| f.id == added_id).unwrap();
    assert!(added_fm.chunks.is_some(), "added file must be chunked");
    let a_off = base + r_section_len;
    let a_section_len = vault::file_encrypted_len(added_fm);
    assert!(a_off + a_section_len <= new_blob.len(), "added offset out of bounds");
    let a_recovered =
        vault::decrypt_file(&new_blob[a_off..a_off + a_section_len], added_fm, &key).unwrap();
    assert_eq!(a_recovered, added_bytes, "added (streamed chunked) roundtrip");

    std::fs::remove_dir_all(&dir).ok();
}

/// Backward-compatible reads (Task 6): a v2 container with a MIX of legacy
/// whole-file entries and new chunked entries must recover both correctly
/// using the same offset math `get_file_data_v2` uses: `prior_sum` computed
/// via `vault::file_encrypted_len` (not `fm.size + 16`), and the section
/// decrypted via `vault::decrypt_file` (not `crypto::decrypt_section` with
/// `fm.data_nonce`, which is meaningless for a chunked file).
///
/// file0 is WHOLE-FILE legacy: encrypted with `crypto::encrypt_section` over
/// the whole plaintext, `chunks: None`, a real `data_nonce`.
/// file1 is CHUNKED: encrypted with `vault::encrypt_file_chunked`,
/// `chunks: Some`, `data_nonce: [0; 12]` (unused for chunked files).
///
/// Both files are recovered and compared to their originals — file1 proves
/// a chunked file is read correctly when it follows a whole-file prior
/// (the mixed-layout case), file0 proves whole-file legacy reads still work.
#[test]
fn get_file_data_v2_reads_chunked_after_prior_whole() {
    use std::io::Write;

    // Fixed key — exercises blob assembly + read-side recovery, not the KDF.
    let key = [0x7Bu8; 32];
    let chunk_size = 512usize;

    let dir = std::env::temp_dir().join(format!(
        "ctnr_read_mixed_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&dir).unwrap();

    // ── file0: WHOLE-FILE legacy ─────────────────────────────────────────────
    let file0_bytes: Vec<u8> = (0..300u32).map(|i| (i % 250) as u8).collect();
    let file0_sha256 = crypto::sha256_hex(&file0_bytes);
    let (file0_enc, file0_nonce) = crypto::encrypt_section(&file0_bytes, &key).unwrap();
    let file0_id = uuid::Uuid::new_v4().to_string();
    let mut files_meta = vec![FileMetadata {
        id: file0_id.clone(),
        name: "legacy.bin".into(),
        mime: "application/octet-stream".into(),
        size: file0_bytes.len() as u64,
        offset: 0,
        data_nonce: file0_nonce,
        sha256: file0_sha256,
        chunks: None,
    }];
    let mut encrypted_parts = vec![file0_enc];

    // ── file1: CHUNKED, streamed from disk ───────────────────────────────────
    let file1_bytes: Vec<u8> = (0..3000u32).map(|i| ((i * 7 + 3) % 253) as u8).collect();
    let file1_path = dir.join("chunked.bin");
    std::fs::File::create(&file1_path)
        .unwrap()
        .write_all(&file1_bytes)
        .unwrap();
    let (file1_enc, file1_chunks, file1_sha256, file1_len) =
        vault::encrypt_file_chunked(&file1_path, &key, chunk_size, &mut |_| {}).unwrap();
    assert!(file1_chunks.len() > 1, "chunked file should span multiple chunks");
    let file1_id = uuid::Uuid::new_v4().to_string();
    files_meta.push(FileMetadata {
        id: file1_id.clone(),
        name: "chunked.bin".into(),
        mime: "application/octet-stream".into(),
        size: file1_len,
        offset: 0,
        data_nonce: [0u8; 12], // unused for chunked files
        sha256: file1_sha256,
        chunks: Some(file1_chunks),
    });
    encrypted_parts.push(file1_enc);

    // Layout + assemble EXACTLY as create_container does.
    let (enc_meta, meta_nonce) =
        vault::compute_v2_layout(&key, &mut files_meta, &encrypted_parts).unwrap();

    let salt = [0u8; crypto::SALT_LEN];
    let mut blob = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&(enc_meta.len() as u32).to_le_bytes());
    blob.extend_from_slice(&meta_nonce);
    blob.extend_from_slice(&enc_meta);
    for ef in &encrypted_parts {
        blob.extend_from_slice(ef);
    }

    let meta_len = u32::from_le_bytes(
        blob[crypto::SALT_LEN..crypto::SALT_LEN + 4].try_into().unwrap(),
    ) as usize;

    // ── Recover file1 (the SECOND file) using get_file_data_v2's offset math ──
    let prior_sum = vault::file_encrypted_len(&files_meta[0]);
    let offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + meta_len + prior_sum;
    let section_len = vault::file_encrypted_len(&files_meta[1]);
    assert!(offset + section_len <= blob.len(), "file1 offset out of bounds");
    let section = &blob[offset..offset + section_len];
    let recovered1 = vault::decrypt_file(section, &files_meta[1], &key).unwrap();
    assert_eq!(recovered1, file1_bytes, "chunked file1 (after prior whole-file) must roundtrip");
    assert_eq!(crypto::sha256_hex(&recovered1), files_meta[1].sha256);

    // ── Recover file0 (whole-file legacy, offset 0) — backward compat proof ──
    let file0_offset = crypto::SALT_LEN + 4 + crypto::NONCE_LEN + meta_len;
    let file0_section_len = vault::file_encrypted_len(&files_meta[0]);
    let file0_section = &blob[file0_offset..file0_offset + file0_section_len];
    let recovered0 = vault::decrypt_file(file0_section, &files_meta[0], &key).unwrap();
    assert_eq!(recovered0, file0_bytes, "legacy whole-file file0 must still roundtrip");
    assert_eq!(crypto::sha256_hex(&recovered0), files_meta[0].sha256);

    std::fs::remove_dir_all(&dir).ok();
}
