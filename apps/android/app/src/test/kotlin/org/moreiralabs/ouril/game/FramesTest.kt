package org.moreiralabs.ouril.game

import org.junit.Assert.assertEquals
import org.junit.Test
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.core.Player
import org.moreiralabs.ouril.core.Status
import org.moreiralabs.ouril.core.VariantRef
import kotlin.random.Random

class FramesTest {
    /** Example 2 of docs/game/rules.md: a capture with a chain. */
    @Test
    fun aChainCaptureIsShownSeedBySeed() {
        val v = VariantRef("cv.standard", 1)
        val before = GameState(listOf(4, 4, 4, 4, 3, 0, 1, 2, 4, 4, 4, 4), listOf(5, 5), Player.SOUTH, 0, Status.PLAYING)
        val result = Core.applyMove(v, before, 4)
        val frames = buildFrames(before, 4, result.events, result.state)

        assertEquals(
            listOf(FrameKind.PICKUP, FrameKind.SOW, FrameKind.SOW, FrameKind.SOW, FrameKind.CAPTURE, FrameKind.CAPTURE, FrameKind.END),
            frames.map { it.kind },
        )
        assertEquals(listOf(4, 5, 6, 7, 7, 6, null), frames.map { it.pit })
        assertEquals(listOf(8, 5), frames[4].stores)
        assertEquals(listOf(10, 5), frames[5].stores)
        assertEquals(result.state.pits, frames.last().pits)
    }

    /**
     * Random games in every playable variant: replaying a move's events frame by frame always
     * lands on the engine's board, so the animation can never show a position the rules didn't
     * produce.
     */
    @Test
    fun framesLandOnTheEnginesBoardInEveryVariant() {
        val random = Random(7)
        for (id in PLAYABLE_VARIANTS) {
            val variant = variantRef(id)
            repeat(12) { game ->
                var state = Core.newGame(variant, if (game % 2 == 0) Player.SOUTH else Player.NORTH)
                var ply = 0
                while (!state.isOver && ply < 400) {
                    val legal = Core.legalMoves(variant, state)
                    val pit = legal[random.nextInt(legal.size)]
                    val result = Core.applyMove(variant, state, pit)
                    val frames = buildFrames(state, pit, result.events, result.state)
                    val where = "$id game $game ply $ply"
                    assertEquals(where, FrameKind.PICKUP, frames.first().kind)
                    assertEquals(where, 0, frames.first().pits[pit])
                    assertEquals(where, FrameKind.END, frames.last().kind)
                    // The last frame before the end is the events replayed on the old board.
                    val replayed = frames[frames.size - 2]
                    assertEquals(where, result.state.pits, replayed.pits)
                    assertEquals(where, result.state.stores, replayed.stores)
                    state = result.state
                    ply++
                }
            }
        }
    }

    @Test
    fun framesTakeTheWebsTimes() {
        assertEquals(200.0, frameDelay(FrameKind.SOW, 1.0), 0.0)
        assertEquals(600.0, frameDelay(FrameKind.CAPTURE, 1.0), 0.0)
        assertEquals(80.0, frameDelay(FrameKind.SOW, 0.4), 1e-9)
        assertEquals(0.0, frameDelay(FrameKind.END, 1.0), 0.0)
        assertEquals(0.0, frameDelay(FrameKind.CAPTURE, 0.0), 0.0)
        // The 3D board's pace: a sow takes 440 ms, as on the web.
        assertEquals(440.0, frameDelay(FrameKind.SOW, PACE_3D), 1e-9)
    }
}
