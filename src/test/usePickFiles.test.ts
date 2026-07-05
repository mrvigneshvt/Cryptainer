import { describe, it, expect, vi, beforeEach } from 'vitest';
import { usePickFiles } from '../hooks/usePickFiles';

// The picker relies on Tauri plugins that don't exist in jsdom, so stub them.
const open = vi.fn();
const stat = vi.fn();
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: (...a: unknown[]) => open(...a) }));
vi.mock('@tauri-apps/plugin-fs', () => ({ stat: (...a: unknown[]) => stat(...a) }));

describe('usePickFiles', () => {
  beforeEach(() => {
    open.mockReset();
    stat.mockReset();
    stat.mockImplementation((path: string) => Promise.resolve({ size: path.length }));
  });

  it('returns path-based FileInputs for a multi-select result', async () => {
    open.mockResolvedValue(['/home/u/a.png', '/home/u/b.pdf']);
    const pick = usePickFiles();
    const result = await pick();
    expect(open).toHaveBeenCalledWith({ multiple: true, directory: false });
    expect(result).toEqual([
      { path: '/home/u/a.png', name: 'a.png', mime: 'image/png', size: '/home/u/a.png'.length },
      { path: '/home/u/b.pdf', name: 'b.pdf', mime: 'application/pdf', size: '/home/u/b.pdf'.length },
    ]);
  });

  it('normalizes a single-string dialog result to an array', async () => {
    open.mockResolvedValue('/x/only.txt');
    const result = await usePickFiles()();
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({ path: '/x/only.txt', name: 'only.txt', mime: 'text/plain' });
  });

  it('returns an empty array when the user cancels', async () => {
    open.mockResolvedValue(null);
    const result = await usePickFiles()();
    expect(result).toEqual([]);
    expect(stat).not.toHaveBeenCalled();
  });

  it('falls back to size 0 when stat throws', async () => {
    open.mockResolvedValue(['/x/broken.bin']);
    stat.mockRejectedValue(new Error('permission denied'));
    const result = await usePickFiles()();
    expect(result[0]).toMatchObject({ path: '/x/broken.bin', size: 0, mime: 'application/octet-stream' });
  });
});
