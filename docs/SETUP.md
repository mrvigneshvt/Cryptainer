# Cryptainer — Development Setup

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| [Node.js](https://nodejs.org/) | 18+ | Frontend build + npm |
| [Rust](https://www.rust-lang.org/tools/install) | Latest stable (edition 2021) | Backend compilation |
| [Tauri CLI](https://tauri.app/v2/guides/getting-started/prerequisites) | ^2 | Build/dev tooling |

### Linux Dependencies

```bash
# Arch Linux / Omarchy
sudo pacman -S --needed base-devel curl wget file openssl gtk3 \
  libayatana-appindicator librsvg webkit2gtk-4.1

# Ubuntu/Debian
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  build-essential \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### macOS

Xcode Command Line Tools are required:

```bash
xcode-select --install
```

### Windows

See [Tauri prerequisites](https://tauri.app/v2/guides/getting-started/prerequisites#windows) for WebView2 and MSVC setup.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/mrvigneshvt/Cryptainer.git
cd cryptainer

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev
```

This will:
1. Install Rust crate dependencies (first run: downloads and compiles all crates)
2. Start the Vite dev server on port 1420
3. Launch the Tauri desktop window connected to the dev server

## Production Build

```bash
# Build the desktop application
npm run tauri build
```

Output will be at `src-tauri/target/release/`:
- **Linux**: `.deb`, `.AppImage`, or system package
- **macOS**: `.dmg`
- **Windows**: `.msi` or `.exe`

## Project Scripts

| Command | Description |
|---|---|
| `npm run tauri dev` | Run in development mode with hot-reload |
| `npm run tauri build` | Build for production |
| `npm run dev` | Run Vite frontend only (no Tauri) |
| `npm run build` | Build frontend only (TypeScript + Vite) |

## Running Tests

```bash
# Rust unit tests (crypto, session, vault, export)
cargo test

# Rust unit tests with stdout (for debug)
cargo test -- --nocapture

# Run only integration tests
cargo test --test v2_integration

# TypeScript type checking
npx tsc --noEmit

# Rust linter
cargo clippy -- -D warnings
```

## Project Layout

```
cryptainer/
├── src/               # React frontend (TypeScript)
│   ├── components/    # UI components
│   ├── hooks/         # Custom hooks (useAutoLock, useMediaQuery)
│   ├── store/         # Zustand state
│   ├── types/         # TypeScript interfaces
│   ├── utils/         # Helpers
│   └── styles/        # Global CSS
├── src-tauri/         # Rust backend
│   ├── src/           # Source code
│   ├── tests/         # Integration tests
│   ├── migrations/    # SQLite migrations
│   └── Cargo.toml    # Rust dependencies
└── docs/              # Documentation
```

## Debugging

### Rust Backend

Logs go to stdout/stderr in the terminal where `npm run tauri dev` is running. Use `eprintln!()` or a logging crate for debug output.

### Frontend

Open the Tauri devtools:
- **Linux/macOS**: Right-click → Inspect Element
- **Windows**: Right-click → Inspect Element
- Or use `Ctrl+Shift+I` (may conflict with OS shortcuts)

The Zustand store state can be inspected in the browser console:

```javascript
// Requires React DevTools + Zustand DevTools (not yet configured)
// For now, log state via store subscription:
import { useVaultStore } from './store/vaultStore';
console.log(useVaultStore.getState());
```

## Adding a New Feature

1. **Types**: Update `src/types/vault.ts` if new TypeScript interfaces are needed
2. **Rust backend**:
   - Add/modify logic in the appropriate module (`crypto.rs`, `storage.rs`, `session.rs`, etc.)
   - Add IPC command handler in `commands.rs`
   - Register the command in `lib.rs`'s `generate_handler!` macro
   - Add unit tests and/or integration tests
3. **Frontend**:
   - Add IPC call to `src/store/vaultStore.ts`
   - Create or update components in `src/components/`
   - Update types in `src/types/vault.ts` if needed
4. **Documentation**: Update relevant docs in `docs/`

---

*Last updated: 2026-06-19*
