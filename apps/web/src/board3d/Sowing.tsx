/**
 * Draws the board's seeds and the player's hand with three.js (ADR 0021): the choreography itself
 * (where the hand goes, how each seed moves) is in `choreography.ts`, shared with the other
 * platforms through the motion vectors in `shared/motion/`.
 *
 * The hand is procedural (capsules and a rounded palm): nothing is downloaded.
 */
import { RoundedBox } from '@react-three/drei';
import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import {
  InstancedBufferAttribute,
  MeshStandardMaterial,
  Object3D,
  Vector3,
  type Group,
  type InstancedMesh,
} from 'three';

import type { FrameKind } from '../game/animation';
import {
  applyFrame,
  createChoreography,
  sampleHand,
  seedColors,
  seedOrientations,
  stepChoreography,
  type HandState,
} from './choreography';
import { useClockDelta } from './clock';
import type { Layout } from './seeds';

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
  const [choreo] = useState(createChoreography);
  const hand = useMemo<RefObject<HandState>>(() => ({ current: choreo.hand }), [choreo]);
  const orientations = useMemo(() => seedOrientations(total), [total]);

  // A new display frame: move the hand and choreograph the seeds that changed container.
  useEffect(() => {
    applyFrame(choreo, layout, total, { pits, stores, highlight, speed }, clock.getElapsedTime());
    invalidate();
  }, [choreo, pits, stores, highlight, layout, total, speed, clock, invalidate]);

  // Per-seed colour, attached as `instanceColor` from the first render so the material
  // compiles with instance colours (setting them later leaves the seeds white).
  const colorAttr = useMemo(() => new InstancedBufferAttribute(seedColors(total), 3), [total]);

  const tmp = useMemo(() => ({ s: new Vector3(1.18, 0.78, 0.95), o: new Object3D() }), []);
  const clockDelta = useClockDelta();

  useFrame((state) => {
    const m = mesh.current;
    if (!m) return;
    const now = state.clock.getElapsedTime();
    const active = stepChoreography(choreo, now, clockDelta(now));
    choreo.motions.forEach((mo, i) => {
      tmp.o.position.copy(mo.pos);
      tmp.o.quaternion.copy(orientations[i]!);
      if (mo.tumble) tmp.o.rotateX(mo.tumble);
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

/** Sleeves in the players' colours: a bright indigo for South (you, so it shows on the cloth),
 *  red for North (the opponent). */
const SLEEVE = { south: '#3f56d6', north: '#c8202a' } as const;

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
