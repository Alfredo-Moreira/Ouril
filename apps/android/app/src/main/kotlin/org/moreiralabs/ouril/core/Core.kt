package org.moreiralabs.ouril.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import uniffi.ouril_ffi.CoreException

/**
 * The Rust core (rules, AI, stats) for the app: the generated UniFFI bindings
 * (`uniffi.ouril_ffi`, from `just bindings-android`) behind typed Kotlin calls. Every rule
 * question goes here; the app never decides a move's outcome itself.
 *
 * A failed call throws [CoreError] whose [CoreError.code] is the web's machine-readable code
 * (`not_own_pit`, `unknown_variant`, `invalid_argument: …`, `illegal_move:<ply>:<code>`).
 */
object Core {
    val version: String get() = uniffi.ouril_ffi.coreVersion()

    /** Every known variant, every version. */
    val variants: List<VariantConfig> by lazy {
        CoreJson.decodeFromString<List<VariantConfig>>(uniffi.ouril_ffi.variantsJson())
    }

    /** The latest default variant for a country code (`"cv"`). */
    fun defaultVariant(country: String): VariantConfig? =
        uniffi.ouril_ffi.defaultVariantJson(country)?.let { CoreJson.decodeFromString<VariantConfig>(it) }

    fun newGame(variant: VariantRef, first: Player): GameState = call {
        CoreJson.decodeFromString(uniffi.ouril_ffi.newGameJson(variant.id, variant.version, first.wire))
    }

    fun legalMoves(variant: VariantRef, state: GameState): List<Int> = call {
        uniffi.ouril_ffi.legalMoves(variant.id, variant.version, encode(state))
    }

    fun applyMove(variant: VariantRef, state: GameState, pit: Int): MoveResult = call {
        moveResult(
            CoreJson.parseToJsonElement(
                uniffi.ouril_ffi.applyMoveJson(variant.id, variant.version, encode(state), pit),
            ).jsonObject,
        )
    }

    /** The computer's move, or null when there is none. Hard can take a while: not on main. */
    fun aiMove(variant: VariantRef, state: GameState, level: Level, seed: Long): Int? = call {
        uniffi.ouril_ffi.aiMove(variant.id, variant.version, encode(state), level.wire, seed)
    }

    fun replay(variant: VariantRef, first: Player, moves: List<Int>): Replay = call {
        val o = CoreJson.parseToJsonElement(
            uniffi.ouril_ffi.replayJson(variant.id, variant.version, first.wire, moves),
        ).jsonObject
        Replay(
            initial = CoreJson.decodeFromJsonElement(GameState.serializer(), o.getValue("initial")),
            steps = o.getValue("steps").jsonArray.map { moveResult(it.jsonObject) },
        )
    }

    private fun encode(state: GameState): String = CoreJson.encodeToString(GameState.serializer(), state)

    private fun moveResult(o: JsonObject) = MoveResult(
        state = CoreJson.decodeFromJsonElement(GameState.serializer(), o.getValue("state")),
        events = o.getValue("events").jsonArray.map { MoveEvent.from(it.jsonObject) },
    )

    private inline fun <T> call(block: () -> T): T =
        try {
            block()
        } catch (e: CoreException) {
            throw CoreError(e.message ?: "core_error", e)
        }
}

/** A failed core call; [code] is machine-readable (see [Core]). */
class CoreError(val code: String, cause: Throwable? = null) : Exception(code, cause)
