import { describe, expect, it } from 'vitest';

import { uuidv7 } from './uuid';

describe('uuidv7', () => {
  it('is a valid version-7 UUID', () => {
    expect(uuidv7()).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
  });

  it('sorts in creation order within one millisecond and when the clock steps back', () => {
    const now = Date.now() + 10_000;
    const ids = [
      ...Array.from({ length: 5000 }, () => uuidv7(now)), // same ms, past the 12-bit counter
      uuidv7(now - 5), // clock went back
      uuidv7(now + 1),
    ];
    expect([...ids].sort()).toEqual(ids);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
