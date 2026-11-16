package org.moreiralabs.ouril.ui.board3d

import org.moreiralabs.ouril.game.FrameKind
import kotlin.math.abs
import kotlin.math.exp

/**
 * The glowing rings on the pits' rims, as the web's `board3d/rings.ts`: which colour and strength
 * each one aims for, and how fast it gets there (a fast attack, a slower release).
 */
class RingTarget(val color: String, val a: Double)

/**
 * Target glow per pit: star yellow means "you can play this" (and the sowing pulse); a capture
 * flashes in the capturer's light tint (a pit on the North row is taken by South, you); focus is
 * cotton white; the hint is sea.
 */
fun ringTargets(n: Int, legal: List<Int>, highlight: Highlight?, hintPit: Int?, focusPit: Int? = null): List<RingTarget> =
    List(n * 2) { i ->
        val isLegal = i in legal
        when {
            highlight?.pit == i && highlight.kind == FrameKind.CAPTURE -> RingTarget(if (i >= n) "#b4beff" else "#ffb4b4", 1.0)
            highlight?.pit == i && (highlight.kind == FrameKind.SOW || highlight.kind == FrameKind.PICKUP) -> RingTarget("#f7d116", 0.85)
            highlight?.pit == i && highlight.kind == FrameKind.SKIP -> RingTarget("#4fd1dc", 0.8)
            focusPit == i -> RingTarget("#f3efe6", 1.0)
            hintPit == i -> RingTarget("#4fd1dc", 0.95)
            isLegal -> RingTarget("#f7d116", 0.75)
            else -> RingTarget("#f7d116", 0.0)
        }
    }

/** A ring as drawn: its opacity and its colour (linear RGB, also its glow). */
class RingState {
    var opacity = 0.0
    val color = srgbHexToLinear("#ffc861")
}

fun createRings(count: Int): List<RingState> = List(count) { RingState() }

/** Eases every ring towards its target. Returns whether any is still changing. */
fun stepRings(rings: List<RingState>, targets: List<RingTarget>, dt: Double, speed: Double): Boolean {
    var active = false
    rings.forEachIndexed { i, ring ->
        val t = targets[i]
        // Fast attack, slower release (asymmetric timing).
        val rising = t.a > ring.opacity
        val k = if (speed > 0) 1 - exp(-dt * (if (rising) 22 else 9)) else 1.0
        ring.opacity = lerp(ring.opacity, t.a, k)
        val target = srgbHexToLinear(t.color)
        for (c in 0..2) ring.color[c] += (target[c] - ring.color[c]) * k
        if (abs(ring.opacity - t.a) > 0.004) active = true
    }
    return active
}
