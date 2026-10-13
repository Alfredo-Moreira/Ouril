import { describe, expect, it } from 'vitest';

import { displayPosition, flipPit, resultFor } from './perspective';

describe('online board perspective', () => {
  it('South sees the board as it is', () => {
    expect(flipPit(0, 'south', 6)).toBe(0);
    expect(flipPit(11, 'south', 6)).toBe(11);
    const pits = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    expect(displayPosition(pits, [3, 4], 'south')).toEqual({ pits, stores: [3, 4] });
  });

  it('North sees its own pits along the bottom, and mapping twice gives the pit back', () => {
    expect(flipPit(6, 'north', 6)).toBe(0);
    expect(flipPit(0, 'north', 6)).toBe(6);
    for (let p = 0; p < 12; p++) expect(flipPit(flipPit(p, 'north', 6), 'north', 6)).toBe(p);
    const pits = [0, 0, 0, 0, 0, 0, 9, 8, 7, 6, 5, 4];
    expect(displayPosition(pits, [3, 4], 'north')).toEqual({
      pits: [9, 8, 7, 6, 5, 4, 0, 0, 0, 0, 0, 0],
      stores: [4, 3],
    });
  });

  it('reads the result from your seat', () => {
    expect(resultFor('south_wins', 'south')).toBe('win');
    expect(resultFor('south_wins', 'north')).toBe('loss');
    expect(resultFor('north_wins', 'north')).toBe('win');
    expect(resultFor('draw', 'north')).toBe('draw');
  });
});
