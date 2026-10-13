/**
 * Online games (ADR 0031) are shown from your side: your pits along the bottom, where South
 * sits. The engine and the server always use real seats; these helpers only map between real
 * pits and the pits drawn on the board. Mapping twice gives back the original pit.
 */
import type { Seat } from '../api/types';

/** Real pit ↔ displayed pit, for a board with `perSide` pits per side. */
export function flipPit(pit: number, you: Seat, perSide: number): number {
  return you === 'south' ? pit : (pit + perSide) % (2 * perSide);
}

/** The position as drawn for `you`: pits rotated and `[yours, theirs]` stores. */
export function displayPosition(
  pits: readonly number[],
  stores: readonly [number, number],
  you: Seat,
): { pits: number[]; stores: [number, number] } {
  if (you === 'south') return { pits: [...pits], stores: [stores[0], stores[1]] };
  const perSide = pits.length / 2;
  return {
    pits: pits.map((_, i) => pits[flipPit(i, you, perSide)]!),
    stores: [stores[1], stores[0]],
  };
}

/** Your result from a finished game's outcome (`south_wins`, `north_wins` or `draw`). */
export function resultFor(outcome: string, you: Seat): 'win' | 'loss' | 'draw' {
  if (outcome === 'draw') return 'draw';
  return outcome === `${you}_wins` ? 'win' : 'loss';
}
