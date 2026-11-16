package org.moreiralabs.ouril.ui.board3d

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * The 3D board's geometry, light and camera, checked on the JVM (Filament itself only runs on a
 * device): the board's top covers exactly the board minus its pits and stores, normals face the
 * side you see, the camera's projection agrees with itself, and the light comes from above.
 */
class BoardGeometryTest {
    @Test
    fun theBoardTopIsTheBoardMinusItsPitsAndStores() {
        val slab = slab(6)
        var area = 0.0
        var capTriangles = 0
        forEachTriangle(slab) { a, b, c, normal ->
            if (normal[1] > 0.999 && a[1] == BoardGeometry.TOP.toFloat()) {
                capTriangles++
                val ux = b[0] - a[0]
                val uz = b[2] - a[2]
                val vx = c[0] - a[0]
                val vz = c[2] - a[2]
                area += abs(ux * vz - uz * vx) / 2.0
            }
        }
        val r = BoardGeometry.CORNER
        // A rounded rectangle whose quadratic corners each cut r²/6 off its rectangle.
        val outline = 4 * BoardGeometry.HALF_W * BoardGeometry.HALF_D - 4 * r * r / 6
        // Holes of 96 sides: a polygon's area, a hair under the ellipse's.
        val polygon = 96 * sin(2 * PI / 96) / (2 * PI)
        val holes = (12 * PI * BoardGeometry.PIT_R * BoardGeometry.PIT_R + 2 * PI * BoardGeometry.STORE_RX * BoardGeometry.STORE_RZ) * polygon
        assertEquals(outline - holes, area, 0.01)
        assertTrue("the top is triangulated", capTriangles > 1000)
    }

    @Test
    fun normalsAreUnitAndFaceTheSideYouSee() {
        for ((name, mesh) in listOf("slab" to slab(6), "bowl" to bowl(), "seed" to sphere(0.105, 20, 14), "capsule" to capsule(0.05, 0.2, 4, 10), "palm" to roundedBox(0.48, 0.13, 0.46, 0.06, 4), "sleeve" to cylinder(0.24, 0.19, 3.0, 20))) {
            val n = mesh.normals
            for (i in 0 until mesh.vertexCount) {
                val l = sqrt((n[3 * i] * n[3 * i] + n[3 * i + 1] * n[3 * i + 1] + n[3 * i + 2] * n[3 * i + 2]).toDouble())
                assertEquals("$name normal $i", 1.0, l, 1e-3)
            }
            assertTrue("$name indices", mesh.indices.all { it in 0 until mesh.vertexCount })
        }
        // Spheres, capsules and the palm: outwards from their centre.
        for ((name, mesh) in listOf("seed" to sphere(0.105, 20, 14), "capsule" to capsule(0.05, 0.2, 4, 10), "palm" to roundedBox(0.48, 0.13, 0.46, 0.06, 4))) {
            for (i in 0 until mesh.vertexCount) {
                val p = mesh.positions
                val n = mesh.normals
                val dot = p[3 * i] * n[3 * i] + p[3 * i + 1] * n[3 * i + 1] + p[3 * i + 2] * n[3 * i + 2]
                assertTrue("$name normal $i points in", dot > -1e-6)
            }
        }
        // A bowl is seen from above: its floor faces up.
        val b = bowl()
        val floor = (0 until b.vertexCount).minBy { b.positions[3 * it + 1] }
        assertTrue("the bowl's floor faces up", b.normals[3 * floor + 1] > 0.9)
    }

    @Test
    fun theProjectionAgreesWithItself() {
        val eye = Vec3()
        CameraRig().apply {
            setInputs(CameraInputs(finished = false, highlightKind = null, speed = 2.2), 0.0)
            step(0.0, 0.0, 0.62, eye)
        }
        val p = Projection(eye, 1080f, 1742f)
        // The camera looks at its target: it shows in the middle of the view.
        val (cx, cy) = p.project(CameraLens.LOOK_AT)!!
        assertEquals(540f, cx, 0.5f)
        assertEquals(871f, cy, 0.5f)
        // A point on the cloth's plane, projected and found again.
        val ground = Vec3(2.0, -0.64, -1.0)
        val (sx, sy) = p.project(ground)!!
        val back = p.onPlane(sx, sy, -0.64)
        assertNotNull(back)
        assertEquals(ground.x, back!!.x, 1e-3)
        assertEquals(ground.z, back.z, 1e-3)
        // On a tall phone screen the whole board fits across.
        for (x in listOf(-BoardGeometry.HALF_W, BoardGeometry.HALF_W)) {
            val (ex, _) = p.project(Vec3(x, 0.0, BoardGeometry.HALF_D))!!
            assertTrue("the board's edge $x is on screen ($ex)", ex in 0f..1080f)
        }
    }

    @Test
    fun theLightComesFromAbove() {
        val sh = BoardLight.irradiance(32)
        // Diffuse light on a surface facing up versus down: basis 1 and y.
        val up = sh[0] + sh[3]
        val down = sh[0] - sh[3]
        assertTrue("up ($up) is brighter than down ($down)", up > down && down > 0)
        // The +Y face's centre looks straight up.
        val d = BoardLight.direction(2, 15, 15, 32)
        assertEquals(1.0, d[1], 0.01)
        // The big warm panel above the board is visible from the origin.
        val panel = BoardLight.radiance(0.0, 5.0 / sqrt(34.0), 3.0 / sqrt(34.0))
        assertTrue(panel[0] > 1)
    }

    @Test
    fun matcIsTheSameFilamentAsTheApp() {
        val repo = File(System.getProperty("ouril.repo"))
        val docker = Regex("""ARG FILAMENT_VERSION=(\S+)""").find(repo.resolve("docker/android.Dockerfile").readText())?.groupValues?.get(1)
        val app = Regex("""filament = "([^"]+)"""").find(repo.resolve("apps/android/gradle/libs.versions.toml").readText())?.groupValues?.get(1)
        assertNotNull(docker)
        assertEquals("docker/android.Dockerfile's matc and the app's filament-android", docker, app)
    }

    private fun forEachTriangle(mesh: MeshData, action: (FloatArray, FloatArray, FloatArray, FloatArray) -> Unit) {
        val p = mesh.positions
        val n = mesh.normals
        val idx = mesh.indices
        for (t in 0 until idx.size / 3) {
            val v = IntArray(3) { idx[3 * t + it] }
            val corner = { k: Int -> floatArrayOf(p[3 * v[k]], p[3 * v[k] + 1], p[3 * v[k] + 2]) }
            action(corner(0), corner(1), corner(2), floatArrayOf(n[3 * v[0]], n[3 * v[0] + 1], n[3 * v[0] + 2]))
        }
    }
}
