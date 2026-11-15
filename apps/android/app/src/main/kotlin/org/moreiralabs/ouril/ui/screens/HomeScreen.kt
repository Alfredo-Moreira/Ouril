package org.moreiralabs.ouril.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.data.Store
import org.moreiralabs.ouril.game.NewGameRequest
import org.moreiralabs.ouril.game.PLAYABLE_VARIANTS
import org.moreiralabs.ouril.game.Starter
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.i18n.trKey
import org.moreiralabs.ouril.ui.components.Body
import org.moreiralabs.ouril.ui.components.ButtonKind
import org.moreiralabs.ouril.ui.components.ChoiceChips
import org.moreiralabs.ouril.ui.components.OurilIcons
import org.moreiralabs.ouril.ui.components.PanuButton
import org.moreiralabs.ouril.ui.components.PanuCard
import org.moreiralabs.ouril.ui.components.PanuStrip
import org.moreiralabs.ouril.ui.components.SectionTitle
import org.moreiralabs.ouril.ui.components.SeedRing
import org.moreiralabs.ouril.ui.components.Wordmark
import org.moreiralabs.ouril.ui.theme.PanuShapes
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/**
 * Home, as the web's landing page: the night hero with the wordmark and the one yellow action
 * (continue the game, or Play now with your last choices), the quick start, how to play, the
 * game's heritage, and what the app does.
 */
@Composable
fun HomeScreen(onPlay: (NewGameRequest?) -> Unit, onLearn: () -> Unit, onAbout: () -> Unit) {
    val context = LocalContext.current
    val store = remember { Store(context) }
    val current by store.currentGame.collectAsState(initial = null)
    val inProgress = current?.state?.isOver == false
    var last by remember { mutableStateOf(NewGameRequest()) }
    LaunchedEffect(Unit) { last = store.lastNewGame() }
    val c = panu

    Column(Modifier.verticalScroll(rememberScrollState())) {
        // Hero: the night, the wordmark, the one star-yellow action.
        Column(
            Modifier
                .fillMaxWidth()
                .background(androidx.compose.ui.graphics.Brush.verticalGradient(listOf(c.night, c.night2)))
                .windowInsetsPadding(WindowInsets.statusBars),
        ) {
            Column(Modifier.padding(horizontal = 24.dp, vertical = 28.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Box(Modifier.size(width = 10.dp, height = 8.dp).clip(CircleShape).background(c.star))
                    Text(tr(R.string.home_tagline), style = PanuType.small.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold), color = c.star)
                }
                Wordmark(tr(R.string.app_title), height = 68.dp)
                Body(tr(R.string.home_hero_lede), color = c.nightMuted, style = PanuType.bodyLarge)
                Spacer(Modifier.height(4.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    PanuButton(onClick = { onPlay(if (inProgress) null else last) }, kind = ButtonKind.PLAY, large = true) {
                        Icon(OurilIcons.Play, contentDescription = null, modifier = Modifier.size(20.dp))
                        Text(tr(if (inProgress) R.string.home_cta_resume else R.string.home_cta_play))
                    }
                    PanuButton(onClick = onLearn, kind = ButtonKind.ON_DARK, large = true) {
                        Icon(OurilIcons.Learn, contentDescription = null, modifier = Modifier.size(20.dp))
                        Text(tr(R.string.home_cta_learn))
                    }
                }
            }
            PanuStrip(Modifier.fillMaxWidth().height(20.dp))
        }

        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(28.dp)) {
            if (inProgress) {
                PanuCard(featured = true) {
                    Text(tr(R.string.home_resume), style = PanuType.card, color = c.ink, modifier = Modifier.semantics { heading() })
                    Body(tr(R.string.home_resume_body, "level" to tr(levelLabel(current!!.level))))
                    PanuButton(onClick = { onPlay(null) }, kind = ButtonKind.PRIMARY) { Text(tr(R.string.home_resume)) }
                }
            }

            // The quick start: its Start game is indigo, since the hero holds the yellow action.
            NewGameForm(
                title = tr(R.string.home_quick_title),
                initial = last,
                replaceWarning = inProgress,
                yellow = false,
                onStart = onPlay,
            )

            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                SectionTitle(tr(R.string.home_steps_title))
                for (n in 1..4) {
                    PanuCard {
                        Box(
                            Modifier
                                .size(width = 52.dp, height = 44.dp)
                                .rotate(-8f)
                                .clip(androidx.compose.foundation.shape.GenericShape { s, _ -> addOval(androidx.compose.ui.geometry.Rect(0f, 0f, s.width, s.height)) })
                                .background(if (n == 1) c.star else c.indigoFill),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text("$n", style = PanuType.section, color = if (n == 1) c.onStar else c.onIndigo, modifier = Modifier.rotate(8f))
                        }
                        Text(trKey("home.steps.$n.title"), style = PanuType.card, color = c.ink)
                        Body(trKey("home.steps.$n.body"))
                    }
                }
            }

            Heritage()

            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                SectionTitle(tr(R.string.home_features_title))
                for ((key, icon) in listOf("offline" to OurilIcons.Seed, "levels" to OurilIcons.Levels, "guest" to OurilIcons.User)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                        Box(Modifier.size(48.dp).clip(RoundedCornerShape(14.dp)).background(c.indigoSoft), contentAlignment = Alignment.Center) {
                            Icon(icon, contentDescription = null, tint = c.indigo, modifier = Modifier.size(24.dp))
                        }
                        Column(Modifier.weight(1f)) {
                            Text(trKey("home.features.$key.title"), style = PanuType.labelStrong, color = c.ink)
                            Body(trKey("home.features.$key.body"))
                        }
                    }
                }
            }

            PanuButton(onClick = onAbout, kind = ButtonKind.OUTLINE, modifier = Modifier.fillMaxWidth()) {
                Icon(OurilIcons.Info, contentDescription = null, modifier = Modifier.size(20.dp))
                Text(tr(R.string.menu_about))
            }
        }
    }
}

/** "Ten islands, one board": the night, the seed ring for the ten islands, the panu strip. */
@Composable
private fun Heritage() {
    val c = panu
    Column(
        Modifier
            .fillMaxWidth()
            .clip(PanuShapes.card)
            .background(androidx.compose.ui.graphics.Brush.linearGradient(listOf(c.night, c.night2))),
    ) {
        Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            SeedRing(Modifier.size(96.dp), ink = c.nightInk)
            Text(tr(R.string.home_heritage_kicker), style = PanuType.small.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold), color = c.star)
            SectionTitle(tr(R.string.home_heritage_title), color = c.nightInk)
            Body(tr(R.string.home_heritage_body), color = c.nightMuted)
            Body(tr(R.string.home_heritage_world_cup), color = c.nightMuted)
            Body(tr(R.string.home_heritage_cloth), color = c.nightMuted)
        }
        PanuStrip(Modifier.fillMaxWidth().height(18.dp))
    }
}

/** Choosing a new game: difficulty, who starts and the rules, then Start game. */
@Composable
fun NewGameForm(
    title: String,
    initial: NewGameRequest,
    replaceWarning: Boolean,
    /** The screen's one yellow action, or indigo where another button already starts a game. */
    yellow: Boolean,
    onStart: (NewGameRequest) -> Unit,
    intro: String? = null,
) {
    var level by remember(initial) { mutableStateOf(initial.level) }
    var starter by remember(initial) { mutableStateOf(initial.starter) }
    var variant by remember(initial) { mutableStateOf(initial.variant) }
    val c = panu
    PanuCard {
        Text(title, style = PanuType.card, color = c.ink, modifier = Modifier.semantics { heading() })
        intro?.let { Body(it) }
        Text(tr(R.string.home_level), style = PanuType.labelStrong, color = c.ink2)
        ChoiceChips(Level.entries, level, { tr(levelLabel(it)) }, { level = it })
        Text(tr(R.string.home_starter), style = PanuType.labelStrong, color = c.ink2)
        ChoiceChips(Starter.entries, starter, {
            tr(when (it) { Starter.ME -> R.string.home_starter_me; Starter.AI -> R.string.home_starter_ai; Starter.RANDOM -> R.string.home_starter_random })
        }, { starter = it })
        Text(tr(R.string.home_variant), style = PanuType.labelStrong, color = c.ink2)
        ChoiceChips(PLAYABLE_VARIANTS, variant, { trKey("variant.$it.short") }, { variant = it })
        Body(trKey("variant.$variant.about"), color = c.muted, style = PanuType.small)
        if (replaceWarning) Body(tr(R.string.home_replace_warning), color = c.muted, style = PanuType.small)
        PanuButton(
            onClick = { onStart(NewGameRequest(level, starter, variant)) },
            kind = if (yellow) ButtonKind.PLAY else ButtonKind.PRIMARY,
            large = true,
        ) {
            Icon(OurilIcons.Play, contentDescription = null, modifier = Modifier.size(20.dp))
            Text(tr(R.string.home_start))
        }
    }
}

fun levelLabel(level: Level) = when (level) {
    Level.EASY -> R.string.game_level_easy
    Level.MEDIUM -> R.string.game_level_medium
    Level.HARD -> R.string.game_level_hard
}
