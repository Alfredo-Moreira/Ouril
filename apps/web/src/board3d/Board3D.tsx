/**
 * The 3D board (ADR 0021): a carved wooden board with glossy seeds, rendered with React Three
 * Fiber. Everything is procedural (geometry and textures built in code), so nothing is
 * downloaded and the board works offline. Lazy-loaded: a loading widget shows until this chunk
 * has drawn (`BoardView`).
 *
 * Game rules never live here. The board receives counts, legal pits and the highlight
 * (`BoardProps`); the seeds and the hand that moves them live in `Sowing.tsx`, which
 * animates *individual* seeds between containers by reconciling
 * each frame's counts (`seeds.ts`), so it can never drift from the engine.
 *
 * Motion follows the project's motion rules (docs/architecture/web-design.md): sown seeds are
 * lifted into a hovering "hand" that carries them from pit to pit and drops them one by one
 * (see `Seeds`); highlights fade with ease-out; durations follow the game's frame timing
 * (`game/animation.ts`); with reduced motion, seeds snap.
 * No endless decorative loops; the camera only moves on captures and at game end.
 *
 * Avoid drei helpers that fetch from the network (Environment presets, useGLTF with Draco,
 * Text fonts): the CSP and offline-first design forbid it.
 */
import { Canvas, useFrame, useThree, type ThreeEvent } from '@react-three/fiber';
import { ContactShadows, Environment, Lightformer } from '@react-three/drei';
import { useEffect, useMemo, useRef, useState, type RefObject } from 'react';
import {
  Color,
  DoubleSide,
  ExtrudeGeometry,
  LatheGeometry,
  MathUtils,
  Path,
  Shape,
  Vector2,
  Vector3,
  type Mesh,
  type MeshStandardMaterial,
  type PerspectiveCamera,
} from 'three';

import type { FrameKind } from '../game/animation';
import { useClockDelta } from './clock';
import {
  BOWL_DEPTH,
  CORNER,
  HALF_D,
  HALF_W,
  pitCenter,
  PIT_R,
  STORE_RX,
  STORE_RZ,
  STORE_X,
  THICK,
  TOP,
  TOTAL_SEEDS_DEFAULT,
} from './geometry';
import { layoutFor, type Layout } from './seeds';
import { Sowing } from './Sowing';
import { woodTexture } from './textures';

export interface Board3DProps {
  pits: number[];
  stores: [number, number];
  legal: readonly number[];
  onPlay?: (pit: number) => void;
  highlight?: { pit?: number; kind: FrameKind } | null;
  hintPit?: number | null;
  /** Pit focused through the accessible (keyboard / screen-reader) controls. */
  focusPit?: number | null;
  /** 1 = normal, 0 = snap (reduced motion / tests). Matches the game's animation speed. */
  speed?: number;
  /** The game has ended: the camera eases into a slightly higher, celebratory angle. */
  finished?: boolean;
  /** Called once the scene has really drawn (the loading widget can go). */
  onReady?: () => void;
}

// --- Scene ------------------------------------------------------------------------------------

function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  );
}

export default function Board3D(input: Board3DProps) {
  const chips = useRef<(HTMLSpanElement | null)[]>([]);
  // With reduced motion (even when 3D was chosen explicitly) seeds snap into place and the
  // camera stays still; highlight fades (opacity only) remain.
  const [reduced] = useState(prefersReducedMotion);
  const props = reduced ? { ...input, speed: 0 } : input;
  const counts = [...props.pits, props.stores[0], props.stores[1]];
  return (
    <div className="board3d__view">
      <Canvas
        className="board3d__canvas"
        shadows="percentage"
        dpr={[1, 1.75]}
        frameloop="demand"
        gl={{ antialias: true, alpha: true, powerPreference: 'high-performance' }}
        camera={{ fov: 30, near: 0.1, far: 100, position: [0, 9, 9] }}
        aria-hidden="true"
      >
        <Scene {...props} chips={chips} />
      </Canvas>
      {/* Seed counts: plain DOM chips placed over the canvas by projecting each pit's position. */}
      <div className="board3d__chips" aria-hidden="true">
        {counts.map((count, i) => (
          <span
            key={i}
            ref={(el) => {
              chips.current[i] = el;
            }}
            className={`board3d__chip${i >= props.pits.length ? ' board3d__chip--store' : ''}`}
          >
            {/* Store counts pop when they change (the outer chip is positioned each frame). */}
            {i >= props.pits.length ? (
              <span key={count} className="board3d__chip-num">
                {count}
              </span>
            ) : (
              count
            )}
          </span>
        ))}
      </div>
    </div>
  );
}

function Scene(props: Board3DProps & { chips: RefObject<(HTMLSpanElement | null)[]> }) {
  // Seeds on the board. During a move some are in the hand (not on the board), so the total is
  // kept from the last frame with no move playing; it can change between positions (tutorial).
  const onBoard = props.pits.reduce((a, b) => a + b, 0) + props.stores[0] + props.stores[1];
  const moving = !!props.highlight;
  const [restTotal, setRestTotal] = useState(onBoard);
  if (!moving && restTotal !== onBoard) setRestTotal(onBoard);
  const total = moving ? Math.max(restTotal, onBoard) : onBoard;
  const layout = useMemo(() => layoutFor(props.pits.length), [props.pits.length]);
  return (
    <>
      <ReadySignal onReady={props.onReady} />
      <CameraRig
        finished={!!props.finished}
        highlightKind={props.highlight?.kind}
        speed={props.speed ?? 1}
      />
      <ambientLight intensity={0.35} />
      <hemisphereLight args={['#fff3e0', '#3b2a1e', 0.55]} />
      <directionalLight
        position={[-4, 9, 6]}
        intensity={2.1}
        color="#ffe2bf"
        castShadow
        shadow-mapSize={[1024, 1024]}
        shadow-camera-left={-7}
        shadow-camera-right={7}
        shadow-camera-top={4}
        shadow-camera-bottom={-4}
        shadow-bias={-0.0004}
        shadow-normalBias={0.02}
      />
      <directionalLight position={[6, 4, -6]} intensity={0.6} color="#9cc7ff" />
      <Environment resolution={64} frames={1}>
        <Lightformer
          form="rect"
          intensity={2.4}
          color="#fff1dc"
          position={[0, 5, 3]}
          scale={[8, 3, 1]}
        />
        <Lightformer
          form="rect"
          intensity={0.8}
          color="#bcd8ff"
          position={[-6, 2, -4]}
          scale={[4, 4, 1]}
        />
        <Lightformer form="ring" intensity={1.2} color="#ffd6a0" position={[5, 3, 4]} scale={2} />
      </Environment>

      <BoardBody layout={layout} />
      <Sowing
        layout={layout}
        total={total || TOTAL_SEEDS_DEFAULT}
        pits={props.pits}
        stores={props.stores}
        highlight={props.highlight ?? null}
        speed={props.speed ?? 1}
      />
      <PitRings layout={layout} {...props} />
      <ChipProjector layout={layout} chips={props.chips} />
      <ContactShadows
        position={[0, -THICK - 0.01, 0]}
        opacity={0.55}
        scale={[HALF_W * 2.6, HALF_D * 3.2]}
        blur={2.6}
        far={1.6}
        frames={1}
        color="#1a0f08"
      />
    </>
  );
}

/**
 * Reports "ready" only once the GPU has really drawn the scene (two rendered frames, after the
 * shaders have compiled), so the loader never gives way to an empty or half-built canvas.
 */
function ReadySignal({ onReady }: { onReady?: () => void }) {
  const frames = useRef(0);
  const invalidate = useThree((s) => s.invalidate);
  useEffect(() => {
    invalidate();
  }, [invalidate]);
  useFrame((state) => {
    if (frames.current > 2) return;
    frames.current++;
    if (frames.current === 2) onReady?.();
    else state.invalidate();
  });
  return null;
}

/** Keeps the whole board in view at any aspect ratio; eases on captures and at game end. */
function CameraRig({
  finished,
  highlightKind,
  speed,
}: {
  finished: boolean;
  highlightKind?: FrameKind;
  speed: number;
}) {
  const camera = useThree((s) => s.camera) as PerspectiveCamera;
  const size = useThree((s) => s.size);
  const invalidate = useThree((s) => s.invalidate);
  const current = useRef({ el: 0.9, az: 0, push: 0 });
  const target = useRef({ el: 0.9, az: 0, push: 0 });
  const lookAt = useMemo(() => new Vector3(0, -0.2, 0.15), []);

  useEffect(() => {
    // A small push-in on captures (feedback for the moment that matters), a calm, slightly
    // higher angle when the game ends. Nothing on ordinary sowing.
    if (speed > 0 && (highlightKind === 'capture' || highlightKind === 'grand_slam')) {
      target.current.push = highlightKind === 'grand_slam' ? 0.07 : 0.035;
      const t = setTimeout(() => {
        target.current.push = 0;
        invalidate();
      }, 260);
      invalidate();
      return () => clearTimeout(t);
    }
  }, [highlightKind, speed, invalidate]);

  useEffect(() => {
    target.current.el = finished ? 1.02 : 0.9;
    target.current.az = finished ? 0.08 : 0;
    invalidate();
  }, [finished, invalidate]);

  const clockDelta = useClockDelta();

  useFrame((state) => {
    const dt = clockDelta(state.clock.getElapsedTime());
    const c = current.current;
    const t = target.current;
    const k = speed > 0 ? 1 - Math.exp(-dt * 6) : 1;
    c.el = MathUtils.lerp(c.el, t.el, k);
    c.az = MathUtils.lerp(c.az, t.az, finished ? 1 - Math.exp(-dt * 1.5) : k);
    c.push = MathUtils.lerp(c.push, t.push, k);

    const aspect = size.width / Math.max(1, size.height);
    // Narrow canvases (phones): look down more steeply so the pits stay readable.
    const narrow = MathUtils.clamp((2.1 - aspect) / 0.6, 0, 1);
    const el = c.el + narrow * 0.22;
    const vf = MathUtils.degToRad(camera.fov);
    const hf = 2 * Math.atan(Math.tan(vf / 2) * aspect);
    const fitW = (HALF_W + 0.5) / Math.tan(hf / 2);
    const fitD = (HALF_D + 1.1) / Math.tan(vf / 2);
    const d = Math.max(fitW, fitD) * (1 - c.push);
    camera.position.set(
      lookAt.x + Math.sin(c.az) * Math.cos(el) * d,
      lookAt.y + Math.sin(el) * d,
      lookAt.z + Math.cos(c.az) * Math.cos(el) * d,
    );
    camera.lookAt(lookAt);
    const moving =
      Math.abs(c.el - t.el) > 1e-4 ||
      Math.abs(c.az - t.az) > 1e-4 ||
      Math.abs(c.push - t.push) > 1e-4;
    if (moving) state.invalidate();
  });
  return null;
}

/** The board: an extruded slab with the 12 pits and 2 stores cut through, plus carved bowls. */
function BoardBody({ layout }: { layout: Layout }) {
  const { slab, bowls, storeBowl, wood, bowlWood } = useMemo(() => {
    const shape = roundedRect(HALF_W, HALF_D, CORNER);
    for (let i = 0; i < layout.n * 2; i++) {
      const c = pitCenter(i, layout.n);
      const hole = new Path();
      hole.absarc(c.x, -c.z, PIT_R, 0, Math.PI * 2, true);
      shape.holes.push(hole);
    }
    for (const x of [STORE_X, -STORE_X]) {
      const hole = new Path();
      hole.absellipse(x, 0, STORE_RX, STORE_RZ, 0, Math.PI * 2, true, 0);
      shape.holes.push(hole);
    }
    const slabGeo = new ExtrudeGeometry(shape, {
      depth: THICK,
      bevelEnabled: true,
      bevelThickness: 0.06,
      bevelSize: 0.05,
      bevelSegments: 4,
      curveSegments: 48,
    });
    slabGeo.rotateX(-Math.PI / 2);
    slabGeo.translate(0, -THICK, 0);

    const profile = [
      new Vector2(PIT_R + 0.002, 0.04),
      new Vector2(PIT_R, 0),
      new Vector2(PIT_R * 0.97, -0.1),
      new Vector2(PIT_R * 0.86, -0.24),
      new Vector2(PIT_R * 0.62, -0.36),
      new Vector2(PIT_R * 0.3, -BOWL_DEPTH + 0.01),
      new Vector2(0, -BOWL_DEPTH),
    ];
    const bowlGeo = new LatheGeometry(profile, 48);

    const woodMap = woodTexture({ base: '#7a4524', dark: '#3e1f0e', light: '#b07745', seed: 11 });
    // UVs are world units: keep the board inside one texture tile (no seam where it wraps).
    woodMap.repeat.set(0.085, 0.24);
    woodMap.offset.set(0.5, 0.5);
    const bowlMap = woodTexture({ base: '#4a2814', dark: '#24110a', light: '#73452a', seed: 23 });
    bowlMap.repeat.set(0.5, 0.5);
    return { slab: slabGeo, bowls: bowlGeo, storeBowl: bowlGeo, wood: woodMap, bowlWood: bowlMap };
  }, [layout.n]);

  useEffect(
    () => () => {
      slab.dispose();
      bowls.dispose();
      wood.dispose();
      bowlWood.dispose();
    },
    [slab, bowls, wood, bowlWood],
  );

  return (
    <group>
      <mesh geometry={slab} castShadow receiveShadow>
        <meshPhysicalMaterial
          map={wood}
          roughness={0.48}
          clearcoat={0.35}
          clearcoatRoughness={0.4}
          color="#ffffff"
        />
      </mesh>
      {Array.from({ length: layout.n * 2 }, (_, i) => {
        const c = pitCenter(i, layout.n);
        return (
          <mesh key={i} geometry={bowls} position={[c.x, 0, c.z]} receiveShadow>
            <meshStandardMaterial map={bowlWood} roughness={0.85} side={DoubleSide} />
          </mesh>
        );
      })}
      {[STORE_X, -STORE_X].map((x) => (
        <mesh
          key={x}
          geometry={storeBowl}
          position={[x, 0, 0]}
          scale={[STORE_RX / PIT_R, 1, STORE_RZ / PIT_R]}
          receiveShadow
        >
          <meshStandardMaterial map={bowlWood} roughness={0.85} side={DoubleSide} />
        </mesh>
      ))}
      {/* Underside so the board reads as solid from low angles. */}
      <mesh position={[0, -THICK - 0.02, 0]} receiveShadow>
        <boxGeometry args={[HALF_W * 2 - 0.4, 0.04, HALF_D * 2 - 0.4]} />
        <meshStandardMaterial color="#2c170b" roughness={0.9} />
      </mesh>
    </group>
  );
}

function roundedRect(hw: number, hd: number, r: number): Shape {
  const s = new Shape();
  s.moveTo(-hw + r, -hd);
  s.lineTo(hw - r, -hd);
  s.quadraticCurveTo(hw, -hd, hw, -hd + r);
  s.lineTo(hw, hd - r);
  s.quadraticCurveTo(hw, hd, hw - r, hd);
  s.lineTo(-hw + r, hd);
  s.quadraticCurveTo(-hw, hd, -hw, hd - r);
  s.lineTo(-hw, -hd + r);
  s.quadraticCurveTo(-hw, -hd, -hw + r, -hd);
  return s;
}

// --- Interaction: rim rings + invisible hit areas ----------------------------------------------

function PitRings({
  layout,
  legal,
  onPlay,
  highlight,
  hintPit,
  focusPit,
  speed = 1,
}: Board3DProps & { layout: Layout }) {
  const [hoverRaw, setHover] = useState<number | null>(null);
  // Only legal pits react to hover; a pit that stops being legal stops glowing.
  const hover = hoverRaw !== null && legal.includes(hoverRaw) ? hoverRaw : null;
  const invalidate = useThree((s) => s.invalidate);
  const rings = useRef<(Mesh | null)[]>([]);

  // Target glow per pit: colour + strength. Captures flash red, then fade.
  const targets = useMemo(() => {
    return Array.from({ length: layout.n * 2 }, (_, i) => {
      const isLegal = legal.includes(i);
      if (highlight?.pit === i && highlight.kind === 'capture') return { color: '#ff5a36', a: 1 };
      if (highlight?.pit === i && (highlight.kind === 'sow' || highlight.kind === 'pickup'))
        return { color: '#ffc861', a: 0.85 };
      if (highlight?.pit === i && highlight.kind === 'skip') return { color: '#5fc4e8', a: 0.8 };
      if (focusPit === i) return { color: '#7fd1ff', a: 1 };
      if (hintPit === i) return { color: '#5fc4e8', a: 0.95 };
      if (isLegal && hover === i) return { color: '#ffd27a', a: 1 };
      if (isLegal) return { color: '#ffc861', a: 0.42 };
      return { color: '#ffc861', a: 0 };
    });
  }, [layout.n, legal, highlight, hintPit, focusPit, hover]);

  useEffect(() => {
    if (hover === null) return;
    document.body.style.cursor = 'pointer';
    return () => {
      document.body.style.cursor = '';
    };
  }, [hover]);

  useEffect(() => {
    invalidate();
  }, [targets, invalidate]);

  const clockDelta = useClockDelta();

  useFrame((state) => {
    const dt = clockDelta(state.clock.getElapsedTime());
    let active = false;
    rings.current.forEach((ring, i) => {
      if (!ring) return;
      const mat = ring.material as MeshStandardMaterial;
      const t = targets[i]!;
      // Fast attack, slower release (asymmetric timing).
      const rising = t.a > mat.opacity;
      const k = speed > 0 ? 1 - Math.exp(-dt * (rising ? 22 : 9)) : 1;
      mat.opacity = MathUtils.lerp(mat.opacity, t.a, k);
      mat.emissive.lerp(new Color(t.color), k);
      mat.color.copy(mat.emissive);
      if (Math.abs(mat.opacity - t.a) > 0.004) active = true;
    });
    if (active) state.invalidate();
  });

  const onOver = (i: number) => (e: ThreeEvent<PointerEvent>) => {
    e.stopPropagation();
    if (e.pointerType === 'mouse') setHover(i);
  };

  return (
    <group>
      {Array.from({ length: layout.n * 2 }, (_, i) => {
        const c = pitCenter(i, layout.n);
        const playable = legal.includes(i) && !!onPlay;
        return (
          <group key={i} position={[c.x, 0, c.z]}>
            <mesh
              ref={(el) => {
                rings.current[i] = el;
              }}
              rotation={[-Math.PI / 2, 0, 0]}
              position={[0, TOP + 0.006, 0]}
            >
              <ringGeometry args={[PIT_R + 0.02, PIT_R + 0.085, 64]} />
              <meshStandardMaterial
                transparent
                opacity={0}
                emissiveIntensity={1.6}
                emissive="#ffc861"
                color="#ffc861"
                toneMapped={false}
                depthWrite={false}
              />
            </mesh>
            {/* Invisible hit area over the pit (generous for touch). */}
            <mesh
              position={[0, 0.05, 0]}
              rotation={[-Math.PI / 2, 0, 0]}
              visible={false}
              onPointerOver={onOver(i)}
              onPointerOut={() => setHover((h) => (h === i ? null : h))}
              onClick={(e) => {
                e.stopPropagation();
                if (playable) onPlay?.(i);
              }}
            >
              <circleGeometry args={[PIT_R + 0.12, 24]} />
              <meshBasicMaterial />
            </mesh>
          </group>
        );
      })}
    </group>
  );
}

/** Keeps the DOM count chips next to their pit or store as the camera moves. */
function ChipProjector({
  layout,
  chips,
}: {
  layout: Layout;
  chips: RefObject<(HTMLSpanElement | null)[]>;
}) {
  const anchors = useMemo(() => {
    const out: Vector3[] = [];
    for (let i = 0; i < layout.n * 2; i++) {
      const c = pitCenter(i, layout.n);
      out.push(new Vector3(c.x, 0.02, c.z > 0 ? c.z + PIT_R + 0.3 : c.z - PIT_R - 0.3));
    }
    out.push(
      new Vector3(STORE_X, 0.02, STORE_RZ + 0.38),
      new Vector3(-STORE_X, 0.02, STORE_RZ + 0.38),
    );
    return out;
  }, [layout.n]);
  const v = useMemo(() => new Vector3(), []);

  useFrame(({ camera, size }) => {
    anchors.forEach((a, i) => {
      const el = chips.current?.[i];
      if (!el) return;
      v.copy(a).project(camera);
      const x = ((v.x + 1) / 2) * size.width;
      const y = ((1 - v.y) / 2) * size.height;
      el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) translate(-50%, -50%)`;
    });
  });
  return null;
}
