package org.moreiralabs.ouril.data

import android.content.Context
import androidx.room.ColumnInfo
import androidx.room.Dao
import androidx.room.Database
import androidx.room.Entity
import androidx.room.Index
import androidx.room.Insert
import androidx.room.PrimaryKey
import androidx.room.Query
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.moreiralabs.ouril.OurilApplication

/**
 * The local database: SQLite through Room (ADR 0010), with the same tables as the web's
 * IndexedDB (docs/architecture/data-and-sync.md). Values are the core's JSON, as on the web.
 */
@Database(entities = [GameRow::class, OutboxRow::class, KvRow::class], version = 1)
abstract class OurilDatabase : RoomDatabase() {
    abstract fun dao(): OurilDao

    companion object {
        /** The app's database: one per process, held by [OurilApplication]. */
        fun get(context: Context): OurilDatabase = (context.applicationContext as OurilApplication).database

        fun open(context: Context): OurilDatabase =
            Room.databaseBuilder(context, OurilDatabase::class.java, "ouril.db").build()
    }
}

/** A finished game. */
@Entity(tableName = "games", indices = [Index("owner", "ended_at")])
data class GameRow(
    @PrimaryKey val id: String,
    /** The guest ID (`guest:<uuid>`) or, once signed in, the user ID. */
    val owner: String,
    @ColumnInfo(name = "ended_at") val endedAt: String,
    /** The `GameRecord`, as JSON. */
    val record: String,
    /** Why the server rejected the upload (`illegal_game`, …). The game stays local. */
    @ColumnInfo(name = "sync_rejected") val syncRejected: String? = null,
)

/** A change waiting to be uploaded. Guests' changes wait until they sign in. */
@Entity(tableName = "outbox", indices = [Index("owner")])
data class OutboxRow(
    /** The mutation's ID, a UUIDv7: in ID order is in creation order. */
    @PrimaryKey val id: String,
    val owner: String,
    /** The mutation (`{ id, type, schema, payload, client_time }`), as JSON. */
    val mutation: String,
)

/** Settings, the guest ID, the game in progress and other single values, as JSON. */
@Entity(tableName = "kv")
data class KvRow(@PrimaryKey val key: String, val value: String)

@Dao
interface OurilDao {
    @Query("SELECT value FROM kv WHERE `key` = :key")
    suspend fun kv(key: String): String?

    @Query("SELECT value FROM kv WHERE `key` = :key")
    fun observeKv(key: String): Flow<String?>

    @Upsert
    suspend fun putKv(row: KvRow)

    @Query("DELETE FROM kv WHERE `key` = :key")
    suspend fun deleteKv(key: String)

    @Upsert
    suspend fun putGame(row: GameRow)

    @Insert
    suspend fun enqueue(row: OutboxRow)

    /** Newest first. */
    @Query("SELECT * FROM games WHERE owner = :owner ORDER BY ended_at DESC, id DESC")
    suspend fun games(owner: String): List<GameRow>

    /** Oldest first, the order to upload them in. */
    @Query("SELECT * FROM outbox WHERE owner = :owner ORDER BY id")
    suspend fun outbox(owner: String): List<OutboxRow>
}
