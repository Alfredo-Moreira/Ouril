package org.moreiralabs.ouril.data

import android.content.Context
import androidx.room.withTransaction
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emitAll
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.moreiralabs.ouril.core.CoreJson
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.core.Player
import org.moreiralabs.ouril.core.VariantRef
import org.moreiralabs.ouril.game.CurrentGame
import org.moreiralabs.ouril.game.NewGameRequest
import org.moreiralabs.ouril.game.uuidV7
import java.time.Instant

@Serializable
data class Settings(
    /** Sound effects and haptics while sowing. */
    val sound: Boolean = true,
    /** The game screen's Hint button. */
    val hints: Boolean = false,
)

/** A finished game (record format 1, or 2 for a forfeit; core/wasm/API.md). */
@Serializable
data class GameRecord(
    val format: Int,
    val id: String,
    val variant: VariantRef,
    @SerialName("first_player") val firstPlayer: Player,
    val moves: List<Int>,
    @SerialName("core_version") val coreVersion: String,
    val result: RecordResult,
    @SerialName("started_at") val startedAt: String,
    @SerialName("ended_at") val endedAt: String,
    val mode: String = "vs_ai",
    @SerialName("ai_level") val aiLevel: Level,
    @SerialName("human_player") val humanPlayer: Player,
)

@Serializable
data class RecordResult(val outcome: String, val stores: List<Int>, val reason: String)

/**
 * Everything the app keeps on the device, as on the web (`storage/repo.ts`): settings and the
 * game in progress per owner, finished games, and the outbox of changes to upload. Guests get
 * a guest ID on first launch and never touch the network; their outbox waits for sign-in
 * (docs/architecture/data-and-sync.md).
 */
class Store(private val db: OurilDatabase) {
    constructor(context: Context) : this(OurilDatabase.get(context))

    private val dao = db.dao()

    /** This device's guest profile, made on first launch. */
    suspend fun guestOwner(): String = db.withTransaction {
        get(KV_GUEST_ID, String.serializer()) ?: "guest:${uuidV7(System.currentTimeMillis())}".also {
            put(KV_GUEST_ID, String.serializer(), it)
        }
    }

    /** Whose data the app shows and writes: the guest's until sign-in comes to Android. */
    suspend fun activeOwner(): String = get(KV_OWNER, String.serializer()) ?: guestOwner()

    val settings: Flow<Settings> = owned { owner ->
        dao.observeKv(settingsKey(owner)).map { it?.let { decode(Settings.serializer(), it) } ?: Settings() }
    }

    suspend fun setSound(on: Boolean) = updateSettings("sound", on) { it.copy(sound = on) }

    suspend fun setHints(on: Boolean) = updateSettings("hints", on) { it.copy(hints = on) }

    /** Save a settings change and queue its `settings_updated` in the same transaction. */
    private suspend fun updateSettings(field: String, value: Boolean, change: (Settings) -> Settings) =
        db.withTransaction {
            val owner = activeOwner()
            val current = get(settingsKey(owner), Settings.serializer()) ?: Settings()
            put(settingsKey(owner), Settings.serializer(), change(current))
            enqueue(owner, "settings_updated", buildJsonObject { put(field, value) })
        }

    val currentGame: Flow<CurrentGame?> = owned { owner ->
        dao.observeKv(currentGameKey(owner)).map { it?.let { decode(CurrentGame.serializer(), it) } }
    }

    suspend fun saveCurrentGame(game: CurrentGame) = put(currentGameKey(activeOwner()), CurrentGame.serializer(), game)

    suspend fun clearCurrentGame() = dao.deleteKv(currentGameKey(activeOwner()))

    /** The last new-game choices on this device (the web keeps them in localStorage). */
    suspend fun lastNewGame(): NewGameRequest = get(KV_LAST_NEW_GAME, NewGameRequest.serializer()) ?: NewGameRequest()

    suspend fun rememberNewGame(request: NewGameRequest) = put(KV_LAST_NEW_GAME, NewGameRequest.serializer(), request)

    /** Keep a finished game, queue its upload and clear the game in progress: one transaction. */
    suspend fun saveFinishedGame(record: GameRecord) = db.withTransaction {
        val owner = activeOwner()
        val json = CoreJson.encodeToJsonElement(GameRecord.serializer(), record)
        dao.putGame(GameRow(record.id, owner, record.endedAt, json.toString()))
        enqueue(owner, "game_finished", json)
        dao.deleteKv(currentGameKey(owner))
    }

    /** The active owner's finished games, newest first. */
    suspend fun games(): List<GameRecord> =
        dao.games(activeOwner()).mapNotNull { decode(GameRecord.serializer(), it.record) }

    /** A mutation for the sync API (`apps/server/API.md`), as the web's `enqueue`. */
    private suspend fun enqueue(owner: String, type: String, payload: JsonElement) {
        val id = uuidV7(System.currentTimeMillis())
        val mutation = buildJsonObject {
            put("id", id)
            put("type", type)
            put("schema", 1)
            put("payload", payload)
            put("client_time", Instant.now().toString())
        }
        dao.enqueue(OutboxRow(id, owner, mutation.toString()))
    }

    private suspend fun <T> get(key: String, serializer: KSerializer<T>): T? = dao.kv(key)?.let { decode(serializer, it) }

    private suspend fun <T> put(key: String, serializer: KSerializer<T>, value: T) =
        dao.putKv(KvRow(key, CoreJson.encodeToString(serializer, value)))

    /** Unreadable values (from a newer version, say) read as missing rather than crashing. */
    private fun <T> decode(serializer: KSerializer<T>, json: String): T? =
        runCatching { CoreJson.decodeFromString(serializer, json) }.getOrNull()

    private fun <T> owned(block: (String) -> Flow<T>): Flow<T> = flow { emitAll(block(activeOwner())) }

    private companion object {
        const val KV_GUEST_ID = "guest_id"
        const val KV_OWNER = "owner"
        const val KV_LAST_NEW_GAME = "last_new_game"
        fun settingsKey(owner: String) = "settings:$owner"
        fun currentGameKey(owner: String) = "current_game:$owner"
    }
}
