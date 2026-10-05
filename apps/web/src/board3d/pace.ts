/** Board timing (ADR 0021). The board is always 3D for now. */

/**
 * How much slower move frames play on the 3D board than the base timings in
 * `game/animation.ts`: a seed needs time to visibly travel from pit to pit in the hand.
 */
export const PACE_3D = 2.2;

/** Multiplier for the game's animation speed. */
export function useBoardPace(): number {
  return PACE_3D;
}
