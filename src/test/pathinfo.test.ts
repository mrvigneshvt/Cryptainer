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
