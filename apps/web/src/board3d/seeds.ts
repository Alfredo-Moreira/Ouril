/**
 * Which container each seed is in, so the 3D board can move *individual* seeds between pits.
 *
 * Containers: pits `0..2n-1`, the South store, the North store, and the "hand" (seeds picked
 * up and not yet sown). The engine only gives counts per pit and store; the hand holds the
 * rest (`total - pits - stores`). [`reconcile`] moves the fewest seeds needed to match new
 * counts, taking from the top of surplus piles and adding on top of deficits. Because every
 * frame from `game/animation.ts` is reconciled the same way, pickup (pit → hand), sowing
 * (hand → pit), captures (pit → store) and end-of-game collection all fall out of one rule, and
 * the board can never drift from the engine: the last frame is always the engine's state.
 */

export type Container = number; // 0..2n-1 pits, then STORE_SOUTH, STORE_NORTH, HAND

export interface Layout {
  /** Pits per side (6). */
  n: number;
  storeSouth: Container;
  storeNorth: Container;
  hand: Container;
  containers: number;
}

export function layoutFor(pitCount: number): Layout {
  const n = pitCount / 2;
  return {
    n,
    storeSouth: pitCount,
    storeNorth: pitCount + 1,
    hand: pitCount + 2,
    containers: pitCount + 3,
  };
}

/** Where every seed is: container and stacking slot (0 = bottom of the pile). */
export interface Placement {
  container: Container;
  slot: number;
}

export interface Counts {
  pits: readonly number[];
  stores: readonly [number, number];
}

/** Target count per container (the hand gets whatever isn't on the board). */
export function targetCounts(layout: Layout, counts: Counts, total: number): number[] {
  const t = new Array<number>(layout.containers).fill(0);
  let onBoard = 0;
  counts.pits.forEach((c, i) => {
    t[i] = c;
    onBoard += c;
  });
  t[layout.storeSouth] = counts.stores[0];
  t[layout.storeNorth] = counts.stores[1];
  onBoard += counts.stores[0] + counts.stores[1];
  t[layout.hand] = Math.max(0, total - onBoard);
  return t;
}

/** Initial placement: seeds dealt into containers in order, matching `counts`. */
export function initialPlacement(layout: Layout, counts: Counts, total: number): Placement[] {
  const t = targetCounts(layout, counts, total);
  const out: Placement[] = [];
  t.forEach((count, container) => {
    for (let slot = 0; slot < count; slot++) out.push({ container, slot });
  });
  // Totals that don't add up (shouldn't happen) still produce `total` seeds, in the hand.
  while (out.length < total) out.push({ container: layout.hand, slot: out.length });
  return out.slice(0, total);
}

/**
 * New placement matching `counts`, plus the IDs of seeds that changed container. Surplus seeds
 * leave from the top of their pile (hand first, so sowing takes from the hand); deficits are
 * filled in container order on top of existing piles.
 */
export function reconcile(
  layout: Layout,
  prev: readonly Placement[],
  counts: Counts,
): { placement: Placement[]; moved: number[] } {
  const total = prev.length;
  const target = targetCounts(layout, counts, total);
  const members: number[][] = Array.from({ length: layout.containers }, () => []);
  prev.forEach((p, id) => members[p.container]!.push(id));
  for (const list of members) list.sort((a, b) => prev[a]!.slot - prev[b]!.slot);

  // Seeds leaving their container: the hand first, then the rest in container order.
  const order = [layout.hand, ...Array.from({ length: layout.containers }, (_, i) => i)].filter(
    (c, i, arr) => arr.indexOf(c) === i,
  );
  const pool: number[] = [];
  for (const c of order) {
    const surplus = members[c]!.length - target[c]!;
    if (surplus > 0) pool.push(...members[c]!.splice(members[c]!.length - surplus, surplus));
  }

  const placement = prev.map((p) => ({ ...p }));
  const moved: number[] = [];
  for (let c = 0; c < layout.containers; c++) {
    while (members[c]!.length < target[c]! && pool.length > 0) {
      const id = pool.shift()!;
      members[c]!.push(id);
      moved.push(id);
    }
  }
  // Re-number slots so piles stay compact (seeds below a removed one settle down).
  members.forEach((list, container) =>
    list.forEach((id, slot) => {
      placement[id] = { container, slot };
    }),
  );
  return { placement, moved };
}
