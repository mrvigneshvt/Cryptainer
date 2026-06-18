# Cryptainer — Idea & Design Philosophy

## What Is Cryptainer?

Cryptainer is an **offline-first encrypted container manager** — a desktop application that lets you create, manage, and view encrypted "containers" for your files. Think of it as a portable, encrypted folder that lives on your machine and can only be unlocked with your password.

Unlike cloud-based encryption services, Cryptainer is **100% offline**. No servers, no accounts, no telemetry. All data stays on your machine, encrypted with keys derived from your password — keys that are never stored on disk and are wiped from memory the moment the container is locked.

## Why Build It?

Existing encrypted file management tools fall into two categories:

1. **Full-disk encryption** (LUKS, BitLocker, FileVault) — encrypts everything but ties you to one machine and one OS, and offers no granular container management or in-app previews.

2. **Cloud-based "vaults"** — convenient but compromise on privacy: your files live on someone else's server, your metadata is exposed, and you're subject to their security posture.

Cryptainer fills the gap: a **local, portable, password-protected container** with rich in-app file previews. You can store a few sensitive documents or dozens of files, organize them with tags, export containers to share, and preview images, video, audio, PDFs, code, and text — all without ever decrypting to disk.

## Design Philosophy

### 1. Security Is the Product

Every design decision starts with the threat model:

- **Files are never stored in plaintext.** The only unencrypted data on disk is container metadata (name, dates, file count). Everything else — file names, contents, structure — is encrypted.
- **Keys are ephemeral.** The AES key is derived from your password on unlock, held in memory wrapped in `Zeroizing<>`, and wiped when the container is locked or the app closes.
- **Tampering is detectable.** Every blob has a SHA-256 checksum in the database. GCM authentication tags ensure ciphertext integrity.

### 2. Offline-First, No Backend

Cryptainer has zero server-side infrastructure:

- **No accounts.** No signup, no login, no password reset.
- **No telemetry.** No analytics, no crash reporting, no network requests.
- **No cloud dependency.** The app works fully disconnected. Your data never leaves your machine unless you export a `.ctnr` file and deliberately share it.

This means you own your security posture entirely. There's no third party to trust, no service to deprecate, no server to take down.

### 3. Usability Without Compromise

Encryption tools often sacrifice UX for security. Cryptainer aims to avoid this:

- **In-app previews** — unlock a container and view images, play videos/audio, read PDFs and code, inspect binary files — all without decrypting to disk or opening external tools.
- **Per-file lazy decryption** — only the file you click on is decrypted, not the entire container. A bounded LRU cache (50 MB default) holds recently viewed files and zeroizes on eviction.
- **Auto-lock** — containers lock automatically after inactivity (configurable, default 5 minutes).
- **Tagging and search** — organize containers with tags, filter by tag, search by name.

### 4. Rust for Security-Critical Code

The cryptographic backend is written in Rust for a reason:

- **Memory safety** — no buffer overflows, use-after-free, or null pointer dereferences in the crypto path.
- **Zero-cost abstractions** — the encryption primitives compile down to the same assembly as handwritten C, with no runtime overhead.
- **`Zeroizing` semantics** — the `zeroize` crate ensures key material is explicitly overwritten on drop, not just freed.

The frontend is React/TypeScript — pragmatic for UI work while using Tauri's IPC boundary to keep all crypto in safe Rust.

## Threat Model

### Protected Against

| Threat | How Cryptainer Mitigates |
|---|---|
| Unauthorized file access | AES-256-GCM authenticated encryption |
| Offline brute-force | Argon2id memory-hard KDF (configurable strength) |
| Database theft / disk theft | Encrypted blobs separated from metadata; both required |
| Blob tampering | SHA-256 checksum in DB + GCM authentication tags |
| Memory dumps (after lock) | `Zeroizing<>` wipes keys on session drop |
| Reused nonce / IV | Random 96-bit nonce per encryption operation |

### NOT Protected Against

- **Compelled disclosure** — you know the password; we can't help.
- **Memory attacks on unlocked containers** — files in RAM when a container is open and files are being previewed.
- **Side-channel attacks** — no constant-time guarantees at this stage.
- **Malware/keyloggers on the host** — app-level encryption can't protect against a compromised OS.

## V2 Format (Current)

The current format (v2) encrypts each file individually within a container, rather than encrypting the entire payload as a single blob (v1). This enables:

- **Lazy decryption** — open a container without decrypting every file. Only decrypt the metadata section to list files, then decrypt each file on-demand.
- **LRU caching** — decrypted file data is cached in memory (up to 50 MB). Least-recently-used entries are evicted and zeroized automatically.
- **Streaming-friendly** — video files can be chunked (2 MB each) for progressive playback.
- **Graceful v1 migration** — v1 containers are automatically migrated to v2 when first unlocked.

## Architecture at a Glance

```
┌─────────────────────┐
│   React Frontend     │  UI, state (Zustand), previews, auto-lock
│   (TypeScript)       │
├─────────────────────┤
│   Tauri IPC Layer   │  Type-safe invoke() calls
├─────────────────────┤
│   Rust Backend       │  Crypto, storage, sessions, export/import
│   (AES-256-GCM)      │
├─────────────────────┤
│   File System + DB   │  .enc blobs on disk + SQLite metadata
└─────────────────────┘
```

## Roadmap Status

| Phase | Status |
|---|---|
| **Core Desktop (v1)** — scaffold, crypto, storage, CRUD | ✅ Complete |
| **Export/Import** — .ctnr format, edit mode | ✅ Complete |
| **Polish** — previews, search/filter/tags, auto-lock, settings | ✅ Complete |
| **v2 Format** — per-file encryption, lazy decryption, LRU cache | ✅ Complete |
| **Mobile** — Android/iOS support, responsive layouts | 📋 Planned |

---

*Cryptainer is free, open-source, and built with Rust + React + Tauri.*
