# Streaming Encryption via File Paths — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make encryption show smooth, real-time progress and run fast for large files by reading files from disk paths in the Rust backend and encrypting them chunk-by-chunk, instead of shipping raw bytes across the Tauri IPC bridge.

**Architecture:** The frontend stops reading file bytes (`Array.from(new Uint8Array(...))`) and instead passes filesystem **paths** picked via the Tauri native dialog. The backend reads each file in ~2 MB chunks, seals each chunk with AES-256-GCM (its own random nonce), records per-chunk metadata in the existing `FileMetadata.chunks` field, and emits byte-level progress after each chunk. Reads become backward-compatible: whole-file containers (`chunks: None`) and new chunked containers (`chunks: Some`) are both decoded through one helper.

**Tech Stack:** Rust (Tauri 2, `aes-gcm`, `sha2`), TypeScript/React, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, Vitest.

## Global Constraints

- On-disk format stays **v2**; no format-version bump. Chunking is expressed entirely through the already-present `FileMetadata.chunks: Option<Vec<ChunkMetadata>>` field.
- Existing containers (files stored as one whole-file GCM section, `chunks: None`) MUST remain readable unchanged.
- AES-256-GCM only, 12-byte nonces (`crypto::NONCE_LEN`), 16-byte tags. Each chunk gets a fresh random nonce from `OsRng`.
- Chunk size constant: `2 * 1024 * 1024` (2 MB), matching the existing `vault::VIDEO_CHUNK_SIZE`.
- Native file picking is Tauri-only. Browser/mock mode (`!isTauri()`) keeps its existing mock create path and is not expected to pick real paths.
- Integrity model: chunk order/offset/nonce/size live inside the GCM-sealed metadata section; the whole-file plaintext SHA-256 (`FileMetadata.sha256`) is verified on read. No AEAD chaining (STREAM) is used — truncation/reorder is caught by the SHA-256 check.
- TDD: every behavioral change lands with a failing test first. Rust tests via `cargo test`, frontend via `npx vitest run`.

---

## File Structure

**Rust (`src-tauri/src/`)**
- `vault.rs` — home of the new pure, testable helpers: `ENCRYPT_CHUNK_SIZE`, `file_encrypted_len`, `decrypt_file`, `encrypt_file_chunked`. (vault.rs already owns `FileMetadata`, `ChunkMetadata`, `compute_v2_layout`, and calls into `crypto`.)
- `crypto.rs` — unchanged (keeps low-level `encrypt_section`/`decrypt_section`).
- `commands.rs` — `FileInput` becomes path-based; `create_container` and `save_edits_v2` read+chunk-encrypt with progress; `get_file_data_v2` and the `save_edits_v2` retained-file loop route through the helpers; fix `bytes_total`/index bugs.

**Frontend (`src/`)**
- `utils/pathinfo.ts` — new: `basename(path)` and `guessMime(path)`.
- `types/vault.ts` — `FileInput` becomes `{ path; name; mime; size }`.
- `hooks/usePickFiles.ts` — new: opens the Tauri dialog, stats sizes, returns `FileInput[]`.
- `components/Container/CreateWizard/Step1Files.tsx` — use the picker instead of `DropZone`.
- `components/Container/ContainerModal/EditView.tsx` — use the picker for "Add new files".
- `components/Container/CreateWizard/index.tsx` — drop `Array.from`; pass paths.
- `components/Container/ContainerModal/index.tsx` — pass path-based `FileInput` to `saveContainerEdits`.
- `store/vaultStore.ts` — types only; invoke shape unchanged.
- `components/UI/ProgressBar.tsx` — percent becomes byte-weighted when `bytesTotal > 0` (this is what actually makes the bar smooth).

---

## Task 1: `file_encrypted_len` helper (Rust)

Encrypted on-disk length of a file's data section. This value is used by the read-side offset recovery; it must account for one GCM tag **per chunk**.

**Files:**
- Modify: `src-tauri/src/vault.rs` (add near `ChunkMetadata`, ~line 76)
- Test: `src-tauri/src/vault.rs` (in the existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `pub fn file_encrypted_len(fm: &FileMetadata) -> usize`

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test file_encrypted_len_whole_vs_chunked`
Expected: FAIL — `cannot find function file_encrypted_len`.

- [ ] **Step 3: Write minimal implementation**

```rust
/// On-disk encrypted length of a file's data section (ciphertext + GCM tags).
/// Whole-file layout: plaintext size + one 16-byte tag.
/// Chunked layout: sum over chunks of (chunk size + 16-byte tag).
pub fn file_encrypted_len(fm: &FileMetadata) -> usize {
    match &fm.chunks {
        Some(chunks) => chunks.iter().map(|c| c.size as usize + 16).sum(),
        None => fm.size as usize + 16,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test file_encrypted_len_whole_vs_chunked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault.rs
git commit -m "feat: add file_encrypted_len helper for chunked layout"
```

---

## Task 2: `decrypt_file` helper (Rust)

Decrypt a file's already-read data section, handling both layouts. Chunk offsets are **relative to the start of the file's data section**.

**Files:**
- Modify: `src-tauri/src/vault.rs`
- Test: `src-tauri/src/vault.rs` tests

**Interfaces:**
- Consumes: `crypto::encrypt_section`, `crypto::decrypt_section`, `crypto::KEY_LEN`
- Produces: `pub fn decrypt_file(section: &[u8], fm: &FileMetadata, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError>`

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test decrypt_file_handles_chunked_and_whole`
Expected: FAIL — `cannot find function decrypt_file`.

- [ ] **Step 3: Write minimal implementation**

```rust
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
                let end = start + c.size as usize + 16;
                if end > section.len() {
                    return Err(CryptoError::IntegrityFailure);
                }
                out.extend_from_slice(&crypto::decrypt_section(&section[start..end], key, &c.nonce)?);
            }
            Ok(out)
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test decrypt_file_handles_chunked_and_whole`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault.rs
git commit -m "feat: add decrypt_file helper for whole + chunked layouts"
```

---

## Task 3: `encrypt_file_chunked` streaming encrypt (Rust)

Read a file from a path in chunks, seal each chunk, hash the plaintext incrementally, and report progress after each chunk. Produces the concatenated ciphertext (plugs into `encrypted_parts`/`compute_v2_layout` exactly like a whole-file ciphertext) plus chunk metadata.

**Files:**
- Modify: `src-tauri/src/vault.rs` (add `use sha2::{Sha256, Digest};` and `use std::io::Read;` as needed)
- Test: `src-tauri/src/vault.rs` tests

**Interfaces:**
- Produces:
  - `pub const ENCRYPT_CHUNK_SIZE: usize = 2 * 1024 * 1024;`
  - `pub fn encrypt_file_chunked(path: &std::path::Path, key: &[u8; 32], chunk_size: usize, progress: &mut dyn FnMut(u64)) -> Result<(Vec<u8>, Vec<ChunkMetadata>, String, u64), CryptoError>`
  - Return tuple: `(concatenated_ciphertext, chunks, sha256_hex, plaintext_len)`.

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test encrypt_file_chunked_roundtrips_and_reports_progress`
Expected: FAIL — `cannot find function encrypt_file_chunked`.

- [ ] **Step 3: Write minimal implementation**

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test encrypt_file_chunked_roundtrips_and_reports_progress`
Expected: PASS. (If `hex`/`sha2` imports are missing in vault.rs, add `use sha2::{Sha256, Digest};` at the top; `hex` is already a workspace dep used by `crypto::sha256_hex`.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/vault.rs
git commit -m "feat: add encrypt_file_chunked streaming encryptor with progress"
```

---

## Task 4: Path-based `FileInput` + `create_container` streaming (Rust)

Switch `FileInput` from carrying bytes to carrying a path, and rewrite the `create_container` encrypt loop to stream from disk with byte progress.

**Files:**
- Modify: `src-tauri/src/commands.rs:39-43` (`FileInput`), `:130-172` (encrypt loop)
- Test: `src-tauri/tests/` integration test (or a `#[cfg(test)]` unit in commands.rs if the harness allows creating a pool; prefer the existing integration test style — see `build_v2_blob`).

**Interfaces:**
- Consumes: `vault::encrypt_file_chunked`, `vault::ENCRYPT_CHUNK_SIZE`, `vault::file_encrypted_len`, `vault::decrypt_file`
- Produces (new `FileInput` shape — both Rust and TS must match):
  ```rust
  pub struct FileInput { pub path: String, pub name: String, pub mime: String, pub size: u64 }
  ```

- [ ] **Step 1: Write the failing test**

Add an integration test that creates two temp files, calls the create path, and asserts the resulting blob decodes both files. If the project has no command-level harness, assert at the helper level instead: build `encrypted_parts` via `encrypt_file_chunked`, run `vault::compute_v2_layout`, assemble a blob exactly as `create_container` does, then read file 2 back using `file_encrypted_len` for `prior_sum` and `decrypt_file`. Name it `create_container_streaming_roundtrips_two_files`.

```rust
// Skeleton (mirrors create_container assembly): encrypt two temp files,
// compute layout, assemble blob, then recover + decrypt the SECOND file
// using file_encrypted_len(prior) for the offset and decrypt_file.
// Assert recovered bytes == original file-2 bytes.
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test create_container_streaming_roundtrips_two_files`
Expected: FAIL (compile error on new `FileInput` fields / helper wiring).

- [ ] **Step 3: Change `FileInput` and the encrypt loop**

Replace `FileInput`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInput {
    pub path: String,
    pub name: String,
    pub mime: String,
    pub size: u64,
}
```

Replace the `create_container` encrypt section (`:130-172`). `total_size` now comes from the inputs' `size`; each file is streamed:

```rust
// 2. Encrypt each file individually (streamed from disk), collect metadata.
let total_size: u64 = input.files.iter().map(|f| f.size).sum();
let total_files = input.files.len() as u64;
let mut files_meta: Vec<FileMetadata> = Vec::with_capacity(input.files.len());
let mut encrypted_files: Vec<Vec<u8>> = Vec::with_capacity(input.files.len());
let mut cumulative_bytes: u64 = 0;

for (i, f) in input.files.iter().enumerate() {
    let base = cumulative_bytes;
    let mut emit = |done_in_file: u64| {
        emit_progress(&app, ProgressPayload {
            operation: "encrypt".into(),
            current: i as u64,
            total: total_files,
            file_name: Some(f.name.clone()),
            bytes_processed: base + done_in_file,
            bytes_total: total_size,
            message: format!("Encrypting {} ({} / {})", f.name, i + 1, total_files),
        });
    };
    emit(0); // show the file starting before the first chunk
    let (encrypted, chunks, sha256, plaintext_len) = vault::encrypt_file_chunked(
        std::path::Path::new(&f.path), &key, vault::ENCRYPT_CHUNK_SIZE, &mut emit,
    )?;
    files_meta.push(FileMetadata {
        id: Uuid::new_v4().to_string(),
        name: f.name.clone(),
        mime: f.mime.clone(),
        size: plaintext_len,
        offset: 0,
        data_nonce: [0u8; 12], // unused for chunked files
        sha256,
        chunks: Some(chunks),
    });
    encrypted_files.push(encrypted);
    cumulative_bytes += plaintext_len;
}

emit_progress(&app, ProgressPayload {
    operation: "encrypt".into(),
    current: total_files, total: total_files, file_name: None,
    bytes_processed: total_size, bytes_total: total_size,
    message: "Encryption complete".into(),
});
```

(Downstream `compute_v2_layout` + blob assembly at `:174-193` are unchanged — `encrypted_files[i]` is still one `Vec<u8>` per file.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test create_container_streaming_roundtrips_two_files`
Expected: PASS. Then `cd src-tauri && cargo test` — the whole suite compiles and passes.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/tests
git commit -m "feat: create_container streams files from paths with byte progress"
```

---

## Task 5: `save_edits_v2` — stream new files, fix retained loop + bugs (Rust)

New files added on edit stream from paths (chunked + progress). Retained files route through the helpers. Fix `bytes_total: 0` and the 0-index/1-index mismatch surfaced in the review.

**Files:**
- Modify: `src-tauri/src/commands.rs:790-870` (retained loop + new-files loop)

**Interfaces:**
- Consumes: `vault::encrypt_file_chunked`, `vault::ENCRYPT_CHUNK_SIZE`, `vault::file_encrypted_len`, `vault::decrypt_file`

- [ ] **Step 1: Write the failing test**

Extend Task 4's roundtrip test into `save_edits_v2_streaming_roundtrips` (or add a sibling): build a container with one whole-file legacy file already in a blob, then run the retained+add assembly and assert BOTH the retained legacy file and a newly-added chunked file decode correctly. Assert the emitted progress for the add-loop has `bytes_total == total_add_bytes` (non-zero).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test save_edits_v2_streaming_roundtrips`
Expected: FAIL.

- [ ] **Step 3: Implementation**

Compute the true total for progress and thread cumulative bytes:

```rust
let total_add_bytes: u64 = files_to_add.iter().map(|f| f.size).sum();
```

Retained loop — replace the hardcoded `enc_len`/decrypt (`:805`, `:824`) with helpers:

```rust
let enc_len = vault::file_encrypted_len(fm);   // was: fm.size as usize + 16
// ...
let actual_offset = SALT_LEN + 4 + crypto::NONCE_LEN + blob_metadata_len + prior_sum;
if actual_offset + enc_len > blob.len() {
    return Err(CryptoError::IntegrityFailure);
}
let plaintext = vault::decrypt_file(&blob[actual_offset..actual_offset + enc_len], fm, &key_arr)?;
```

(Keep the retained files' existing whole-file re-encryption via `crypto::encrypt_section` — they stay `chunks: None` in the new metadata. Only the READ of the old bytes changes.)

New-files loop (`:844-870`) — stream from path with byte progress and fix the index/total:

```rust
let mut add_done: u64 = 0;
for f in &files_to_add {
    let base = add_done;
    let mut emit = |done_in_file: u64| {
        emit_progress(&app, ProgressPayload {
            operation: "encrypt".into(),
            current: progress_idx,
            total: total_ops,
            file_name: Some(f.name.clone()),
            bytes_processed: base + done_in_file, // cumulative added bytes
            bytes_total: total_add_bytes,         // was 0 — the review bug
            message: format!("Encrypting {} ({} / {})", f.name, progress_idx + 1, total_ops),
        });
    };
    emit(0);
    let (enc, chunks, sha256, plaintext_len) = vault::encrypt_file_chunked(
        std::path::Path::new(&f.path), &key_arr, vault::ENCRYPT_CHUNK_SIZE, &mut emit,
    )?;
    new_meta.push(FileMetadata {
        id: Uuid::new_v4().to_string(),
        name: f.name.clone(), mime: f.mime.clone(),
        size: plaintext_len, offset: 0,
        data_nonce: [0u8; 12], sha256, chunks: Some(chunks),
    });
    total_size += plaintext_len;
    add_done += plaintext_len;
    encrypted_parts.push(enc);
    progress_idx += 1;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test save_edits_v2_streaming_roundtrips && cd src-tauri && cargo test`
Expected: PASS (whole suite green).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: save_edits_v2 streams added files, fixes bytes_total and index"
```

---

## Task 6: Backward-compatible reads through the helpers (Rust)

Route `get_file_data_v2` through `file_encrypted_len` + `decrypt_file` so both whole-file (old) and chunked (new) files read correctly. `download_files` delegates to this, so it needs no change.

**Files:**
- Modify: `src-tauri/src/commands.rs:591` (prior_sum), `:615` (enc_len), `:622` (decrypt)
- Verify only (no change expected): `unlock_v2` (`:429`) does not itself compute per-file offsets.

**Interfaces:**
- Consumes: `vault::file_encrypted_len`, `vault::decrypt_file`

- [ ] **Step 1: Write the failing test**

Add `get_file_data_v2_reads_chunked_after_prior_whole` (helper-level, mirroring `get_file_data_v2`'s offset math): build a 2-file blob where file 0 is whole-file legacy and file 1 is chunked; compute `prior_sum` via `file_encrypted_len(file0)`; recover file 1's section and `decrypt_file` it; assert equality. This fails today because the offset math uses `fm.size + 16` for the whole-file prior (correct) but there is no path that yields a chunked file 1 to read — the test drives the helper contract.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test get_file_data_v2_reads_chunked_after_prior_whole`
Expected: FAIL.

- [ ] **Step 3: Implementation**

```rust
// prior_sum (was: .map(|f| f.size as usize + 16))
let prior_sum: usize = session.metadata.files.iter()
    .take(idx)
    .map(vault::file_encrypted_len)
    .sum();
```

```rust
// enc_len (was: let enc_len = fm.size as usize + 16;)
let enc_len = vault::file_encrypted_len(&fm);
```

```rust
// decrypt (was: crypto::decrypt_section(&encrypted, &key_arr, &fm.data_nonce)?)
let plaintext = vault::decrypt_file(&encrypted, &fm, &key_arr)?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test get_file_data_v2_reads_chunked_after_prior_whole && cd src-tauri && cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: route v2 reads through file_encrypted_len + decrypt_file"
```

---

## Task 7: Frontend path-info util (name + mime from a path)

**Files:**
- Create: `src/utils/pathinfo.ts`
- Test: `src/test/pathinfo.test.ts`

**Interfaces:**
- Produces: `export function basename(path: string): string`, `export function guessMime(path: string): string`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from 'vitest';
import { basename, guessMime } from '../utils/pathinfo';

describe('pathinfo', () => {
  it('basename handles unix and windows separators', () => {
    expect(basename('/home/u/a.png')).toBe('a.png');
    expect(basename('C:\\Users\\u\\b.PDF')).toBe('b.PDF');
    expect(basename('noslash.txt')).toBe('noslash.txt');
  });
  it('guessMime maps common extensions, falls back to octet-stream', () => {
    expect(guessMime('/x/a.png')).toBe('image/png');
    expect(guessMime('/x/a.MP4')).toBe('video/mp4');
    expect(guessMime('/x/a.unknownext')).toBe('application/octet-stream');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/test/pathinfo.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```ts
export function basename(path: string): string {
  const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return i >= 0 ? path.slice(i + 1) : path;
}

const MIME: Record<string, string> = {
  png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif',
  webp: 'image/webp', svg: 'image/svg+xml', pdf: 'application/pdf',
  txt: 'text/plain', md: 'text/markdown', json: 'application/json',
  mp4: 'video/mp4', webm: 'video/webm', mov: 'video/quicktime',
  mp3: 'audio/mpeg', wav: 'audio/wav',
};

export function guessMime(path: string): string {
  const ext = basename(path).split('.').pop()?.toLowerCase() ?? '';
  return MIME[ext] ?? 'application/octet-stream';
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/test/pathinfo.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/utils/pathinfo.ts src/test/pathinfo.test.ts
git commit -m "feat: add pathinfo util (basename + guessMime)"
```

---

## Task 8: Native file-picker hook

Opens the Tauri dialog (multi-select), stats each path for size, returns path-based `FileInput[]`.

**Files:**
- Modify: `src/types/vault.ts:33` (`FileInput` shape)
- Create: `src/hooks/usePickFiles.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface FileInput { path: string; name: string; mime: string; size: number }
  // hook: returns () => Promise<FileInput[]> (empty array if the user cancels)
  export function usePickFiles(): () => Promise<FileInput[]>
  ```
- Consumes: `basename`, `guessMime` (Task 7); `@tauri-apps/plugin-dialog` `open`; `@tauri-apps/plugin-fs` `stat`.

- [ ] **Step 1: Update `FileInput` in `src/types/vault.ts`**

```ts
export interface FileInput {
  path: string;
  name: string;
  mime: string;
  size: number;
}
```

- [ ] **Step 2: Implement the hook**

```ts
import { open } from '@tauri-apps/plugin-dialog';
import { stat } from '@tauri-apps/plugin-fs';
import { basename, guessMime } from '../utils/pathinfo';
import type { FileInput } from '../types/vault';

/** Returns a picker that opens the native dialog and yields path-based FileInputs. */
export function usePickFiles(): () => Promise<FileInput[]> {
  return async () => {
    const selected = await open({ multiple: true, directory: false });
    if (!selected) return [];
    const paths = Array.isArray(selected) ? selected : [selected];
    return Promise.all(paths.map(async (path) => {
      let size = 0;
      try { size = (await stat(path)).size ?? 0; } catch { /* size stays 0 */ }
      return { path, name: basename(path), mime: guessMime(path), size };
    }));
  };
}
```

- [ ] **Step 3: Typecheck**

Run: `npx tsc --noEmit`
Expected: no errors. (If `stat` isn't exported by the installed `@tauri-apps/plugin-fs` version, use `size` from `lstat`/`metadata` per that version's API — verify against `node_modules/@tauri-apps/plugin-fs`.)

- [ ] **Step 4: Commit**

```bash
git add src/types/vault.ts src/hooks/usePickFiles.ts
git commit -m "feat: add usePickFiles hook (native dialog -> path FileInputs)"
```

---

## Task 9: Create wizard — pick paths, drop `Array.from`

**Files:**
- Modify: `src/components/Container/CreateWizard/Step1Files.tsx` (replace `DropZone` with the picker; hold `FileInput[]` instead of `File[]`)
- Modify: `src/components/Container/CreateWizard/index.tsx:36-45` (remove `arrayBuffer`/`Array.from`; pass `FileInput[]` straight through)

**Interfaces:**
- Consumes: `usePickFiles` (Task 8), `FileInput` (Task 8)

- [ ] **Step 1: Update `Step1Files.tsx`**

Change `onNext` to `(data: { name: string; files: FileInput[] })`, replace `useState<File[]>` with `useState<FileInput[]>`, and replace `<DropZone .../>` with an "Add files" button that calls the picker and appends results. Keep the existing summary line (`files.length` + `formatBytes(sum of f.size)`). Keep a per-file remove control (map over `FileInput[]`, key by `path`).

```tsx
const pickFiles = usePickFiles();
// ...
<button type="button" className="step1-btn" onClick={async () => {
  const picked = await pickFiles();
  if (picked.length) setFiles(prev => [...prev, ...picked]);
}}>Add files…</button>
```

- [ ] **Step 2: Update `CreateWizard/index.tsx`**

Delete the `arrayBuffer`/`Array.from` conversion in `handleStep1Next`/create. `step1Data.files` is already `FileInput[]`; pass it directly:

```tsx
// BEFORE: fileInputs built via file.arrayBuffer() + Array.from(new Uint8Array(...))
// AFTER: files are already FileInput[] from the picker
await createContainer({
  name: step1Data.name,
  files: step1Data.files, // FileInput[] with { path, name, mime, size }
  kdf_params, password, /* ...unchanged fields... */
});
```

- [ ] **Step 3: Typecheck + existing tests**

Run: `npx tsc --noEmit && npx vitest run`
Expected: no type errors; suite green.

- [ ] **Step 4: Commit**

```bash
git add src/components/Container/CreateWizard/Step1Files.tsx src/components/Container/CreateWizard/index.tsx
git commit -m "feat: create wizard picks file paths, drops Array.from byte transfer"
```

---

## Task 10: Edit view — pick paths for "Add new files"

**Files:**
- Modify: `src/components/Container/ContainerModal/EditView.tsx` (replace the add-files `DropZone`/`File[]` with the picker + `FileInput[]`)
- Modify: `src/components/Container/ContainerModal/index.tsx:93` (`handleSave` builds `FileInput[]` — remove any `arrayBuffer` conversion; pass picker results through)

**Interfaces:**
- Consumes: `usePickFiles`, `FileInput`

- [ ] **Step 1: Update `EditView.tsx`**

Replace the `DropZone` used for new files with an "Add files…" button calling `usePickFiles`; store added files as `FileInput[]`; render the list from `f.name`/`formatBytes(f.size)`. The `filesToAdd` handed to the parent is now `FileInput[]` directly (no byte reading).

- [ ] **Step 2: Update `ContainerModal/index.tsx` `handleSave`**

Ensure `handleSave` passes the `FileInput[]` from EditView straight to `saveContainerEdits(container.id, password, fileInputs, filesToRemove)` with no `arrayBuffer`/`Array.from` step.

- [ ] **Step 3: Typecheck + tests**

Run: `npx tsc --noEmit && npx vitest run`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add src/components/Container/ContainerModal/EditView.tsx src/components/Container/ContainerModal/index.tsx
git commit -m "feat: edit view picks file paths for added files"
```

---

## Task 11: Byte-weighted progress percentage (what makes it smooth)

The backend now emits `bytes_processed`/`bytes_total` after every chunk, but `ProgressBar` still draws `percent = current/total` (file count) — so without this task the bar still steps per file. Drive the percentage from bytes when a byte total is known.

**Files:**
- Modify: `src/components/UI/ProgressBar.tsx` (percent computation)
- Test: `src/test/ProgressBar.test.tsx`

**Interfaces:**
- Consumes: existing `ProgressBarProps` (`current`, `total`, `bytesProcessed`, `bytesTotal`, `indeterminate`)

- [ ] **Step 1: Write the failing test**

```tsx
it('uses byte-weighted percent when bytesTotal is known', () => {
  render(
    <ProgressBar operation="encrypt" current={0} total={2}
      bytesProcessed={135_000_000} bytesTotal={200_000_000} />
  );
  // 135/200 = 67.5% -> 68%, NOT 0% (current/total = 0/2)
  expect(screen.getByText('68%')).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/test/ProgressBar.test.tsx`
Expected: FAIL — shows `0%` (file-count based).

- [ ] **Step 3: Implement**

In `ProgressBar.tsx`, where `percent` is computed, prefer bytes when available:

```tsx
const hasBytes = bytesProcessed !== undefined && bytesTotal !== undefined && bytesTotal > 0;
const percent = isIndeterminate
  ? 0
  : hasBytes
    ? Math.round((bytesProcessed! / bytesTotal!) * 100)
    : (total > 0 ? Math.round((current / total) * 100) : 0);
```

Keep the "X / Y files" footer driven by `current`/`total` as-is.

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/test/ProgressBar.test.tsx`
Expected: PASS (and existing ProgressBar tests still pass).

- [ ] **Step 5: Commit**

```bash
git add src/components/UI/ProgressBar.tsx src/test/ProgressBar.test.tsx
git commit -m "feat: byte-weighted progress percentage for smooth encryption progress"
```

---

## Task 12: End-to-end verification

**Files:** none (verification only)

- [ ] **Step 1:** `cd src-tauri && cargo test` — all Rust tests pass.
- [ ] **Step 2:** `npx tsc --noEmit && npx vitest run` — no type errors, all frontend tests pass.
- [ ] **Step 3:** Use the `verify`/`run` skill to launch the app and:
  - Create a container from a large file (e.g. the 135 MB AppImage) via the native picker — confirm the bar rises **smoothly** with a real "X MB / Y MB", not a single jump to 50%/100%.
  - Open an **existing** (pre-change) container and download a file — confirm it still decrypts (backward compat).
  - Edit that container, add a file, save — confirm smooth progress and that both old and new files open afterward.
- [ ] **Step 4: Commit** any verification notes if the repo tracks them; otherwise no commit.

---

## Self-Review Notes

- **Spec coverage:** paths not bytes (T4/T8/T9/T10), chunked streaming encrypt with progress (T3/T4/T5), backward-compatible reads (T2/T6), native multi-select picker without drag-drop (T8/T9/T10), the two review bugs `bytes_total=0` + index (T5), smooth percentage (T11). All covered.
- **Type consistency:** `FileInput { path, name, mime, size }` is identical in Rust (T4) and TS (T8). Helper names are stable across tasks: `file_encrypted_len`, `decrypt_file`, `encrypt_file_chunked`, `ENCRYPT_CHUNK_SIZE`.
- **Known follow-ups (out of scope):** (a) `get_file_data`/`download` still return bytes over IPC as `number[]` — reads aren't streamed; (b) `save_edits_v2` still fully re-encrypts retained files rather than copying their ciphertext. Both are noted for a later pass.
