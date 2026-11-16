package org.moreiralabs.ouril.ui.board3d

import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * The 3D board's geometry, as the web's `board3d/geometry.ts`: sizes, pit and store positions,
 * and where each seed rests. World units, y up, South towards the camera (+z). The motion vectors
 * in `shared/motion/` check that both boards put every seed in the same place.
 */
object BoardGeometry {
    const val SPACING = 1.25
    const val ROW_Z = 0.72
    const val PIT_R = 0.5
    const val STORE_X = 4.75
    const val STORE_RX = 0.58
    const val STORE_RZ = 1.3
    const val HALF_W = 5.7
    const val HALF_D = 1.85
    const val THICK = 0.62
    const val CORNER = 0.95
    const val BOWL_DEPTH = 0.44

    /** Height of the board's top surface (the bevel rises above y = 0). */
    const val TOP = 0.06
    const val TOTAL_SEEDS_DEFAULT = 48
}

/** A point or direction in the board's world, mutable like three.js's `Vector3`. */
class Vec3(var x: Double = 0.0, var y: Double = 0.0, var z: Double = 0.0) {
    fun set(x: Double, y: Double, z: Double): Vec3 {
        this.x = x
        this.y = y
        this.z = z
        return this
    }

    fun copy(v: Vec3): Vec3 = set(v.x, v.y, v.z)

    fun clone(): Vec3 = Vec3(x, y, z)

    fun add(v: Vec3): Vec3 = set(x + v.x, y + v.y, z + v.z)

    fun setY(y: Double): Vec3 {
        this.y = y
        return this
    }

    fun lerpVectors(a: Vec3, b: Vec3, t: Double): Vec3 =
        set(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t)

    fun lerp(v: Vec3, t: Double): Vec3 = set(x + (v.x - x) * t, y + (v.y - y) * t, z + (v.z - z) * t)

    fun distanceToSquared(v: Vec3): Double {
        val dx = x - v.x
        val dy = y - v.y
        val dz = z - v.z
        return dx * dx + dy * dy + dz * dz
    }

    fun distanceTo(v: Vec3): Double = sqrt(distanceToSquared(v))

    fun normalize(): Vec3 {
        val length = sqrt(x * x + y * y + z * z)
        val inv = 1.0 / (if (length == 0.0) 1.0 else length)
        return set(x * inv, y * inv, z * inv)
    }

    override fun toString() = "($x, $y, $z)"
}

/** three.js's `MathUtils.lerp`, with its exact arithmetic. */
fun lerp(x: Double, y: Double, t: Double): Double = (1 - t) * x + t * y

fun clamp(value: Double, lo: Double, hi: Double): Double = max(lo, min(hi, value))

/** Strong ease-in-out for movement on screen (the web's `ease.inOut`, Emil's cubic-bezier(0.77, 0, 0.175, 1)). */
fun easeInOut(t: Double): Double = if (t < 0.5) 4 * t * t * t else 1 - (-2 * t + 2).pow(3) / 2

/** Deterministic pseudo-random numbers (mulberry32), the same sequence as the web's `rng`. */
fun rng(seed: Int): () -> Double {
    var a = seed
    return {
        a += 0x6d2b79f5
        var t = a
        t = (t xor (t ushr 15)) * (t or 1)
        t = t xor (t + (t xor (t ushr 7)) * (t or 61))
        ((t xor (t ushr 14)).toLong() and 0xffffffffL).toDouble() / 4294967296.0
    }
}

fun pitCenter(i: Int, n: Int): Vec3 {
    if (i < n) return Vec3((i - (n - 1) / 2.0) * BoardGeometry.SPACING, 0.0, BoardGeometry.ROW_Z)
    val col = 2 * n - 1 - i
    return Vec3((col - (n - 1) / 2.0) * BoardGeometry.SPACING, 0.0, -BoardGeometry.ROW_Z)
}

fun containerCenter(c: Int, layout: BoardLayout): Vec3 = when {
    c < layout.n * 2 -> pitCenter(c, layout.n)
    c == layout.storeSouth -> Vec3(BoardGeometry.STORE_X, 0.0, 0.0)
    c == layout.storeNorth -> Vec3(-BoardGeometry.STORE_X, 0.0, 0.0)
    else -> Vec3()
}

/** A small, stable jitter per (container, slot), so piles look hand-placed, not gridded. */
private fun jitter(container: Int, slot: Int, axis: Int): Double {
    val r = rng(container * 1009 + slot * 31 + axis * 7)()
    return (r - 0.5) * 0.05
}

/** Resting position of a seed in a pit or a store. Seeds in the hand are placed by the hand. */
fun slotPosition(p: Placement, layout: BoardLayout): Vec3 {
    val c = p.container
    val slot = p.slot
    val center = containerCenter(c, layout)
    if (c < layout.n * 2) {
        // Layers in the bowl: 7 on the floor (one in the middle), then wider rings above.
        var layer = 0
        var index = slot
        var size = 7
        while (index >= size) {
            index -= size
            layer++
            size = 7 + layer * 2
        }
        val y = -BoardGeometry.BOWL_DEPTH + 0.1 + layer * 0.12
        var x = 0.0
        var z = 0.0
        if (!(layer == 0 && index == 0)) {
            val ringCount = if (layer == 0) 6 else size
            val ringIndex = if (layer == 0) index - 1 else index
            val radius = min(0.17 + layer * 0.05, 0.32)
            val a = (ringIndex.toDouble() / ringCount) * PI * 2 + layer * 0.5
            x = cos(a) * radius
            z = sin(a) * radius
        }
        return Vec3(
            center.x + x + jitter(c, slot, 0),
            y + jitter(c, slot, 1) * 0.4,
            center.z + z + jitter(c, slot, 2),
        )
    }
    // Stores: two columns along the long axis, layered.
    val perLayer = 14
    val layer = slot / perLayer
    val i = slot % perLayer
    val col = i % 2
    val row = i / 2
    val z = (row - 3) * 0.3
    val widthAtZ = BoardGeometry.STORE_RX * sqrt(max(0.05, 1 - (z / (BoardGeometry.STORE_RZ * 0.92)).pow(2)))
    val x = (if (col == 0) -1 else 1) * min(0.16, widthAtZ * 0.45)
    return Vec3(
        center.x + x + jitter(c, slot, 0),
        -BoardGeometry.BOWL_DEPTH + 0.1 + layer * 0.12,
        center.z + z + jitter(c, slot, 2),
    )
}
