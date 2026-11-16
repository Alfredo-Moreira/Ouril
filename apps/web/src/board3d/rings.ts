/**
 * The glowing rings on the pits' rims (ADR 0021): which colour and strength each one aims for,
 * and how fast it gets there (a fast attack, a slower release). Plain state and functions, so
 * every platform lights the pits the same way (checked by the motion vectors in `shared/motion/`).
 */
import { Color, MathUtils } from 'three';

import type { FrameKind } from '../game/animation';

export interface RingTarget {
  color: string;
  a: number;
}

export interface RingInputs {
  legal: readonly number[];
  highlight?: { pit?: number; kind: FrameKind } | null;
  hintPit?: number | null;
  /** Pit focused through the accessible (keyboard / screen-reader) controls. */
  focusPit?: number | null;
  /** Legal pit under the mouse. */
  hover?: number | null;
}

/**
 * Target glow per pit: colour + strength. Star yellow means "you can play this" (and the sowing
 * pulse); a capture flashes in the capturer's light tint (a pit on the North row is taken by
 * South, you); focus is cotton white; the hint is sea. Every ring keeps 3:1 or more against the
 * wood, the resting playable ring included.
 */
export function ringTargets(
  n: number,
  { legal, highlight, hintPit, focusPit, hover }: RingInputs,
): RingTarget[] {
  return Array.from({ length: n * 2 }, (_, i) => {
    const isLegal = legal.includes(i);
    if (highlight?.pit === i && highlight.kind === 'capture')
      return { color: i >= n ? '#b4beff' : '#ffb4b4', a: 1 };
    if (highlight?.pit === i && (highlight.kind === 'sow' || highlight.kind === 'pickup'))
      return { color: '#f7d116', a: 0.85 };
    if (highlight?.pit === i && highlight.kind === 'skip') return { color: '#4fd1dc', a: 0.8 };
    if (focusPit === i) return { color: '#f3efe6', a: 1 };
    if (hintPit === i) return { color: '#4fd1dc', a: 0.95 };
    if (isLegal && hover === i) return { color: '#ffe45c', a: 1 };
    if (isLegal) return { color: '#f7d116', a: 0.75 };
    return { color: '#f7d116', a: 0 };
  });
}

/** A ring as drawn: its opacity and its colour (linear RGB, also its glow). */
export interface RingState {
  opacity: number;
  color: Color;
}

export function createRings(count: number): RingState[] {
  return Array.from({ length: count }, () => ({ opacity: 0, color: new Color('#ffc861') }));
}

const targetColor = new Color();

/** Eases every ring towards its target. Returns whether any is still changing. */
export function stepRings(
  rings: RingState[],
  targets: readonly RingTarget[],
  dt: number,
  speed: number,
): boolean {
  let active = false;
  rings.forEach((ring, i) => {
    const t = targets[i]!;
    // Fast attack, slower release (asymmetric timing).
    const rising = t.a > ring.opacity;
    const k = speed > 0 ? 1 - Math.exp(-dt * (rising ? 22 : 9)) : 1;
    ring.opacity = MathUtils.lerp(ring.opacity, t.a, k);
    ring.color.lerp(targetColor.set(t.color), k);
    if (Math.abs(ring.opacity - t.a) > 0.004) active = true;
  });
  return active;
}
