package org.moreiralabs.ouril.game

import android.animation.ValueAnimator
import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.CoreError
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.data.GameRecord
import org.moreiralabs.ouril.data.Store
import java.time.Instant

/**
 * Drives one game against the computer, as the web's `useGameController`: your moves, the
 * computer's (searched off the main thread), the move animation frame by frame, saving after
 * every move, undo, hints, forfeit, and the finished game's record. No rule logic: legality,
 * results and positions all come from the core.
 */
class GameViewModel(app: Application) : AndroidViewModel(app) {
    private val store = Store(app)

    data class Finished(val outcome: Outcome, val record: GameRecord)

    /** What the last move did, for the line under the board (and screen readers). */
    data class Announcement(val id: Int, val events: List<MoveEvent>, val byHuman: Boolean)

    data class UiState(
        val game: CurrentGame? = null,
        val frame: Frame? = null,
        val legal: List<Int> = emptyList(),
        val animating: Boolean = false,
        val finished: Finished? = null,
        val hintPit: Int? = null,
        val announcement: Announcement? = null,
        val error: String? = null,
        /** No game to resume and none requested: the screen offers a new one. */
        val empty: Boolean = false,
    ) {
        val humanTurn: Boolean
            get() = game != null && !game.state.isOver && game.state.toMove == game.humanPlayer
        val canPlay: Boolean get() = humanTurn && !animating && finished == null
        val aiThinking: Boolean
            get() = game != null && !game.state.isOver && !humanTurn && !animating && finished == null
        val canUndo: Boolean get() = canPlay && game != null && undoPossible(game)
        val canForfeit: Boolean get() = finished == null && !animating && game != null && !game.state.isOver
    }

    private val _state = MutableStateFlow(UiState())
    val state: StateFlow<UiState> = _state.asStateFlow()

    /** Each frame as it's shown, for sounds and haptics. */
    private val _frames = MutableSharedFlow<FrameKind>(extraBufferCapacity = 8)
    /** Each animation frame as it shows, for sounds and haptics. */
    val frames: SharedFlow<FrameKind> = _frames
    private val _ends = MutableSharedFlow<Outcome>(extraBufferCapacity = 1)
    /** A game that just ended (by the rules or a forfeit), once, for its sound. */
    val ends: SharedFlow<Outcome> = _ends

    /** The system's animations are off: frames come faster and the board's seeds snap. */
    val reducedMotion: Boolean = !ValueAnimator.areAnimatorsEnabled()

    /** The game's pace: the 3D board's (a hand carries the seeds), shorter with reduced motion. */
    val speed: Double = (if (reducedMotion) REDUCED_MOTION_SPEED else 1.0) * PACE_3D

    // Incremented by every new position, so a stale search or hint is ignored.
    private var epoch = 0
    private var aiJob: Job? = null
    private var started = false

    /** Resume the saved game, or start `request` (forfeiting an unfinished game first). */
    fun start(request: NewGameRequest?) {
        if (started) return
        started = true
        viewModelScope.launch {
            val saved = store.currentGame.first()
            val game = if (request != null) {
                if (saved != null && !saved.state.isOver) store.saveFinishedGame(forfeitRecord(saved, Instant.now()))
                store.rememberNewGame(request)
                withContext(Dispatchers.Default) { startGame(request) }.also { store.saveCurrentGame(it) }
            } else {
                saved
            }
            if (game == null) {
                _state.update { it.copy(empty = true) }
                return@launch
            }
            show(game)
            // A game that ended before its record was saved (the app closed in between).
            if (game.state.isOver) finish(game, null)
        }
    }

    /** A new game with `request` (Play again, Rematch, or the empty screen's form). */
    fun restartWith(request: NewGameRequest) {
        epoch++
        aiJob?.cancel()
        started = true
        viewModelScope.launch {
            val s = _state.value
            val unfinished = s.game?.takeIf { !it.state.isOver && s.finished == null }
            if (unfinished != null) store.saveFinishedGame(forfeitRecord(unfinished, Instant.now()))
            store.rememberNewGame(request)
            val game = withContext(Dispatchers.Default) { startGame(request) }
            store.saveCurrentGame(game)
            _state.value = UiState()
            show(game)
        }
    }

    fun play(pit: Int) {
        val s = _state.value
        val game = s.game ?: return
        if (!s.canPlay || pit !in s.legal) return
        viewModelScope.launch { commit(game, pit, byHuman = true) }
    }

    fun undo() {
        val s = _state.value
        val game = s.game ?: return
        if (!s.canUndo) return
        val prev = undoLastHumanMove(game) ?: return
        epoch++
        show(prev)
        viewModelScope.launch { store.saveCurrentGame(prev) }
    }

    /** Hints reuse the computer's search for your side (docs/architecture/ai.md). */
    fun hint() {
        val s = _state.value
        val game = s.game ?: return
        if (!s.canPlay) return
        val myEpoch = epoch
        viewModelScope.launch {
            val pit = withContext(Dispatchers.Default) {
                Core.aiMove(game.variant, game.state, Level.MEDIUM, aiSeedFor(game))
            }
            if (epoch == myEpoch) _state.update { it.copy(hintPit = pit) }
        }
    }

    /** End the game now as a loss for you (recorded like any game, ADR 0022). */
    fun forfeit() {
        val s = _state.value
        val game = s.game ?: return
        if (!s.canForfeit) return
        epoch++
        aiJob?.cancel()
        viewModelScope.launch {
            val record = forfeitRecord(game, Instant.now())
            store.saveFinishedGame(record)
            _state.update { it.copy(hintPit = null, finished = Finished(Outcome.LOSS, record)) }
            _ends.tryEmit(Outcome.LOSS)
        }
    }

    private fun show(game: CurrentGame) {
        _state.update {
            it.copy(
                game = game,
                frame = null,
                legal = if (game.state.isOver) emptyList() else Core.legalMoves(game.variant, game.state),
                hintPit = null,
            )
        }
        nextTurn()
    }

    /** The computer's turn: search off the main thread, then play after a short, readable pause. */
    private fun nextTurn() {
        val s = _state.value
        val game = s.game ?: return
        if (!s.aiThinking) return
        val myEpoch = epoch
        aiJob?.cancel()
        aiJob = viewModelScope.launch {
            val startedAt = System.currentTimeMillis()
            val pit = try {
                withContext(Dispatchers.Default) {
                    Core.aiMove(game.variant, game.state, game.level, aiSeedFor(game))
                }
            } catch (e: CoreError) {
                _state.update { it.copy(error = e.code) }
                return@launch
            }
            val wait = (AI_PAUSE_MS * speed).toLong() - (System.currentTimeMillis() - startedAt)
            if (wait > 0) delay(wait)
            if (epoch != myEpoch || pit == null) return@launch
            commit(game, pit, byHuman = false)
        }
    }

    private suspend fun commit(before: CurrentGame, pit: Int, byHuman: Boolean) {
        val (next, result) = try {
            withContext(Dispatchers.Default) { playMove(before, pit) }
        } catch (e: CoreError) {
            _state.update { it.copy(error = e.code) }
            return
        }
        epoch++
        _state.update { it.copy(hintPit = null, error = null, animating = true) }
        // Save first: closing the app mid-animation resumes from the new position.
        store.saveCurrentGame(next)
        for (f in buildFrames(before.state, pit, result.events, result.state)) {
            _state.update { it.copy(frame = f) }
            _frames.tryEmit(f.kind)
            val d = frameDelay(f.kind, speed).toLong()
            if (d > 0) delay(d)
        }
        _state.update {
            it.copy(
                frame = null,
                animating = false,
                announcement = Announcement((it.announcement?.id ?: 0) + 1, result.events, byHuman),
            )
        }
        show(next)
        if (next.state.isOver) finish(next, result.events)
    }

    private suspend fun finish(game: CurrentGame, events: List<MoveEvent>?) {
        val reason = withContext(Dispatchers.Default) { endReason(game, events) }
        val record = finishedRecord(game, reason, Instant.now())
        store.saveFinishedGame(record)
        val outcome = outcomeFor(game.state.status, game.humanPlayer) ?: return
        _state.update { it.copy(finished = Finished(outcome, record)) }
        _ends.tryEmit(outcome)
    }
}
