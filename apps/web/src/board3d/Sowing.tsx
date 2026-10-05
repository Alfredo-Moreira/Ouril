/**
 * The player's hand and the seeds: the sowing and capture choreography of the 3D board
 * (ADR 0021, docs/architecture/web-design.md).
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
 * it is now, so it never jumps, whatever the frame timing. The hand is procedural (capsules and
 * a rounded palm): nothing is downloaded. Seeds come from the reconciler in `seeds.ts`, so the
 * board can never drift from the engine.
 */
import { RoundedBox } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useMemo, useRef, type RefObject } from 'react';
import {
  Color,
  InstancedBufferAttribute,
  MathUtils,
  MeshStandardMaterial,
  Object3D,
  Quaternion,
  Vector3,
  type Group,
  type InstancedMesh,
} from 'three';

import { frameDelay, type FrameKind } from '../game/animation';
import { useClockDelta } from './clock';
import { containerCenter, ease, slotPosition } from './geometry';
import { initialPlacement, reconcile, type Layout, type Placement } from './seeds';
import { rng } from './textures';

// --- Hand track ---------------------------------------------------------------------------------

/** Height of the handful (the centre of the held seeds) while carrying, and while grabbing. */
const CARRY_Y = 0.8;
const DIP_Y = 0.3;
/** How far off-board the hand enters from and withdraws to (towards the mover's side). */
const OFFSTAGE = new Vector3(0, 2.6, 3.2);

interface HandKey {
  at: number;
  pos: Vector3;
  /** 0 = open hand, 1 = closed around the seeds. */
  grip: number;
}

interface HandState {
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

function sampleHand(h: HandState, now: number): void {
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

interface SeedMotion {
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
}

const SEED_COLORS = ['#8b8d84', '#7a7f75', '#9b9a8e', '#6e7268', '#a5a092', '#868372', '#757a6c'];

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
  };
}

export interface SowingProps {
  layout: Layout;
  total: number;
  pits: number[];
  stores: [number, number];
  highlight: { pit?: number; kind: FrameKind } | null;
  /** The game's frame pace (0 = snap, no hand). */
  speed: number;
}

export function Sowing({ layout, total, pits, stores, highlight, speed }: SowingProps) {
  const mesh = useRef<InstancedMesh>(null);
  const clock = useThree((s) => s.clock);
  const invalidate = useThree((s) => s.invalidate);
  const placement = useRef<Placement[] | null>(null);
  const motions = useRef<SeedMotion[]>([]);
  const hand = useRef<HandState>({
    keys: [],
    side: 1,
    pos: new Vector3(),
    grip: 0,
    hideAt: -Infinity,
  });
  const orientations = useMemo(
    () =>
      Array.from({ length: total }, (_, i) => {
        const r = rng(i * 97 + 3);
        return new Quaternion().setFromAxisAngle(
          new Vector3(r() - 0.5, 1, r() - 0.5).normalize(),
          r() * Math.PI * 2,
        );
      }),
    [total],
  );

  // A new display frame: move the hand and choreograph the seeds that changed container.
  useEffect(() => {
    const counts = { pits, stores };
    const now = clock.getElapsedTime();
    const h = hand.current;
    const animate = speed > 0;
    const kind = highlight?.kind ?? 'end';
    const D = frameDelay(kind === 'end' ? 'sow' : kind, speed) / 1000;
    const pit = highlight?.pit;
    const over = (c: number, y: number) => containerCenter(c, layout).setY(y);
    const offstage = (p: Vector3, side: number) =>
      p.clone().add(new Vector3(OFFSTAGE.x, OFFSTAGE.y, OFFSTAGE.z * side));

    if (!placement.current || placement.current.length !== total) {
      placement.current = initialPlacement(layout, counts, total);
      motions.current = placement.current.map((p) => newMotion(slotPosition(p, layout)));
      h.keys = [];
      h.hideAt = -Infinity;
      invalidate();
      return;
    }

    const prev = placement.current;
    const { placement: next, moved } = reconcile(layout, prev, counts);
    placement.current = next;
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
      const m = motions.current[id]!;
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
    invalidate();
  }, [pits, stores, highlight, layout, total, speed, clock, invalidate]);

  // Per-seed colour, attached as `instanceColor` from the first render so the material
  // compiles with instance colours (setting them later leaves the seeds white).
  const colorAttr = useMemo(() => {
    const r = rng(42);
    const arr = new Float32Array(total * 3);
    for (let i = 0; i < total; i++) {
      const c = new Color(SEED_COLORS[Math.floor(r() * SEED_COLORS.length)]!);
      c.offsetHSL(0, 0, (r() - 0.5) * 0.06);
      c.toArray(arr, i * 3);
    }
    return new InstancedBufferAttribute(arr, 3);
  }, [total]);

  const tmp = useMemo(
    () => ({ s: new Vector3(1.18, 0.78, 0.95), o: new Object3D(), slot: new Vector3() }),
    [],
  );
  const clockDelta = useClockDelta();

  useFrame((state) => {
    const m = mesh.current;
    if (!m) return;
    const now = state.clock.getElapsedTime();
    const delta = clockDelta(now);
    const h = hand.current;
    sampleHand(h, now);
    const handShown = now < h.hideAt;
    let active = handShown && (h.hideAt !== Infinity || now < (h.keys.at(-1)?.at ?? 0) + 0.1);
    const follow = 1 - Math.exp(-delta * 18);

    motions.current.forEach((mo, i) => {
      let tumble = 0;
      const inHand = () => {
        handOffset(mo.slot, tmp.slot).add(h.pos);
        // Seeds sway a touch in the grip; only while the hand holds them.
        tmp.slot.y += Math.sin(now * 5 + i * 1.7) * 0.006;
        return tmp.slot;
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

      tmp.o.position.copy(mo.pos);
      tmp.o.quaternion.copy(orientations[i]!);
      if (tumble) tmp.o.rotateX(tumble);
      tmp.o.scale.copy(tmp.s);
      tmp.o.updateMatrix();
      m.setMatrixAt(i, tmp.o.matrix);
    });
    m.instanceMatrix.needsUpdate = true;
    if (active) state.invalidate();
  });

  return (
    <>
      <instancedMesh
        ref={mesh}
        args={[undefined, undefined, total]}
        instanceColor={colorAttr}
        castShadow
        receiveShadow
      >
        <sphereGeometry args={[0.105, 20, 14]} />
        <meshPhysicalMaterial roughness={0.32} clearcoat={0.7} clearcoatRoughness={0.25} />
      </instancedMesh>
      <HandModel hand={hand} />
    </>
  );
}

// --- The hand model -----------------------------------------------------------------------------

interface HandRig {
  root: Group | null;
  knuckles: (Group | null)[];
  tips: (Group | null)[];
  thumb: Group | null;
}

/**
 * A right hand, palm down, in model units (the model is scaled by `HAND_SCALE`): index to
 * little finger, x across the palm and the two segment lengths. A real palm is about the size
 * of a pit, so the hand is drawn at that scale.
 */
const FINGERS = [
  { x: -0.17, len: [0.21, 0.17] },
  { x: -0.06, len: [0.24, 0.19] },
  { x: 0.055, len: [0.23, 0.18] },
  { x: 0.165, len: [0.18, 0.14] },
] as const;
const FINGER_R = 0.052;
const HAND_SCALE = 1.7;
/** The hand's anchor is the centre of the handful; the palm sits above it (model units). */
const PALM_Y = 0.18;

function poseHand(rig: HandRig, h: HandState, shown: boolean) {
  const root = rig.root;
  if (!root) return;
  root.visible = shown;
  if (!shown) return;
  root.position.copy(h.pos);
  // South's hand reaches in from the camera side, fingers pointing away; North's mirrors it.
  root.rotation.set(0.08 + h.grip * 0.1, h.side > 0 ? 0 : Math.PI, 0);
  const g = h.grip;
  rig.knuckles.forEach((k) => {
    if (k) k.rotation.x = -(0.35 + g * 0.95);
  });
  rig.tips.forEach((t) => {
    if (t) t.rotation.x = -(0.25 + g * 1.05);
  });
  if (rig.thumb) {
    rig.thumb.rotation.set(-g * 0.5, 0.6 - g * 0.8, 0);
  }
}

const noRaycast = () => null;

/** Sleeve colours from the flag: blue for South (you), red for North (the computer). */
const SLEEVE = { south: '#1f4f8f', north: '#c4252b' } as const;

function HandModel({ hand }: { hand: RefObject<HandState> }) {
  const rig = useRef<HandRig>({ root: null, knuckles: [], tips: [], thumb: null });
  const sleeveSide = useRef<'south' | 'north'>('south');
  const skin = useMemo(
    () => new MeshStandardMaterial({ color: '#7a4a2f', roughness: 0.58, metalness: 0 }),
    [],
  );
  const sleeve = useMemo(
    () => new MeshStandardMaterial({ color: SLEEVE.south, roughness: 0.9, metalness: 0 }),
    [],
  );
  useFrame((state) => {
    const h = hand.current;
    const now = state.clock.getElapsedTime();
    sampleHand(h, now);
    poseHand(rig.current, h, now < h.hideAt);
    const side = h.side > 0 ? 'south' : 'north';
    if (sleeveSide.current !== side) {
      sleeveSide.current = side;
      sleeve.color.set(SLEEVE[side]);
    }
  });
  return (
    <group
      ref={(el) => {
        rig.current.root = el;
      }}
      visible={false}
    >
      <group scale={HAND_SCALE}>
        {/* Palm, and the rounded back of the hand. */}
        <RoundedBox
          args={[0.48, 0.13, 0.46]}
          radius={0.06}
          smoothness={4}
          position={[0, PALM_Y, 0]}
          material={skin}
          castShadow
          raycast={noRaycast}
        />
        <mesh
          position={[0, PALM_Y + 0.05, 0.02]}
          scale={[0.25, 0.07, 0.24]}
          material={skin}
          castShadow
          raycast={noRaycast}
        >
          <sphereGeometry args={[1, 20, 12]} />
        </mesh>
        {FINGERS.map((f, i) => (
          <group
            key={i}
            position={[f.x, PALM_Y - 0.01, -0.2]}
            ref={(el) => {
              rig.current.knuckles[i] = el;
            }}
          >
            <mesh material={skin} castShadow raycast={noRaycast}>
              <sphereGeometry args={[FINGER_R * 1.12, 12, 10]} />
            </mesh>
            <mesh
              position={[0, 0, -f.len[0] / 2]}
              rotation={[Math.PI / 2, 0, 0]}
              material={skin}
              castShadow
              raycast={noRaycast}
            >
              <capsuleGeometry args={[FINGER_R, f.len[0] - FINGER_R, 4, 10]} />
            </mesh>
            <group
              position={[0, 0, -f.len[0] + 0.015]}
              ref={(el) => {
                rig.current.tips[i] = el;
              }}
            >
              <mesh
                position={[0, 0, -f.len[1] / 2]}
                rotation={[Math.PI / 2, 0, 0]}
                material={skin}
                castShadow
                raycast={noRaycast}
              >
                <capsuleGeometry args={[FINGER_R * 0.92, f.len[1] - FINGER_R, 4, 10]} />
              </mesh>
            </group>
          </group>
        ))}
        <group
          position={[-0.25, PALM_Y - 0.02, 0.06]}
          ref={(el) => {
            rig.current.thumb = el;
          }}
        >
          <mesh
            position={[0, 0, -0.12]}
            rotation={[Math.PI / 2, 0, 0]}
            material={skin}
            castShadow
            raycast={noRaycast}
          >
            <capsuleGeometry args={[0.062, 0.17, 4, 10]} />
          </mesh>
        </group>
        {/* Wrist, forearm and a long sleeve: a right arm coming in from the player's right,
            rising out of the frame rather than ending on screen. */}
        <group position={[0.04, PALM_Y + 0.01, 0.2]} rotation={[-0.62, 0.62, 0, 'YXZ']}>
          <mesh
            position={[0, 0, 0.45]}
            rotation={[Math.PI / 2, 0, 0]}
            material={skin}
            castShadow
            raycast={noRaycast}
          >
            <capsuleGeometry args={[0.15, 0.8, 4, 14]} />
          </mesh>
          <mesh
            position={[0, 0, 2.35]}
            rotation={[Math.PI / 2, 0, 0]}
            material={sleeve}
            castShadow
            raycast={noRaycast}
          >
            <cylinderGeometry args={[0.24, 0.19, 3.0, 20]} />
          </mesh>
        </group>
      </group>
    </group>
  );
}
