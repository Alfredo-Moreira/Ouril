package org.moreiralabs.ouril.game

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.core.MoveResult
import org.moreiralabs.ouril.core.Player
import org.moreiralabs.ouril.core.Status
import org.moreiralabs.ouril.core.VariantRef
import org.moreiralabs.ouril.data.GameRecord
import org.moreiralabs.ouril.data.RecordResult
import java.security.SecureRandom
import java.time.Instant

/**
 * A game against the computer as plain data plus pure helpers, as on the web
 * (`game/session.ts`). Every rule question goes to the engine; this file only tracks moves.
 */

/** Who starts a new game. */
enum class Starter { ME, AI, RANDOM }

/** The Cape Verdean rules players can choose, default first. */
val PLAYABLE_VARIANTS = listOf("cv.standard", "cv.continuous", "cv.across", "cv.across-continuous")

/** The local player always sits South (the bottom row, or the left column upright). */
val HUMAN = Player.SOUTH

/** The choices for a new game (remembered for the home screen's Play now). */
@Serializable
data class NewGameRequest(
    val level: Level = Level.EASY,
    val starter: Starter = Starter.RANDOM,
    val variant: String = "cv.standard",
)

/** The game in progress, saved after every move so it resumes after the app closes. */
@Serializable
data class CurrentGame(
    val id: String,
    val variant: VariantRef,
    @SerialName("first_player") val firstPlayer: Player,
    @SerialName("human_player") val humanPlayer: Player = HUMAN,
    val level: Level,
    val moves: List<Int>,
    val state: GameState,
    @SerialName("ai_seed") val aiSeed: Long,
    @SerialName("started_at") val startedAt: String,
)

/** The latest version of a variant by id; the Cape Verde default when unknown. */
fun variantRef(id: String?): VariantRef {
    val latest = Core.variants.filter { it.id == id }.maxByOrNull { it.version }
    val v = latest ?: Core.defaultVariant("cv") ?: error("unknown_variant")
    return VariantRef(v.id, v.version)
}

private val random = SecureRandom()

fun startGame(request: NewGameRequest, now: Instant = Instant.now()): CurrentGame {
    val variant = variantRef(request.variant)
    val humanFirst = request.starter == Starter.ME || (request.starter == Starter.RANDOM && random.nextBoolean())
    val first = if (humanFirst) HUMAN else HUMAN.opponent
    return CurrentGame(
        id = uuidV7(now.toEpochMilli()),
        variant = variant,
        firstPlayer = first,
        level = request.level,
        moves = emptyList(),
        state = Core.newGame(variant, first),
        // JavaScript-safe (the web uses Number.MAX_SAFE_INTEGER): the same seeds work there.
        aiSeed = random.nextLong() and MAX_SAFE_SEED,
        startedAt = now.toString(),
    )
}

private const val MAX_SAFE_SEED = 9_007_199_254_740_991L

/** Apply `pit` for whoever is to move. Throws [org.moreiralabs.ouril.core.CoreError] if illegal. */
fun playMove(game: CurrentGame, pit: Int): Pair<CurrentGame, MoveResult> {
    val result = Core.applyMove(game.variant, game.state, pit)
    return game.copy(moves = game.moves + pit, state = result.state) to result
}

/** Deterministic AI seed for the next ply (reproducible games for bug reports). */
fun aiSeedFor(game: CurrentGame): Long = (game.aiSeed + game.moves.size) % MAX_SAFE_SEED

/** The player has made at least one move (the computer may have opened). */
fun undoPossible(game: CurrentGame): Boolean =
    if (game.firstPlayer == game.humanPlayer) game.moves.isNotEmpty() else game.moves.size >= 2

/**
 * Back to before the local player's last move (also removing the computer's replies after it),
 * rebuilt with the engine's `replay`; null when there is nothing to undo.
 */
fun undoLastHumanMove(game: CurrentGame): CurrentGame? {
    if (game.moves.isEmpty()) return null
    val rep = Core.replay(game.variant, game.firstPlayer, game.moves)
    fun moverAt(ply: Int): Player = (if (ply == 0) rep.initial else rep.steps[ply - 1].state).toMove
    var ply = game.moves.size - 1
    while (ply >= 0 && moverAt(ply) != game.humanPlayer) ply--
    if (ply < 0) return null
    val state = if (ply == 0) rep.initial else rep.steps[ply - 1].state
    return game.copy(moves = game.moves.take(ply), state = state)
}

/** How a finished game ended: from the last move's events, or by replaying it. */
fun endReason(game: CurrentGame, events: List<MoveEvent>?): String {
    val fromEvents = events?.filterIsInstance<MoveEvent.GameOver>()?.firstOrNull()?.reason
    if (fromEvents != null) return fromEvents
    val rep = Core.replay(game.variant, game.firstPlayer, game.moves)
    return rep.steps.lastOrNull()?.events?.filterIsInstance<MoveEvent.GameOver>()?.firstOrNull()?.reason
        ?: "no_moves"
}

/** The record of a game the rules ended (format 1), as the web saves it. */
fun finishedRecord(game: CurrentGame, reason: String, endedAt: Instant) = GameRecord(
    format = 1,
    id = game.id,
    variant = game.variant,
    firstPlayer = game.firstPlayer,
    moves = game.moves,
    coreVersion = Core.version,
    result = RecordResult(outcome = statusWire(game.state.status), stores = game.state.stores, reason = reason),
    startedAt = game.startedAt,
    endedAt = endedAt.toString(),
    aiLevel = game.level,
    humanPlayer = game.humanPlayer,
)

/** Record format 2 (ADR 0022): the opponent wins, stores as they are, moves still in play. */
fun forfeitRecord(game: CurrentGame, endedAt: Instant) = GameRecord(
    format = 2,
    id = game.id,
    variant = game.variant,
    firstPlayer = game.firstPlayer,
    moves = game.moves,
    coreVersion = Core.version,
    result = RecordResult(
        outcome = statusWire(if (game.humanPlayer == Player.SOUTH) Status.NORTH_WINS else Status.SOUTH_WINS),
        stores = game.state.stores,
        reason = "resigned",
    ),
    startedAt = game.startedAt,
    endedAt = endedAt.toString(),
    aiLevel = game.level,
    humanPlayer = game.humanPlayer,
)

private fun statusWire(status: Status) = when (status) {
    Status.SOUTH_WINS -> "south_wins"
    Status.NORTH_WINS -> "north_wins"
    Status.DRAW -> "draw"
    Status.PLAYING -> error("game_not_over")
}

enum class Outcome { WIN, LOSS, DRAW }

/** From the local player's point of view. */
fun outcomeFor(status: Status, me: Player): Outcome? = when (status) {
    Status.PLAYING -> null
    Status.DRAW -> Outcome.DRAW
    Status.SOUTH_WINS -> if (me == Player.SOUTH) Outcome.WIN else Outcome.LOSS
    Status.NORTH_WINS -> if (me == Player.NORTH) Outcome.WIN else Outcome.LOSS
}

/** A UUIDv7 (time-ordered), the id format of game records. */
fun uuidV7(unixMillis: Long): String {
    val bytes = ByteArray(16).also { random.nextBytes(it) }
    for (i in 0 until 6) bytes[i] = (unixMillis ushr (40 - 8 * i)).toByte()
    bytes[6] = ((bytes[6].toInt() and 0x0f) or 0x70).toByte()
    bytes[8] = ((bytes[8].toInt() and 0x3f) or 0x80).toByte()
    val hex = bytes.joinToString("") { "%02x".format(it) }
    return "${hex.substring(0, 8)}-${hex.substring(8, 12)}-${hex.substring(12, 16)}-" +
        "${hex.substring(16, 20)}-${hex.substring(20)}"
}
