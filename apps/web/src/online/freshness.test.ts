import { describe, expect, it } from 'vitest';

import type { OnlineGame } from '../api/types';
import { newer } from './freshness';

function game(moves: number[], updated: string, id = 'g1'): OnlineGame {
  return {
    id,
    variant_id: 'cv.standard',
    variant_version: 1,
    status: 'active',
    first_player: 'south',
    you: 'south',
    moves,
    your_turn: true,
    invited_by_you: true,
    created_at: '2026-10-13T10:00:00Z',
    updated_at: updated,
  };
}

describe('newer', () => {
  it('keeps the copy with more moves when an old poll lands late', () => {
    const afterMove = game([0], '2026-10-13T10:00:05Z');
    const stalePoll = game([], '2026-10-13T10:00:01Z');
    expect(newer(afterMove, stalePoll)).toBe(afterMove);
  });

  it('keeps the later update when the moves are the same (a resign, a timeout)', () => {
    const finished = { ...game([0, 6], '2026-10-13T10:01:00Z'), status: 'finished' as const };
    const active = game([0, 6], '2026-10-13T10:00:30Z');
    expect(newer(finished, active)).toBe(finished);
    expect(newer(active, finished)).toBe(finished);
  });

  it('takes a newer copy, the first one, or another game', () => {
    const a = game([0], '2026-10-13T10:00:05Z');
    const b = game([0, 6], '2026-10-13T10:00:09Z');
    expect(newer(a, b)).toBe(b);
    expect(newer(null, a)).toBe(a);
    const rematch = game([], '2026-10-13T09:00:00Z', 'g2');
    expect(newer(b, rematch)).toBe(rematch);
  });
});
