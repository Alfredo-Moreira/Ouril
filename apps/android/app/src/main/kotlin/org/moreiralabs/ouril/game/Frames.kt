package org.moreiralabs.ouril.game

import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.core.Player

/**
 * The engine's move events as display frames, one board picture per seed drop or capture
 * (the same as the web's `game/animation.ts`). Presentation only: the last frame is always
 * the engine's resulting state, so the board can never drift from the rules.
 */
enum class FrameKind { PICKUP, SOW, SKIP, CAPTURE, GRAND_SLAM, EXTRA_TURN, COLLECT, END }

data class Frame(
    val kind: FrameKind,
    val pits: List<Int>,
    val stores: List<Int>,
    /** The pit picked up, sown into, skipped or captured. */
    val pit: Int? = null,
    /** Seeds captured or collected. */
    val seeds: Int? = null,
    val player: Player? = null,
)

fun buildFrames(before: GameState, pit: Int, events: List<MoveEvent>, after: GameState): List<Frame> {
    val mover = before.toMove
    val pits = before.pits.toMutableList()
    val stores = before.stores.toMutableList()
    val frames = mutableListOf<Frame>()
    fun snap(kind: FrameKind, pit: Int? = null, seeds: Int? = null, player: Player? = null) {
        frames += Frame(kind, pits.toList(), stores.toList(), pit, seeds, player)
    }

    // Seeds leave the chosen pit before sowing.
    pits[pit] = 0
    snap(FrameKind.PICKUP, pit, player = mover)

    for (e in events) {
        when (e) {
            is MoveEvent.Sow -> {
                pits[e.pit] += 1
                snap(FrameKind.SOW, e.pit)
            }
            is MoveEvent.SkipOrigin -> snap(FrameKind.SKIP, e.pit)
            // Relay sowing (ADR 0023): an own pit's seeds are picked up and sown on.
            is MoveEvent.Relay -> {
                pits[e.pit] = 0
                snap(FrameKind.PICKUP, e.pit, player = mover)
            }
            is MoveEvent.Capture -> {
                pits[e.pit] = 0
                stores[mover.store] += e.seeds
                snap(FrameKind.CAPTURE, e.pit, e.seeds, mover)
            }
            MoveEvent.GrandSlam -> snap(FrameKind.GRAND_SLAM, player = mover)
            MoveEvent.ExtraTurn -> snap(FrameKind.EXTRA_TURN, player = mover)
            is MoveEvent.CollectRemaining -> {
                val n = pits.size / 2
                val start = if (e.player == Player.SOUTH) 0 else n
                for (i in start until start + n) pits[i] = 0
                stores[e.player.store] += e.seeds
                snap(FrameKind.COLLECT, seeds = e.seeds, player = e.player)
            }
            // game_over and unknown future events: no board change.
            is MoveEvent.GameOver, is MoveEvent.Unknown -> Unit
        }
    }

    frames += Frame(FrameKind.END, after.pits, after.stores)
    return frames
}

/** Milliseconds each frame kind shows at speed 1 (the web's `FRAME_BASE_MS`); the 3D board plays them at [PACE_3D]. */
val FRAME_BASE_MS: Map<FrameKind, Int> = mapOf(
    FrameKind.PICKUP to 320,
    FrameKind.SOW to 200,
    FrameKind.SKIP to 260,
    // Long enough for the hand to grab the seeds and carry them to the store.
    FrameKind.CAPTURE to 600,
    FrameKind.GRAND_SLAM to 700,
    FrameKind.EXTRA_TURN to 500,
    FrameKind.COLLECT to 500,
    FrameKind.END to 0,
)

/** How much slower move frames play on the 3D board: a seed needs time to travel in the hand (the web's `PACE_3D`). */
const val PACE_3D = 2.2

/** The game's animation speed with the system's animations turned off (the web's reduced motion). */
const val REDUCED_MOTION_SPEED = 0.4

/** The computer waits at least this long (times the speed) before its move shows. */
const val AI_PAUSE_MS = 350

/** Milliseconds to show a frame (`speed` 0 = instant, for tests). Same timings as the web. */
fun frameDelay(kind: FrameKind, speed: Double): Double =
    if (speed <= 0) 0.0 else FRAME_BASE_MS.getValue(kind) * speed
