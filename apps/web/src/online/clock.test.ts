import { describe, expect, it } from 'vitest';

import { clockOffset, formatClock, timeLeft } from './clock';

describe('live clocks', () => {
  it('formats minutes and seconds, with tenths in the last ten seconds', () => {
    expect(formatClock(300_000)).toBe('5:00');
    expect(formatClock(245_900)).toBe('4:05');
    expect(formatClock(9_450)).toBe('0:09.4');
    expect(formatClock(-5)).toBe('0:00.0');
  });

  it('counts down only the side to move, from the turn start, in server time', () => {
    const clocks = {
      south_ms: 60_000,
      north_ms: 50_000,
      turn_started_at: '2026-10-14T10:00:00.000Z',
      running: true,
    };
    // The device's clock runs 2 s behind the server's.
    const offset = clockOffset('2026-10-14T10:00:05.000Z', Date.parse('2026-10-14T10:00:03.000Z'));
    expect(offset).toBe(2_000);
    const deviceNow = Date.parse('2026-10-14T10:00:08.000Z'); // 10 s into the turn, server time
    expect(timeLeft(clocks, 'north', deviceNow, offset)).toEqual({ south: 60_000, north: 40_000 });
    expect(timeLeft({ ...clocks, running: false }, 'north', deviceNow, offset)).toEqual({
      south: 60_000,
      north: 50_000,
    });
    expect(timeLeft(clocks, 'south', deviceNow + 120_000, offset).south).toBe(0);
  });
});
