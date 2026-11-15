package org.moreiralabs.ouril.ui.board

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.game.Frame
import org.moreiralabs.ouril.game.FrameKind
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.ui.theme.PanuType
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin
import kotlin.random.Random

/**
 * The board, drawn in Compose (ADR 0008): a carved board on a panu cloth, the grey ouris, the
 * seed counts, rings for what you can play, and each move shown frame by frame (the frames come
 * from the core's events, `game/Frames.kt`), with the sown seed flying from pit to pit.
 *
 * Geometry is in board units, the board lying across (South's row along the bottom, its store
 * on the right). [upright] turns it a quarter turn clockwise for a phone held upright: South's
 * row runs down the left column into its store at the bottom, North's up the right column, so
 * play still goes counter-clockwise and your store sits next to your nameplate.
 *
 * Accessibility doesn't depend on the drawing: every pit is also an invisible control with its
 * own label ("Your pit 3, 4 seeds"), and the playable ones are buttons.
 */
@Composable
fun Board(
    pits: List<Int>,
    stores: List<Int>,
    legal: List<Int>,
    onPlay: ((Int) -> Unit)?,
    frame: Frame?,
    hintPit: Int?,
    upright: Boolean,
    modifier: Modifier = Modifier,
    /** How long the current frame shows (the flying seed lands within it). */
    frameMillis: Long = 200,
) {
    val n = pits.size / 2
    val geometry = remember(n) { Geometry(n) }
    val measurer = rememberTextMeasurer()

    // The seed in flight: from the last pit touched to the pit of a sow frame.
    var lastPit by remember { mutableStateOf<Int?>(null) }
    var flight by remember { mutableStateOf<Pair<Int, Int>?>(null) }
    val progress = remember { Animatable(1f) }
    LaunchedEffect(frame) {
        val f = frame
        if (f == null) {
            lastPit = null
            return@LaunchedEffect
        }
        val from = lastPit
        if (f.kind == FrameKind.SOW && f.pit != null && from != null) {
            flight = from to f.pit
            progress.snapTo(0f)
            progress.animateTo(1f, tween(min(frameMillis, 220L).toInt().coerceAtLeast(60), easing = FastOutSlowInEasing))
        }
        if (f.pit != null) lastPit = f.pit
    }

    BoxWithConstraints(modifier) {
        val density = LocalDensity.current
        val w = with(density) { maxWidth.toPx() }
        val h = with(density) { maxHeight.toPx() }
        val layout = geometry.fit(w, h, upright)

        Canvas(Modifier.fillMaxSize().clearAndSetSemantics { }) {
            drawCloth(upright)
            drawBoard(layout)
            for (i in 0 until 2 * n) drawPit(layout, i, pits[i])
            drawStore(layout, 2 * n, stores[0])
            drawStore(layout, 2 * n + 1, stores[1])
            drawRings(layout, legal.takeIf { onPlay != null && frame == null }.orEmpty(), frame, hintPit, n)
            flight?.let { (from, to) ->
                if (progress.value < 1f) drawFlyingSeed(layout, from, to, progress.value)
            }
            for (i in 0 until 2 * n) drawChip(layout, i, pits[i], measurer, store = null)
            drawChip(layout, 2 * n, stores[0], measurer, store = YOU_CHIP)
            drawChip(layout, 2 * n + 1, stores[1], measurer, store = THEM_CHIP)
        }

        // The pits as controls: labelled for screen readers, and the touch targets.
        for (i in 0 until 2 * n) {
            val center = layout.point(geometry.pit(i))
            val r = layout.scale * Geometry.PIT_R * 1.15f
            val playable = onPlay != null && frame == null && i in legal
            val own = i < n
            val label = tr(
                if (own) R.string.board_pit_yours else R.string.board_pit_opponent,
                "number" to (if (own) i + 1 else i - n + 1),
                "count" to pits[i],
            )
            val hinted = tr(R.string.board_pit_hint, "label" to label)
            Box(
                Modifier
                    .offset(with(density) { (center.x - r).toDp() }, with(density) { (center.y - r).toDp() })
                    .size(with(density) { (2 * r).toDp() })
                    .then(
                        if (playable) {
                            Modifier.clickable(role = Role.Button, onClick = { onPlay(i) })
                                .semantics { contentDescription = if (hintPit == i) hinted else label }
                        } else {
                            Modifier.semantics { contentDescription = label }
                        },
                    ),
            )
        }
    }
}

private val YOU_CHIP = Color(0xFF3346B8)
private val THEM_CHIP = Color(0xFFC8202A)
private val COTTON = Color(0xFFF5F1E8)

/** Board units: the board lies across, 12.8 × 5, with a margin of cloth around it. */
private class Geometry(val n: Int) {
    companion object {
        const val W = 12.8f
        const val H = 5f
        const val MARGIN = 0.55f
        const val PIT_R = 0.62f
        const val SPACING = 1.5f
        const val FIRST_X = 2.65f
        const val SOUTH_Y = 3.4f
        const val NORTH_Y = 1.6f
        const val STORE_RX = 0.62f
        const val STORE_RY = 1.75f
    }

    private val spacing = SPACING * 6f / n

    /** Pit `i` (South 0..n-1 left to right along the bottom; North n..2n-1 right to left on top). */
    fun pit(i: Int): Offset =
        if (i < n) {
            Offset(FIRST_X + i * spacing, SOUTH_Y)
        } else {
            Offset(FIRST_X + (2 * n - 1 - i) * spacing, NORTH_Y)
        }

    /** South's store on the right, North's on the left. */
    fun store(south: Boolean): Offset = Offset(if (south) W - 1.15f else 1.15f, H / 2)

    fun container(c: Int): Offset = when {
        c < 2 * n -> pit(c)
        c == 2 * n -> store(true)
        else -> store(false)
    }

    fun fit(width: Float, height: Float, upright: Boolean): Layout {
        val bw = (if (upright) H else W) + 2 * MARGIN
        val bh = (if (upright) W else H) + 2 * MARGIN
        val s = min(width / bw, height / bh)
        return Layout(this, s, (width - bw * s) / 2, (height - bh * s) / 2, upright)
    }
}

private class Layout(val g: Geometry, val scale: Float, val ox: Float, val oy: Float, val upright: Boolean) {
    /** A point in board units to canvas pixels (turned a quarter turn clockwise when upright). */
    fun point(p: Offset): Offset =
        if (upright) {
            Offset(ox + (Geometry.H - p.y + Geometry.MARGIN) * scale, oy + (p.x + Geometry.MARGIN) * scale)
        } else {
            Offset(ox + (p.x + Geometry.MARGIN) * scale, oy + (p.y + Geometry.MARGIN) * scale)
        }

    fun container(c: Int): Offset = point(g.container(c))
}

// --- Drawing -----------------------------------------------------------------------------------

/** The panu cloth: indigo strips sewn edge to edge, stitched seams, low-contrast lozenges. */
private fun DrawScope.drawCloth(upright: Boolean) {
    val strip = (if (upright) size.width else size.height) / 6f
    val shades = listOf(Color(0xFF1D2B70), Color(0xFF223280))
    val cotton = COTTON
    val count = ((if (upright) size.width else size.height) / strip).toInt() + 1
    for (s in 0..count) {
        val start = s * strip
        val color = shades[s % 2]
        if (upright) {
            drawRect(color, Offset(start, 0f), Size(strip, size.height))
        } else {
            drawRect(color, Offset(0f, start), Size(size.width, strip))
        }
        // Lozenges on every other strip.
        if (s % 2 == 1) {
            val r = strip * 0.3f
            var along = strip / 2
            val length = if (upright) size.height else size.width
            while (along < length + strip) {
                val c = if (upright) Offset(start + strip / 2, along) else Offset(along, start + strip / 2)
                val d = Path().apply {
                    moveTo(c.x, c.y - r); lineTo(c.x + r, c.y); lineTo(c.x, c.y + r); lineTo(c.x - r, c.y); close()
                }
                drawPath(d, cotton.copy(alpha = 0.18f), style = Stroke(width = strip * 0.035f))
                along += strip
            }
        }
        // The seam: a darker line with cotton stitches across it.
        if (upright) {
            drawRect(Color(0xFF0D1640).copy(alpha = 0.5f), Offset(start, 0f), Size(strip * 0.02f, size.height))
        } else {
            drawRect(Color(0xFF0D1640).copy(alpha = 0.5f), Offset(0f, start), Size(size.width, strip * 0.02f))
        }
        val stitch = strip * 0.09f
        var t = 0f
        val length = if (upright) size.height else size.width
        while (t < length) {
            val a = if (upright) Offset(start - stitch / 2, t) else Offset(t, start - stitch / 2)
            val b = if (upright) Offset(start + stitch / 2, t + stitch * 0.8f) else Offset(t + stitch * 0.8f, start + stitch / 2)
            drawLine(cotton.copy(alpha = 0.28f), a, b, strokeWidth = strip * 0.02f)
            t += stitch * 2.2f
        }
    }
}

private fun DrawScope.drawBoard(layout: Layout) {
    val s = layout.scale
    val topLeft = layout.point(Offset(0f, if (layout.upright) Geometry.H else 0f))
    val boardSize = if (layout.upright) Size(Geometry.H * s, Geometry.W * s) else Size(Geometry.W * s, Geometry.H * s)
    val origin = Offset(topLeft.x, topLeft.y)
    val radius = CornerRadius(0.95f * s)
    // A soft shadow on the cloth.
    for (k in 4 downTo 1) {
        drawRoundRect(
            Color.Black.copy(alpha = 0.07f),
            origin + Offset(0f, 0.16f * s * k / 2),
            Size(boardSize.width + 0.06f * s * k, boardSize.height + 0.06f * s * k),
            radius,
        )
    }
    val across = if (layout.upright) Brush.horizontalGradient(listOf(Color(0xFF8A4F2B), Color(0xFF6E3B1E)), origin.x, origin.x + boardSize.width)
    else Brush.verticalGradient(listOf(Color(0xFF8A4F2B), Color(0xFF6E3B1E)), origin.y, origin.y + boardSize.height)
    drawRoundRect(across, origin, boardSize, radius)
    // Grain: long, slightly wavy lines, the same on every draw.
    val rnd = Random(11)
    repeat(46) {
        val pos = rnd.nextFloat()
        val amp = (0.02f + rnd.nextFloat() * 0.05f) * s
        val color = if (rnd.nextFloat() > 0.3f) Color(0xFF3E1F0E) else Color(0xFFB07745)
        val path = Path()
        val steps = 24
        for (k in 0..steps) {
            val u = k / steps.toFloat()
            val wave = sin(u * PI.toFloat() * (2f + rnd.nextFloat())) * amp
            val p = if (layout.upright) Offset(origin.x + pos * boardSize.width + wave, origin.y + u * boardSize.height)
            else Offset(origin.x + u * boardSize.width, origin.y + pos * boardSize.height + wave)
            if (k == 0) path.moveTo(p.x, p.y) else path.lineTo(p.x, p.y)
        }
        drawPath(path, color.copy(alpha = 0.08f + rnd.nextFloat() * 0.1f), style = Stroke(width = (0.01f + rnd.nextFloat() * 0.03f) * s))
    }
    // A light top edge.
    drawRoundRect(Color.White.copy(alpha = 0.08f), origin, boardSize, radius, style = Stroke(width = 0.05f * s))
}

private val BOWL = Brush.radialGradient(listOf(Color(0xFF24110A), Color(0xFF3A1D0D), Color(0xFF5A3420)))

private fun DrawScope.drawBowl(center: Offset, rx: Float, ry: Float) {
    val brush = Brush.radialGradient(
        listOf(Color(0xFF24110A), Color(0xFF3A1D0D), Color(0xFF5A3420)),
        center = center + Offset(0f, -ry * 0.15f),
        radius = maxOf(rx, ry) * 1.05f,
    )
    drawOval(brush, Offset(center.x - rx, center.y - ry), Size(2 * rx, 2 * ry))
    // The rim catches the light along its lower edge.
    drawArc(
        Color(0xFFC08A5C).copy(alpha = 0.45f), 20f, 140f, false,
        Offset(center.x - rx, center.y - ry), Size(2 * rx, 2 * ry), style = Stroke(width = 0.05f * (rx + ry)),
    )
}

private fun DrawScope.drawPit(layout: Layout, i: Int, count: Int) {
    val c = layout.container(i)
    val r = Geometry.PIT_R * layout.scale
    drawBowl(c, r, r)
    drawSeeds(c, count, i, r, r, layout.scale)
}

private fun DrawScope.drawStore(layout: Layout, c: Int, count: Int) {
    val center = layout.container(c)
    val (rx, ry) = if (layout.upright) Geometry.STORE_RY to Geometry.STORE_RX else Geometry.STORE_RX to Geometry.STORE_RY
    drawBowl(center, rx * layout.scale, ry * layout.scale)
    drawSeeds(center, count, c, rx * layout.scale, ry * layout.scale, layout.scale)
}

/** The ouris in a bowl: rings around the centre, then a jittered pile; the same every draw. */
private fun DrawScope.drawSeeds(center: Offset, count: Int, container: Int, rx: Float, ry: Float, s: Float) {
    val shown = min(count, 40)
    val rnd = Random(container * 1009 + 7)
    for (k in 0 until shown) {
        val ring = when {
            k == 0 -> 0f
            k < 7 -> 0.42f
            k < 16 -> 0.72f
            else -> 0.25f + rnd.nextFloat() * 0.55f
        }
        val slots = when {
            k < 7 -> 6
            k < 16 -> 9
            else -> 1
        }
        val angle = if (k in 1 until 16) (k * 2 * PI / slots).toFloat() + (if (k >= 7) 0.3f else 0f) else rnd.nextFloat() * 2 * PI.toFloat()
        val jitter = (rnd.nextFloat() - 0.5f) * 0.06f * s
        val p = center + Offset(cos(angle) * ring * rx * 0.78f + jitter, sin(angle) * ring * ry * 0.78f + jitter)
        drawSeed(p, s, rnd.nextFloat() * 180f, rnd.nextFloat())
    }
}

private fun DrawScope.drawSeed(p: Offset, s: Float, rotation: Float, tone: Float) {
    val w = 0.24f * s
    val h = 0.18f * s
    val base = listOf(Color(0xFF8B8D84), Color(0xFF7A7F75), Color(0xFF9B9A8E), Color(0xFF6E7268))[(tone * 4).toInt().coerceIn(0, 3)]
    rotate(rotation, pivot = p) {
        drawOval(Color.Black.copy(alpha = 0.3f), Offset(p.x - w / 2, p.y - h / 2 + 0.03f * s), Size(w, h))
        drawOval(
            Brush.radialGradient(listOf(Color(0xFFE1DDD0), base, Color(0xFF5E6259)), center = p + Offset(-w * 0.18f, -h * 0.22f), radius = w * 0.7f),
            Offset(p.x - w / 2, p.y - h / 2), Size(w, h),
        )
    }
}

/** Rings: star yellow for what you can play, sea for a hint, the frame's pit while sowing. */
private fun DrawScope.drawRings(layout: Layout, legal: List<Int>, frame: Frame?, hintPit: Int?, n: Int) {
    val r = (Geometry.PIT_R + 0.05f) * layout.scale
    val width = 0.085f * layout.scale
    fun ring(i: Int, color: Color) = drawCircle(color, r, layout.container(i), style = Stroke(width))
    for (i in legal) ring(i, Color(0xFFF7D116).copy(alpha = 0.75f))
    hintPit?.let { ring(it, Color(0xFF4FD1DC)) }
    val pit = frame?.pit ?: return
    if (pit >= 2 * n) return
    when (frame.kind) {
        FrameKind.PICKUP, FrameKind.SOW -> ring(pit, Color(0xFFF7D116))
        FrameKind.SKIP -> ring(pit, Color(0xFF4FD1DC))
        // A capture flashes in the capturer's light tint: a North pit is taken by South (you).
        FrameKind.CAPTURE -> ring(pit, if (pit >= n) Color(0xFFB4BEFF) else Color(0xFFFFB4B4))
        else -> Unit
    }
}

private fun DrawScope.drawFlyingSeed(layout: Layout, from: Int, to: Int, t: Float) {
    val a = layout.container(from)
    val b = layout.container(to)
    val lift = 0.6f * layout.scale * sin(PI.toFloat() * t)
    val p = Offset(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t - lift)
    drawSeed(p, layout.scale * 1.15f, 30f, 0.3f)
}

/** The seed count: a basalt chip by each pit (outside the row), store chips in owners' colours. */
private fun DrawScope.drawChip(layout: Layout, c: Int, count: Int, measurer: TextMeasurer, store: Color?) {
    val s = layout.scale
    val n = layout.g.n
    val center = layout.g.container(c)
    // Pit chips sit on the board's outer edge beyond their row; store chips below the store.
    val offset = when {
        store != null -> Offset(0f, Geometry.STORE_RY + 0.32f)
        c < n -> Offset(0f, Geometry.PIT_R + 0.36f)
        else -> Offset(0f, -(Geometry.PIT_R + 0.36f))
    }
    val p = layout.point(center + offset)
    val text = measurer.measure(count.toString(), PanuType.count.copy(color = COTTON, fontSize = PanuType.count.fontSize * (s / 46f).coerceIn(0.85f, 1.4f)))
    val pad = 0.12f * s
    val w = maxOf(text.size.width + pad * 2, text.size.height + pad)
    val h = text.size.height + pad * 0.6f
    drawRoundRect(store ?: Color(0xFF0B1024).copy(alpha = 0.84f), Offset(p.x - w / 2, p.y - h / 2), Size(w, h), CornerRadius(h / 2))
    drawText(text, topLeft = Offset(p.x - text.size.width / 2, p.y - text.size.height / 2))
}
