import { describe, expect, it } from 'vitest';

import type { GameState, MoveEvent } from '../engine/types';
import { loadCoreSync } from '../test/wasm';
import { buildFrames, frameDelay, type FrameKind } from './animation';
import { defaultVariantRef, variantRef } from './session';

const state = (
  pits: number[],
  stores: [number, number],
  to_move: 'south' | 'north',
): GameState => ({
  pits,
  stores,
  to_move,
  moves_since_capture: 0,
  status: 'playing',
});

describe('buildFrames (engine events -> display frames)', () => {
  it('maps sow / skip / capture / grand_slam / extra_turn / collect in order', () => {
    const before = state([0, 0, 0, 0, 0, 3, 1, 1, 0, 0, 0, 0], [10, 20], 'south');
    const events: MoveEvent[] = [
      { type: 'sow', pit: 6 },
      { type: 'skip_origin', pit: 5 },
      { type: 'sow', pit: 7 },
      { type: 'capture', pit: 7, seeds: 2 },
      { type: 'capture', pit: 6, seeds: 2 },
      { type: 'grand_slam' },
      { type: 'extra_turn' },
      { type: 'collect_remaining', player: 'north', seeds: 0 },
      { type: 'game_over', result: 'south_wins', reason: 'threshold' },
    ];
    const after = state([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], [15, 20], 'north');
    const frames = buildFrames(before, 5, events, after);

    expect(frames.map((f) => [f.kind, f.pit])).toEqual<[FrameKind, number | undefined][]>([
      ['pickup', 5],
      ['sow', 6],
      ['skip', 5],
      ['sow', 7],
      ['capture', 7],
      ['capture', 6],
      ['grand_slam', undefined],
      ['extra_turn', undefined],
      ['collect', undefined],
      ['end', undefined],
    ]);
    // Pickup empties the origin pit.
    expect(frames[0]!.pits[5]).toBe(0);
    expect(frames[0]!.player).toBe('south');
    // Each sow adds exactly one seed.
    expect(frames[1]!.pits[6]).toBe(2);
    expect(frames[3]!.pits[7]).toBe(2);
    // Captures empty the pit and credit the mover's store.
    expect(frames[4]!.pits[7]).toBe(0);
    expect(frames[4]!.stores).toEqual([12, 20]);
    expect(frames[4]!.seeds).toBe(2);
    expect(frames[5]!.stores).toEqual([14, 20]);
  });

  it('credits a capture by North to the North store', () => {
    const before = state([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1], [0, 0], 'north');
    const frames = buildFrames(
      before,
      11,
      [
        { type: 'sow', pit: 0 },
        { type: 'capture', pit: 0, seeds: 2 },
      ],
      state([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], [0, 2], 'south'),
    );
    expect(frames.find((f) => f.kind === 'capture')!.stores).toEqual([0, 2]);
  });

  it('collect_remaining empties that player’s row into their store', () => {
    const before = state([1, 2, 0, 0, 0, 3, 4, 0, 0, 0, 0, 5], [5, 5], 'south');
    const frames = buildFrames(
      before,
      0,
      [
        { type: 'sow', pit: 1 },
        { type: 'collect_remaining', player: 'north', seeds: 9 },
        { type: 'collect_remaining', player: 'south', seeds: 6 },
      ],
      state([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], [11, 14], 'north'),
    );
    const [northCollect, southCollect] = frames.filter((f) => f.kind === 'collect');
    expect(northCollect!.pits.slice(6)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(northCollect!.pits.slice(0, 6)).toEqual([0, 3, 0, 0, 0, 3]);
    expect(northCollect!.stores).toEqual([5, 14]);
    expect(northCollect!.player).toBe('north');
    expect(southCollect!.stores).toEqual([11, 14]);
  });

  it('ignores unknown future events and always ends on the engine’s resulting state', () => {
    const before = state([4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4], [0, 0], 'south');
    const after = state([0, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 4], [0, 0], 'north');
    const frames = buildFrames(
      before,
      0,
      [{ type: 'brand_new_event' } as unknown as MoveEvent, { type: 'sow', pit: 1 }],
      after,
    );
    expect(frames.map((f) => f.kind)).toEqual(['pickup', 'sow', 'end']);
    expect(frames.at(-1)!.pits).toEqual(after.pits);
    expect(frames.at(-1)!.stores).toEqual(after.stores);
  });

  it('never mutates the input state', () => {
    const before = state([4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4], [0, 0], 'south');
    const copy = structuredClone(before);
    buildFrames(before, 0, [{ type: 'sow', pit: 1 }], before);
    expect(before).toEqual(copy);
  });

  it('replaying the real engine’s events reproduces its resulting position, move by move', () => {
    // Plays whole games with the real core and checks that the frame just before `end` already
    // equals the engine's position. This catches any event the adapter mis-maps (sowing laps,
    // chains, grand slam, collection at the end).
    const core = loadCoreSync();
    const variant = defaultVariantRef(core);
    let movesChecked = 0;
    const kinds = new Set<FrameKind>();
    for (let game = 0; game < 25; game++) {
      let s = core.newGame(variant, game % 2 === 0 ? 'south' : 'north');
      for (let ply = 0; ply < 400 && s.status === 'playing'; ply++) {
        const pit = core.aiMove(variant, s, game % 3 === 0 ? 'medium' : 'easy', game * 1000 + ply);
        expect(pit).toBeDefined();
        const result = core.applyMove(variant, s, pit!);
        const frames = buildFrames(s, pit!, result.events, result.state);
        frames.forEach((f) => kinds.add(f.kind));
        const beforeEnd = frames.at(-2)!;
        expect({ pits: beforeEnd.pits, stores: beforeEnd.stores }).toEqual({
          pits: result.state.pits,
          stores: result.state.stores,
        });
        // Seeds are conserved once sowing is done (earlier frames have seeds "in hand").
        for (const f of frames.slice(-2)) {
          const total = f.pits.reduce((a, b) => a + b, 0) + f.stores[0] + f.stores[1];
          expect(total).toBe(48);
        }
        // Seeds in hand never go negative and never exceed what was picked up.
        const picked = s.pits[pit!]!;
        const onBoard = (f: (typeof frames)[number]) =>
          f.pits.reduce((a, b) => a + b, 0) + f.stores[0] + f.stores[1];
        for (const f of frames.slice(0, -2)) {
          expect(48 - onBoard(f)).toBeGreaterThanOrEqual(0);
          expect(48 - onBoard(f)).toBeLessThanOrEqual(picked);
        }
        s = result.state;
        movesChecked++;
      }
    }
    expect(movesChecked).toBeGreaterThan(100);
    expect(kinds).toContain('sow');
    expect(kinds).toContain('capture');
  });
});

describe('buildFrames with relay sowing (cv.continuous, ADR 0023)', () => {
  it('a relay picks the pit up again; frames still end on the engine’s position', () => {
    const core = loadCoreSync();
    const variant = variantRef(core, 'cv.continuous');
    expect(variant.id).toBe('cv.continuous');
    let relays = 0;
    let movesChecked = 0;
    for (let game = 0; game < 20; game++) {
      let s = core.newGame(variant, game % 2 === 0 ? 'south' : 'north');
      for (let ply = 0; ply < 400 && s.status === 'playing'; ply++) {
        const pit = core.aiMove(variant, s, 'easy', game * 1000 + ply)!;
        const result = core.applyMove(variant, s, pit);
        const frames = buildFrames(s, pit, result.events, result.state);
        const relayEvents = result.events.filter((e) => e.type === 'relay');
        relays += relayEvents.length;
        // Each relay is shown as a pickup of that pit, which is then empty.
        const pickups = frames.filter((f) => f.kind === 'pickup');
        expect(pickups).toHaveLength(1 + relayEvents.length);
        for (const [i, e] of relayEvents.entries()) {
          expect(pickups[i + 1]!.pit).toBe(e.type === 'relay' ? e.pit : -1);
          expect(pickups[i + 1]!.pits[pickups[i + 1]!.pit!]).toBe(0);
        }
        const beforeEnd = frames.at(-2)!;
        expect({ pits: beforeEnd.pits, stores: beforeEnd.stores }).toEqual({
          pits: result.state.pits,
          stores: result.state.stores,
        });
        // Seeds in hand are never negative (relays can hold more than the first pickup).
        for (const f of frames) {
          const onBoard = f.pits.reduce((a, b) => a + b, 0) + f.stores[0] + f.stores[1];
          expect(onBoard).toBeLessThanOrEqual(48);
        }
        s = result.state;
        movesChecked++;
      }
    }
    expect(movesChecked).toBeGreaterThan(100);
    expect(relays).toBeGreaterThan(20);
  });
});

describe('frameDelay', () => {
  it('is zero at speed 0 (tests / no animation) and scales with speed', () => {
    expect(frameDelay('sow', 0)).toBe(0);
    expect(frameDelay('sow', 2)).toBe(2 * frameDelay('sow', 1));
    expect(frameDelay('end', 1)).toBe(0);
    expect(frameDelay('capture', 1)).toBeGreaterThan(frameDelay('sow', 1));
  });
});
