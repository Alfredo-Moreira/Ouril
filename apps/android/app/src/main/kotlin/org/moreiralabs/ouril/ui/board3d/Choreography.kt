package org.moreiralabs.ouril.ui.board3d

import org.moreiralabs.ouril.game.FrameKind
import org.moreiralabs.ouril.game.frameDelay
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * The sowing and capture choreography of the 3D board, as the web's `board3d/choreography.ts`
 * (the reference): [applyFrame] plans a display frame (the hand's keyframes and how each seed
 * moves) and [stepChoreography] advances everything to a time. `MotionVectorsTest` replays the
 * web's recordings in `shared/motion/vectors` against this code (docs/architecture/board-motion.md).
 *
 * A hand comes in from the mover's side of the table and plays the move the way a person does:
 * it reaches into the pit, closes around the seeds and lifts them (pickup); glides to the next
 * pit and lets one seed fall (sow); passes over the origin pit (skip); dips into a captured pit,
 * carries the seeds to the store and opens (capture); then withdraws. Other changes (collecting
 * the remaining seeds, undo) fly seeds along an arc.
 */

/** Height of the handful (the centre of the held seeds) while carrying, and while grabbing. */
const val CARRY_Y = 0.8
const val DIP_Y = 0.3

/** How far off-board the hand enters from and withdraws to (towards the mover's side). */
private val OFFSTAGE = Vec3(0.0, 2.6, 3.2)

class HandKey(val at: Double, val pos: Vec3, val grip: Double)

class HandState {
    var keys: List<HandKey> = emptyList()

    /** +1: South (comes in from the camera side), -1: North (from the far side). */
    var side = 1

    /** Sampled each rendered frame. */
    val pos = Vec3()
    var grip = 0.0

    /** The hand is drawn until this time (infinity while a move is being played). */
    var hideAt = Double.NEGATIVE_INFINITY
}

private fun smooth(t: Double) = t * t * (3 - 2 * t)

private fun easeOut(t: Double) = 1 - (1 - t).pow(3)

fun sampleHand(h: HandState, now: Double) {
    val k = h.keys
    if (k.isEmpty()) return
    if (now <= k[0].at) {
        h.pos.copy(k[0].pos)
        h.grip = k[0].grip
        return
    }
    for (i in 0 until k.size - 1) {
        val a = k[i]
        val b = k[i + 1]
        if (now < b.at) {
            val u = if (b.at > a.at) (now - a.at) / (b.at - a.at) else 1.0
            h.pos.lerpVectors(a.pos, b.pos, easeInOut(u))
            h.grip = lerp(a.grip, b.grip, smooth(u))
            return
        }
    }
    val last = k.last()
    h.pos.copy(last.pos)
    h.grip = last.grip
}

/** A seed's place in the handful, relative to the hand's anchor. */
private fun handOffset(slot: Int, out: Vec3): Vec3 {
    val a = slot * 2.39996
    val r = 0.075 * sqrt(slot.toDouble())
    return out.set(cos(a) * r, slot * 0.012 - 0.04, sin(a) * r)
}

enum class SeedMode { REST, FLY, CARRY }

enum class SeedPhase { WAIT, LIFT, HELD, FALL }

class SeedMotion(at: Vec3) {
    var mode = SeedMode.REST
    val pos = at.clone()
    val from = at.clone()
    val to = at.clone()

    // fly
    var start = 0.0
    var dur = 0.0
    var arc = 0.0

    // carry: joins the hand at `attach` (lifting for `lift` s), leaves it at `release`, then
    // falls for `fall` s into `to`.
    var slot = 0
    var attach = 0.0
    var lift = 0.0
    var release = Double.POSITIVE_INFINITY
    var fall = 0.0
    var phase = SeedPhase.WAIT

    /** The seed's tilt this frame, about its own x axis, on top of its resting orientation. */
    var tumble = 0.0
}

/** What the board shows in one display frame. */
data class Highlight(val pit: Int?, val kind: FrameKind)

class BoardFrame(
    val pits: List<Int>,
    val stores: List<Int>,
    val highlight: Highlight?,
    /** The game's frame pace (0 = snap, no hand). */
    val speed: Double,
)

/** The hand and every seed, carried from one display frame to the next. */
class Choreography {
    var placement: List<Placement>? = null
    var motions: List<SeedMotion> = emptyList()
    val hand = HandState()
}

/** A new display frame: move the hand and choreograph the seeds that changed container. */
fun applyFrame(c: Choreography, layout: BoardLayout, total: Int, f: BoardFrame, now: Double) {
    val h = c.hand
    val animate = f.speed > 0
    val kind = f.highlight?.kind ?: FrameKind.END
    val d = frameDelay(if (kind == FrameKind.END) FrameKind.SOW else kind, f.speed) / 1000
    val pit = f.highlight?.pit
    fun over(container: Int, y: Double) = containerCenter(container, layout).setY(y)
    fun offstage(p: Vec3, side: Int) = p.clone().add(Vec3(OFFSTAGE.x, OFFSTAGE.y, OFFSTAGE.z * side))

    val prev = c.placement
    if (prev == null || prev.size != total) {
        val placement = initialPlacement(layout, f.pits, f.stores, total)
        c.placement = placement
        c.motions = placement.map { SeedMotion(slotPosition(it, layout)) }
        h.keys = emptyList()
        h.hideAt = Double.NEGATIVE_INFINITY
        return
    }

    val (next, moved) = reconcile(layout, prev, f.pits, f.stores)
    c.placement = next
    val movedSet = moved.toHashSet()
    val handCount = next.count { it.container == layout.hand }

    // --- The hand's keyframes for this frame.
    if (!animate) {
        h.keys = emptyList()
        h.hideAt = Double.NEGATIVE_INFINITY
    } else {
        sampleHand(h, now)
        var start = HandKey(now, h.pos.clone(), h.grip)
        val onStage = now < h.hideAt
        fun key(fr: Double, pos: Vec3, grip: Double) = HandKey(now + fr * d, pos, grip)
        if (kind == FrameKind.PICKUP && pit != null) {
            val side = if (pit < layout.n) 1 else -1
            if (!onStage || side != h.side) {
                h.side = side
                start = HandKey(now, offstage(over(pit, CARRY_Y), side), 0.0)
            }
            h.keys = listOf(
                start,
                key(0.42, over(pit, DIP_Y), 0.0),
                key(0.58, over(pit, DIP_Y), 1.0),
                key(1.0, over(pit, CARRY_Y), 1.0),
            )
            h.hideAt = Double.POSITIVE_INFINITY
        } else if (kind == FrameKind.SOW && pit != null) {
            val above = over(pit, CARRY_Y)
            h.keys = listOf(
                start,
                key(0.5, above, 1.0),
                key(0.6, above, 0.45),
                key(0.9, above, if (handCount > 0) 1.0 else 0.45),
            )
            h.hideAt = Double.POSITIVE_INFINITY
        } else if (kind == FrameKind.SKIP && pit != null) {
            h.keys = listOf(start, key(0.8, over(pit, CARRY_Y), 1.0))
            h.hideAt = Double.POSITIVE_INFINITY
        } else if (kind == FrameKind.CAPTURE && pit != null) {
            val store = if (h.side > 0) layout.storeSouth else layout.storeNorth
            if (!onStage) start = HandKey(now, offstage(over(pit, CARRY_Y), h.side), 0.0)
            h.keys = listOf(
                start,
                key(0.24, over(pit, DIP_Y), 0.0),
                key(0.38, over(pit, DIP_Y), 1.0),
                key(0.5, over(pit, CARRY_Y), 1.0),
                key(0.8, over(store, CARRY_Y + 0.1), 1.0),
                key(0.9, over(store, CARRY_Y + 0.1), 0.1),
            )
            h.hideAt = Double.POSITIVE_INFINITY
        } else if (kind == FrameKind.GRAND_SLAM || kind == FrameKind.EXTRA_TURN) {
            // The hand pauses where it is.
        } else if (onStage) {
            // The move is over: the hand withdraws to the mover's side.
            val exit = max(0.45, 0.32 * f.speed)
            h.keys = listOf(start, HandKey(now + exit, offstage(start.pos, h.side), 0.3))
            h.hideAt = now + exit
        }
    }

    // --- The seeds.
    var lifted = 0
    var captured = 0
    val lifting = moved.count { next[it].container == layout.hand }
    var order = 0
    next.forEachIndexed { id, p ->
        val m = c.motions[id]
        val was = prev[id]
        val toHand = p.container == layout.hand
        val fromHand = was.container == layout.hand

        if (!animate) {
            m.mode = SeedMode.REST
            m.to.copy(if (toHand) over(pit ?: 0, CARRY_Y) else slotPosition(p, layout))
            m.pos.copy(m.to)
            return@forEachIndexed
        }

        if (toHand) {
            m.slot = p.slot
            if (!fromHand) {
                // Pickup: the seeds join the closing hand one after another.
                val stagger = if (lifting > 1) min(0.03, (d * 0.25) / lifting) else 0.0
                m.mode = SeedMode.CARRY
                m.phase = SeedPhase.WAIT
                m.attach = now + 0.55 * d + lifted++ * stagger
                m.lift = 0.4 * d
                m.release = Double.POSITIVE_INFINITY
            }
            return@forEachIndexed
        }

        val to = slotPosition(p, layout)
        if (fromHand && id in movedSet) {
            // Sowing: this seed slips out of the hand over its pit.
            m.mode = SeedMode.CARRY
            m.to.copy(to)
            m.release = now + 0.56 * d
            m.fall = max(0.16, 0.38 * d)
            return@forEachIndexed
        }

        if (kind == FrameKind.CAPTURE && id in movedSet) {
            // Capture: grabbed from the pit, carried to the store, dropped in.
            m.mode = SeedMode.CARRY
            m.phase = SeedPhase.WAIT
            m.to.copy(to)
            m.slot = captured
            m.attach = now + 0.36 * d + captured * 0.012
            m.lift = 0.14 * d
            m.release = now + 0.88 * d + captured * 0.018
            m.fall = max(0.14, 0.14 * d)
            captured++
            return@forEachIndexed
        }

        if (to.distanceToSquared(m.to) < 1e-6 && m.mode != SeedMode.CARRY) return@forEachIndexed
        // Collecting, undo, resume: an arc to the new container. Slot shuffles: a short hop.
        val between = id in movedSet || fromHand
        val stagger = if (between && moved.size > 1) min(0.035, (d * 0.4) / moved.size) else 0.0
        m.mode = SeedMode.FLY
        m.from.copy(m.pos)
        m.to.copy(to)
        m.start = now + (if (between) order++ * stagger else 0.0)
        m.dur = if (between) max(0.24, d * 0.9) else 0.15
        m.arc = if (between) 0.35 + m.from.distanceTo(to) * 0.1 else 0.03
    }
}

private val scratch = Vec3()

/**
 * Advances the hand and every seed to [now] ([delta]: seconds since the last call, for the seeds
 * that follow the hand). Returns whether anything is still moving.
 */
fun stepChoreography(c: Choreography, now: Double, delta: Double): Boolean {
    val h = c.hand
    sampleHand(h, now)
    val handShown = now < h.hideAt
    var active = handShown && (h.hideAt != Double.POSITIVE_INFINITY || now < (h.keys.lastOrNull()?.at ?: 0.0) + 0.1)
    val follow = 1 - exp(-delta * 18)

    c.motions.forEachIndexed { i, mo ->
        var tumble = 0.0
        fun inHand(): Vec3 {
            handOffset(mo.slot, scratch).add(h.pos)
            // Seeds sway a touch in the grip; only while the hand holds them.
            scratch.y += sin(now * 5 + i * 1.7) * 0.006
            return scratch
        }

        when (mo.mode) {
            SeedMode.REST -> mo.pos.copy(mo.to)
            SeedMode.FLY -> {
                val t = if (mo.dur <= 0) 1.0 else clamp((now - mo.start) / mo.dur, 0.0, 1.0)
                val e = easeInOut(t)
                mo.pos.lerpVectors(mo.from, mo.to, e)
                mo.pos.y += sin(PI * e) * mo.arc
                if (mo.arc > 0.1) tumble = e * PI * 0.6
                if (t < 1) active = true else mo.mode = SeedMode.REST
            }
            SeedMode.CARRY -> {
                active = true
                if (mo.phase == SeedPhase.WAIT && now >= mo.attach) {
                    mo.phase = SeedPhase.LIFT
                    mo.from.copy(mo.pos)
                }
                if ((mo.phase == SeedPhase.LIFT || mo.phase == SeedPhase.HELD) && now >= mo.release) {
                    mo.phase = SeedPhase.FALL
                    mo.from.copy(mo.pos)
                }
                when (mo.phase) {
                    SeedPhase.LIFT -> {
                        val u = if (mo.lift <= 0) 1.0 else clamp((now - mo.attach) / mo.lift, 0.0, 1.0)
                        mo.pos.lerpVectors(mo.from, inHand(), easeOut(u))
                        mo.pos.y += sin(PI * u) * 0.08
                        tumble = u * PI * 0.4
                        if (u >= 1) mo.phase = SeedPhase.HELD
                    }
                    SeedPhase.HELD -> mo.pos.lerp(inHand(), follow)
                    SeedPhase.FALL -> {
                        val t = clamp((now - mo.release) / mo.fall, 0.0, 1.0)
                        // Accelerating fall into the bowl, then a small bounce.
                        val land = 0.82
                        if (t < land) {
                            val fr = t / land
                            mo.pos.lerpVectors(mo.from, mo.to, easeOut(fr))
                            mo.pos.y = lerp(mo.from.y, mo.to.y, fr * fr)
                        } else {
                            val b = (t - land) / (1 - land)
                            mo.pos.copy(mo.to)
                            mo.pos.y += sin(PI * b) * 0.05
                        }
                        tumble = t * PI * 0.8
                        if (t >= 1) {
                            mo.mode = SeedMode.REST
                            mo.phase = SeedPhase.WAIT
                            mo.release = Double.POSITIVE_INFINITY
                        }
                    }
                    // WAIT: the seed stays where it is until the hand closes on it.
                    SeedPhase.WAIT -> Unit
                }
            }
        }
        mo.tumble = tumble
    }
    return active
}

/**
 * The seeds on the board while a move plays, as the web's 3D scene keeps them: seeds in the hand
 * aren't on the board, so the total is remembered from the last frame with no move playing (it
 * can change between positions, as in the tutorial).
 */
class SeedTotal {
    private var rest = -1

    fun of(pits: List<Int>, stores: List<Int>, moving: Boolean): Int {
        val onBoard = pits.sum() + stores[0] + stores[1]
        if (rest < 0) rest = onBoard
        if (!moving && rest != onBoard) rest = onBoard
        return if (moving) max(rest, onBoard) else onBoard
    }
}

// --- How each seed looks ------------------------------------------------------------------------

private val SEED_COLORS = listOf("#8b8d84", "#7a7f75", "#9b9a8e", "#6e7268", "#a5a092", "#868372", "#757a6c")

/** A rotation as a unit quaternion (x, y, z, w), like three.js's `Quaternion`. */
class Quat(val x: Double, val y: Double, val z: Double, val w: Double)

/** Each seed's resting orientation, the same on every load (and on the web). */
fun seedOrientations(total: Int): List<Quat> = List(total) { i ->
    val r = rng(i * 97 + 3)
    val axis = Vec3(r() - 0.5, 1.0, r() - 0.5).normalize()
    val angle = r() * PI * 2
    val s = sin(angle / 2)
    Quat(axis.x * s, axis.y * s, axis.z * s, cos(angle / 2))
}

/** Each seed's colour (linear RGB, three values per seed), the same on every load (and on the web). */
fun seedColors(total: Int): DoubleArray {
    val r = rng(42)
    val out = DoubleArray(total * 3)
    for (i in 0 until total) {
        val rgb = srgbHexToLinear(SEED_COLORS[floor(r() * SEED_COLORS.size).toInt()])
        val lifted = offsetLightness(rgb, (r() - 0.5) * 0.06)
        lifted.copyInto(out, i * 3)
    }
    return out
}

/** A `#rrggbb` colour in linear RGB, as three.js's `Color` converts it. */
fun srgbHexToLinear(hex: String): DoubleArray {
    val v = hex.removePrefix("#").toInt(16)
    fun channel(c: Int): Double {
        val s = c / 255.0
        return if (s < 0.04045) s * 0.0773993808 else (s * 0.9478672986 + 0.0521327014).pow(2.4)
    }
    return doubleArrayOf(channel(v shr 16 and 255), channel(v shr 8 and 255), channel(v and 255))
}

/** three.js's `Color.offsetHSL(0, 0, dl)` on a linear colour: lightness up or down, in HSL. */
private fun offsetLightness(rgb: DoubleArray, dl: Double): DoubleArray {
    val (r, g, b) = Triple(rgb[0], rgb[1], rgb[2])
    val mx = maxOf(r, g, b)
    val mn = minOf(r, g, b)
    var hue = 0.0
    var saturation = 0.0
    val lightness = (mn + mx) / 2.0
    if (mn != mx) {
        val delta = mx - mn
        saturation = if (lightness <= 0.5) delta / (mx + mn) else delta / (2 - mx - mn)
        hue = when (mx) {
            r -> (g - b) / delta + (if (g < b) 6 else 0)
            g -> (b - r) / delta + 2
            else -> (r - g) / delta + 4
        }
        hue /= 6
    }
    val h = ((hue % 1) + 1) % 1
    val s = clamp(saturation, 0.0, 1.0)
    val l = clamp(lightness + dl, 0.0, 1.0)
    if (s == 0.0) return doubleArrayOf(l, l, l)
    val p = if (l <= 0.5) l * (1 + s) else l + s - (l * s)
    val q = (2 * l) - p
    return doubleArrayOf(hue2rgb(q, p, h + 1.0 / 3), hue2rgb(q, p, h), hue2rgb(q, p, h - 1.0 / 3))
}

private fun hue2rgb(p: Double, q: Double, t0: Double): Double {
    var t = t0
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1.0 / 6) return p + (q - p) * 6 * t
    if (t < 1.0 / 2) return q
    if (t < 2.0 / 3) return p + (q - p) * 6 * (2.0 / 3 - t)
    return p
}
