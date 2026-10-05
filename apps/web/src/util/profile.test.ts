import { describe, expect, it } from 'vitest';

import { normalizeDisplayName, normalizeHandle } from './profile';

describe('profile normalization (mirrors apps/server/src/validate.rs)', () => {
  it('display names are trimmed, 1–50 characters, without control or format characters', () => {
    expect(normalizeDisplayName('  Ana  ')).toBe('Ana');
    expect(normalizeDisplayName('   ')).toBeNull();
    expect(normalizeDisplayName('x'.repeat(51))).toBeNull();
    expect(normalizeDisplayName('Ana‮evil')).toBeNull();
    expect(normalizeDisplayName('Ana Lopes')).toBeNull();
  });

  it('handles drop a leading @, are lowercased and must match [a-z0-9_]{3,20}', () => {
    expect(normalizeHandle('@Mindelo_Master')).toBe('mindelo_master');
    expect(normalizeHandle('ab')).toBeNull();
    expect(normalizeHandle('has space')).toBeNull();
    expect(normalizeHandle('a'.repeat(21))).toBeNull();
  });
});
