import { describe, expect, it } from 'vitest';

import { buildFrames } from '../game/animation';
import { loadCoreSync } from '../test/wasm';
import { initialPlacement, layoutFor, reconcile, targetCounts, type Placement } from './seeds';

const L = layoutFor(12);
const countIn = (p: Placement[], c: number) => p.filter((x) => x.container === c).length;

describe('seed placement', () => {
  it('deals seeds to match the counts, the hand holds the rest', () => {
    const counts = {
      pits: [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
      stores: [0, 0] as [number, number],
    };
    const p = initialPlacement(L, counts, 48);
    expect(p).toHaveLength(48);
    for (let i = 0; i < 12; i++) expect(countIn(p, i)).toBe(4);
    expect(countIn(p, L.hand)).toBe(0);
    expect(
      targetCounts(L, { pits: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], stores: [10, 30] }, 48)[L.hand],
    ).toBe(8);
  });

  it('pickup moves the pit into the hand, then sowing drops one seed from the hand', () => {
    const start = {
      pits: [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
      stores: [0, 0] as [number, number],
    };
    const p0 = initialPlacement(L, start, 48);
    const picked = reconcile(L, p0, { ...start, pits: [4, 4, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4] });
    expect(picked.moved).toHaveLength(4);
    expect(countIn(picked.placement, L.hand)).toBe(4);
    const sown = reconcile(L, picked.placement, {
      ...start,
      pits: [4, 4, 0, 5, 4, 4, 4, 4, 4, 4, 4, 4],
    });
    expect(sown.moved).toHaveLength(1);
    const [id] = sown.moved;
    expect(picked.placement[id!]!.container).toBe(L.hand); // it came from the hand
    expect(sown.placement[id!]).toEqual({ container: 3, slot: 4 }); // on top of the pile
  });

  it('replays every frame of a real move without losing or duplicating a seed', () => {
    const core = loadCoreSync();
    const config = core.defaultVariant('cv')!;
    const variant = { id: config.id, version: config.version };
    let state = core.newGame(variant, 'south');
    let placement = initialPlacement(L, state, 48);
    // A long sequence of legal moves, frame by frame, as the 3D board sees them.
    for (let ply = 0; ply < 40 && state.status === 'playing'; ply++) {
      const legal = core.legalMoves(variant, state);
      const move = legal[ply % legal.length]!;
      const result = core.applyMove(variant, state, move);
      for (const frame of buildFrames(state, move, result.events, result.state)) {
        placement = reconcile(L, placement, frame).placement;
        const t = targetCounts(L, frame, 48);
        for (let c = 0; c < L.containers; c++) expect(countIn(placement, c)).toBe(t[c]);
        // Slots in each container are 0..count-1 exactly once.
        for (let c = 0; c < L.containers; c++) {
          const slots = placement.filter((x) => x.container === c).map((x) => x.slot);
          expect(slots.sort((a, b) => a - b)).toEqual(slots.map((_, i) => i));
        }
      }
      state = result.state;
    }
  });
});
