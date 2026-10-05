import { describe, expect, it } from 'vitest';

import i18n from '../i18n';
import type { MoveEvent } from '../engine/types';
import { describeEvents } from './describeEvents';

const t = (key: string, opts?: Record<string, unknown>) => i18n.t(key, opts);

describe('describeEvents', () => {
  it('says nothing for a plain sowing move', () => {
    expect(
      describeEvents(
        t,
        [
          { type: 'sow', pit: 1 },
          { type: 'sow', pit: 2 },
        ],
        true,
      ),
    ).toEqual([]);
  });

  it('sums chained captures into one sentence, from the right point of view', () => {
    const events: MoveEvent[] = [
      { type: 'sow', pit: 7 },
      { type: 'capture', pit: 7, seeds: 3 },
      { type: 'capture', pit: 6, seeds: 2 },
    ];
    const you = describeEvents(t, events, true);
    const ai = describeEvents(t, events, false);
    expect(you).toHaveLength(1);
    expect(you[0]).toMatch(/5/);
    expect(ai).toEqual([i18n.t('game.ai_captured', { count: 5 })]);
    expect(you[0]).not.toEqual(ai[0]);
  });

  it('uses the singular for one seed', () => {
    expect(describeEvents(t, [{ type: 'capture', pit: 7, seeds: 1 }], false)).toEqual([
      'The computer captured 1 seed',
    ]);
  });

  it('describes skip, grand slam and extra turn', () => {
    const lines = describeEvents(
      t,
      [{ type: 'skip_origin', pit: 0 }, { type: 'grand_slam' }, { type: 'extra_turn' }],
      true,
    );
    expect(lines).toEqual([
      i18n.t('game.event.skip_origin'),
      i18n.t('game.event.grand_slam'),
      i18n.t('game.event.extra_turn_you'),
    ]);
    expect(describeEvents(t, [{ type: 'extra_turn' }], false)).toEqual([
      i18n.t('game.event.extra_turn_ai'),
    ]);
  });

  it('never returns a raw i18n key', () => {
    const all: MoveEvent[] = [
      { type: 'skip_origin', pit: 0 },
      { type: 'capture', pit: 7, seeds: 2 },
      { type: 'grand_slam' },
      { type: 'extra_turn' },
    ];
    for (const byHuman of [true, false]) {
      for (const line of describeEvents(t, all, byHuman)) expect(line).not.toMatch(/^game\./);
    }
  });
});
