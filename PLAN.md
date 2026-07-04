# PLAN: Overlay Progress Modals for All Encrypt/Decrypt Operations

## Problem

The existing `ProgressBar` component renders **inline inside buttons and modal bodies** (LockView unlock button, EditView save button, Step2Config create button, DownloadView content area). This is cramped, visually inconsistent, and blocks the user's view of the underlying UI. The solution is a **modal-on-top-of-modal** overlay pattern for better UX.

## Architecture

```
┌────────────────────────────────────────────┐
│  App / Modal / Wizard (parent)             │  owns isLoading / isDownloading
│  ├─ renders <ProgressOverlay>              │  portal, z-index 1100
│  │   └─ <ProgressBar> inside card          │  reuses existing component
│  └─ child view (LockView/EditView/         │  stripped of inline progress
│      Step2Config/DownloadView)             │  keeps isLoading for disabling
└────────────────────────────────────────────┘
```

## Changes

### New component
- **`ProgressOverlay`** — portal component at `document.body`, z-index 1100, renders `ProgressBar` in a centered card with fadeIn/slideIn animations. Supports `fallbackMessage` prop for null-progress state.

### Extended ProgressBar
- Added `import` and `export` to operation union
- Added verb labels: "Importing", "Exporting"
- Added icons: 📥 (import), 📤 (export)

### Backend progress emissions
- **`export_container`**: emits "Reading container for export…" after metadata fetch, "Writing export file…" before file write
- **`import_container`**: emits "Reading import file…" after file read, "Writing imported container…" before blob write
- All use indeterminate mode (bytes_processed: 0, bytes_total: 0)

### Frontend wiring (overlay replaces inline ProgressBar)

| Parent | View | Loading state | Fallback message |
|---|---|---|---|
| ContainerModal | LockView (unlock) | `isLoading` | "Unlocking container…" |
| ContainerModal | EditView (save) | `isLoading` | "Saving changes…" |
| ContainerModal | Preview | `isLoading` | "Loading file…" |
| ContainerModal/DownloadView | Download | `isDownloading` | "Downloading files…" |
| CreateWizard | Step2Config (create) | `isLoading` | "Encrypting files…" |
| App | Import/Export | `isImporting` / `isExporting` | "Processing…" |

## Assumptions

1. Preview briefly shows overlay via `isLoading` — acceptable flash.
2. `Modal.css` z-index remains 1000; overlay uses 1100.
3. Import/export are fast enough that indeterminate progress is acceptable.
4. `useTauriProgress` resets progress to `null` when `active` becomes false.
5. `save_edits_v1` (legacy) has no progress emissions — v1 auto-migrates to v2 on unlock.

## Verification

- Frontend tests pass
- `tsc --noEmit` — zero errors
- `cargo check` — clean
