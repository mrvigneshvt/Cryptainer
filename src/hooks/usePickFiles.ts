import { open } from '@tauri-apps/plugin-dialog';
import { stat } from '@tauri-apps/plugin-fs';
import { basename, guessMime } from '../utils/pathinfo';
import type { FileInput } from '../types/vault';

/**
 * Returns a picker that opens the native multi-select file dialog and yields
 * path-based FileInputs (the backend streams bytes from `path` itself).
 * Resolves to an empty array if the user cancels.
 */
export function usePickFiles(): () => Promise<FileInput[]> {
  return async () => {
    const selected = await open({ multiple: true, directory: false });
    if (!selected) return [];
    const paths = Array.isArray(selected) ? selected : [selected];
    return Promise.all(
      paths.map(async (path) => {
        let size = 0;
        try {
          size = (await stat(path)).size ?? 0;
        } catch {
          /* size stays 0 if stat fails */
        }
        return { path, name: basename(path), mime: guessMime(path), size };
      })
    );
  };
}
