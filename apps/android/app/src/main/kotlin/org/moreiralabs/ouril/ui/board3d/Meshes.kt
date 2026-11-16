package org.moreiralabs.ouril.ui.board3d

import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * The 3D board's geometry, built in code as on the web (`Board3D.tsx`, `Sowing.tsx`): nothing is
 * downloaded. World units, y up. Normals point to the side you see (the materials draw both faces).
 */
class MeshData(val positions: FloatArray, val normals: FloatArray, val uvs: FloatArray?, val indices: IntArray) {
    val vertexCount get() = positions.size / 3
}

private class FloatList {
    var data = FloatArray(1024)
    var size = 0

    fun add(vararg v: Float) {
        if (size + v.size > data.size) data = data.copyOf(maxOf(data.size * 2, size + v.size))
        v.copyInto(data, size)
        size += v.size
    }

    fun toArray() = data.copyOf(size)
}

private class IntList {
    var data = IntArray(1024)
    var size = 0

    fun add(vararg v: Int) {
        if (size + v.size > data.size) data = data.copyOf(maxOf(data.size * 2, size + v.size))
        v.copyInto(data, size)
        size += v.size
    }

    fun toArray() = data.copyOf(size)
}

/** A UV sphere, as three.js's `SphereGeometry`. */
fun sphere(radius: Double, widthSegments: Int, heightSegments: Int): MeshData {
    val p = FloatList()
    val n = FloatList()
    val uv = FloatList()
    val idx = IntList()
    for (iy in 0..heightSegments) {
        val v = iy.toDouble() / heightSegments
        for (ix in 0..widthSegments) {
            val u = ix.toDouble() / widthSegments
            val x = -cos(u * 2 * PI) * sin(v * PI)
            val y = cos(v * PI)
            val z = sin(u * 2 * PI) * sin(v * PI)
            p.add((x * radius).toFloat(), (y * radius).toFloat(), (z * radius).toFloat())
            n.add(x.toFloat(), y.toFloat(), z.toFloat())
            uv.add(u.toFloat(), (1 - v).toFloat())
        }
    }
    val row = widthSegments + 1
    for (iy in 0 until heightSegments) {
        for (ix in 0 until widthSegments) {
            val a = iy * row + ix + 1
            val b = iy * row + ix
            val c = (iy + 1) * row + ix
            val d = (iy + 1) * row + ix + 1
            if (iy != 0) idx.add(a, b, d)
            if (iy != heightSegments - 1) idx.add(b, c, d)
        }
    }
    return MeshData(p.toArray(), n.toArray(), uv.toArray(), idx.toArray())
}

/** A profile (radius, height) turned about the y axis, as three.js's `LatheGeometry` (its normals too). */
fun lathe(points: List<Pair<Double, Double>>, segments: Int): MeshData {
    // Normals of the profile, as three.js computes them: averaged across each joint.
    val init = DoubleArray(points.size * 2)
    var prevX = 0.0
    var prevY = 0.0
    for (j in points.indices) {
        when (j) {
            0 -> {
                val dx = points[1].first - points[0].first
                val dy = points[1].second - points[0].second
                prevX = dy
                prevY = -dx
                val l = sqrt(prevX * prevX + prevY * prevY)
                init[0] = prevX / l
                init[1] = prevY / l
            }
            points.size - 1 -> {
                init[2 * j] = prevX
                init[2 * j + 1] = prevY
            }
            else -> {
                val dx = points[j + 1].first - points[j].first
                val dy = points[j + 1].second - points[j].second
                val cx = dy
                val cy = -dx
                var nx = cx + prevX
                var ny = cy + prevY
                val l = sqrt(nx * nx + ny * ny)
                nx /= l
                ny /= l
                init[2 * j] = nx
                init[2 * j + 1] = ny
                prevX = cx
                prevY = cy
            }
        }
    }
    val p = FloatList()
    val n = FloatList()
    val uv = FloatList()
    for (i in 0..segments) {
        val phi = i * 2 * PI / segments
        val s = sin(phi)
        val c = cos(phi)
        for (j in points.indices) {
            val (r, y) = points[j]
            p.add((r * s).toFloat(), y.toFloat(), (r * c).toFloat())
            var nx = init[2 * j] * s
            var ny = init[2 * j + 1]
            var nz = init[2 * j] * c
            val l = sqrt(nx * nx + ny * ny + nz * nz).takeIf { it > 0 } ?: 1.0
            nx /= l
            ny /= l
            nz /= l
            n.add(nx.toFloat(), ny.toFloat(), nz.toFloat())
            uv.add((i.toDouble() / segments).toFloat(), (j.toDouble() / (points.size - 1)).toFloat())
        }
    }
    val idx = IntList()
    for (i in 0 until segments) {
        for (j in 0 until points.size - 1) {
            val base = j + i * points.size
            val a = base
            val b = base + points.size
            val c = base + points.size + 1
            val d = base + 1
            idx.add(a, b, d, c, d, b)
        }
    }
    return MeshData(p.toArray(), n.toArray(), uv.toArray(), idx.toArray())
}

/** A capsule along y: [length] between the two half-spheres, as three.js's `CapsuleGeometry`. */
fun capsule(radius: Double, length: Double, capSegments: Int, radialSegments: Int): MeshData {
    val profile = mutableListOf<Pair<Double, Double>>()
    val steps = capSegments * 2
    for (k in 0..steps) {
        val a = 1.5 * PI + k * (0.5 * PI) / steps
        profile += radius * cos(a) to -length / 2 + radius * sin(a)
    }
    for (k in 0..steps) {
        val a = k * (0.5 * PI) / steps
        profile += radius * cos(a) to length / 2 + radius * sin(a)
    }
    // The poles sit exactly on the axis (cos(3π/2) isn't exactly 0 in floating point).
    profile[0] = 0.0 to profile[0].second
    profile[profile.lastIndex] = 0.0 to profile.last().second
    return lathe(profile, radialSegments)
}

/** A capped cylinder along y, as three.js's `CylinderGeometry`. */
fun cylinder(radiusTop: Double, radiusBottom: Double, height: Double, radialSegments: Int): MeshData {
    val p = FloatList()
    val n = FloatList()
    val idx = IntList()
    val half = height / 2
    val slope = (radiusBottom - radiusTop) / height
    for (y in 0..1) {
        val r = y * (radiusBottom - radiusTop) + radiusTop
        for (x in 0..radialSegments) {
            val theta = x.toDouble() / radialSegments * 2 * PI
            val s = sin(theta)
            val c = cos(theta)
            p.add((r * s).toFloat(), (-y * height + half).toFloat(), (r * c).toFloat())
            val l = sqrt(1 + slope * slope)
            n.add((s / l).toFloat(), (slope / l).toFloat(), (c / l).toFloat())
        }
    }
    val row = radialSegments + 1
    for (x in 0 until radialSegments) {
        val a = x
        val b = row + x
        val c = row + x + 1
        val d = x + 1
        idx.add(a, b, d, b, c, d)
    }
    for ((top, r, y) in listOf(Triple(true, radiusTop, half), Triple(false, radiusBottom, -half))) {
        val center = p.size / 3
        p.add(0f, y.toFloat(), 0f)
        n.add(0f, if (top) 1f else -1f, 0f)
        for (x in 0..radialSegments) {
            val theta = x.toDouble() / radialSegments * 2 * PI
            p.add((r * sin(theta)).toFloat(), y.toFloat(), (r * cos(theta)).toFloat())
            n.add(0f, if (top) 1f else -1f, 0f)
        }
        for (x in 0 until radialSegments) idx.add(center, center + 1 + x, center + 2 + x)
    }
    return MeshData(p.toArray(), n.toArray(), null, idx.toArray())
}

/** A box with rounded edges and corners of [radius], as drei's `RoundedBox`. */
fun roundedBox(width: Double, height: Double, depth: Double, radius: Double, smoothness: Int): MeshData {
    val half = doubleArrayOf(width / 2, height / 2, depth / 2)
    // Grid lines along an axis: the flat middle is one cell, each rounded band `smoothness` cells.
    fun lines(h: Double): List<Double> {
        val inner = h - radius
        val band = (0..smoothness).map { -h + radius * it / smoothness }
        return band + band.reversed().map { -it }.let { if (inner > 0) it else it.drop(1) }
    }
    val p = FloatList()
    val n = FloatList()
    val idx = IntList()
    // Each face: the axis it faces along, its sign, and the two axes across it.
    val faces = listOf(
        intArrayOf(0, 1, 2, 1), intArrayOf(0, -1, 2, 1), intArrayOf(1, 1, 0, 2),
        intArrayOf(1, -1, 0, 2), intArrayOf(2, 1, 0, 1), intArrayOf(2, -1, 0, 1),
    )
    for ((axis, sign, ua, va) in faces.map { listOf(it[0], it[1], it[2], it[3]) }) {
        val us = lines(half[ua])
        val vs = lines(half[va])
        val start = p.size / 3
        for (v in vs) {
            for (u in us) {
                val q = DoubleArray(3)
                q[axis] = sign * half[axis]
                q[ua] = u
                q[va] = v
                val inner = DoubleArray(3) { k -> q[k].coerceIn(-(half[k] - radius), half[k] - radius) }
                var nx = q[0] - inner[0]
                var ny = q[1] - inner[1]
                var nz = q[2] - inner[2]
                val l = sqrt(nx * nx + ny * ny + nz * nz)
                if (l < 1e-9) {
                    nx = 0.0; ny = 0.0; nz = 0.0
                    when (axis) { 0 -> nx = sign.toDouble(); 1 -> ny = sign.toDouble(); else -> nz = sign.toDouble() }
                } else {
                    nx /= l; ny /= l; nz /= l
                }
                p.add((inner[0] + nx * radius).toFloat(), (inner[1] + ny * radius).toFloat(), (inner[2] + nz * radius).toFloat())
                n.add(nx.toFloat(), ny.toFloat(), nz.toFloat())
            }
        }
        val row = us.size
        for (j in 0 until vs.size - 1) {
            for (i in 0 until us.size - 1) {
                val a = start + j * row + i
                idx.add(a, a + 1, a + row + 1, a, a + row + 1, a + row)
            }
        }
    }
    return MeshData(p.toArray(), n.toArray(), null, idx.toArray())
}

/** A flat ring in the xz plane, facing up, as three.js's `RingGeometry` laid on the board. */
fun ring(inner: Double, outer: Double, segments: Int): MeshData {
    val p = FloatList()
    val n = FloatList()
    val idx = IntList()
    for (s in 0..segments) {
        val a = s * 2 * PI / segments
        p.add((inner * cos(a)).toFloat(), 0f, (-inner * sin(a)).toFloat())
        p.add((outer * cos(a)).toFloat(), 0f, (-outer * sin(a)).toFloat())
        n.add(0f, 1f, 0f, 0f, 1f, 0f)
    }
    for (s in 0 until segments) {
        val a = 2 * s
        idx.add(a, a + 1, a + 3, a, a + 3, a + 2)
    }
    return MeshData(p.toArray(), n.toArray(), null, idx.toArray())
}

/** An axis-aligned box centred on the origin. */
fun box(width: Double, height: Double, depth: Double): MeshData {
    val hx = (width / 2).toFloat()
    val hy = (height / 2).toFloat()
    val hz = (depth / 2).toFloat()
    val p = FloatList()
    val n = FloatList()
    val idx = IntList()
    fun face(nx: Float, ny: Float, nz: Float, corners: List<FloatArray>) {
        val start = p.size / 3
        for (c in corners) {
            p.add(c[0], c[1], c[2])
            n.add(nx, ny, nz)
        }
        idx.add(start, start + 1, start + 2, start, start + 2, start + 3)
    }
    face(1f, 0f, 0f, listOf(floatArrayOf(hx, -hy, -hz), floatArrayOf(hx, hy, -hz), floatArrayOf(hx, hy, hz), floatArrayOf(hx, -hy, hz)))
    face(-1f, 0f, 0f, listOf(floatArrayOf(-hx, -hy, hz), floatArrayOf(-hx, hy, hz), floatArrayOf(-hx, hy, -hz), floatArrayOf(-hx, -hy, -hz)))
    face(0f, 1f, 0f, listOf(floatArrayOf(-hx, hy, -hz), floatArrayOf(-hx, hy, hz), floatArrayOf(hx, hy, hz), floatArrayOf(hx, hy, -hz)))
    face(0f, -1f, 0f, listOf(floatArrayOf(-hx, -hy, hz), floatArrayOf(-hx, -hy, -hz), floatArrayOf(hx, -hy, -hz), floatArrayOf(hx, -hy, hz)))
    face(0f, 0f, 1f, listOf(floatArrayOf(-hx, -hy, hz), floatArrayOf(hx, -hy, hz), floatArrayOf(hx, hy, hz), floatArrayOf(-hx, hy, hz)))
    face(0f, 0f, -1f, listOf(floatArrayOf(hx, -hy, -hz), floatArrayOf(-hx, -hy, -hz), floatArrayOf(-hx, hy, -hz), floatArrayOf(hx, hy, -hz)))
    return MeshData(p.toArray(), n.toArray(), null, idx.toArray())
}

/** The bowl carved into each pit (a lathe of the web's profile); stores scale it. */
fun bowl(): MeshData {
    val r = BoardGeometry.PIT_R
    return lathe(
        listOf(
            r + 0.002 to 0.04,
            r to 0.0,
            r * 0.97 to -0.1,
            r * 0.86 to -0.24,
            r * 0.62 to -0.36,
            r * 0.3 to -BoardGeometry.BOWL_DEPTH + 0.01,
            0.0 to -BoardGeometry.BOWL_DEPTH,
        ),
        48,
    )
}

// --- The board's slab ----------------------------------------------------------------------------

private class P2(val x: Double, val z: Double)

/** A closed outline on the board's top, with each point's offset direction for the bevel. */
private class Contour(val points: List<P2>, val offsets: List<P2>, val visible: List<P2>)

private const val BEVEL_THICKNESS = 0.06
private const val BEVEL_SIZE = 0.05
private const val BEVEL_SEGMENTS = 4
private const val CURVE_SEGMENTS = 48

/** The longest straight piece of an outline or a cell's rim (world units). */
private const val EDGE_STEP = 0.08

/** The points from [a] (included) to [b] (left out), at most [EDGE_STEP] apart; the same both ways. */
private fun edge(a: P2, b: P2): List<P2> {
    val dx = b.x - a.x
    val dz = b.z - a.z
    val count = maxOf(1, kotlin.math.ceil(sqrt(dx * dx + dz * dz) / EDGE_STEP).toInt())
    return (0 until count).map { P2(a.x + dx * it / count, a.z + dz * it / count) }
}

/**
 * The carved slab (the web's `ExtrudeGeometry`: a rounded rectangle 0.62 deep with a 0.06 bevel,
 * every pit and store cut through). Flat-shaded triangles, as three.js builds it; the underside,
 * never seen, is left out. UVs are world units (x, -z), the web's `WorldUVGenerator`.
 */
fun slab(n: Int): MeshData {
    val hw = BoardGeometry.HALF_W
    val hd = BoardGeometry.HALF_D
    val r = BoardGeometry.CORNER
    val cellW = BoardGeometry.SPACING
    val x0 = -n * cellW / 2
    val splits = (0..n).map { x0 + it * cellW }

    // The outline: far edge, corner, right edge, corner, near edge, corner, left edge, corner. Its
    // straight edges break at the cell walls and every EDGE_STEP, exactly as the cells' rims do, so
    // the top and the walls share their points. Each piece includes its start and leaves out its end.
    fun corner(ax: Double, az: Double, cx: Double, cz: Double, bx: Double, bz: Double) = (0 until CURVE_SEGMENTS).map { j ->
        val t = j.toDouble() / CURVE_SEGMENTS
        val u = 1 - t
        P2(u * u * ax + 2 * u * t * cx + t * t * bx, u * u * az + 2 * u * t * cz + t * t * bz)
    }
    val farRight = corner(hw - r, -hd, hw, -hd, hw, -hd + r)
    val nearRight = corner(hw, hd - r, hw, hd, hw - r, hd)
    val nearLeft = corner(-hw + r, hd, -hw, hd, -hw, hd - r)
    val farLeft = corner(-hw, -hd + r, -hw, -hd, -hw + r, -hd)
    val rightEdge = edge(P2(hw, -hd + r), P2(hw, hd - r))
    val leftEdge = edge(P2(-hw, hd - r), P2(-hw, -hd + r))
    val xs = listOf(-hw + r) + splits + listOf(hw - r)
    val outline = mutableListOf<P2>()
    for (k in 0 until xs.size - 1) outline += edge(P2(xs[k], -hd), P2(xs[k + 1], -hd))
    outline += farRight + rightEdge + nearRight
    for (k in xs.size - 1 downTo 1) outline += edge(P2(xs[k], hd), P2(xs[k - 1], hd))
    outline += nearLeft + leftEdge + farLeft
    val outer = outlineContour(outline)

    // Holes: the pits (circles) and the stores (ellipses), 96 points each as the web samples them.
    fun ellipse(cx: Double, cz: Double, rx: Double, rz: Double): Contour {
        val pts = mutableListOf<P2>()
        val inward = mutableListOf<P2>()
        for (j in 0 until CURVE_SEGMENTS * 2) {
            val a = j * 2 * PI / (CURVE_SEGMENTS * 2)
            pts += P2(cx + rx * cos(a), cz + rz * sin(a))
            val gx = cos(a) / rx
            val gz = sin(a) / rz
            val l = sqrt(gx * gx + gz * gz)
            inward += P2(-gx / l, -gz / l)
        }
        return Contour(pts, inward, inward)
    }
    val pits = (0 until 2 * n).associateWith { pitCenter(it, n).let { c -> ellipse(c.x, c.z, BoardGeometry.PIT_R, BoardGeometry.PIT_R) } }
    val south = ellipse(BoardGeometry.STORE_X, 0.0, BoardGeometry.STORE_RX, BoardGeometry.STORE_RZ)
    val north = ellipse(-BoardGeometry.STORE_X, 0.0, BoardGeometry.STORE_RX, BoardGeometry.STORE_RZ)

    val out = SlabBuilder()
    // The top: one cell per pit, each zipped between its rim and its hole, then the two ends.
    fun rim(vararg corners: P2) = corners.indices.flatMap { edge(corners[it], corners[(it + 1) % corners.size]) }
    for (k in 0 until n) {
        val a = splits[k]
        val b = splits[k + 1]
        out.cap(rim(P2(a, 0.0), P2(b, 0.0), P2(b, hd), P2(a, hd)), pits.getValue(k).points, pitCenter(k, n))
        out.cap(rim(P2(a, -hd), P2(b, -hd), P2(b, 0.0), P2(a, 0.0)), pits.getValue(2 * n - 1 - k).points, pitCenter(2 * n - 1 - k, n))
    }
    // The ends: their part of the outline, closed by the outermost cell walls.
    val xr = splits.last()
    val right = edge(P2(xr, -hd), P2(hw - r, -hd)) + farRight + rightEdge + nearRight +
        edge(P2(hw - r, hd), P2(xr, hd)) + edge(P2(xr, hd), P2(xr, 0.0)) + edge(P2(xr, 0.0), P2(xr, -hd))
    out.cap(right, south.points, Vec3(BoardGeometry.STORE_X, 0.0, 0.0))
    val xl = splits.first()
    val left = edge(P2(xl, hd), P2(-hw + r, hd)) + nearLeft + leftEdge + farLeft +
        edge(P2(-hw + r, -hd), P2(xl, -hd)) + edge(P2(xl, -hd), P2(xl, 0.0)) + edge(P2(xl, 0.0), P2(xl, hd))
    out.cap(left, north.points, Vec3(-BoardGeometry.STORE_X, 0.0, 0.0))

    // The bevel and walls of every outline.
    out.walls(outer)
    pits.values.forEach { out.walls(it) }
    out.walls(south)
    out.walls(north)
    return out.build()
}

/** The outline's outward directions (for the bevel) from its neighbours. */
private fun outlineContour(points: List<P2>): Contour {
    val offsets = points.indices.map { i ->
        val prev = points[(i - 1 + points.size) % points.size]
        val next = points[(i + 1) % points.size]
        var nx = next.z - prev.z
        var nz = -(next.x - prev.x)
        val l = sqrt(nx * nx + nz * nz)
        nx /= l
        nz /= l
        // Outward: away from the board's centre.
        if (nx * points[i].x + nz * points[i].z < 0) P2(-nx, -nz) else P2(nx, nz)
    }
    return Contour(points, offsets, offsets)
}

private class SlabBuilder {
    private val p = FloatList()
    private val n = FloatList()
    private val uv = FloatList()

    /** A cell of the top: the triangles between its rim ([rim]) and its hole, both seen from [c]. */
    fun cap(rim: List<P2>, hole: List<P2>, c: Vec3) {
        fun sorted(pts: List<P2>) = pts.map { it to atan2(it.z - c.z, it.x - c.x) }.sortedBy { it.second }
        val o = sorted(rim)
        val h = sorted(hole)
        fun angle(list: List<Pair<P2, Double>>, i: Int) = list[i % list.size].second + (i / list.size) * 2 * PI
        var i = 0
        var j = 0
        while (i < o.size || j < h.size) {
            val takeRim = j >= h.size || (i < o.size && angle(o, i + 1) <= angle(h, j + 1))
            if (takeRim) {
                tri(o[i % o.size].first, o[(i + 1) % o.size].first, h[j % h.size].first)
                i++
            } else {
                tri(o[i % o.size].first, h[(j + 1) % h.size].first, h[j % h.size].first)
                j++
            }
        }
    }

    private fun tri(a: P2, b: P2, c: P2) {
        val y = BoardGeometry.TOP
        for (q in listOf(a, b, c)) {
            p.add(q.x.toFloat(), y.toFloat(), q.z.toFloat())
            n.add(0f, 1f, 0f)
            uv.add(q.x.toFloat(), (-q.z).toFloat())
        }
    }

    /** The bevel (four bands) and the wall below an outline. */
    fun walls(contour: Contour) {
        // Offset and height of each ring, from the top edge down (the web's front bevel, then the wall).
        val rings = (0 until BEVEL_SEGMENTS).map { b ->
            val t = b.toDouble() / BEVEL_SEGMENTS
            BEVEL_SIZE * sin(t * PI / 2) to BEVEL_THICKNESS * cos(t * PI / 2)
        } + listOf(BEVEL_SIZE to 0.0, BEVEL_SIZE to -BoardGeometry.THICK)
        val count = contour.points.size
        for (k in 0 until rings.size - 1) {
            val (s0, y0) = rings[k]
            val (s1, y1) = rings[k + 1]
            for (i in 0 until count) {
                val i2 = (i + 1) % count
                val a = at(contour, i, s0, y0)
                val b = at(contour, i2, s0, y0)
                val c = at(contour, i2, s1, y1)
                val d = at(contour, i, s1, y1)
                // three.js's side-wall UVs: along x or along the shape's y, and down the extrusion.
                val alongX = abs(a[2] - b[2]) < abs(a[0] - b[0])
                fun uvOf(v: DoubleArray) = Pair(if (alongX) v[0] else -v[2], 1 - (v[1] + BoardGeometry.THICK))
                val visible = contour.visible[i]
                wallTri(a, b, d, uvOf(a), uvOf(b), uvOf(d), visible)
                wallTri(b, c, d, uvOf(b), uvOf(c), uvOf(d), visible)
            }
        }
    }

    private fun at(contour: Contour, i: Int, size: Double, y: Double): DoubleArray {
        val q = contour.points[i]
        val o = contour.offsets[i]
        return doubleArrayOf(q.x + o.x * size, y, q.z + o.z * size)
    }

    private fun wallTri(
        a: DoubleArray, b: DoubleArray, c: DoubleArray,
        ua: Pair<Double, Double>, ub: Pair<Double, Double>, uc: Pair<Double, Double>,
        visible: P2,
    ) {
        val ux = b[0] - a[0]
        val uy = b[1] - a[1]
        val uz = b[2] - a[2]
        val vx = c[0] - a[0]
        val vy = c[1] - a[1]
        val vz = c[2] - a[2]
        var nx = uy * vz - uz * vy
        var ny = uz * vx - ux * vz
        var nz = ux * vy - uy * vx
        val l = sqrt(nx * nx + ny * ny + nz * nz)
        if (l < 1e-12) return
        nx /= l
        ny /= l
        nz /= l
        // Face the side you see: outwards (or into a hole), and up on the bevel.
        if (nx * visible.x + ny * 0.5 + nz * visible.z < 0) {
            nx = -nx; ny = -ny; nz = -nz
        }
        for ((q, t) in listOf(a to ua, b to ub, c to uc)) {
            p.add(q[0].toFloat(), q[1].toFloat(), q[2].toFloat())
            n.add(nx.toFloat(), ny.toFloat(), nz.toFloat())
            uv.add(t.first.toFloat(), t.second.toFloat())
        }
    }

    fun build(): MeshData {
        val positions = p.toArray()
        return MeshData(positions, n.toArray(), uv.toArray(), IntArray(positions.size / 3) { it })
    }
}
