/**
 * The 3D board's camera (ADR 0021): it keeps the whole board in view at any aspect ratio, pushes
 * in a little on captures and eases into a slightly higher angle when the game ends. Nothing
 * moves on ordinary sowing. Plain state and functions, so every platform frames the board the
 * same way (checked by the motion vectors in `shared/motion/`).
 */
import { MathUtils, Vector3 } from 'three';

import type { FrameKind } from '../game/animation';
import { HALF_D, HALF_W } from './geometry';

/** Vertical field of view, in degrees. */
export const CAMERA_FOV = 30;
export const CAMERA_NEAR = 0.1;
export const CAMERA_FAR = 100;
export const CAMERA_LOOK_AT = new Vector3(0, -0.2, 0.15);

interface Angles {
  el: number;
  az: number;
  push: number;
}

export interface CameraInputs {
  finished: boolean;
  highlightKind?: FrameKind;
  speed: number;
}

export interface CameraRig {
  current: Angles;
  target: Angles;
  /** When the push-in of a capture lets go (scene seconds), if one is pending. */
  releaseAt: number | null;
  inputs: CameraInputs | null;
}

export function createCameraRig(): CameraRig {
  return {
    current: { el: 0.9, az: 0, push: 0 },
    target: { el: 0.9, az: 0, push: 0 },
    releaseAt: null,
    inputs: null,
  };
}

/** The board's state changed (a new display frame, the game ending). */
export function setCameraInputs(rig: CameraRig, inputs: CameraInputs, now: number): void {
  const before = rig.inputs;
  rig.inputs = { ...inputs };
  if (!before || before.highlightKind !== inputs.highlightKind || before.speed !== inputs.speed) {
    // A small push-in on captures (feedback for the moment that matters). A new highlight
    // cancels a pending release, as an effect's cleanup would.
    rig.releaseAt = null;
    const kind = inputs.highlightKind;
    if (inputs.speed > 0 && (kind === 'capture' || kind === 'grand_slam')) {
      rig.target.push = kind === 'grand_slam' ? 0.07 : 0.035;
      rig.releaseAt = now + 0.26;
    }
  }
  if (!before || before.finished !== inputs.finished) {
    // A calm, slightly higher angle when the game ends.
    rig.target.el = inputs.finished ? 1.02 : 0.9;
    rig.target.az = inputs.finished ? 0.08 : 0;
  }
}

/**
 * Advances the camera to `now` (`dt`: seconds since the last frame) for a canvas of `aspect`
 * (width / height). Writes the camera's position into `out` and returns whether it's still
 * easing.
 */
export function stepCameraRig(
  rig: CameraRig,
  now: number,
  dt: number,
  aspect: number,
  out: Vector3,
): boolean {
  const c = rig.current;
  const t = rig.target;
  if (rig.releaseAt !== null && now >= rig.releaseAt) {
    t.push = 0;
    rig.releaseAt = null;
  }
  const speed = rig.inputs?.speed ?? 1;
  const finished = rig.inputs?.finished ?? false;
  const k = speed > 0 ? 1 - Math.exp(-dt * 6) : 1;
  c.el = MathUtils.lerp(c.el, t.el, k);
  c.az = MathUtils.lerp(c.az, t.az, finished ? 1 - Math.exp(-dt * 1.5) : k);
  c.push = MathUtils.lerp(c.push, t.push, k);

  // Narrow canvases (phones): look down more steeply so the pits stay readable.
  const narrow = MathUtils.clamp((2.1 - aspect) / 0.6, 0, 1);
  const el = c.el + narrow * 0.22;
  const vf = MathUtils.degToRad(CAMERA_FOV);
  const hf = 2 * Math.atan(Math.tan(vf / 2) * aspect);
  const fitW = (HALF_W + 0.5) / Math.tan(hf / 2);
  const fitD = (HALF_D + 1.1) / Math.tan(vf / 2);
  const d = Math.max(fitW, fitD) * (1 - c.push);
  out.set(
    CAMERA_LOOK_AT.x + Math.sin(c.az) * Math.cos(el) * d,
    CAMERA_LOOK_AT.y + Math.sin(el) * d,
    CAMERA_LOOK_AT.z + Math.cos(c.az) * Math.cos(el) * d,
  );
  return (
    Math.abs(c.el - t.el) > 1e-4 ||
    Math.abs(c.az - t.az) > 1e-4 ||
    Math.abs(c.push - t.push) > 1e-4 ||
    rig.releaseAt !== null
  );
}
