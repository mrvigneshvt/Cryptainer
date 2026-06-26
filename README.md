# Cryptainer

> **Offline Encrypted Container Manager** — seal your files into portable, password-protected containers on your own device. AES-256-GCM + Argon2id, 100% offline, zero telemetry.

### 🌐 [cryptainer.forked.online](https://cryptainer.forked.online) · ⬇️ [Download](#download) · 📖 [Docs](docs/)

[![Website](https://img.shields.io/badge/Website-cryptainer.forked.online-00FF41?style=for-the-badge&logo=tauri&logoColor=black)](https://cryptainer.forked.online)
[![Release](https://img.shields.io/github/v/release/mrvigneshvt/Cryptainer?style=for-the-badge&color=00FF41)](https://github.com/mrvigneshvt/Cryptainer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri%20v2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)

## Download

Cryptainer is free and open source. Grab a build, or [compile from source](#build-from-source).

| Platform | Download | Notes |
|---|---|---|
| 🐧 **Linux** | [**AppImage**](https://github.com/mrvigneshvt/Cryptainer/releases/latest/download/Cryptainer-linux.AppImage) · [**.deb**](https://github.com/mrvigneshvt/Cryptainer/releases/latest/download/Cryptainer-linux-amd64.deb) | AppImage runs on any distro — no install |
| 🤖 **Android** | [**APK**](https://github.com/mrvigneshvt/Cryptainer/releases/latest/download/Cryptainer-android.apk) | universal, signed release |
| 🪟 Windows | _coming soon_ | [build from source](#build-from-source) |
| 🍎 macOS | _coming soon_ | [build from source](#build-from-source) |
| 📱 iOS | _coming soon_ | [build from source](#build-from-source) |

> All releases & checksums → **[github.com/mrvigneshvt/Cryptainer/releases](https://github.com/mrvigneshvt/Cryptainer/releases/latest)**

One Rust + Tauri core runs everywhere, so a `.ctnr` sealed on one platform opens, byte-for-byte, on another.

## Features

### Core Functionality
- **🔐 Military-Grade Encryption**: AES-256-GCM authenticated encryption with Argon2id key derivation
- **📦 Encrypted Containers**: Store multiple files in a single encrypted container
- **🔓 Password Protection**: Unlock containers with your password — no keys stored on disk
- **⚡ Lazy Decryption**: Open a container instantly — files decrypt on-demand when you preview them
- **✏️ Edit Mode**: Add or remove files from existing containers with automatic re-encryption
- **💾 Portable Format**: Export/import containers using the `.ctnr` file format
- **🗄️ Local Storage**: All data stored locally — no cloud, no servers, 100% offline

### Security Features
- **Memory-Safe Key Handling**: AES keys are wrapped in `Zeroizing<>` and wiped from memory immediately when containers are locked
- **LRU Cache with Zeroization**: Decrypted file data is cached (50 MB) and zeroized on eviction or explicit release
- **Session Management**: Unlocked containers held in memory only, cleared on lock/app close
- **Integrity Protection**: SHA-256 checksums + GCM authentication tags detect tampering or corruption
- **V1 → V2 Auto-Migration**: Legacy containers are automatically migrated to the v2 per-file encryption format on unlock
- **Configurable Security Levels**: Choose from Fast, Standard, High, or Paranoid Argon2id settings
- **Auto-Lock**: Automatically lock containers after configurable period of inactivity (default 5 minutes)

### File Support
- **Images**: PNG, JPG, GIF, WebP (with preview)
- **Videos**: MP4, WebM (with playback)
- **Audio**: MP3, WAV, OGG (with playback)
- **Documents**: PDF (with embedded viewer)
- **Code Files**: 20+ languages with syntax highlighting (Rust, TypeScript, Python, etc.)
- **Text Files**: UTF-8 text with line numbers
- **Binary Files**: Hex dump view with offset/hex/ASCII display

### User Experience
- **🔍 Search & Filter**: Search containers by name or tags
- **🏷️ Tag System**: Organize containers with custom tags, filter by tag
- **📊 Sort Options**: Sort by name, date, size, or file count
- **⚙️ Settings**: Configure auto-lock timeout
- **🔑 Password Hints**: Optional hints to help remember passwords
- **💪 Password Strength**: Visual indicator for password strength
- **🎨 Themes**: PRISM (light, frosted glass) and CIPHER (dark, terminal) — two complete design systems

## Tech Stack

### Backend
- **Rust** — Systems programming with memory safety (edition 2021)
- **Tauri v2** — Secure cross-platform app framework (desktop + mobile)
- **AES-256-GCM** — Authenticated encryption (via `aes-gcm` v0.10)
- **Argon2id** — Memory-hard password hashing (via `argon2` v0.5)
- **SQLite** — Local metadata storage (via `sqlx` v0.7)
- **LRU Cache** — Bounded in-memory cache with zeroization (via `lru` v0.12)
- **Zeroize** — Automatic secure memory wiping (via `zeroize` v1.7)

### Frontend
- **React 18** — UI framework
- **TypeScript** — Type-safe JavaScript
- **Vite** — Fast build tooling
- **Zustand** — Lightweight state management
- **Prism.js** — Syntax highlighting (via `prism-react-renderer`)
- **react-dropzone** — Drag-and-drop file selection

## Build from source

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI](https://tauri.app/v2/guides/getting-started/prerequisites)

### Desktop (Linux / macOS / Windows)

```bash
# Clone the repository
git clone https://github.com/mrvigneshvt/Cryptainer.git
cd cryptainer

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build a release bundle (.AppImage, .deb, .dmg, .msi …)
npm run tauri build
```

### Android

```bash
# One-time: initialise the Android project (needs Android SDK + NDK)
npm run tauri android init

# Build a signed release APK / AAB
npm run tauri android build
```

See [`docs/SETUP.md`](docs/SETUP.md) for detailed setup instructions and platform-specific dependencies.

## Usage

### Creating a Container
1. Click **"+ New Container"**
2. Select files to encrypt (drag-and-drop or file picker)
3. Choose security level (Standard recommended)
4. Set a strong password (min 8 characters)
5. Optional: Add password hint and tags
6. Click **"Create & Encrypt"**

### Opening a Container
1. Click on a container card in the vault grid
2. Enter your password
3. Browse files — click any file to preview it
4. Files decrypt on-demand; previews stream for videos/audio

### Editing a Container
1. Open an unlocked container
2. Click **"Edit"**
3. Remove files or add new ones
4. Enter password to verify and save changes
5. Container is automatically re-encrypted with fresh nonces

### Export/Import
- **Export**: Click the ↓ button on a container card → save `.ctnr` file
- **Import**: Click **"Import .ctnr"** → select one or more files → containers appear in vault

## Architecture

```
┌─────────────────────┐
│   React App         │  UI components, Zustand store, previews, auto-lock
│   (TypeScript)      │
├─────────────────────┤
│   Tauri IPC Layer   │  10 type-safe invoke() commands
├─────────────────────┤
│   Rust Backend      │  AES-256-GCM, Argon2id, SQLite, LRU cache
│   (src-tauri/src/)  │
│                     │
│  ┌───────────────┐  │
│  │   crypto.rs   │  │  Encrypt/decrypt, key derivation
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  storage.rs   │  │  SQLite CRUD (sqlx)
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  session.rs   │  │  Session store + LRU cache with zeroization
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  export.rs    │  │  .ctnr file format
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │  commands.rs  │  │  10 IPC command handlers
│  └───────────────┘  │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  File System + DB   │  .enc blobs on disk + SQLite metadata DB
└─────────────────────┘
```

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the complete module graph, component tree, and data flow.

## Container Format

Cryptainer uses two container formats:

| Format | Description | Status |
|---|---|---|
| **v1 (legacy)** | Single-blob encryption — entire payload encrypted as one AES-256-GCM output | Migrated to v2 on unlock |
| **v2 (current)** | Per-file encryption — metadata encrypted separately, each file encrypted individually, lazy decryption, LRU cache | Active |

See [`docs/CRYPTO.md`](docs/CRYPTO.md) for the full cryptographic specification and blob layout.

## Security Model

### What IS Protected
- ✅ File contents (AES-256-GCM authenticated encryption)
- ✅ File names and metadata (inside encrypted metadata section)
- ✅ Container structure and organization
- ✅ Integrity (SHA-256 checksums + GCM authentication tags)

### What is NOT Protected (Visible in Plaintext)
- Container name (shown in UI for identification)
- Creation/modification dates
- File count and total size
- Password hint (intentionally visible for user convenience)
- Algorithm and KDF parameters

### Threat Model
- **Protects against**: Unauthorized access, offline attacks, database theft, blob tampering
- **Does NOT protect against**: Compelled disclosure (password required), memory attacks on unlocked containers, keyloggers/malware on the host system

## Development

```bash
# Rust unit tests
cargo test

# TypeScript type checking
npx tsc --noEmit

# Rust linting
cargo clippy -- -D warnings
```

See [`docs/SETUP.md`](docs/SETUP.md) for full development workflow and [`docs/CHANGELOG.md`](docs/CHANGELOG.md) for release history.

## Project Structure

```
cryptainer/
├── astro-landing/            # Marketing site → cryptainer.forked.online
├── docs/                     # Documentation
├── src/                      # React frontend
├── src-tauri/                # Rust backend
│   ├── src/                  # Rust source modules
│   ├── tests/                # Integration tests
│   └── migrations/           # SQLite migrations
├── package.json              # npm dependencies
└── vite.config.ts            # Vite configuration
```

## Documentation

| Document | Description |
|---|---|
| [`docs/IDEA.md`](docs/IDEA.md) | Project vision, design philosophy, threat model |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Module graph, component tree, data flow, dependencies |
| [`docs/API.md`](docs/API.md) | Complete IPC command reference |
| [`docs/CRYPTO.md`](docs/CRYPTO.md) | Cryptographic specifications, blob format, key handling |
| [`docs/SETUP.md`](docs/SETUP.md) | Development setup guide |
| [`docs/CHANGELOG.md`](docs/CHANGELOG.md) | Release history |

## License

MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Tauri](https://tauri.app/) for the secure app framework
- [Argon2](https://github.com/P-H-C/phc-winner-argon2) for password hashing
- [AES-GCM](https://github.com/RustCrypto/AEADs) for authenticated encryption
- [Prism.js](https://prismjs.com/) for syntax highlighting

---

**⚠️ Security Notice**: This is cryptographic software. While we follow best practices, you are responsible for your data. Always keep backups of important files and use strong, unique passwords.

*Made with Rust + Tauri · [cryptainer.forked.online](https://cryptainer.forked.online) · built by [forked.online](https://platform.forked.online)*
