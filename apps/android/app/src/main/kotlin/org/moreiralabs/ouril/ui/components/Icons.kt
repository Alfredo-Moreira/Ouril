package org.moreiralabs.ouril.ui.components

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/**
 * The app's icons: the web's set (apps/web/src/components/Icons.tsx) on the same 24-unit grid
 * and 2-unit rounded stroke, so every Ouril app speaks one icon language. Always decorative:
 * the name comes from the text or label next to them.
 */
object OurilIcons {
    val Play = icon(stroke("M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z"), fill("M10 8.5v7l5.5-3.5z"))
    val Learn = icon(stroke("M3 7.5 12 4l9 3.5-9 3.5z"), stroke("M7 9.5v4.5c0 1.4 2.2 2.5 5 2.5s5-1.1 5-2.5V9.5"), stroke("M21 7.5V13"))
    val Rules = icon(stroke("M6 3.5h9l3 3V20.5H6z"), stroke("M9 10h6M9 13.5h6M9 17h4"))
    val Settings = icon(
        stroke("M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6z"),
        stroke(
            "M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
        ),
    )
    val Info = icon(stroke("M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z"), stroke("M12 11v5.5M12 7.6v.1"))
    val Pause = icon(stroke("M9 5v14M15 5v14"))
    val Undo = icon(stroke("M9 14 4 9l5-5"), stroke("M4 9h10.5a5.5 5.5 0 0 1 0 11H11"))
    val Bulb = icon(stroke("M9 18h6M10 21h4"), stroke("M12 3a6 6 0 0 0-3.6 10.8c.6.5 1 1.2 1.1 2V16h5v-.2c.1-.8.5-1.5 1.1-2A6 6 0 0 0 12 3z"))
    val Home = icon(stroke("M4 10.5 12 4l8 6.5V20h-5v-5.5h-6V20H4z"))
    val Flag = icon(stroke("M5 21V4"), stroke("M5 4.5c4-2 7 2 11 0 1-.5 2-.6 3-.5v9c-1-.1-2 0-3 .5-4 2-7-2-11 0"))
    val Seed = icon(fill("M5.5 12.5a6.5 5.2 0 1 0 13 0a6.5 5.2 0 1 0-13 0z"))
    val Globe = icon(stroke("M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18z"), stroke("M3 12h18M12 3c2.6 2.6 3.8 5.6 3.8 9s-1.2 6.4-3.8 9c-2.6-2.6-3.8-5.6-3.8-9S9.4 5.6 12 3z"))
    val Levels = icon(stroke("M4 20V10M10 20V4M16 20v-7M22 20H2"))
    val User = icon(stroke("M12 5a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7z"), stroke("M5 20c1.2-3.4 3.8-5 7-5s5.8 1.6 7 5"))

    private sealed interface Part { val d: String }
    private data class Stroke(override val d: String) : Part
    private data class Fill(override val d: String) : Part

    private fun stroke(d: String): Part = Stroke(d)
    private fun fill(d: String): Part = Fill(d)

    private fun icon(vararg parts: Part): ImageVector {
        val b = ImageVector.Builder(defaultWidth = 24.dp, defaultHeight = 24.dp, viewportWidth = 24f, viewportHeight = 24f)
        for (p in parts) {
            when (p) {
                is Stroke -> b.addPath(
                    pathData = addPathNodes(p.d),
                    stroke = SolidColor(Color.Black),
                    strokeLineWidth = 2f,
                    strokeLineCap = StrokeCap.Round,
                    strokeLineJoin = StrokeJoin.Round,
                )
                is Fill -> b.addPath(pathData = addPathNodes(p.d), fill = SolidColor(Color.Black))
            }
        }
        return b.build()
    }
}
