package org.moreiralabs.ouril.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.game.Frame
import org.moreiralabs.ouril.game.TUTORIAL_STEPS
import org.moreiralabs.ouril.game.buildFrames
import org.moreiralabs.ouril.game.frameDelay
import org.moreiralabs.ouril.game.variantRef
import org.moreiralabs.ouril.i18n.stringId
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.i18n.trKey
import org.moreiralabs.ouril.ui.board.Board
import org.moreiralabs.ouril.ui.components.Body
import org.moreiralabs.ouril.ui.components.ButtonKind
import org.moreiralabs.ouril.ui.components.PanuButton
import org.moreiralabs.ouril.ui.theme.PanuShapes
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/**
 * The interactive tutorial, as on the web: ten lessons on fixed positions (`TUTORIAL_STEPS`),
 * each allowing only the moves it teaches. The core plays every move; the board shows it.
 */
@Composable
fun TutorialScreen(onFinish: () -> Unit) {
    var index by rememberSaveable { mutableIntStateOf(0) }
    val step = TUTORIAL_STEPS[index]
    val variant = remember(step.variant) { variantRef(step.variant) }
    var state by remember(index) { mutableStateOf<GameState>(step.position) }
    var moveNo by remember(index) { mutableIntStateOf(0) }
    var frame by remember(index) { mutableStateOf<Frame?>(null) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val c = panu

    val done = moveNo >= step.moves.size
    val legal = remember(state) { Core.legalMoves(variant, state) }
    val allowed = if (done || frame != null) emptyList() else step.moves[moveNo]?.let { only -> legal.filter { it in only } } ?: legal

    fun reset(i: Int) {
        index = i
    }

    fun play(pit: Int) {
        if (pit !in allowed) return
        val before = state
        val result = Core.applyMove(variant, before, pit)
        scope.launch {
            for (f in buildFrames(before, pit, result.events, result.state)) {
                frame = f
                val d = frameDelay(f.kind, 1f)
                if (d > 0) delay(d)
            }
            frame = null
            state = result.state
            moveNo += 1
        }
    }

    BoxWithConstraints(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars)) {
        val upright = maxHeight > maxWidth
        // A lesson can say where your pits are differently for the upright board (`.intro_upright`).
        val intro = "tutorial.${step.id}.intro"
        val text = when {
            done -> trKey("tutorial.${step.id}.done")
            moveNo > 0 -> trKey("tutorial.${step.id}.next")
            upright && context.stringId("${intro}_upright") != 0 -> trKey("${intro}_upright")
            else -> trKey(intro)
        }
        Column(Modifier.fillMaxSize().padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(tr(R.string.tutorial_title), style = PanuType.title, color = c.ink, modifier = Modifier.semantics { heading() })
            Text(tr(R.string.tutorial_progress, "step" to index + 1, "total" to TUTORIAL_STEPS.size), style = PanuType.small, color = c.muted)
            Text(trKey("tutorial.${step.id}.title"), style = PanuType.section, color = c.ink, modifier = Modifier.semantics { heading() })
            Body(text, style = PanuType.bodyLarge, modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
            Board(
                pits = frame?.pits ?: state.pits,
                stores = frame?.stores ?: state.stores,
                legal = allowed,
                onPlay = ::play,
                frame = frame,
                hintPit = null,
                upright = upright,
                modifier = Modifier.weight(1f).fillMaxWidth().clip(PanuShapes.card),
                frameMillis = frame?.let { frameDelay(it.kind, 1f) } ?: 200,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                PanuButton({ state = step.position; moveNo = 0; frame = null }) { Text(tr(R.string.tutorial_retry)) }
                if (index > 0) PanuButton({ reset(index - 1) }) { Text(tr(R.string.tutorial_back)) }
                if (done && index < TUTORIAL_STEPS.lastIndex) {
                    PanuButton({ reset(index + 1) }, kind = ButtonKind.PRIMARY) { Text(tr(R.string.tutorial_continue)) }
                }
                if (done && index == TUTORIAL_STEPS.lastIndex) {
                    PanuButton(onFinish, kind = ButtonKind.PRIMARY) { Text(tr(R.string.tutorial_finish)) }
                }
            }
        }
    }
}
