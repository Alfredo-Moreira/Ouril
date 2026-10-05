import { describe, expect, it } from 'vitest';

import { detectDevice } from './mode';

describe('device detection', () => {
  it('jsdom has no WebGL', () => {
    expect(detectDevice().webgl).toBe(false);
  });
});
