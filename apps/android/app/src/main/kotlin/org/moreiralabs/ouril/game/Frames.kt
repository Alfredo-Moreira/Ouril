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

/** Milliseconds to show a frame (`speed` 0 = instant, for tests). Same timings as the web. */
fun frameDelay(kind: FrameKind, speed: Float): Long {
    if (speed <= 0f) return 0
    val base = when (kind) {
        FrameKind.PICKUP -> 320
        FrameKind.SOW -> 200
        FrameKind.SKIP -> 260
        FrameKind.CAPTURE -> 600
        FrameKind.GRAND_SLAM -> 700
        FrameKind.EXTRA_TURN -> 500
        FrameKind.COLLECT -> 500
        FrameKind.END -> 0
    }
    return (base * speed).toLong()
}
