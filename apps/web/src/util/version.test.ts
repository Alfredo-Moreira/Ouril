import { describe, expect, it } from 'vitest';

import { compareVersions, updateStatus } from './version';

describe('compareVersions', () => {
  it('compares numerically, part by part', () => {
    expect(compareVersions('1.10.0', '1.9.2')).toBe(1);
    expect(compareVersions('1.2.0', '1.2.0')).toBe(0);
    expect(compareVersions('0.9.9', '1.0.0')).toBe(-1);
    expect(compareVersions('1.2', '1.2.0')).toBe(0);
    expect(compareVersions('1.2.3-beta', '1.2.3')).toBe(0);
  });
});

describe('updateStatus', () => {
  const meta = (min: string, rec: string) => ({
    min_supported: { web: min },
    recommended: { web: rec },
  });
  it('is required below min_supported, recommended below recommended, else current', () => {
    expect(updateStatus(meta('1.2.0', '1.4.0'), '1.1.9')).toBe('required');
    expect(updateStatus(meta('1.2.0', '1.4.0'), '1.3.0')).toBe('recommended');
    expect(updateStatus(meta('1.2.0', '1.4.0'), '1.4.0')).toBe('current');
  });
});
