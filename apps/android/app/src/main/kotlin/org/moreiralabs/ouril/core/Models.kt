package org.moreiralabs.ouril.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonPrimitive

/**
 * The core's values, with exactly the shapes of the web API and the test vectors
 * (core/wasm/API.md, core/ffi/API.md): JSON with snake_case keys and snake_case enums.
 */
val CoreJson = Json {
    ignoreUnknownKeys = true
    // Saved records carry every field (`mode` too), as the web writes them; nulls are left out.
    encodeDefaults = true
    explicitNulls = false
}

@Serializable
enum class Player {
    @SerialName("south") SOUTH,
    @SerialName("north") NORTH;

    val opponent: Player get() = if (this == SOUTH) NORTH else SOUTH
    val wire: String get() = if (this == SOUTH) "south" else "north"
    /** Index into `GameState.stores`. */
    val store: Int get() = if (this == SOUTH) 0 else 1
}

@Serializable
enum class Level {
    @SerialName("easy") EASY,
    @SerialName("medium") MEDIUM,
    @SerialName("hard") HARD;

    val wire: String get() = name.lowercase()
}

@Serializable
enum class Status {
    @SerialName("playing") PLAYING,
    @SerialName("south_wins") SOUTH_WINS,
    @SerialName("north_wins") NORTH_WINS,
    @SerialName("draw") DRAW,
}

/** `id@version` of a variant. */
@Serializable
data class VariantRef(val id: String, val version: Int)

/** The variant's resolved rules; only the keys the app reads (the rest are ignored). */
@Serializable
data class VariantConfig(
    val id: String,
    val version: Int,
    val name: String,
    val country: String,
    @SerialName("default_for_country") val defaultForCountry: Boolean,
    @SerialName("pits_per_side") val pitsPerSide: Int,
    @SerialName("seeds_per_pit") val seedsPerPit: Int,
)

/** A position. `pits` has 2 × pits per side entries: South `0..n-1`, then North. */
@Serializable
data class GameState(
    val pits: List<Int>,
    /** `[south, north]`. */
    val stores: List<Int>,
    @SerialName("to_move") val toMove: Player,
    @SerialName("moves_since_capture") val movesSinceCapture: Int,
    val status: Status,
    /** Opaque (position hashes for repetition): pass it back unchanged. */
    val history: List<String>? = null,
) {
    val isOver: Boolean get() = status != Status.PLAYING
}

/**
 * What a move did, in order (test-vectors.md, "Event types"). Unknown future event types are
 * kept as [Unknown], so an older app never fails on a newer core's events.
 */
sealed interface MoveEvent {
    data class Sow(val pit: Int) : MoveEvent
    data class SkipOrigin(val pit: Int) : MoveEvent
    data class Relay(val pit: Int) : MoveEvent
    data class Capture(val pit: Int, val seeds: Int) : MoveEvent
    data object GrandSlam : MoveEvent
    data object ExtraTurn : MoveEvent
    data class CollectRemaining(val player: Player, val seeds: Int) : MoveEvent
    data class GameOver(val result: String, val reason: String) : MoveEvent
    data class Unknown(val type: String) : MoveEvent

    companion object {
        fun from(o: JsonObject): MoveEvent {
            fun int(k: String) = o.getValue(k).jsonPrimitive.int
            fun str(k: String) = o.getValue(k).jsonPrimitive.content
            return when (val type = str("type")) {
                "sow" -> Sow(int("pit"))
                "skip_origin" -> SkipOrigin(int("pit"))
                "relay" -> Relay(int("pit"))
                "capture" -> Capture(int("pit"), int("seeds"))
                "grand_slam" -> GrandSlam
                "extra_turn" -> ExtraTurn
                "collect_remaining" ->
                    CollectRemaining(if (str("player") == "south") Player.SOUTH else Player.NORTH, int("seeds"))
                "game_over" -> GameOver(str("result"), str("reason"))
                else -> Unknown(type)
            }
        }
    }
}

data class MoveResult(val state: GameState, val events: List<MoveEvent>)

/** `replay`: the initial position and, for each move, its result. */
data class Replay(val initial: GameState, val steps: List<MoveResult>)
