package org.moreiralabs.ouril.ui.board3d

/**
 * Which container each seed is in, so the board can move *individual* seeds between pits (the
 * web's `board3d/seeds.ts`).
 *
 * Containers: pits `0..2n-1`, the South store, the North store, and the "hand" (seeds picked up
 * and not yet sown). The engine only gives counts per pit and store; the hand holds the rest
 * (`total - pits - stores`). [reconcile] moves the fewest seeds needed to match new counts,
 * taking from the top of surplus piles and adding on top of deficits, so pickup, sowing,
 * captures and collecting all fall out of one rule, and the board can never drift from the engine.
 */
data class BoardLayout(val n: Int) {
    val storeSouth = 2 * n
    val storeNorth = 2 * n + 1
    val hand = 2 * n + 2
    val containers = 2 * n + 3

    companion object {
        fun forPits(pitCount: Int) = BoardLayout(pitCount / 2)
    }
}

/** Where a seed is: container and stacking slot (0 = bottom of the pile). */
data class Placement(val container: Int, val slot: Int)

/** Target count per container (the hand gets whatever isn't on the board). */
fun targetCounts(layout: BoardLayout, pits: List<Int>, stores: List<Int>, total: Int): IntArray {
    val t = IntArray(layout.containers)
    var onBoard = 0
    pits.forEachIndexed { i, c ->
        t[i] = c
        onBoard += c
    }
    t[layout.storeSouth] = stores[0]
    t[layout.storeNorth] = stores[1]
    onBoard += stores[0] + stores[1]
    t[layout.hand] = maxOf(0, total - onBoard)
    return t
}

/** Initial placement: seeds dealt into containers in order, matching the counts. */
fun initialPlacement(layout: BoardLayout, pits: List<Int>, stores: List<Int>, total: Int): List<Placement> {
    val t = targetCounts(layout, pits, stores, total)
    val out = mutableListOf<Placement>()
    t.forEachIndexed { container, count ->
        for (slot in 0 until count) out += Placement(container, slot)
    }
    // Totals that don't add up (shouldn't happen) still produce `total` seeds, in the hand.
    while (out.size < total) out += Placement(layout.hand, out.size)
    return out.take(total)
}

/**
 * New placement matching the counts, plus the IDs of seeds that changed container. Surplus seeds
 * leave from the top of their pile (hand first, so sowing takes from the hand); deficits are
 * filled in container order on top of existing piles.
 */
fun reconcile(
    layout: BoardLayout,
    prev: List<Placement>,
    pits: List<Int>,
    stores: List<Int>,
): Pair<List<Placement>, List<Int>> {
    val total = prev.size
    val target = targetCounts(layout, pits, stores, total)
    val members = List(layout.containers) { mutableListOf<Int>() }
    prev.forEachIndexed { id, p -> members[p.container] += id }
    for (list in members) list.sortBy { prev[it].slot }

    // Seeds leaving their container: the hand first, then the rest in container order.
    val order = (listOf(layout.hand) + (0 until layout.containers)).distinct()
    val pool = ArrayDeque<Int>()
    for (c in order) {
        val surplus = members[c].size - target[c]
        if (surplus > 0) {
            val leaving = members[c].subList(members[c].size - surplus, members[c].size)
            pool.addAll(leaving)
            leaving.clear()
        }
    }

    val placement = prev.toMutableList()
    val moved = mutableListOf<Int>()
    for (c in 0 until layout.containers) {
        while (members[c].size < target[c] && pool.isNotEmpty()) {
            val id = pool.removeFirst()
            members[c] += id
            moved += id
        }
    }
    // Re-number slots so piles stay compact (seeds below a removed one settle down).
    members.forEachIndexed { container, list ->
        list.forEachIndexed { slot, id -> placement[id] = Placement(container, slot) }
    }
    return placement to moved
}
