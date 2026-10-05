/**
 * Turns the engine's move events into display frames (engine.md: "`events` lets every UI
 * animate each seed drop and capture exactly as the engine computed it").
 *
 * This is presentation only: the last frame is always the engine's resulting state, so the
 * board can never drift from the rules even if an event type is unknown.
 */
import type { GameState, MoveEvent, Player } from '../engine/types';

export type FrameKind =
  'pickup' | 'sow' | 'skip' | 'capture' | 'grand_slam' | 'extra_turn' | 'collect' | 'end';

export interface Frame {
  kind: FrameKind;
  pits: number[];
  stores: [number, number];
  /** Pit to highlight in this frame (sown, skipped or captured). */
  pit?: number;
  /** Seeds captured or collected in this frame. */
  seeds?: number;
  player?: Player;
}

const storeIndex = (p: Player) => (p === 'south' ? 0 : 1);

export function buildFrames(
  before: GameState,
  pit: number,
  events: MoveEvent[],
  after: GameState,
): Frame[] {
  const mover = before.to_move;
  const pits = [...before.pits];
  const stores: [number, number] = [before.stores[0], before.stores[1]];
  const snap = (f: Omit<Frame, 'pits' | 'stores'>): Frame => ({
    ...f,
    pits: [...pits],
    stores: [stores[0], stores[1]],
  });
  const frames: Frame[] = [];

  // Seeds leave the chosen pit before sowing.
  pits[pit] = 0;
  frames.push(snap({ kind: 'pickup', pit, player: mover }));

  for (const e of events) {
    switch (e.type) {
      case 'sow':
        pits[e.pit] = (pits[e.pit] ?? 0) + 1;
        frames.push(snap({ kind: 'sow', pit: e.pit }));
        break;
      case 'skip_origin':
        frames.push(snap({ kind: 'skip', pit: e.pit }));
        break;
      case 'relay':
        // Relay sowing (ADR 0023): the seeds of an own pit are picked up and sown on. Shown
        // like the opening pickup (the 3D hand reaches in and grabs them).
        pits[e.pit] = 0;
        frames.push(snap({ kind: 'pickup', pit: e.pit, player: mover }));
        break;
      case 'capture':
        pits[e.pit] = 0;
        stores[storeIndex(mover)] += e.seeds;
        frames.push(snap({ kind: 'capture', pit: e.pit, seeds: e.seeds, player: mover }));
        break;
      case 'grand_slam':
        frames.push(snap({ kind: 'grand_slam', player: mover }));
        break;
      case 'extra_turn':
        frames.push(snap({ kind: 'extra_turn', player: mover }));
        break;
      case 'collect_remaining': {
        const n = pits.length / 2;
        const start = e.player === 'south' ? 0 : n;
        for (let i = start; i < start + n; i++) pits[i] = 0;
        stores[storeIndex(e.player)] += e.seeds;
        frames.push(snap({ kind: 'collect', seeds: e.seeds, player: e.player }));
        break;
      }
      default:
        // game_over and unknown future events: no board change.
        break;
    }
  }

  frames.push({ kind: 'end', pits: [...after.pits], stores: [after.stores[0], after.stores[1]] });
  return frames;
}

/** Milliseconds to show each frame kind. 0 = instant (reduced motion, tests). */
export function frameDelay(kind: FrameKind, speed: number): number {
  if (speed <= 0) return 0;
  const base: Record<FrameKind, number> = {
    pickup: 320,
    sow: 200,
    skip: 260,
    // Long enough for the 3D hand to grab the seeds and carry them to the store.
    capture: 600,
    grand_slam: 700,
    extra_turn: 500,
    collect: 500,
    end: 0,
  };
  return base[kind] * speed;
}
