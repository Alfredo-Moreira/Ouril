package org.moreiralabs.ouril.ui.board3d

import kotlin.math.sqrt
import kotlin.math.tan

/**
 * The 3D board's camera as a pinhole, for what Compose draws over and under the 3D view: the
 * seed-count chips, the pits' touch targets, the cloth. The same projection Filament renders
 * with (a symmetric perspective with [CameraLens.FOV] vertically, looking at [CameraLens.LOOK_AT]).
 */
class Projection(eye: Vec3, val width: Float, val height: Float) {
    val eye: Vec3 = eye.clone()
    private val forward = unit(CameraLens.LOOK_AT.x - eye.x, CameraLens.LOOK_AT.y - eye.y, CameraLens.LOOK_AT.z - eye.z)
    private val right = unit(cross(forward, Vec3(0.0, 1.0, 0.0)))
    private val up = cross(right, forward)
    private val tanV = tan(Math.toRadians(CameraLens.FOV) / 2)
    private val tanH = tanV * width / height

    /** Where a world point shows, in pixels from the view's top left; null behind the camera. */
    fun project(p: Vec3): Pair<Float, Float>? {
        val dx = p.x - eye.x
        val dy = p.y - eye.y
        val dz = p.z - eye.z
        val depth = dx * forward.x + dy * forward.y + dz * forward.z
        if (depth <= CameraLens.NEAR) return null
        val x = (dx * right.x + dy * right.y + dz * right.z) / (depth * tanH)
        val y = (dx * up.x + dy * up.y + dz * up.z) / (depth * tanV)
        return ((x + 1) / 2 * width).toFloat() to ((1 - y) / 2 * height).toFloat()
    }

    /** The point on the horizontal plane at [planeY] seen at pixel ([sx], [sy]); null above the horizon. */
    fun onPlane(sx: Float, sy: Float, planeY: Double): Vec3? {
        val nx = sx / width * 2.0 - 1
        val ny = 1 - sy / height * 2.0
        val dir = Vec3(
            forward.x + right.x * nx * tanH + up.x * ny * tanV,
            forward.y + right.y * nx * tanH + up.y * ny * tanV,
            forward.z + right.z * nx * tanH + up.z * ny * tanV,
        )
        if (dir.y >= -1e-9) return null
        val t = (planeY - eye.y) / dir.y
        return Vec3(eye.x + dir.x * t, planeY, eye.z + dir.z * t)
    }

    private companion object {
        fun cross(a: Vec3, b: Vec3) = Vec3(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)

        fun unit(v: Vec3) = unit(v.x, v.y, v.z)

        fun unit(x: Double, y: Double, z: Double): Vec3 {
            val l = sqrt(x * x + y * y + z * z)
            return Vec3(x / l, y / l, z / l)
        }
    }
}
