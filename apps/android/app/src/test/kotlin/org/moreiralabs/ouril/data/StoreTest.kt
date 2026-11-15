package org.moreiralabs.ouril.data

import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.moreiralabs.ouril.core.CoreJson
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.game.NewGameRequest
import org.moreiralabs.ouril.game.Starter
import org.moreiralabs.ouril.game.forfeitRecord
import org.moreiralabs.ouril.game.startGame
import org.robolectric.RobolectricTestRunner
import java.time.Instant

/** The local database, with the web's data model (docs/architecture/data-and-sync.md). */
@RunWith(RobolectricTestRunner::class)
class StoreTest {
    private val db = Room.inMemoryDatabaseBuilder(ApplicationProvider.getApplicationContext(), OurilDatabase::class.java)
        .allowMainThreadQueries()
        .build()
    private val store = Store(db)

    @After
    fun close() = db.close()

    @Test
    fun aGuestGetsOneIdOnFirstLaunch() = runTest {
        val id = store.activeOwner()
        assertTrue(id, Regex("^guest:[0-9a-f-]{36}$").matches(id))
        assertEquals(id, store.guestOwner())
        assertEquals(id, Store(db).activeOwner())
    }

    @Test
    fun settingsChangesAreSavedAndQueued() = runTest {
        assertEquals(Settings(sound = true, hints = false), store.settings.first())
        store.setSound(false)
        store.setHints(true)
        assertEquals(Settings(sound = false, hints = true), store.settings.first())

        val queued = db.dao().outbox(store.activeOwner()).map { CoreJson.parseToJsonElement(it.mutation).jsonObject }
        assertEquals(listOf("settings_updated", "settings_updated"), queued.map { it.getValue("type").jsonPrimitive.content })
        assertEquals("""{"sound":false}""", queued[0].getValue("payload").toString())
        assertEquals("""{"hints":true}""", queued[1].getValue("payload").toString())
        assertEquals(1, queued[0].getValue("schema").jsonPrimitive.int)
    }

    /** As the web's `saveFinishedGame`: the game, its upload and clearing the game in progress. */
    @Test
    fun aFinishedGameIsKeptQueuedAndNoLongerInProgress() = runTest {
        val now = Instant.parse("2026-11-14T10:00:00Z")
        val game = startGame(NewGameRequest(Level.HARD, Starter.ME), now)
        store.saveCurrentGame(game)
        assertEquals(game, store.currentGame.first())

        val record = forfeitRecord(game, now.plusSeconds(30))
        store.saveFinishedGame(record)

        assertNull(store.currentGame.first())
        assertEquals(listOf(record), store.games())
        val queued = db.dao().outbox(store.activeOwner()).single()
        val mutation = CoreJson.parseToJsonElement(queued.mutation).jsonObject
        assertEquals("game_finished", mutation.getValue("type").jsonPrimitive.content)
        assertEquals(record, CoreJson.decodeFromJsonElement(GameRecord.serializer(), mutation.getValue("payload")))
        assertEquals(queued.id, mutation.getValue("id").jsonPrimitive.content)
    }

    @Test
    fun theLastNewGameChoicesAreRemembered() = runTest {
        assertEquals(NewGameRequest(), store.lastNewGame())
        val choice = NewGameRequest(Level.MEDIUM, Starter.AI, "cv.across")
        store.rememberNewGame(choice)
        assertEquals(choice, store.lastNewGame())
    }
}
