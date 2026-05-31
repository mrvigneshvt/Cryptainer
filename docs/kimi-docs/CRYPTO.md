# Cryptainer — Cryptographic Documentation

## Algorithm Choices

### Encryption: AES-256-GCM
- **Algorithm**: AES-256 in Galois/Counter Mode (GCM)
- **Key Size**: 256 bits
- **Nonce Size**: 96 bits (12 bytes) - GCM standard
- **Tag Size**: 128 bits (16 bytes) - appended automatically
- **Rationale**: 
  - Provides both confidentiality and authenticity
  - Hardware-accelerated on modern CPUs (AES-NI)
  - Standard for authenticated encryption
  - Resistant to padding oracle attacks (no padding needed)

### Key Derivation: Argon2id
- **Algorithm**: Argon2id (winner of Password Hashing Competition)
- **Rationale**:
  - Memory-hard function resists GPU/ASIC attacks
  - Side-channel resistant (unlike Argon2d)
  - Current best practice for password KDF

## KDF Parameters

### Standard Security Level (Default)
```rust
ARGON2_MEMORY_KB:   65536  // 64 MB
ARGON2_ITERATIONS:  2
ARGON2_PARALLELISM: 1
```

### Security Presets

| Preset | Memory (KB) | Iterations | Parallelism | Use Case |
|--------|-------------|------------|-------------|----------|
| Fast | 16,384 | 1 | 1 | Low-end devices |
| Standard | 65,536 | 2 | 1 | Recommended default |
| High | 131,072 | 3 | 1 | Extra security |
| Paranoid | 262,144 | 4 | 2 | Maximum security |

**Trade-off**: Higher memory/iterations = slower unlock but more brute-force resistance

## Encryption Flow

### Encryption
1. Generate random 16-byte salt
2. Generate random 12-byte nonce
3. Derive 32-byte key using Argon2id(password, salt, params)
4. Encrypt plaintext with AES-256-GCM(key, nonce)
5. Output: `[salt (16)] + [nonce (12)] + [ciphertext + tag]`

### Decryption
1. Extract salt (bytes 0-15)
2. Extract nonce (bytes 16-27)
3. Extract ciphertext+tag (bytes 28+)
4. Derive key using Argon2id(password, salt, params)
5. Decrypt and verify GCM tag
6. If tag verification fails → CryptoError::Decryption (wrong password or tampered)

## Key Material Handling

### Memory Security
- All key material wrapped in `Zeroizing<[u8; 32]>`
- Automatically wiped from memory when dropped
- Prevents key leakage in memory dumps

### Storage Security
- **Keys are NEVER stored on disk**
- Only encrypted blobs are persisted
- Keys derived on-the-fly from password
- Salt stored with ciphertext (safe to expose)

## Integrity Protection

### Per-Container Checksums
- SHA-256 of encrypted blob stored in database
- Verified before decryption attempts
- Detects corruption or tampering early

### GCM Authentication
- Each encryption produces 16-byte authentication tag
- Decryption fails if tag doesn't match
- Prevents ciphertext tampering even with known plaintext

## .ctnr File Format

### Binary Layout
```
[0..4]    Magic bytes: "CNTR" (ASCII)
[4]       Null byte: 0x00
[5]       Version: u8 (current: 0x01)
[6..10]   Header length: u32 little-endian
[10..10+header_len]  JSON header (plaintext)
[10+header_len..]   Encrypted blob
```

### Header Fields
- `id`: Container UUID
- `name`: Container name
- `algo`: Encryption algorithm
- `kdf`: Key derivation function
- `kdf_params`: KDF configuration
- `hint`: Password hint (optional)
- `tags`: Comma-separated tags (optional)
- `file_count`: Number of files
- `total_size`: Total size in bytes
- `created_at`: ISO-8601 timestamp
- `modified_at`: ISO-8601 timestamp
- `blob_sha256`: SHA-256 of encrypted blob

## Security Guarantees

### What IS Protected
- File contents (encrypted with AES-256-GCM)
- File names and metadata (inside encrypted payload)
- Container structure and organization

### What is NOT Protected (Visible in Plaintext)
- Container name (in header for UX)
- Creation/modification dates (in header)
- File count and total size (in header)
- Password hint (in header, intentionally)
- Algorithm and KDF parameters (in header)
- **SHA-256 of encrypted blob** (in header for integrity)

### Threat Model
- **Protects against**: Unauthorized access to files, offline attacks, database theft
- **Does NOT protect against**: Compelled disclosure (user has password), memory attacks (keys in RAM when unlocked), side-channel attacks on unlocked container

## Audit Notes

- All crypto operations use well-reviewed crates (`aes-gcm`, `argon2`)
- No custom crypto implementations
- All parameters documented and justified
- Memory safety enforced via Rust's ownership + `zeroize`
- Constant-time operations not yet audited (Phase 3 enhancement)

## Last Updated
2026-03-14 - Created (Phase 1 implementation in progress)
