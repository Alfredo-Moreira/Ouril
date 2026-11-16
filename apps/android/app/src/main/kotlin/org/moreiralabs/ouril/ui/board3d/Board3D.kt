package org.moreiralabs.ouril.ui.board3d

import android.graphics.Bitmap
import android.graphics.Paint
import android.view.TextureView
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import org.moreiralabs.ouril.BuildConfig
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.game.Frame
import org.moreiralabs.ouril.game.FrameKind
import org.moreiralabs.ouril.game.frameDelay
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.ui.board.Board
import org.moreiralabs.ouril.ui.theme.PanuType
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

/**
 * The game board: the 3D board where the device can draw it (as on the web, ADR 0036), the 2D
 * board otherwise (and in JVM tests). [frameSpeed] is the game's pace; with [reducedMotion] the
 * 3D board's seeds snap into place and the camera stays still.
 */
@Composable
fun GameBoard(
    pits: List<Int>,
    stores: List<Int>,
    legal: List<Int>,
    onPlay: ((Int) -> Unit)?,
    frame: Frame?,
    hintPit: Int?,
    upright: Boolean,
    frameSpeed: Double,
    reducedMotion: Boolean,
    finished: Boolean,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val density = LocalDensity.current.density
    val scene = remember {
        if (Board3DSupport.available(context)) {
            runCatching { BoardScene(context, density) }.onFailure { Board3DSupport.failed(it.toString()) }.getOrNull()
        } else {
            null
        }
    }
    var failed by remember { mutableStateOf(false) }
    if (scene != null && !failed) {
        DisposableEffect(scene) {
            scene.onFailure = {
                Board3DSupport.failed(it.toString())
                failed = true
            }
            onDispose { scene.destroy() }
        }
        Board3D(
            pits = pits,
            stores = stores,
            legal = legal,
            onPlay = onPlay,
            highlight = frame?.takeIf { it.kind != FrameKind.END }?.let { Highlight(it.pit, it.kind) },
            hintPit = hintPit,
            speed = if (reducedMotion) 0.0 else frameSpeed,
            finished = finished,
            scene = scene,
            modifier = modifier,
        )
    } else {
        Box(modifier) {
            Board(
                pits = pits,
                stores = stores,
                legal = legal,
                onPlay = onPlay,
                frame = frame,
                hintPit = hintPit,
                upright = upright,
                modifier = Modifier.fillMaxSize(),
                frameMillis = frame?.let { frameDelay(it.kind, frameSpeed).toLong() } ?: 440,
            )
            // Test builds say why the 3D board isn't showing (a technical note, never in a release).
            val reason = Board3DSupport.failure
            if (reason != null && BuildConfig.BUILD_TYPE != "release") {
                Text(
                    reason,
                    style = PanuType.tab.copy(color = Color(0xFFF3EFE6)),
                    modifier = Modifier.align(Alignment.TopStart).padding(6.dp).background(Color(0xCC0B1024)).padding(6.dp),
                )
            }
        }
    }
}

/** The board's motion between frames: the choreography, the camera and the rings, on one clock. */
class BoardMotion(pitCount: Int) {
    val choreo = Choreography()
    private val rig = CameraRig()
    val rings = createRings(pitCount)
    var ringTargets: List<RingTarget> = emptyList()

    /** The 3D board's pace (0 = reduced motion). */
    var speed = 1.0
    val total = SeedTotal()
    val eye = Vec3()
    private val origin = System.nanoTime()
    private var last: Double? = null

    /** Seconds on the scene's clock (the frame clock's nanoseconds, or now). */
    fun now(nanos: Long = System.nanoTime()) = (nanos - origin) / 1e9

    fun frame(layout: BoardLayout, total: Int, f: BoardFrame) = applyFrame(choreo, layout, total, f, now())

    fun camera(inputs: CameraInputs) = rig.setInputs(inputs, now())

    /** Advances everything to [now] for a view of [aspect]; returns whether anything still moves. */
    fun step(now: Double, aspect: Double): Boolean {
        // Seconds since the last frame, capped so a stall doesn't jump (the web's clock delta).
        val dt = last?.let { min(0.05, max(0.0, now - it)) } ?: 0.0
        last = now
        val seeds = stepChoreography(choreo, now, dt)
        val camera = rig.step(now, dt, aspect, eye)
        val lit = ringTargets.isNotEmpty() && stepRings(rings, ringTargets, dt, speed)
        return seeds || camera || lit
    }
}

@Composable
private fun Board3D(
    pits: List<Int>,
    stores: List<Int>,
    legal: List<Int>,
    onPlay: ((Int) -> Unit)?,
    highlight: Highlight?,
    hintPit: Int?,
    speed: Double,
    finished: Boolean,
    scene: BoardScene,
    modifier: Modifier,
) {
    val n = pits.size / 2
    val layout = remember(n) { BoardLayout(n) }
    val motion = remember(n) { BoardMotion(2 * n) }
    val total = motion.total.of(pits, stores, highlight != null)
    var projection by remember { mutableStateOf<Projection?>(null) }
    val cloth = remember { Cloth() }

    LaunchedEffect(pits, stores, highlight, speed, total) {
        motion.speed = speed
        motion.frame(layout, total, BoardFrame(pits, stores, highlight, speed))
        scene.request()
    }
    LaunchedEffect(legal, highlight, hintPit) {
        motion.ringTargets = ringTargets(n, legal, highlight, hintPit)
        scene.request()
    }
    LaunchedEffect(finished, highlight?.kind, speed) {
        motion.camera(CameraInputs(finished, highlight?.kind, speed))
        scene.request()
    }
    DisposableEffect(scene, motion) {
        scene.onFrame = { nanos ->
            val w = scene.width
            val h = scene.height
            if (w == 0 || h == 0) {
                false
            } else {
                val now = motion.now(nanos)
                val active = motion.step(now, w.toDouble() / h)
                scene.update(motion.choreo, motion.rings, motion.eye, now)
                val p = projection
                if (p == null || p.width != w.toFloat() || p.height != h.toFloat() || p.eye.distanceToSquared(motion.eye) > 1e-12) {
                    projection = Projection(motion.eye, w.toFloat(), h.toFloat())
                }
                active
            }
        }
        scene.request()
        onDispose { scene.onFrame = { false } }
    }

    Box(modifier.background(NIGHT)) {
        // The cloth, in its own colours (the web doesn't tone-map it), with the board's soft shadow.
        Canvas(Modifier.fillMaxSize()) {
            val p = projection ?: return@Canvas
            drawIntoCanvas { cloth.draw(it.nativeCanvas, p) }
        }
        AndroidView(
            factory = { context ->
                TextureView(context).also {
                    it.isOpaque = false
                    it.importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
                    scene.attach(it)
                }
            },
            modifier = Modifier.fillMaxSize().clearAndSetSemantics { },
        )
        val chipSize = chipFontSize()
        for (c in 0 until 2 * n + 2) {
            val anchor = chipAnchor(c, n)
            val store = c >= 2 * n
            Chip(
                count = if (store) stores[c - 2 * n] else pits[c],
                store = store,
                south = c == 2 * n,
                fontSize = if (store) chipSize.second else chipSize.first,
                modifier = Modifier.centeredAt { projection?.project(anchor) },
            )
        }
        PitControls(n, pits, legal, onPlay, highlight, hintPit) { projection }
    }
}

private val NIGHT = Color(0xFF0B1024)

/** Where a container's count chip sits: outside its row for pits, below the stores (the web's `ChipProjector`). */
private fun chipAnchor(c: Int, n: Int): Vec3 {
    if (c < 2 * n) {
        val p = pitCenter(c, n)
        return Vec3(p.x, 0.02, if (p.z > 0) p.z + BoardGeometry.PIT_R + 0.3 else p.z - BoardGeometry.PIT_R - 0.3)
    }
    val x = if (c == 2 * n) BoardGeometry.STORE_X else -BoardGeometry.STORE_X
    return Vec3(x, 0.02, BoardGeometry.STORE_RZ + 0.38)
}

/** The web's chip sizes: clamp(0.7rem, 1.4vw, 0.88rem) for pits and clamp(0.85rem, 1.7vw, 1.08rem) for stores. */
@Composable
private fun chipFontSize(): Pair<Float, Float> {
    val w = LocalConfiguration.current.screenWidthDp.toFloat()
    return (w * 0.014f).coerceIn(11.2f, 14.08f) to (w * 0.017f).coerceIn(13.6f, 17.28f)
}

/** The web's ease-out, `cubic-bezier(0.23, 1, 0.32, 1)`. */
private val EaseOut = CubicBezierEasing(0.23f, 1f, 0.32f, 1f)

/** A seed count over the 3D view: a basalt pill, the stores' in their owners' colours (South is you). */
@Composable
private fun Chip(count: Int, store: Boolean, south: Boolean, fontSize: Float, modifier: Modifier) {
    // Store counts pop when they change (the web's `score-pop`: 1.3× at 35% of 280 ms).
    val pop = remember { Animatable(1f) }
    if (store) {
        LaunchedEffect(count) {
            pop.snapTo(1f)
            pop.animateTo(1.3f, tween(98, easing = EaseOut))
            pop.animateTo(1f, tween(182, easing = EaseOut))
        }
    }
    val shape = RoundedCornerShape(50)
    val background = when {
        !store -> Color(0xD60B1024)
        south -> Color(0xFF3346B8)
        else -> Color(0xFFC8202A)
    }
    Box(
        modifier
            .shadow(3.dp, shape, ambientColor = Color(0x59000000), spotColor = Color(0x59000000))
            .background(background, shape)
            .border(1.dp, Color(0xFFF3EFE6).copy(alpha = if (store) 0.4f else 0.22f), shape)
            .height((fontSize * 1.75f).dp)
            .defaultMinSize(minWidth = (fontSize * 1.75f).dp)
            .padding(horizontal = (fontSize * 0.4f).dp)
            .clearAndSetSemantics { },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            count.toString(),
            style = PanuType.count.copy(fontSize = fontSize.sp, color = Color(0xFFF3EFE6)),
            modifier = Modifier.graphicsLayer {
                scaleX = pop.value
                scaleY = pop.value
            },
        )
    }
}

/** Places the content centred on a projected point (read at layout time, so the camera moving only re-lays it out). */
private fun Modifier.centeredAt(point: () -> Pair<Float, Float>?) = layout { measurable, constraints ->
    val placeable = measurable.measure(constraints.copy(minWidth = 0, minHeight = 0))
    layout(placeable.width, placeable.height) {
        point()?.let { (x, y) -> placeable.place((x - placeable.width / 2f).roundToInt(), (y - placeable.height / 2f).roundToInt()) }
    }
}

/**
 * The pits as controls over the 3D view: labelled for screen readers, and the touch targets (the
 * web's hit circles, radius 0.62 over each pit). The playable ones are buttons.
 */
@Composable
private fun PitControls(
    n: Int,
    pits: List<Int>,
    legal: List<Int>,
    onPlay: ((Int) -> Unit)?,
    highlight: Highlight?,
    hintPit: Int?,
    projection: () -> Projection?,
) {
    for (i in 0 until 2 * n) {
        val center = pitCenter(i, n).setY(0.05)
        val edge = Vec3(center.x + BoardGeometry.PIT_R + 0.12, 0.05, center.z)
        val play = onPlay?.takeIf { highlight == null && i in legal }
        val own = i < n
        val label = tr(
            if (own) R.string.board_pit_yours else R.string.board_pit_opponent,
            "number" to (if (own) i + 1 else i - n + 1),
            "count" to pits[i],
        )
        val hinted = tr(R.string.board_pit_hint, "label" to label)
        Box(
            Modifier
                .layout { measurable, _ ->
                    val p = projection()
                    val c = p?.project(center)
                    val e = p?.project(edge)
                    val r = if (c != null && e != null) abs(e.first - c.first).roundToInt() else 0
                    val placeable = measurable.measure(Constraints.fixed(2 * r, 2 * r))
                    layout(2 * r, 2 * r) {
                        if (c != null) placeable.place(c.first.roundToInt() - r, c.second.roundToInt() - r)
                    }
                }
                .then(
                    if (play != null) {
                        Modifier.clickable(role = Role.Button, onClick = { play(i) })
                            .semantics { contentDescription = if (hintPit == i) hinted else label }
                    } else {
                        Modifier.semantics { contentDescription = label }
                    },
                ),
        )
    }
}

/**
 * The panu cloth under the board, drawn in perspective tile by tile (one 5-unit tile per
 * bitmap, as the web repeats its texture), then the board's contact shadow.
 */
private class Cloth {
    private val tile: Bitmap = BoardTextures.clothTile().apply { setHasMipMap(true) }
    private val shadow: Bitmap = BoardTextures.contactShadow(SHADOW_W, SHADOW_D)
    private val paint = Paint(Paint.FILTER_BITMAP_FLAG or Paint.ANTI_ALIAS_FLAG)
    private val tileVerts = FloatArray((GRID + 1) * (GRID + 1) * 2)
    private val shadowVerts = FloatArray((SHADOW_GRID_X + 1) * (SHADOW_GRID_Z + 1) * 2)

    fun draw(canvas: android.graphics.Canvas, p: Projection) {
        val corners = listOf(0f to 0f, p.width to 0f, 0f to p.height, p.width to p.height)
            .map { (x, y) -> p.onPlane(x, y, CLOTH_Y) ?: return }
        val tx0 = floor(corners.minOf { it.x } / TILE).toInt()
        val tx1 = floor(corners.maxOf { it.x } / TILE).toInt()
        val tz0 = floor(corners.minOf { it.z } / TILE).toInt()
        val tz1 = floor(corners.maxOf { it.z } / TILE).toInt()
        for (tz in tz0..tz1) for (tx in tx0..tx1) {
            if (!mesh(p, tx * TILE, tz * TILE, TILE, TILE, CLOTH_Y, GRID, GRID, tileVerts)) continue
            canvas.drawBitmapMesh(tile, GRID, GRID, tileVerts, 0, null, 0, paint)
        }
        if (mesh(p, -SHADOW_W / 2, -SHADOW_D / 2, SHADOW_W, SHADOW_D, SHADOW_Y, SHADOW_GRID_X, SHADOW_GRID_Z, shadowVerts)) {
            canvas.drawBitmapMesh(shadow, SHADOW_GRID_X, SHADOW_GRID_Z, shadowVerts, 0, null, 0, paint)
        }
    }

    /** A grid over a world rectangle on the plane at [y], projected to the view. */
    private fun mesh(p: Projection, x0: Double, z0: Double, w: Double, d: Double, y: Double, gx: Int, gz: Int, out: FloatArray): Boolean {
        var k = 0
        val point = Vec3()
        for (j in 0..gz) for (i in 0..gx) {
            point.set(x0 + w * i / gx, y, z0 + d * j / gz)
            val (sx, sy) = p.project(point) ?: return false
            out[k++] = sx
            out[k++] = sy
        }
        return true
    }

    private companion object {
        /** One texture tile: 5 world units, four strips of 1.25 (the pits' spacing). */
        const val TILE = 5.0
        const val GRID = 8
        const val CLOTH_Y = -BoardGeometry.THICK - 0.02
        const val SHADOW_Y = -BoardGeometry.THICK - 0.01
        const val SHADOW_W = BoardGeometry.HALF_W * 2.6
        const val SHADOW_D = BoardGeometry.HALF_D * 3.2
        const val SHADOW_GRID_X = 12
        const val SHADOW_GRID_Z = 6
    }
}
