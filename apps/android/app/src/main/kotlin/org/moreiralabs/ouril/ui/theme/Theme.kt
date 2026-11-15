package org.moreiralabs.ouril.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import org.moreiralabs.ouril.R

/**
 * Panu, the design system of every Ouril app (docs/architecture/web-design.md): cotton white by
 * day and an indigo night by dark, from panu di terra; star yellow and red from the flag; basalt
 * ink. The values are the web's tokens; `styles.contrast.test.ts` checks their contrast there.
 */
@Immutable
data class PanuColors(
    val bg: Color,
    val surface: Color,
    val surface2: Color,
    val surface3: Color,
    val ink: Color,
    val ink2: Color,
    val muted: Color,
    val line: Color,
    val lineStrong: Color,
    /** The brand, links, selected states and "you" as text or outline. */
    val indigo: Color,
    val indigoFill: Color,
    val indigoStrong: Color,
    val indigoSoft: Color,
    val onIndigo: Color,
    val selectedBg: Color,
    val selectedInk: Color,
    val star: Color,
    val starHover: Color,
    val starEdge: Color,
    val starSoft: Color,
    val onStar: Color,
    val red: Color,
    val onRed: Color,
    val green: Color,
    val sea: Color,
    val focus: Color,
    val youFill: Color,
    val onYou: Color,
    val themFill: Color,
    val onThem: Color,
    val isDark: Boolean,
) {
    // The night stage (the hero, behind the board, the result band) is the same in both themes.
    val night = Color(0xFF0B1024)
    val night2 = Color(0xFF141E4D)
    val nightInk = Color(0xFFF3EFE6)
    val nightMuted = Color(0xFFB9BDD6)
}

private val Cotton = PanuColors(
    bg = Color(0xFFF5F1E8),
    surface = Color(0xFFFFFDF8),
    surface2 = Color(0xFFF8F4EC),
    surface3 = Color(0xFFEFE9DD),
    ink = Color(0xFF14171F),
    ink2 = Color(0xFF3B4152),
    muted = Color(0xFF5F6678),
    line = Color(0x1A14171F),
    lineStrong = Color(0x3314171F),
    indigo = Color(0xFF1F2D78),
    indigoFill = Color(0xFF1F2D78),
    indigoStrong = Color(0xFF121B52),
    indigoSoft = Color(0x1A1F2D78),
    onIndigo = Color(0xFFFFFDF8),
    selectedBg = Color(0xFF1F2D78),
    selectedInk = Color(0xFFFFFDF8),
    star = Color(0xFFF7D116),
    starHover = Color(0xFFFFDC3D),
    starEdge = Color(0xFF8F6B00),
    starSoft = Color(0x3DF7D116),
    onStar = Color(0xFF14171F),
    red = Color(0xFFC8202A),
    onRed = Color(0xFFFFFDF8),
    green = Color(0xFF1A774A),
    sea = Color(0xFF0C7580),
    focus = Color(0xFF2346D6),
    youFill = Color(0xFF1F2D78),
    onYou = Color(0xFFFFFDF8),
    themFill = Color(0xFFC8202A),
    onThem = Color(0xFFFFFDF8),
    isDark = false,
)

private val Night = PanuColors(
    bg = Color(0xFF0B1024),
    surface = Color(0xFF121A38),
    surface2 = Color(0xFF18214A),
    surface3 = Color(0xFF1F2958),
    ink = Color(0xFFF3EFE6),
    ink2 = Color(0xFFC9CBE0),
    muted = Color(0xFF9097B8),
    line = Color(0x1AF3EFE6),
    lineStrong = Color(0x33F3EFE6),
    indigo = Color(0xFF9AA8FF),
    indigoFill = Color(0xFF3346B8),
    indigoStrong = Color(0xFF22308A),
    indigoSoft = Color(0x249AA8FF),
    onIndigo = Color(0xFFFFFDF8),
    selectedBg = Color(0xFF9AA8FF),
    selectedInk = Color(0xFF0B1024),
    star = Color(0xFFF7D116),
    starHover = Color(0xFFFFDC3D),
    starEdge = Color(0xFF8F6B00),
    starSoft = Color(0x29F7D116),
    onStar = Color(0xFF14171F),
    red = Color(0xFFFF6B6B),
    onRed = Color(0xFF0B1024),
    green = Color(0xFF4CC38A),
    sea = Color(0xFF4FD1DC),
    focus = Color(0xFFF3EFE6),
    youFill = Color(0xFF3346B8),
    onYou = Color(0xFFFFFDF8),
    themFill = Color(0xFFFF6B6B),
    onThem = Color(0xFF0B1024),
    isDark = true,
)

val LocalPanu = staticCompositionLocalOf { Cotton }

/** Bricolage Grotesque at a large optical size, for the wordmark and page titles. */
val BricolageDisplay = FontFamily(
    Font(R.font.bricolage_grotesque, FontWeight.ExtraBold, variationSettings = settings(800, 72f)),
    Font(R.font.bricolage_grotesque, FontWeight.Bold, variationSettings = settings(700, 72f)),
)

/** Bricolage at a text optical size, for section titles, scores and counts. */
val Bricolage = FontFamily(
    Font(R.font.bricolage_grotesque, FontWeight.ExtraBold, variationSettings = settings(800, 24f)),
    Font(R.font.bricolage_grotesque, FontWeight.Bold, variationSettings = settings(700, 24f)),
)

/** Figtree, the interface face. */
val Figtree = FontFamily(
    Font(R.font.figtree, FontWeight.Normal, variationSettings = FontVariation.Settings(FontVariation.weight(400))),
    Font(R.font.figtree, FontWeight.Medium, variationSettings = FontVariation.Settings(FontVariation.weight(500))),
    Font(R.font.figtree, FontWeight.SemiBold, variationSettings = FontVariation.Settings(FontVariation.weight(600))),
    Font(R.font.figtree, FontWeight.Bold, variationSettings = FontVariation.Settings(FontVariation.weight(700))),
    Font(R.font.figtree, FontWeight.ExtraBold, variationSettings = FontVariation.Settings(FontVariation.weight(800))),
)

private fun settings(weight: Int, opsz: Float) =
    FontVariation.Settings(FontVariation.weight(weight), FontVariation.Setting("opsz", opsz))

/** The type scale (1.25 steps), as on the web. Numbers always use tabular figures. */
object PanuType {
    val hero = TextStyle(fontFamily = BricolageDisplay, fontWeight = FontWeight.ExtraBold, fontSize = 44.sp, letterSpacing = (-0.03).em, lineHeight = 46.sp)
    val title = TextStyle(fontFamily = BricolageDisplay, fontWeight = FontWeight.ExtraBold, fontSize = 34.sp, letterSpacing = (-0.02).em, lineHeight = 38.sp)
    val section = TextStyle(fontFamily = Bricolage, fontWeight = FontWeight.ExtraBold, fontSize = 24.sp, letterSpacing = (-0.02).em, lineHeight = 28.sp)
    val card = TextStyle(fontFamily = Bricolage, fontWeight = FontWeight.ExtraBold, fontSize = 20.sp, letterSpacing = (-0.01).em, lineHeight = 24.sp)
    val score = TextStyle(fontFamily = Bricolage, fontWeight = FontWeight.ExtraBold, fontSize = 30.sp, fontFeatureSettings = "tnum", lineHeight = 30.sp)
    val count = TextStyle(fontFamily = Bricolage, fontWeight = FontWeight.ExtraBold, fontSize = 13.sp, fontFeatureSettings = "tnum")
    val body = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.Normal, fontSize = 16.sp, lineHeight = 24.sp)
    val bodyLarge = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.Normal, fontSize = 18.sp, lineHeight = 27.sp)
    val label = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.SemiBold, fontSize = 16.sp, lineHeight = 20.sp)
    val labelStrong = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.Bold, fontSize = 16.sp, lineHeight = 20.sp)
    val small = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.Medium, fontSize = 14.sp, lineHeight = 20.sp)
    val tab = TextStyle(fontFamily = Figtree, fontWeight = FontWeight.SemiBold, fontSize = 12.sp)
}

object PanuShapes {
    val button = RoundedCornerShape(14.dp)
    val card = RoundedCornerShape(22.dp)
    val plate = RoundedCornerShape(16.dp)
    val sheet = RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp)
    val dialog = RoundedCornerShape(24.dp)
}

@Composable
fun OurilTheme(dark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val colors = if (dark) Night else Cotton
    // Material components (sheets, dialogs, ripples) take the same colours.
    val scheme = if (dark) {
        darkColorScheme(
            primary = colors.indigo, onPrimary = colors.selectedInk, background = colors.bg,
            onBackground = colors.ink, surface = colors.surface, onSurface = colors.ink,
            surfaceVariant = colors.surface2, onSurfaceVariant = colors.ink2, error = colors.red,
        )
    } else {
        lightColorScheme(
            primary = colors.indigo, onPrimary = colors.onIndigo, background = colors.bg,
            onBackground = colors.ink, surface = colors.surface, onSurface = colors.ink,
            surfaceVariant = colors.surface2, onSurfaceVariant = colors.ink2, error = colors.red,
        )
    }
    CompositionLocalProvider(LocalPanu provides colors) {
        MaterialTheme(colorScheme = scheme, content = content)
    }
}

/** The current Panu colours. */
val panu: PanuColors @Composable get() = LocalPanu.current
