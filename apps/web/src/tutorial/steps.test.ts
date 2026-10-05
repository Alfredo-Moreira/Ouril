import { describe, expect, it } from 'vitest';

import { defaultVariantRef, variantRef } from '../game/session';
import { loadCoreSync } from '../test/wasm';
import { TUTORIAL_STEPS } from './steps';

const core = loadCoreSync();
const variant = defaultVariantRef(core);

const total = (s: { pits: number[]; stores: [number, number] }) =>
  s.pits.reduce((a, b) => a + b, 0) + s.stores[0] + s.stores[1];

describe('tutorial positions (checked against the real engine)', () => {
  it.each(TUTORIAL_STEPS.map((s) => [s.id, s] as const))(
    '%s has 48 seeds and allowed moves are legal',
    (_, step) => {
      expect(total(step.position)).toBe(48);
      const rules = variantRef(core, step.variant);
      expect(rules.id).toBe(step.variant ?? 'cv.standard');
      let state = step.position;
      for (const allowed of step.moves) {
        const legal = core.legalMoves(rules, state);
        const pit = allowed === 'any' ? legal[0]! : allowed[0]!;
        if (allowed !== 'any') allowed.forEach((p) => expect(legal).toContain(p));
        state = core.applyMove(rules, state, pit).state;
      }
    },
  );

  const play = (id: string, pit: number) => {
    const step = TUTORIAL_STEPS.find((s) => s.id === id)!;
    return core.applyMove(variant, step.position, pit);
  };

  it('capture: captures 2 from North pit 6', () => {
    const r = play('capture', 4);
    expect(r.events).toContainEqual({ type: 'capture', pit: 6, seeds: 2 });
  });

  it('chain: captures pit 7 (3) then pit 6 (2)', () => {
    const caps = play('chain', 4).events.filter((e) => e.type === 'capture');
    expect(caps).toEqual([
      { type: 'capture', pit: 7, seeds: 3 },
      { type: 'capture', pit: 6, seeds: 2 },
    ]);
  });

  it('lap: skips the origin pit', () => {
    const r = play('lap', 0);
    expect(r.events).toContainEqual({ type: 'skip_origin', pit: 0 });
    expect(r.state.pits[0]).toBe(0);
  });

  it('single_seed: only pits 2 and 5 are legal', () => {
    const step = TUTORIAL_STEPS.find((s) => s.id === 'single_seed')!;
    expect(core.legalMoves(variant, step.position)).toEqual([2, 5]);
  });

  it('feeding: only pit 5 is legal', () => {
    const step = TUTORIAL_STEPS.find((s) => s.id === 'feeding')!;
    expect(core.legalMoves(variant, step.position)).toEqual([5]);
  });

  it('grand_slam: grand slam, extra turn, then only pit 5', () => {
    const r = play('grand_slam', 4);
    expect(r.events.map((e) => e.type)).toEqual(
      expect.arrayContaining(['grand_slam', 'extra_turn']),
    );
    expect(r.state.to_move).toBe('south');
    expect(core.legalMoves(variant, r.state)).toEqual([5]);
  });

  const playContinuous = (id: string, pit: number) => {
    const step = TUTORIAL_STEPS.find((s) => s.id === id)!;
    return core.applyMove(variantRef(core, 'cv.continuous'), step.position, pit);
  };

  it('continuous: the opening from pit 1 carries on from pit 5 into North', () => {
    const r = playContinuous('continuous', 0);
    expect(r.events).toContainEqual({ type: 'relay', pit: 4, seeds: 5 });
    expect(r.state.pits.slice(6)).toEqual([5, 5, 5, 5, 4, 4]);
  });

  it('across: three seeds into empty pits 2-4 capture across, chained back to pit 2', () => {
    const step = TUTORIAL_STEPS.find((s) => s.id === 'across')!;
    const r = core.applyMove(variantRef(core, 'cv.across'), step.position, 0);
    expect(r.events.filter((e) => e.type === 'capture')).toEqual([
      { type: 'capture', pit: 8, seeds: 4 },
      { type: 'capture', pit: 9, seeds: 4 },
      { type: 'capture', pit: 10, seeds: 4 },
    ]);
  });

  it('continuous_capture: a lone seed relays from pit 5 and captures 2 in North pit 1', () => {
    const r = playContinuous('continuous_capture', 3);
    expect(r.events).toContainEqual({ type: 'relay', pit: 4, seeds: 2 });
    expect(r.events).toContainEqual({ type: 'capture', pit: 6, seeds: 2 });
  });
});
