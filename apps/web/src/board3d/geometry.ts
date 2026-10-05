/** Board geometry shared by the 3D scene: sizes, pit and store positions, seed resting slots. */
import { Vector3 } from 'three';

import type { Layout, Placement } from './seeds';
import { rng } from './textures';

// --- Board geometry (world units; South is +z, towards the camera) ----------------------------

export const SPACING = 1.25;
export const ROW_Z = 0.72;
export const PIT_R = 0.5;
export const STORE_X = 4.75;
export const STORE_RX = 0.58;
export const STORE_RZ = 1.3;
export const HALF_W = 5.7;
export const HALF_D = 1.85;
export const THICK = 0.62;
export const CORNER = 0.95;
export const BOWL_DEPTH = 0.44;
/** Height of the board's top surface (the extrusion's bevel rises above y = 0). */
export const TOP = 0.06;
export const TOTAL_SEEDS_DEFAULT = 48;

export const ease = {
  /** Strong ease-in-out for movement on screen (Emil: cubic-bezier(0.77, 0, 0.175, 1)). */
  inOut: (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2),
};

export function pitCenter(i: number, n: number): Vector3 {
  if (i < n) return new Vector3((i - (n - 1) / 2) * SPACING, 0, ROW_Z);
  const col = 2 * n - 1 - i;
  return new Vector3((col - (n - 1) / 2) * SPACING, 0, -ROW_Z);
}

export function containerCenter(c: number, layout: Layout): Vector3 {
  if (c < layout.n * 2) return pitCenter(c, layout.n);
  if (c === layout.storeSouth) return new Vector3(STORE_X, 0, 0);
  if (c === layout.storeNorth) return new Vector3(-STORE_X, 0, 0);
  return new Vector3(0, 0, 0);
}

/** A small, stable jitter per (container, slot), so piles look hand-placed, not gridded. */
function jitter(container: number, slot: number, axis: number): number {
  const r = rng(container * 1009 + slot * 31 + axis * 7)();
  return (r - 0.5) * 0.05;
}

/** Resting position of a seed in a pit or a store. Seeds in the hand are placed by the hand. */
export function slotPosition(p: Placement, layout: Layout): Vector3 {
  const { container: c, slot } = p;
  const center = containerCenter(c, layout);
  if (c < layout.n * 2) {
    // Layers in the bowl: 7 on the floor (one in the middle), then wider rings above.
    let layer = 0;
    let index = slot;
    let size = 7;
    while (index >= size) {
      index -= size;
      layer++;
      size = 7 + layer * 2;
    }
    const y = -BOWL_DEPTH + 0.1 + layer * 0.12;
    let x = 0;
    let z = 0;
    if (!(layer === 0 && index === 0)) {
      const ringCount = layer === 0 ? 6 : size;
      const ringIndex = layer === 0 ? index - 1 : index;
      const radius = Math.min(0.17 + layer * 0.05, 0.32);
      const a = (ringIndex / ringCount) * Math.PI * 2 + layer * 0.5;
      x = Math.cos(a) * radius;
      z = Math.sin(a) * radius;
    }
    return new Vector3(
      center.x + x + jitter(c, slot, 0),
      y + jitter(c, slot, 1) * 0.4,
      center.z + z + jitter(c, slot, 2),
    );
  }
  // Stores: two columns along the long axis, layered.
  const perLayer = 14;
  const layer = Math.floor(slot / perLayer);
  const i = slot % perLayer;
  const col = i % 2;
  const row = Math.floor(i / 2);
  const z = (row - 3) * 0.3;
  const widthAtZ = STORE_RX * Math.sqrt(Math.max(0.05, 1 - (z / (STORE_RZ * 0.92)) ** 2));
  const x = (col === 0 ? -1 : 1) * Math.min(0.16, widthAtZ * 0.45);
  return new Vector3(
    center.x + x + jitter(c, slot, 0),
    -BOWL_DEPTH + 0.1 + layer * 0.12,
    center.z + z + jitter(c, slot, 2),
  );
}
