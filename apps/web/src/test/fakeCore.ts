/**
 * Test helper: a tiny scripted stand-in for the WASM core, so UI tests can check how the app
 * talks to the engine (which calls, with which arguments) without depending on real rules.
 *
 * Fake rules (NOT Ouril rules, only for wiring tests):
 * - 2×6 board. A pit is legal for the side to move if it is on that side and non-empty.
 * - Playing a pit puts all its seeds straight into the mover's store (`capture` event on that
 *   pit) and passes the turn.
 * - The first side to reach `threshold` seeds in its store wins (`game_over`, `threshold`).
 * - The fake AI always plays its lowest legal pit.
 */
import { vi, type Mock } from 'vitest';

import type {
  CoreApi,
  GameRecord,
  GameState,
  MoveEvent,
  MoveResult,
  Player,
  Stats,
  VariantConfig,
  VariantRef,
} from '../engine/types';

export const FAKE_VARIANT: VariantRef = { id: 'fake.test', version: 7 };

export interface FakeCoreOptions {
  /** Initial pits (12 entries). */
  pits?: number[];
  threshold?: number;
  /** What deriveStats returns (defaults to counting records by outcome for South). */
  stats?: Stats;
}

export type FakeCore = { [K in keyof CoreApi]: Mock<CoreApi[K]> };

const other = (p: Player): Player => (p === 'south' ? 'north' : 'south');

export function createFakeCore(options: FakeCoreOptions = {}): FakeCore {
  const initial = options.pits ?? [4, 0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4];
  const threshold = options.threshold ?? 8;
  const config = { id: FAKE_VARIANT.id, version: FAKE_VARIANT.version } as VariantConfig;

  const legal = (state: GameState): number[] => {
    if (state.status !== 'playing') return [];
    const start = state.to_move === 'south' ? 0 : 6;
    const out: number[] = [];
    for (let i = start; i < start + 6; i++) if ((state.pits[i] ?? 0) > 0) out.push(i);
    return out;
  };

  const newGame = (_v: VariantRef, first: Player): GameState => ({
    pits: [...initial],
    stores: [0, 0],
    to_move: first,
    moves_since_capture: 0,
    status: 'playing',
  });

  const apply = (_v: VariantRef, state: GameState, pit: number): MoveResult => {
    if (!legal(state).includes(pit)) throw new Error('not_own_pit');
    const mover = state.to_move;
    const seeds = state.pits[pit]!;
    const pits = [...state.pits];
    pits[pit] = 0;
    const stores: [number, number] = [state.stores[0], state.stores[1]];
    stores[mover === 'south' ? 0 : 1] += seeds;
    const events: MoveEvent[] = [{ type: 'capture', pit, seeds }];
    let status: GameState['status'] = 'playing';
    if (stores[mover === 'south' ? 0 : 1] >= threshold) {
      status = mover === 'south' ? 'south_wins' : 'north_wins';
      events.push({ type: 'game_over', result: status, reason: 'threshold' });
    }
    return {
      state: { pits, stores, to_move: other(mover), moves_since_capture: 0, status },
      events,
    };
  };

  const core = {
    coreVersion: vi.fn(() => '9.9.9-fake'),
    variants: vi.fn(() => [config]),
    defaultVariant: vi.fn(() => config),
    newGame: vi.fn(newGame),
    legalMoves: vi.fn((_v: VariantRef, s: GameState) => legal(s)),
    applyMove: vi.fn(apply),
    aiMove: vi.fn((_v: VariantRef, s: GameState) => legal(s)[0]),
    replay: vi.fn((v: VariantRef, first: Player, moves: number[]) => {
      const start = newGame(v, first);
      const steps: MoveResult[] = [];
      let s = start;
      for (const m of moves) {
        const r = apply(v, s, m);
        steps.push(r);
        s = r.state;
      }
      return { initial: start, steps };
    }),
    deriveStats: vi.fn((records: GameRecord[]): Stats => {
      if (options.stats) return options.stats;
      const wins = records.filter((r) => r.result.outcome === 'south_wins').length;
      const draws = records.filter((r) => r.result.outcome === 'draw').length;
      const overall = {
        played: records.length,
        wins,
        losses: records.length - wins - draws,
        draws,
      };
      return { overall, by_level: {}, current_streak: 0, best_streak: 0 };
    }),
  };
  return core as unknown as FakeCore;
}

/** A finished-game record (shape of core/wasm/API.md format 1). */
export function makeRecord(overrides: Partial<GameRecord> = {}): GameRecord {
  const endedAt = overrides.ended_at ?? '2026-01-02T10:00:00.000Z';
  return {
    format: 1,
    id: overrides.id ?? `rec-${Math.random().toString(36).slice(2)}`,
    variant: { id: 'cv.standard', version: 1 },
    first_player: 'south',
    moves: [2],
    core_version: '0.1.0',
    result: { outcome: 'south_wins', stores: [25, 10], reason: 'threshold' },
    started_at: endedAt,
    ended_at: endedAt,
    mode: 'vs_ai',
    ai_level: 'easy',
    human_player: 'south',
    ...overrides,
  };
}
