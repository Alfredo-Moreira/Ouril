import { describe, expect, it } from 'vitest';

import { nextTrack } from './playlist';

describe('nextTrack (random game playlist)', () => {
  it('handles no tracks and a single track', () => {
    expect(nextTrack(0, null)).toBeNull();
    expect(nextTrack(1, null)).toBe(0);
    expect(nextTrack(1, 0)).toBe(0);
  });

  it('never repeats the current track, and reaches every other one', () => {
    const seen = new Set<number>();
    for (let i = 0; i < 100; i++) {
      const n = nextTrack(4, 2, () => i / 100)!;
      expect(n).not.toBe(2);
      expect(n).toBeGreaterThanOrEqual(0);
      expect(n).toBeLessThan(4);
      seen.add(n);
    }
    expect([...seen].sort()).toEqual([0, 1, 3]);
  });
});
