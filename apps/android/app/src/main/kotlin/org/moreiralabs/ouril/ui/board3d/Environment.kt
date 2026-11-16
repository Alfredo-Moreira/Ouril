package org.moreiralabs.ouril.ui.board3d

import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.pow
import kotlin.math.sqrt

/**
 * The 3D board's light, as on the web (`Board3D.tsx`): an ambient light, a hemisphere light (warm
 * sky, indigo ground), two directional lights (Filament's own, in `BoardScene`), and an environment
 * of three soft light panels (drei's Lightformers) that the seeds' glaze reflects. Here the
 * environment becomes Filament's indirect light: spherical harmonics for the diffuse part and a
 * small cubemap for reflections, computed once in code. Intensities are three.js's (the camera's
 * exposure is 1).
 */
object BoardLight {
    /** A light panel facing the origin: a rectangle, or a ring of inner and outer radius. */
    class Panel(val position: Vec3, val color: DoubleArray, val intensity: Double, val width: Double, val height: Double, val ring: Boolean = false)

    val PANELS = listOf(
        Panel(Vec3(0.0, 5.0, 3.0), srgbHexToLinear("#fff1dc"), 2.4, 8.0, 3.0),
        Panel(Vec3(-6.0, 2.0, -4.0), srgbHexToLinear("#bcd8ff"), 0.8, 4.0, 4.0),
        Panel(Vec3(5.0, 3.0, 4.0), srgbHexToLinear("#ffd6a0"), 1.2, 0.5, 1.0, ring = true),
    )

    const val AMBIENT = 0.35
    const val HEMISPHERE = 0.55
    val SKY = srgbHexToLinear("#fff3e0")
    val GROUND = srgbHexToLinear("#1b2a6e")

    /** The light coming from direction ([dx], [dy], [dz]) (unit): a panel's colour, or black. */
    fun radiance(dx: Double, dy: Double, dz: Double): DoubleArray {
        for (p in PANELS) {
            val px = p.position.x
            val py = p.position.y
            val pz = p.position.z
            val toward = dx * px + dy * py + dz * pz
            if (toward <= 0) continue
            val t = (px * px + py * py + pz * pz) / toward
            val hx = dx * t - px
            val hy = dy * t - py
            val hz = dz * t - pz
            // The panel's axes: it faces the origin, x across (up × z), y up its face.
            val len = sqrt(px * px + py * py + pz * pz)
            val zx = -px / len
            val zy = -py / len
            val zz = -pz / len
            var xx = zz
            var xz = -zx
            val xl = sqrt(xx * xx + xz * xz)
            xx /= xl
            xz /= xl
            val yx = zy * xz
            val yy = zz * xx - zx * xz
            val yz = -zy * xx
            val u = hx * xx + hz * xz
            val v = hx * yx + hy * yy + hz * yz
            val inside = if (p.ring) {
                val r = sqrt(u * u + v * v)
                r >= p.width && r <= p.height
            } else {
                abs(u) <= p.width / 2 && abs(v) <= p.height / 2
            }
            if (inside) return DoubleArray(3) { p.color[it] * p.intensity }
        }
        return DoubleArray(3)
    }

    /** The direction of texel ([i], [j]) of cubemap face [face] (+X, -X, +Y, -Y, +Z, -Z), unit. */
    fun direction(face: Int, i: Int, j: Int, size: Int): DoubleArray {
        val s = 2 * (i + 0.5) / size - 1
        val t = 2 * (j + 0.5) / size - 1
        val d = when (face) {
            0 -> doubleArrayOf(1.0, -t, -s)
            1 -> doubleArrayOf(-1.0, -t, s)
            2 -> doubleArrayOf(s, 1.0, t)
            3 -> doubleArrayOf(s, -1.0, -t)
            4 -> doubleArrayOf(s, -t, 1.0)
            else -> doubleArrayOf(-s, -t, -1.0)
        }
        val l = sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2])
        return doubleArrayOf(d[0] / l, d[1] / l, d[2] / l)
    }

    /** The environment as a cubemap: RGB per texel, faces one after another. */
    fun cubemap(size: Int): FloatArray {
        val out = FloatArray(6 * size * size * 3)
        var k = 0
        for (face in 0 until 6) for (j in 0 until size) for (i in 0 until size) {
            val d = direction(face, i, j, size)
            val l = radiance(d[0], d[1], d[2])
            out[k++] = l[0].toFloat()
            out[k++] = l[1].toFloat()
            out[k++] = l[2].toFloat()
        }
        return out
    }

    /**
     * Filament's irradiance: nine spherical-harmonic coefficients (RGB each), already convolved
     * with the cosine lobe and divided by π, so that diffuse light = albedo × Σ cᵢ·basisᵢ(n) with
     * basis (1, y, z, x, yx, yz, 3z²−1, zx, x²−y²). The panels, plus the ambient and hemisphere lights.
     */
    fun irradiance(size: Int = 64): FloatArray {
        val k = doubleArrayOf(0.282095, 0.488603, 0.488603, 0.488603, 1.092548, 1.092548, 0.315392, 1.092548, 0.546274)
        val band = doubleArrayOf(PI, 2 * PI / 3, 2 * PI / 3, 2 * PI / 3, PI / 4, PI / 4, PI / 4, PI / 4, PI / 4)
        val sums = Array(9) { DoubleArray(3) }
        for (face in 0 until 6) for (j in 0 until size) for (i in 0 until size) {
            val s = 2 * (i + 0.5) / size - 1
            val t = 2 * (j + 0.5) / size - 1
            val solidAngle = 4.0 / size / size / (1 + s * s + t * t).pow(1.5)
            val d = direction(face, i, j, size)
            val l = radiance(d[0], d[1], d[2])
            if (l[0] == 0.0 && l[1] == 0.0 && l[2] == 0.0) continue
            val (x, y, z) = Triple(d[0], d[1], d[2])
            val basis = doubleArrayOf(1.0, y, z, x, y * x, y * z, 3 * z * z - 1, z * x, x * x - y * y)
            for (b in 0 until 9) for (c in 0 until 3) sums[b][c] += l[c] * basis[b] * solidAngle
        }
        val out = FloatArray(27)
        for (b in 0 until 9) for (c in 0 until 3) {
            out[3 * b + c] = (band[b] / PI * k[b] * k[b] * sums[b][c]).toFloat()
        }
        // three.js's ambient and hemisphere lights reach the diffuse term divided by π.
        for (c in 0 until 3) {
            out[c] += ((AMBIENT + HEMISPHERE * (SKY[c] + GROUND[c]) / 2) / PI).toFloat()
            out[3 + c] += (HEMISPHERE * (SKY[c] - GROUND[c]) / 2 / PI).toFloat()
        }
        return out
    }
}
