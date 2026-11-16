/**
 * The sowing and capture choreography of the 3D board (ADR 0021, docs/architecture/web-design.md),
 * as plain state and functions: `applyFrame` plans a display frame (the hand's keyframes and how
 * each seed moves) and `stepChoreography` advances everything to a time. `Sowing.tsx` draws the
 * result with three.js. The Android app plays the same steps, and the motion vectors in
 * `shared/motion/` check that every platform does (docs/architecture/board-motion.md).
 *
 * A hand comes in from the mover's side of the table and plays the move the way a person does:
 *   pickup   it reaches into the pit, closes around the seeds and lifts them;
 *   sow      it glides to the next pit and lets one seed fall; the rest stay in its grip;
 *   skip     it passes over the origin pit;
 *   capture  it dips into the captured pit, grabs the seeds, carries them to the store and
 *            opens to let them fall in;
 *   end      it withdraws.
 * Other changes (collecting the remaining seeds, undo) fly seeds along an arc.
 *
 * The hand follows a small keyframe track rebuilt on every display frame, starting from where
 * it is now, so it never jumps, whatever the frame timing. Seeds come from the reconciler in
 * `seeds.ts`, so the board can never drift from the engine.
 */
import { Color, MathUtils, Quaternion, Vector3 } from 'three';

import { frameDelay, type FrameKind } from '../game/animation';
import { containerCenter, ease, slotPosition } from './geometry';
import { initialPlacement, reconcile, type Layout, type Placement } from './seeds';
import { rng } from './textures';

// --- Hand track ---------------------------------------------------------------------------------

/** Height of the handful (the centre of the held seeds) while carrying, and while grabbing. */
export const CARRY_Y = 0.8;
export const DIP_Y = 0.3;
/** How far off-board the hand enters from and withdraws to (towards the mover's side). */
export const OFFSTAGE = new Vector3(0, 2.6, 3.2);

export interface HandKey {
  at: number;
  pos: Vector3;
  /** 0 = open hand, 1 = closed around the seeds. */
  grip: number;
}

export interface HandState {
  keys: HandKey[];
  /** +1: South (comes in from the camera side), -1: North (from the far side). */
  side: 1 | -1;
  /** Sampled each rendered frame. */
  pos: Vector3;
  grip: number;
  /** The hand is drawn until this time (Infinity while a move is being played). */
  hideAt: number;
}

const smooth = (t: number) => t * t * (3 - 2 * t);
const easeOut = (t: number) => 1 - Math.pow(1 - t, 3);

export function sampleHand(h: HandState, now: number): void {
  const k = h.keys;
  if (k.length === 0) return;
  if (now <= k[0]!.at) {
    h.pos.copy(k[0]!.pos);
    h.grip = k[0]!.grip;
    return;
  }
  for (let i = 0; i < k.length - 1; i++) {
    const a = k[i]!;
    const b = k[i + 1]!;
    if (now < b.at) {
      const u = b.at > a.at ? (now - a.at) / (b.at - a.at) : 1;
      h.pos.lerpVectors(a.pos, b.pos, ease.inOut(u));
      h.grip = MathUtils.lerp(a.grip, b.grip, smooth(u));
      return;
    }
  }
  const last = k[k.length - 1]!;
  h.pos.copy(last.pos);
  h.grip = last.grip;
}

/** A seed's place in the handful, relative to the hand's anchor. */
function handOffset(slot: number, out: Vector3): Vector3 {
  const a = slot * 2.39996;
  const r = 0.075 * Math.sqrt(slot);
  return out.set(Math.cos(a) * r, slot * 0.012 - 0.04, Math.sin(a) * r);
}

// --- Seeds --------------------------------------------------------------------------------------

type SeedMode = 'rest' | 'fly' | 'carry';

export interface SeedMotion {
  mode: SeedMode;
  pos: Vector3;
  from: Vector3;
  to: Vector3;
  // fly
  start: number;
  dur: number;
  arc: number;
  // carry: joins the hand at `attach` (lifting for `lift` s), leaves it at `release`, then
  // falls for `fall` s into `to`.
  slot: number;
  attach: number;
  lift: number;
  release: number;
  fall: number;
  phase: 'wait' | 'lift' | 'held' | 'fall';
  /** The seed's tilt this frame, about its own x axis, on top of its resting orientation. */
  tumble: number;
}

function newMotion(at: Vector3): SeedMotion {
  return {
    mode: 'rest',
    pos: at.clone(),
    from: at.clone(),
    to: at.clone(),
    start: 0,
    dur: 0,
    arc: 0,
    slot: 0,
    attach: 0,
    lift: 0,
    release: Infinity,
    fall: 0,
    phase: 'wait',
    tumble: 0,
  };
}

/** What the board shows in one display frame. */
export interface BoardFrame {
  pits: readonly number[];
  stores: readonly [number, number];
  highlight: { pit?: number; kind: FrameKind } | null;
  /** The game's frame pace (0 = snap, no hand). */
  speed: number;
}

/** The hand and every seed, carried from one display frame to the next. */
export interface Choreography {
  placement: Placement[] | null;
  motions: SeedMotion[];
  hand: HandState;
}

export function createChoreography(): Choreography {
  return {
    placement: null,
    motions: [],
    hand: { keys: [], side: 1, pos: new Vector3(), grip: 0, hideAt: -Infinity },
  };
}

/** A new display frame: move the hand and choreograph the seeds that changed container. */
export function applyFrame(
  c: Choreography,
  layout: Layout,
  total: number,
  { pits, stores, highlight, speed }: BoardFrame,
  now: number,
): void {
  const counts = { pits, stores };
  const h = c.hand;
  const animate = speed > 0;
  const kind = highlight?.kind ?? 'end';
  const D = frameDelay(kind === 'end' ? 'sow' : kind, speed) / 1000;
  const pit = highlight?.pit;
  const over = (container: number, y: number) => containerCenter(container, layout).setY(y);
  const offstage = (p: Vector3, side: number) =>
    p.clone().add(new Vector3(OFFSTAGE.x, OFFSTAGE.y, OFFSTAGE.z * side));

  if (!c.placement || c.placement.length !== total) {
    c.placement = initialPlacement(layout, counts, total);
    c.motions = c.placement.map((p) => newMotion(slotPosition(p, layout)));
    h.keys = [];
    h.hideAt = -Infinity;
    return;
  }

  const prev = c.placement;
  const { placement: next, moved } = reconcile(layout, prev, counts);
  c.placement = next;
  const movedSet = new Set(moved);
  const handCount = next.filter((p) => p.container === layout.hand).length;

  // --- The hand's keyframes for this frame.
  if (!animate) {
    h.keys = [];
    h.hideAt = -Infinity;
  } else {
    sampleHand(h, now);
    let start: HandKey = { at: now, pos: h.pos.clone(), grip: h.grip };
    const onStage = now < h.hideAt;
    const key = (f: number, pos: Vector3, grip: number): HandKey => ({
      at: now + f * D,
      pos,
      grip,
    });
    if (kind === 'pickup' && pit !== undefined) {
      const side = pit < layout.n ? 1 : -1;
      if (!onStage || side !== h.side) {
        h.side = side;
        start = { at: now, pos: offstage(over(pit, CARRY_Y), side), grip: 0 };
      }
      h.keys = [
        start,
        key(0.42, over(pit, DIP_Y), 0),
        key(0.58, over(pit, DIP_Y), 1),
        key(1, over(pit, CARRY_Y), 1),
      ];
      h.hideAt = Infinity;
    } else if (kind === 'sow' && pit !== undefined) {
      const above = over(pit, CARRY_Y);
      h.keys = [
        start,
        key(0.5, above, 1),
        key(0.6, above, 0.45),
        key(0.9, above, handCount > 0 ? 1 : 0.45),
      ];
      h.hideAt = Infinity;
    } else if (kind === 'skip' && pit !== undefined) {
      h.keys = [start, key(0.8, over(pit, CARRY_Y), 1)];
      h.hideAt = Infinity;
    } else if (kind === 'capture' && pit !== undefined) {
      const store = h.side > 0 ? layout.storeSouth : layout.storeNorth;
      if (!onStage) start = { at: now, pos: offstage(over(pit, CARRY_Y), h.side), grip: 0 };
      h.keys = [
        start,
        key(0.24, over(pit, DIP_Y), 0),
        key(0.38, over(pit, DIP_Y), 1),
        key(0.5, over(pit, CARRY_Y), 1),
        key(0.8, over(store, CARRY_Y + 0.1), 1),
        key(0.9, over(store, CARRY_Y + 0.1), 0.1),
      ];
      h.hideAt = Infinity;
    } else if (kind === 'grand_slam' || kind === 'extra_turn') {
      // The hand pauses where it is.
    } else if (onStage) {
      // The move is over: the hand withdraws to the mover's side.
      const exit = Math.max(0.45, 0.32 * speed);
      h.keys = [start, { at: now + exit, pos: offstage(start.pos, h.side), grip: 0.3 }];
      h.hideAt = now + exit;
    }
  }

  // --- The seeds.
  let lifted = 0;
  let captured = 0;
  const lifting = moved.filter((id) => next[id]!.container === layout.hand).length;
  let order = 0;
  next.forEach((p, id) => {
    const m = c.motions[id]!;
    const was = prev[id]!;
    const toHand = p.container === layout.hand;
    const fromHand = was.container === layout.hand;

    if (!animate) {
      m.mode = 'rest';
      m.to.copy(toHand ? over(pit ?? 0, CARRY_Y) : slotPosition(p, layout));
      m.pos.copy(m.to);
      return;
    }

    if (toHand) {
      m.slot = p.slot;
      if (!fromHand) {
        // Pickup: the seeds join the closing hand one after another.
        const stagger = lifting > 1 ? Math.min(0.03, (D * 0.25) / lifting) : 0;
        m.mode = 'carry';
        m.phase = 'wait';
        m.attach = now + 0.55 * D + lifted++ * stagger;
        m.lift = 0.4 * D;
        m.release = Infinity;
      }
      return;
    }

    const to = slotPosition(p, layout);
    if (fromHand && movedSet.has(id)) {
      // Sowing: this seed slips out of the hand over its pit.
      m.mode = 'carry';
      m.to.copy(to);
      m.release = now + 0.56 * D;
      m.fall = Math.max(0.16, 0.38 * D);
      return;
    }

    if (kind === 'capture' && movedSet.has(id)) {
      // Capture: grabbed from the pit, carried to the store, dropped in.
      m.mode = 'carry';
      m.phase = 'wait';
      m.to.copy(to);
      m.slot = captured;
      m.attach = now + 0.36 * D + captured * 0.012;
      m.lift = 0.14 * D;
      m.release = now + 0.88 * D + captured * 0.018;
      m.fall = Math.max(0.14, 0.14 * D);
      captured++;
      return;
    }

    if (to.distanceToSquared(m.to) < 1e-6 && m.mode !== 'carry') return;
    // Collecting, undo, resume: an arc to the new container. Slot shuffles: a short hop.
    const between = movedSet.has(id) || fromHand;
    const stagger = between && moved.length > 1 ? Math.min(0.035, (D * 0.4) / moved.length) : 0;
    m.mode = 'fly';
    m.from.copy(m.pos);
    m.to.copy(to);
    m.start = now + (between ? order++ * stagger : 0);
    m.dur = between ? Math.max(0.24, D * 0.9) : 0.15;
    m.arc = between ? 0.35 + m.from.distanceTo(to) * 0.1 : 0.03;
  });
}

const scratch = new Vector3();

/**
 * Advances the hand and every seed to `now` (`delta`: seconds since the last call, for the seeds
 * that follow the hand). Returns whether anything is still moving.
 */
export function stepChoreography(c: Choreography, now: number, delta: number): boolean {
  const h = c.hand;
  sampleHand(h, now);
  const handShown = now < h.hideAt;
  let active = handShown && (h.hideAt !== Infinity || now < (h.keys.at(-1)?.at ?? 0) + 0.1);
  const follow = 1 - Math.exp(-delta * 18);

  c.motions.forEach((mo, i) => {
    let tumble = 0;
    const inHand = () => {
      handOffset(mo.slot, scratch).add(h.pos);
      // Seeds sway a touch in the grip; only while the hand holds them.
      scratch.y += Math.sin(now * 5 + i * 1.7) * 0.006;
      return scratch;
    };

    if (mo.mode === 'rest') {
      mo.pos.copy(mo.to);
    } else if (mo.mode === 'fly') {
      const t = mo.dur <= 0 ? 1 : MathUtils.clamp((now - mo.start) / mo.dur, 0, 1);
      const e = ease.inOut(t);
      mo.pos.lerpVectors(mo.from, mo.to, e);
      mo.pos.y += Math.sin(Math.PI * e) * mo.arc;
      if (mo.arc > 0.1) tumble = e * Math.PI * 0.6;
      if (t < 1) active = true;
      else mo.mode = 'rest';
    } else {
      active = true;
      if (mo.phase === 'wait' && now >= mo.attach) {
        mo.phase = 'lift';
        mo.from.copy(mo.pos);
      }
      if ((mo.phase === 'lift' || mo.phase === 'held') && now >= mo.release) {
        mo.phase = 'fall';
        mo.from.copy(mo.pos);
      }
      if (mo.phase === 'lift') {
        const u = mo.lift <= 0 ? 1 : MathUtils.clamp((now - mo.attach) / mo.lift, 0, 1);
        mo.pos.lerpVectors(mo.from, inHand(), easeOut(u));
        mo.pos.y += Math.sin(Math.PI * u) * 0.08;
        tumble = u * Math.PI * 0.4;
        if (u >= 1) mo.phase = 'held';
      } else if (mo.phase === 'held') {
        mo.pos.lerp(inHand(), follow);
      } else if (mo.phase === 'fall') {
        const t = MathUtils.clamp((now - mo.release) / mo.fall, 0, 1);
        // Accelerating fall into the bowl, then a small bounce.
        const land = 0.82;
        if (t < land) {
          const f = t / land;
          mo.pos.lerpVectors(mo.from, mo.to, easeOut(f));
          mo.pos.y = MathUtils.lerp(mo.from.y, mo.to.y, f * f);
        } else {
          const b = (t - land) / (1 - land);
          mo.pos.copy(mo.to);
          mo.pos.y += Math.sin(Math.PI * b) * 0.05;
        }
        tumble = t * Math.PI * 0.8;
        if (t >= 1) {
          mo.mode = 'rest';
          mo.phase = 'wait';
          mo.release = Infinity;
        }
      }
      // 'wait': the seed stays where it is until the hand closes on it.
    }
    mo.tumble = tumble;
  });
  return active;
}

// --- How each seed looks ------------------------------------------------------------------------

const SEED_COLORS = ['#8b8d84', '#7a7f75', '#9b9a8e', '#6e7268', '#a5a092', '#868372', '#757a6c'];

/** Each seed's resting orientation, the same on every load. */
export function seedOrientations(total: number): Quaternion[] {
  return Array.from({ length: total }, (_, i) => {
    const r = rng(i * 97 + 3);
    return new Quaternion().setFromAxisAngle(
      new Vector3(r() - 0.5, 1, r() - 0.5).normalize(),
      r() * Math.PI * 2,
    );
  });
}

/** Each seed's colour (linear RGB, three values per seed), the same on every load. */
export function seedColors(total: number): Float32Array {
  const r = rng(42);
  const arr = new Float32Array(total * 3);
  for (let i = 0; i < total; i++) {
    const c = new Color(SEED_COLORS[Math.floor(r() * SEED_COLORS.length)]!);
    c.offsetHSL(0, 0, (r() - 0.5) * 0.06);
    c.toArray(arr, i * 3);
  }
  return arr;
}
