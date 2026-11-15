package org.moreiralabs.ouril.game

import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.CoreJson
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.core.Player
import org.moreiralabs.ouril.core.Status
import org.moreiralabs.ouril.data.GameRecord
import java.time.Instant

class SessionTest {
    private val now = Instant.parse("2026-11-14T10:00:00Z")

    private fun computerMove(game: CurrentGame) = Core.aiMove(game.variant, game.state, game.level, aiSeedFor(game))!!

    @Test
    fun youStartWhenYouAsk() {
        val g = startGame(NewGameRequest(Level.MEDIUM, Starter.ME, "cv.continuous"), now)
        assertEquals(Player.SOUTH, g.firstPlayer)
        assertEquals(Player.SOUTH, g.state.toMove)
        assertEquals("cv.continuous", g.variant.id)
        assertEquals(Level.MEDIUM, g.level)
        assertEquals(now.toString(), g.startedAt)
        assertTrue(g.moves.isEmpty())
        assertTrue(g.aiSeed in 0..9_007_199_254_740_991L)
        assertFalse(undoPossible(g))
    }

    @Test
    fun theComputerStartsWhenAsked() {
        val g = startGame(NewGameRequest(starter = Starter.AI), now)
        assertEquals(Player.NORTH, g.firstPlayer)
        assertEquals(Player.NORTH, g.state.toMove)
    }

    @Test
    fun undoGoesBackToBeforeYourLastMove() {
        var g = startGame(NewGameRequest(starter = Starter.ME), now)
        val start = g.state
        g = playMove(g, 2).first
        g = playMove(g, computerMove(g)).first
        assertTrue(undoPossible(g))

        val undone = undoLastHumanMove(g)!!
        assertEquals(emptyList<Int>(), undone.moves)
        assertEquals(start, undone.state)
        assertNull(undoLastHumanMove(undone))
    }

    @Test
    fun undoKeepsTheComputersOpeningMove() {
        var g = startGame(NewGameRequest(starter = Starter.AI), now)
        g = playMove(g, computerMove(g)).first
        val opening = g.moves
        val afterOpening = g.state
        assertFalse(undoPossible(g))
        g = playMove(g, Core.legalMoves(g.variant, g.state).first()).first
        g = playMove(g, computerMove(g)).first
        assertTrue(undoPossible(g))

        val undone = undoLastHumanMove(g)!!
        assertEquals(opening, undone.moves)
        assertEquals(afterOpening, undone.state)
    }

    @Test
    fun theComputerPlaysTheSameMoveForTheSameGame() {
        val g = startGame(NewGameRequest(Level.HARD, Starter.AI), now)
        assertEquals(computerMove(g), computerMove(g.copy()))
        assertEquals(aiSeedFor(g) + 1, aiSeedFor(playMove(g, computerMove(g)).first))
    }

    @Test
    fun outcomesAreFromYourSide() {
        assertEquals(Outcome.WIN, outcomeFor(Status.SOUTH_WINS, Player.SOUTH))
        assertEquals(Outcome.LOSS, outcomeFor(Status.NORTH_WINS, Player.SOUTH))
        assertEquals(Outcome.WIN, outcomeFor(Status.NORTH_WINS, Player.NORTH))
        assertEquals(Outcome.DRAW, outcomeFor(Status.DRAW, Player.SOUTH))
        assertNull(outcomeFor(Status.PLAYING, Player.SOUTH))
    }

    @Test
    fun gameIdsAreTimeOrderedUuidV7() {
        val id = uuidV7(now.toEpochMilli())
        assertTrue(id, Regex("^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$").matches(id))
        assertEquals(now.toEpochMilli(), id.replace("-", "").take(12).toLong(16))
        assertTrue(uuidV7(1_000) < uuidV7(2_000))
    }

    @Test
    fun unknownVariantsFallBackToTheCapeVerdeDefault() {
        assertEquals("cv.standard", variantRef(null).id)
        assertEquals("cv.standard", variantRef("zz.nope").id)
        assertEquals("cv.across", variantRef("cv.across").id)
    }

    @Test
    fun aSavedGameResumesExactly() {
        var g = startGame(NewGameRequest(starter = Starter.ME), now)
        g = playMove(g, 2).first
        val json = CoreJson.encodeToString(CurrentGame.serializer(), g)
        assertEquals(g, CoreJson.decodeFromString(CurrentGame.serializer(), json))
    }

    /**
     * A whole game against the computer, then the records the app saves for it and for a
     * forfeit, read back by the core's stats (`ouril-sync`): the same record format as the web.
     */
    @Test
    fun finishedGamesAreRecordsTheCoreReads() {
        var g = startGame(NewGameRequest(Level.EASY, Starter.ME), now)
        var events = emptyList<MoveEvent>()
        while (!g.state.isOver) {
            val pit = if (g.state.toMove == g.humanPlayer) Core.legalMoves(g.variant, g.state).last() else computerMove(g)
            val (next, result) = playMove(g, pit)
            g = next
            events = result.events
        }
        val finished = finishedRecord(g, endReason(g, events), now.plusSeconds(600))
        val forfeit = forfeitRecord(startGame(NewGameRequest(starter = Starter.ME), now), now.plusSeconds(60))
        val json = CoreJson.encodeToString(ListSerializer(GameRecord.serializer()), listOf(finished, forfeit))

        assertTrue(json, "\"mode\":\"vs_ai\"" in json)
        val stats = CoreJson.parseToJsonElement(uniffi.ouril_ffi.deriveStatsJson(json)).jsonObject
        val overall = stats.getValue("overall").jsonObject
        assertEquals(2, overall.getValue("played").jsonPrimitive.int)
        val won = outcomeFor(g.state.status, g.humanPlayer)
        assertEquals((if (won == Outcome.LOSS) 1 else 0) + 1, overall.getValue("losses").jsonPrimitive.int)
        assertEquals(if (won == Outcome.WIN) 1 else 0, overall.getValue("wins").jsonPrimitive.int)
    }
}
