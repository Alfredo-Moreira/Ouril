package org.moreiralabs.ouril.ui.board3d

import org.moreiralabs.ouril.game.FrameKind
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.atan
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.max
import kotlin.math.sin
import kotlin.math.tan

/**
 * The 3D board's camera, as the web's `board3d/cameraRig.ts`: it keeps the whole board in view at
 * any aspect ratio, pushes in a little on captures and eases into a slightly higher angle when the
 * game ends. Nothing moves on ordinary sowing.
 */
object CameraLens {
    /** Vertical field of view, in degrees. */
    const val FOV = 30.0
    const val NEAR = 0.1
    const val FAR = 100.0
    val LOOK_AT = Vec3(0.0, -0.2, 0.15)
}

data class CameraInputs(val finished: Boolean, val highlightKind: FrameKind?, val speed: Double)

class CameraRig {
    private class Angles(var el: Double, var az: Double, var push: Double)

    private val current = Angles(0.9, 0.0, 0.0)
    private val target = Angles(0.9, 0.0, 0.0)

    /** When the push-in of a capture lets go (scene seconds), if one is pending. */
    private var releaseAt: Double? = null
    private var inputs: CameraInputs? = null

    /** The board's state changed (a new display frame, the game ending). */
    fun setInputs(next: CameraInputs, now: Double) {
        val before = inputs
        inputs = next
        if (before == null || before.highlightKind != next.highlightKind || before.speed != next.speed) {
            // A small push-in on captures. A new highlight cancels a pending release.
            releaseAt = null
            val kind = next.highlightKind
            if (next.speed > 0 && (kind == FrameKind.CAPTURE || kind == FrameKind.GRAND_SLAM)) {
                target.push = if (kind == FrameKind.GRAND_SLAM) 0.07 else 0.035
                releaseAt = now + 0.26
            }
        }
        if (before == null || before.finished != next.finished) {
            // A calm, slightly higher angle when the game ends.
            target.el = if (next.finished) 1.02 else 0.9
            target.az = if (next.finished) 0.08 else 0.0
        }
    }

    /**
     * Advances the camera to [now] ([dt]: seconds since the last frame) for a view of [aspect]
     * (width / height). Writes the camera's position into [out] and returns whether it's still easing.
     */
    fun step(now: Double, dt: Double, aspect: Double, out: Vec3): Boolean {
        val c = current
        val t = target
        releaseAt?.let {
            if (now >= it) {
                t.push = 0.0
                releaseAt = null
            }
        }
        val speed = inputs?.speed ?: 1.0
        val finished = inputs?.finished ?: false
        val k = if (speed > 0) 1 - exp(-dt * 6) else 1.0
        c.el = lerp(c.el, t.el, k)
        c.az = lerp(c.az, t.az, if (finished) 1 - exp(-dt * 1.5) else k)
        c.push = lerp(c.push, t.push, k)

        // Narrow views (phones): look down more steeply so the pits stay readable.
        val narrow = clamp((2.1 - aspect) / 0.6, 0.0, 1.0)
        val el = c.el + narrow * 0.22
        val vf = CameraLens.FOV * (PI / 180)
        val hf = 2 * atan(tan(vf / 2) * aspect)
        val fitW = (BoardGeometry.HALF_W + 0.5) / tan(hf / 2)
        val fitD = (BoardGeometry.HALF_D + 1.1) / tan(vf / 2)
        val d = max(fitW, fitD) * (1 - c.push)
        val look = CameraLens.LOOK_AT
        out.set(
            look.x + sin(c.az) * cos(el) * d,
            look.y + sin(el) * d,
            look.z + cos(c.az) * cos(el) * d,
        )
        return abs(c.el - t.el) > 1e-4 || abs(c.az - t.az) > 1e-4 || abs(c.push - t.push) > 1e-4 || releaseAt != null
    }
}
