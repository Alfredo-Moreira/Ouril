package org.moreiralabs.ouril.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.data.Settings
import org.moreiralabs.ouril.data.Store
import org.moreiralabs.ouril.game.FrameKind
import org.moreiralabs.ouril.game.GameViewModel
import org.moreiralabs.ouril.game.NewGameRequest
import org.moreiralabs.ouril.game.Outcome
import org.moreiralabs.ouril.game.SoundEffects
import org.moreiralabs.ouril.game.Starter
import org.moreiralabs.ouril.game.frameDelay
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.i18n.trKey
import org.moreiralabs.ouril.ui.GameRoute
import org.moreiralabs.ouril.ui.board.Board
import org.moreiralabs.ouril.ui.components.Body
import org.moreiralabs.ouril.ui.components.ButtonKind
import org.moreiralabs.ouril.ui.components.FillingSeedRing
import org.moreiralabs.ouril.ui.components.Nameplate
import org.moreiralabs.ouril.ui.components.OurilIcons
import org.moreiralabs.ouril.ui.components.PanuButton
import org.moreiralabs.ouril.ui.components.PanuIconButton
import org.moreiralabs.ouril.ui.components.PanuStrip
import org.moreiralabs.ouril.ui.components.StatusPill
import org.moreiralabs.ouril.ui.theme.PanuShapes
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/**
 * The game against the computer, full screen: the opponent's nameplate above the board and
 * yours below, one status line, the board (upright on a phone held upright), and your actions.
 * The menu, the rules and the result open as sheets and dialogs.
 */
@Composable
fun GameScreen(route: GameRoute, onExit: () -> Unit, onAllRules: (String) -> Unit) {
    val vm: GameViewModel = viewModel()
    LaunchedEffect(route) { vm.start(route.request) }
    val s by vm.state.collectAsState()
    val context = LocalContext.current
    val store = remember { Store(context) }
    val settings by store.settings.collectAsState(initial = Settings())
    val haptics = LocalHapticFeedback.current
    val sounds = remember { SoundEffects(context) }
    LaunchedEffect(sounds) { withContext(Dispatchers.IO) { sounds.load() } }
    DisposableEffect(sounds) { onDispose { sounds.release() } }
    LaunchedEffect(settings.sound) {
        if (!settings.sound) return@LaunchedEffect
        launch { vm.ends.collect { sounds.end(it == Outcome.WIN) } }
        vm.frames.collect { kind ->
            sounds.frame(kind)
            when (kind) {
                FrameKind.SOW -> haptics.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
                FrameKind.CAPTURE, FrameKind.GRAND_SLAM -> haptics.performHapticFeedback(HapticFeedbackType.Confirm)
                else -> Unit
            }
        }
    }
    val c = panu
    var menu by remember { mutableStateOf(false) }
    var confirmForfeit by remember { mutableStateOf(false) }
    var rules by remember { mutableStateOf(false) }

    if (s.empty) {
        // No game in progress: the new-game form in its place.
        var last by remember { mutableStateOf(NewGameRequest()) }
        LaunchedEffect(Unit) { last = store.lastNewGame() }
        Column(
            Modifier.fillMaxSize().background(c.bg).windowInsetsPadding(WindowInsets.safeDrawing)
                .verticalScroll(rememberScrollState()).padding(20.dp),
        ) {
            NewGameForm(
                title = tr(R.string.game_new),
                intro = tr(R.string.game_none_body),
                initial = last,
                replaceWarning = false,
                yellow = true,
                onStart = { vm.restartWith(it) },
            )
        }
        return
    }
    val game = s.game ?: return
    val pits = s.frame?.pits ?: game.state.pits
    val stores = s.frame?.stores ?: game.state.stores
    val you = tr(R.string.game_side_you)
    val ai = tr(R.string.game_side_ai, "level" to tr(levelLabel(game.level)))
    val status = tr(
        when {
            s.finished != null || game.state.isOver -> R.string.game_status_over
            s.animating -> R.string.game_status_sowing
            s.aiThinking -> R.string.game_status_ai_thinking
            s.humanTurn -> R.string.game_status_your_turn
            else -> R.string.game_status_ai_turn
        },
    )
    val announcement = s.announcement?.let { describe(it.events, it.byHuman) }.orEmpty()

    BoxWithConstraints(Modifier.fillMaxSize().background(c.bg).windowInsetsPadding(WindowInsets.safeDrawing)) {
        val upright = maxHeight > maxWidth
        val board: @Composable (Modifier) -> Unit = { m ->
            Box(m.clip(PanuShapes.card)) {
                Board(
                    pits = pits,
                    stores = stores,
                    legal = if (s.canPlay) s.legal else emptyList(),
                    onPlay = vm::play,
                    frame = s.frame,
                    hintPit = s.hintPit,
                    upright = upright,
                    modifier = Modifier.fillMaxSize(),
                    frameMillis = s.frame?.let { frameDelay(it.kind, 1f) } ?: 200,
                )
            }
        }
        val tools: @Composable () -> Unit = {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                PanuIconButton({ menu = true }, tr(R.string.game_menu_open)) { Icon(OurilIcons.Pause, null, Modifier.size(22.dp)) }
                Spacer(Modifier.weight(1f))
                PanuIconButton({ rules = true }, tr(R.string.game_rules)) { Icon(OurilIcons.Info, null, Modifier.size(22.dp)) }
            }
        }
        val actions: @Composable () -> Unit = {
            // The menu is the top bar's pause button, so these are your moves' tools only.
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PanuIconButton(vm::undo, tr(R.string.game_undo), enabled = s.canUndo) {
                    Icon(OurilIcons.Undo, null, Modifier.size(22.dp))
                }
                if (settings.hints) {
                    PanuIconButton(vm::hint, tr(R.string.game_hint), enabled = s.canPlay) {
                        Icon(OurilIcons.Bulb, null, Modifier.size(22.dp))
                    }
                }
            }
        }
        val statusLine: @Composable ColumnScope.() -> Unit = {
            StatusPill(status, Modifier.align(Alignment.CenterHorizontally).semantics { liveRegion = LiveRegionMode.Polite })
        }

        if (upright) {
            Column(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                tools()
                Nameplate(ai, stores[1], you = false, turn = s.aiThinking)
                statusLine()
                board(Modifier.weight(1f).fillMaxWidth())
                Text(
                    announcement,
                    style = PanuType.small,
                    color = c.ink2,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth().semantics { liveRegion = LiveRegionMode.Polite },
                )
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Nameplate(you, stores[0], you = true, turn = s.canPlay, modifier = Modifier.weight(1f))
                    actions()
                }
            }
        } else {
            Row(Modifier.fillMaxSize().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Column(Modifier.width(280.dp).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    tools()
                    Nameplate(ai, stores[1], you = false, turn = s.aiThinking)
                    statusLine()
                    Nameplate(you, stores[0], you = true, turn = s.canPlay)
                    actions()
                    Body(announcement, style = PanuType.small)
                }
                board(Modifier.weight(1f).fillMaxHeight())
            }
        }
    }

    if (menu && s.finished == null) {
        Sheet(onDismiss = { menu = false }) {
            Text(tr(R.string.game_menu_title), style = PanuType.title.copy(fontSize = PanuType.section.fontSize), color = c.ink)
            Body(tr(R.string.game_menu_body))
            PanuButton({ menu = false }, Modifier.fillMaxWidth(), ButtonKind.PLAY) { Text(tr(R.string.game_menu_resume)) }
            PanuButton(onExit, Modifier.fillMaxWidth()) {
                Icon(OurilIcons.Home, null, Modifier.size(20.dp)); Text(tr(R.string.game_menu_save_exit))
            }
            PanuButton({ menu = false; confirmForfeit = true }, Modifier.fillMaxWidth(), ButtonKind.OUTLINE, enabled = s.canForfeit) {
                Icon(OurilIcons.Flag, null, Modifier.size(20.dp), tint = c.red); Text(tr(R.string.game_menu_forfeit), color = c.red)
            }
        }
    }
    if (confirmForfeit && s.finished == null) {
        Sheet(onDismiss = { confirmForfeit = false }) {
            Text(tr(R.string.game_forfeit_title), style = PanuType.section, color = c.ink)
            Body(tr(R.string.game_forfeit_body))
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                PanuButton({ confirmForfeit = false }, Modifier.weight(1f)) { Text(tr(R.string.game_menu_resume)) }
                PanuButton({ confirmForfeit = false; vm.forfeit() }, Modifier.weight(1f), ButtonKind.DANGER) {
                    Icon(OurilIcons.Flag, null, Modifier.size(20.dp)); Text(tr(R.string.game_forfeit_confirm))
                }
            }
        }
    }
    if (rules) {
        Sheet(onDismiss = { rules = false }) {
            Text(tr(R.string.game_rules_title, "name" to trKey("variant.${game.variant.id}.short")), style = PanuType.section, color = c.ink)
            RulesContent(game.variant.id)
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                PanuButton({ rules = false }, kind = ButtonKind.PRIMARY) { Text(tr(R.string.game_rules_close)) }
                PanuButton({ rules = false; onAllRules(game.variant.id) }, kind = ButtonKind.OUTLINE) { Text(tr(R.string.game_rules_all)) }
            }
        }
    }
    s.finished?.let { f ->
        Result(
            outcome = f.outcome,
            youScore = f.record.result.stores[0],
            aiScore = f.record.result.stores[1],
            reason = f.record.result.reason,
            onAgain = {
                vm.restartWith(NewGameRequest(game.level, Starter.RANDOM, game.variant.id))
            },
            onHome = onExit,
        )
    }
}

/** A move's events as short sentences (as the web's `describeEvents`). */
@Composable
private fun describe(events: List<MoveEvent>, byHuman: Boolean): String {
    val lines = mutableListOf<String>()
    val captured = events.filterIsInstance<MoveEvent.Capture>().sumOf { it.seeds }
    if (events.any { it is MoveEvent.SkipOrigin }) lines += tr(R.string.game_event_skip_origin)
    val relays = events.count { it is MoveEvent.Relay }
    if (relays > 0) lines += tr(R.string.game_event_relay, "count" to relays)
    if (captured > 0) lines += tr(if (byHuman) R.string.game_captured else R.string.game_ai_captured, "count" to captured)
    if (events.any { it is MoveEvent.GrandSlam }) lines += tr(R.string.game_event_grand_slam)
    if (events.any { it is MoveEvent.ExtraTurn }) lines += tr(if (byHuman) R.string.game_event_extra_turn_you else R.string.game_event_extra_turn_ai)
    return lines.joinToString(" ")
}

/** A bottom sheet in the Panu style (the web's dialogs on phones). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Sheet(onDismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    val c = panu
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = c.surface,
        shape = PanuShapes.sheet,
    ) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(bottom = 28.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            content = content,
        )
    }
}

/**
 * The result: the seed ring on a band with the panu strip. A win fills the ring one seed at a
 * time; a loss is calm basalt (its main action is Rematch); a draw is sea.
 */
@Composable
private fun Result(outcome: Outcome, youScore: Int, aiScore: Int, reason: String, onAgain: () -> Unit, onHome: () -> Unit) {
    val c = panu
    val band = when (outcome) {
        Outcome.WIN -> listOf(c.night, androidx.compose.ui.graphics.Color(0xFF1F2D78))
        Outcome.LOSS -> listOf(androidx.compose.ui.graphics.Color(0xFF1C1F29), androidx.compose.ui.graphics.Color(0xFF3B4152))
        Outcome.DRAW -> listOf(androidx.compose.ui.graphics.Color(0xFF0A4F57), androidx.compose.ui.graphics.Color(0xFF0C7580))
    }
    val key = when (outcome) {
        Outcome.WIN -> "win"
        Outcome.LOSS -> "loss"
        Outcome.DRAW -> "draw"
    }
    Dialog(onDismissRequest = onHome) {
        Column(Modifier.clip(PanuShapes.dialog).background(c.surface), horizontalAlignment = Alignment.CenterHorizontally) {
            Box(Modifier.fillMaxWidth().height(110.dp)) {
                Column(Modifier.fillMaxWidth().height(96.dp).background(androidx.compose.ui.graphics.Brush.linearGradient(band))) {
                    Spacer(Modifier.weight(1f))
                    PanuStrip(Modifier.fillMaxWidth().height(16.dp))
                }
                Box(
                    Modifier.align(Alignment.BottomCenter).size(92.dp).clip(androidx.compose.foundation.shape.CircleShape).background(c.surface),
                    contentAlignment = Alignment.Center,
                ) {
                    FillingSeedRing(Modifier.size(64.dp), animate = outcome == Outcome.WIN, ink = if (outcome == Outcome.WIN) c.ink else c.muted)
                }
            }
            Column(Modifier.padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(trKey("game.over.$key"), style = PanuType.title, color = c.ink)
                Body(trKey("game.over.tagline.$key"), modifier = Modifier, color = c.ink2)
                Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                    ScoreSide(youScore, tr(R.string.game_over_you))
                    Text("–", style = PanuType.section, color = c.muted, modifier = Modifier.padding(bottom = 22.dp))
                    ScoreSide(aiScore, tr(R.string.game_over_ai))
                }
                Body(trKey("game.over.reason.$reason"), color = c.muted, style = PanuType.small)
                PanuButton(onAgain, Modifier.fillMaxWidth(), ButtonKind.PLAY) {
                    Text(tr(if (outcome == Outcome.LOSS) R.string.game_over_rematch else R.string.game_over_play_again))
                }
                PanuButton(onHome, Modifier.fillMaxWidth(), ButtonKind.OUTLINE) { Text(tr(R.string.game_over_home)) }
            }
        }
    }
}

@Composable
private fun ScoreSide(score: Int, label: String) {
    val c = panu
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(score.toString(), style = PanuType.title.copy(fontSize = PanuType.hero.fontSize), color = c.ink)
        Text(label, style = PanuType.small, color = c.muted)
    }
}
