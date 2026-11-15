package org.moreiralabs.ouril.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.moreiralabs.ouril.ui.theme.PanuShapes
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu
import kotlin.math.cos
import kotlin.math.sin

// --- Buttons ---------------------------------------------------------------------------------

/**
 * Button kinds, as on the web (web-design.md, "Components"): PLAY is the one star-yellow action
 * that starts or continues a game; PRIMARY is indigo; NEUTRAL, OUTLINE and DANGER the rest;
 * ON_DARK sits on the night stage.
 */
enum class ButtonKind { PLAY, PRIMARY, NEUTRAL, OUTLINE, DANGER, ON_DARK }

/**
 * A tactile button: a 1 dp edge and a darker lip under it, sinking when pressed (2 dp with a
 * lip). Disabled is flat and neutral. At least 48 dp tall.
 */
@Composable
fun PanuButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    kind: ButtonKind = ButtonKind.NEUTRAL,
    enabled: Boolean = true,
    large: Boolean = false,
    content: @Composable RowScope.() -> Unit,
) {
    val c = panu
    val (bg, ink, edge, lip) = when {
        !enabled -> listOf(c.surface3, c.muted, Color.Transparent, Color.Transparent)
        kind == ButtonKind.PLAY -> listOf(c.star, c.onStar, c.starEdge, c.starEdge)
        kind == ButtonKind.PRIMARY -> listOf(c.indigoFill, c.onIndigo, c.indigoStrong, c.indigoStrong)
        kind == ButtonKind.DANGER -> listOf(c.red, c.onRed, c.red, Color.Black.copy(alpha = 0.3f))
        kind == ButtonKind.OUTLINE -> listOf(Color.Transparent, c.ink, c.lineStrong, Color.Transparent)
        kind == ButtonKind.ON_DARK -> listOf(c.nightInk.copy(alpha = 0.08f), c.nightInk, c.nightInk.copy(alpha = 0.4f), Color.Transparent)
        else -> listOf(c.surface, c.ink, c.lineStrong, c.lineStrong)
    }
    val lipDepth = when {
        !enabled -> 0.dp
        kind == ButtonKind.PLAY || kind == ButtonKind.PRIMARY || kind == ButtonKind.DANGER -> 3.dp
        kind == ButtonKind.NEUTRAL -> 2.dp
        else -> 0.dp
    }
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val sink by animateDpAsState(
        targetValue = if (pressed && enabled) (if (lipDepth >= 3.dp) 2.dp else 1.dp) else 0.dp,
        animationSpec = tween(100),
        label = "press",
    )
    val shape = if (large) RoundedCornerShape(16.dp) else PanuShapes.button
    Row(
        modifier = modifier
            .offset(y = sink)
            .defaultMinSize(minHeight = if (large) 56.dp else 48.dp)
            .clip(shape)
            .drawBehind {
                // The lip: a band of the edge colour along the bottom, shrinking as it sinks.
                val depth = (lipDepth - sink).coerceAtLeast(0.dp).toPx()
                drawRect(bg)
                if (depth > 0f) drawRect(lip, topLeft = Offset(0f, size.height - depth), size = Size(size.width, depth))
            }
            .border(BorderStroke(1.dp, edge), shape)
            .clickable(
                interactionSource = interaction,
                indication = ripple(),
                enabled = enabled,
                role = Role.Button,
                onClick = onClick,
            )
            .padding(horizontal = if (large) 24.dp else 18.dp, vertical = 10.dp)
            .padding(bottom = (lipDepth - sink).coerceAtLeast(0.dp) / 2),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        androidx.compose.runtime.CompositionLocalProvider(
            androidx.compose.material3.LocalContentColor provides ink,
            androidx.compose.material3.LocalTextStyle provides (if (kind == ButtonKind.PLAY) PanuType.labelStrong else PanuType.label)
                .copy(color = ink, fontSize = if (large) PanuType.bodyLarge.fontSize else PanuType.label.fontSize),
        ) {
            content()
        }
    }
}

/** A square icon button (48 dp) for the game screen's tools. */
@Composable
fun PanuIconButton(
    onClick: () -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    icon: @Composable () -> Unit,
) {
    val c = panu
    Box(
        modifier = modifier
            .size(48.dp)
            .alpha(if (enabled) 1f else 0.4f)
            .clip(RoundedCornerShape(14.dp))
            .background(c.surface)
            .border(1.dp, c.line, RoundedCornerShape(14.dp))
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.runtime.CompositionLocalProvider(androidx.compose.material3.LocalContentColor provides c.ink) {
            icon()
        }
    }
}

// --- Choices -----------------------------------------------------------------------------------

/** One choice among a few (difficulty, who starts, rules): chips, the selected one solid. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun <T> ChoiceChips(
    options: List<T>,
    selected: T,
    label: @Composable (T) -> String,
    onSelect: (T) -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = panu
    FlowRow(
        modifier = modifier.selectableGroup(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        options.forEach { option ->
            val on = option == selected
            Box(
                modifier = Modifier
                    .defaultMinSize(minHeight = 48.dp)
                    .clip(CircleShape)
                    .background(if (on) c.selectedBg else c.surface3)
                    .selectable(selected = on, role = Role.RadioButton, onClick = { onSelect(option) })
                    .padding(horizontal = 18.dp, vertical = 10.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(label(option), style = PanuType.label, color = if (on) c.selectedInk else c.ink2)
            }
        }
    }
}

// --- Surfaces ----------------------------------------------------------------------------------

/** A card: surface, 22 dp corners, a 1 dp line. Featured cards carry the panu strip on top. */
@Composable
fun PanuCard(modifier: Modifier = Modifier, featured: Boolean = false, content: @Composable ColumnScope.() -> Unit) {
    val c = panu
    Column(
        modifier = modifier
            .clip(PanuShapes.card)
            .background(c.surface)
            .border(1.dp, c.line, PanuShapes.card),
    ) {
        if (featured) PanuStrip(Modifier.fillMaxWidth().height(14.dp))
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(10.dp), content = content)
    }
}

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier, color: Color = panu.ink) {
    Text(text, style = PanuType.section, color = color, modifier = modifier.semantics { heading() })
}

// --- The mark ------------------------------------------------------------------------------------

/**
 * The Ouril mark: ten ouri seeds in a ring, the first (top) one star yellow. It reads as an O,
 * as a pit seen from above, and as the flag's ring of ten islands. With [fill] below 1, only
 * that share of the seeds is drawn (the win dialog fills the ring one seed at a time).
 */
@Composable
fun SeedRing(
    modifier: Modifier = Modifier,
    ink: Color = panu.indigo,
    star: Color = panu.star,
    fill: Float = 1f,
) {
    Canvas(modifier.clearAndSetSemantics { }) {
        drawSeedRing(ink, star, fill)
    }
}

/** The ring in a 64-unit box, scaled to this canvas: radius 21, seeds 12.4 × 8.8. */
fun DrawScope.drawSeedRing(ink: Color, star: Color, fill: Float = 1f) {
    val unit = size.minDimension / 64f
    val center = Offset(size.width / 2f, size.height / 2f)
    val shown = (fill * 10f).toInt().coerceIn(0, 10)
    for (i in 0 until shown) {
        val angle = -90f + i * 36f
        val a = Math.toRadians(angle.toDouble())
        val p = Offset(center.x + 21f * unit * cos(a).toFloat(), center.y + 21f * unit * sin(a).toFloat())
        rotate(angle + 90f, pivot = p) {
            drawOval(
                color = if (i == 0) star else ink,
                topLeft = Offset(p.x - 6.2f * unit, p.y - 4.4f * unit),
                size = Size(12.4f * unit, 8.8f * unit),
            )
        }
    }
}

/** The ring filling one seed at a time over about 900 ms (a win), or at once. */
@Composable
fun FillingSeedRing(modifier: Modifier = Modifier, animate: Boolean, ink: Color = panu.indigo) {
    val progress = remember { Animatable(if (animate) 0f else 1f) }
    LaunchedEffect(animate) { if (animate) progress.animateTo(1f, tween(900)) }
    SeedRing(modifier, ink = ink, fill = progress.value)
}

/**
 * The wordmark: "Ouril" with the seed ring as its O. The title's first letter is the ring, so
 * the name still comes from the strings (`app.title`).
 */
@Composable
fun Wordmark(title: String, height: Dp, modifier: Modifier = Modifier, ink: Color = panu.nightInk) {
    Row(modifier.semantics { contentDescription = title; heading() }, verticalAlignment = Alignment.Bottom) {
        SeedRing(Modifier.size(height * 0.86f).offset(y = -(height * 0.03f)), ink = ink)
        Spacer(Modifier.width(height * 0.02f))
        Text(
            text = if (title.startsWith("O")) title.drop(1) else title,
            style = PanuType.hero.copy(fontSize = with(androidx.compose.ui.platform.LocalDensity.current) { (height * 1.1f).toSp() }, lineHeight = with(androidx.compose.ui.platform.LocalDensity.current) { (height * 1.1f).toSp() }),
            color = ink,
            modifier = Modifier.clearAndSetSemantics { },
        )
    }
}

// --- The panu strip --------------------------------------------------------------------------------

/** Stitched seams, white lozenges and a star-yellow centre on indigo, after panu di terra. */
@Composable
fun PanuStrip(modifier: Modifier = Modifier) {
    Canvas(modifier.clearAndSetSemantics { }) { drawPanuStrip() }
}

fun DrawScope.drawPanuStrip() {
    val h = size.height
    val tile = h * 2f
    val cotton = Color(0xFFF5F1E8)
    drawRect(Color(0xFF1F2D78))
    val dash = PathEffect.dashPathEffect(floatArrayOf(h * 0.13f, h * 0.13f))
    val inset = h * 0.07f
    drawLine(cotton.copy(alpha = 0.5f), Offset(0f, inset), Offset(size.width, inset), strokeWidth = h * 0.04f, pathEffect = dash)
    drawLine(cotton.copy(alpha = 0.5f), Offset(0f, h - inset), Offset(size.width, h - inset), strokeWidth = h * 0.04f, pathEffect = dash)
    clipRect {
        var x = 0f
        while (x < size.width + tile) {
            for (cx in listOf(x + tile * 0.25f, x + tile * 0.75f)) {
                val r = h * 0.27f
                val diamond = Path().apply {
                    moveTo(cx, h / 2 - r); lineTo(cx + r, h / 2); lineTo(cx, h / 2 + r); lineTo(cx - r, h / 2); close()
                }
                drawPath(diamond, cotton.copy(alpha = 0.9f), style = Stroke(width = h * 0.07f))
            }
            val s = h * 0.1f
            val center = Path().apply {
                moveTo(x + tile / 2, h / 2 - s); lineTo(x + tile / 2 + s, h / 2); lineTo(x + tile / 2, h / 2 + s); lineTo(x + tile / 2 - s, h / 2); close()
            }
            drawPath(center, Color(0xFFF7D116))
            x += tile
        }
    }
}

// --- Game pieces ---------------------------------------------------------------------------------

/**
 * A player's nameplate: a seed-shaped avatar with the initial, the name and the score. The
 * player to move gets an outline in their colour (the words stay in the status line).
 */
@Composable
fun Nameplate(name: String, score: Int, you: Boolean, turn: Boolean, modifier: Modifier = Modifier) {
    val c = panu
    val color = if (you) c.indigo else c.red
    val lift by animateDpAsState(if (turn) (-2).dp else 0.dp, tween(240), label = "turn")
    Row(
        modifier = modifier
            .offset(y = lift)
            .defaultMinSize(minHeight = 56.dp)
            .clip(PanuShapes.plate)
            .background(c.surface)
            .border(if (turn) 2.dp else 1.dp, if (turn) color else c.line, PanuShapes.plate)
            .padding(start = 8.dp, end = 14.dp, top = 6.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Box(
            Modifier
                .size(width = 42.dp, height = 36.dp)
                .rotate(-10f)
                .clip(androidx.compose.foundation.shape.GenericShape { s, _ -> addOval(androidx.compose.ui.geometry.Rect(0f, 0f, s.width, s.height)) })
                .background(if (you) c.youFill else c.themFill),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                name.take(1),
                style = PanuType.card.copy(fontSize = PanuType.label.fontSize),
                color = if (you) c.onYou else c.onThem,
                modifier = Modifier.rotate(10f).clearAndSetSemantics { },
            )
        }
        Text(name, style = PanuType.labelStrong, color = c.ink, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
        Text(score.toString(), style = PanuType.score, color = c.ink)
    }
}

/** The line that says whose turn it is (one live region on the game screen). */
@Composable
fun StatusPill(text: String, modifier: Modifier = Modifier) {
    val c = panu
    Text(
        text,
        style = PanuType.small.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
        color = c.ink,
        modifier = modifier
            .clip(CircleShape)
            .background(c.surface)
            .border(1.dp, c.line, CircleShape)
            .padding(horizontal = 14.dp, vertical = 6.dp),
    )
}

/** Plain text in the body style. */
@Composable
fun Body(text: String, modifier: Modifier = Modifier, color: Color = panu.ink2, style: TextStyle = PanuType.body) {
    Text(text, style = style, color = color, modifier = modifier)
}
