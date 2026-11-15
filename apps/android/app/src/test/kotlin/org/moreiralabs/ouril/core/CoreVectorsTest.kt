package org.moreiralabs.ouril.core

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Every rule test vector (`core/test-vectors`, docs/architecture/test-vectors.md) through the
 * Kotlin bindings and [Core], exactly as the app calls the core. A vector that can't run (an
 * unknown variant version, a bad setup) fails: nothing is skipped. Seeds are conserved in
 * every state.
 */
class CoreVectorsTest {
    private val root = File(System.getProperty("ouril.repo"), "core/test-vectors")

    @Test
    fun allTestVectorsPass() {
        val files = root.walkTopDown()
            .onEnter { it.name != "schema" }
            .filter { it.isFile && it.extension == "json" }
            .sorted()
            .toList()
        assertTrue("no test vectors under $root", files.isNotEmpty())

        val failures = mutableListOf<String>()
        var runs = 0
        for (file in files) {
            val rel = file.relativeTo(root).path.removeSuffix(".json").replace('\\', '/')
            val vector = CoreJson.parseToJsonElement(file.readText()).jsonObject
            if (vector["id"]?.jsonPrimitive?.content != rel) {
                failures += "$rel: id ${vector["id"]} must equal the file path"
                continue
            }
            for (ref in vector.getValue("variants").jsonArray) {
                val id = ref.jsonObject.getValue("id").jsonPrimitive.content
                val version = ref.jsonObject.getValue("version").jsonPrimitive.int
                val config = Core.variants.find { it.id == id && it.version == version }
                if (config == null) {
                    failures += "$rel: unknown variant $id@$version (not skipped: failing)"
                    continue
                }
                runs++
                try {
                    run(VariantRef(id, version), config, vector)
                } catch (e: Exception) {
                    failures += "$rel [$id@$version]: ${e.message}"
                }
            }
        }
        println("ran $runs vector runs from ${files.size} files")
        assertTrue(
            "${failures.size} of ${files.size} vector files failed:\n${failures.joinToString("\n")}",
            failures.isEmpty(),
        )
    }

    @Test
    fun errorsCarryTheWebCodes() {
        val v = VariantRef("cv.standard", 1)
        val s = Core.newGame(v, Player.SOUTH)
        assertEquals("not_own_pit", runCatching { Core.applyMove(v, s, 7) }.exceptionOrNull().code())
        assertEquals("not_own_pit", runCatching { Core.applyMove(v, s, -1) }.exceptionOrNull().code())
        assertEquals("unknown_variant", runCatching { Core.legalMoves(VariantRef("zz.nope", 1), s) }.exceptionOrNull().code())
        val replay = runCatching { Core.replay(v, Player.SOUTH, listOf(7)) }.exceptionOrNull().code()
        assertTrue(replay, replay.startsWith("illegal_move:0:"))
    }

    @Test
    fun theCapeVerdeDefaultIsStandard() {
        val cv = Core.defaultVariant("cv")
        assertEquals("cv.standard", cv?.id)
        assertEquals(null, Core.defaultVariant("zz"))
        assertTrue(Core.version.isNotBlank())
    }

    private fun Throwable?.code(): String = (this as? CoreError)?.code ?: "no CoreError: $this"

    private fun run(variant: VariantRef, config: VariantConfig, vector: JsonObject) {
        var state = setup(variant, config, vector.getValue("setup"))
        for ((i, step) in vector.getValue("steps").jsonArray.map { it.jsonObject }.withIndex()) {
            val ctx = "step $i"
            step["expect_legal_moves"]?.let { e ->
                val expected = e.jsonArray.map { it.jsonPrimitive.int }
                val got = Core.legalMoves(variant, state)
                check(got == expected) { "$ctx: legal moves $got, expected $expected" }
            }
            val move = step["move"]?.jsonPrimitive?.int ?: continue
            val expectError = step["expect_error"]?.jsonPrimitive?.content
            if (expectError != null) {
                val error = runCatching { Core.applyMove(variant, state, move) }.exceptionOrNull()
                check(error is CoreError && error.code == expectError) {
                    "$ctx: move $move gave ${error ?: "no error"}, expected error $expectError"
                }
                continue // The state is unchanged.
            }
            val result = Core.applyMove(variant, state, move)
            conserve(config, result.state, ctx)
            check(result.events.none { it is MoveEvent.Unknown }) { "$ctx: an event the app doesn't know: ${result.events}" }
            step["expect"]?.jsonObject?.let { expect(it, result, ctx) }
            state = result.state
        }
    }

    private fun expect(e: JsonObject, r: MoveResult, ctx: String) {
        e["pits"]?.let { check(r.state.pits == it.ints()) { "$ctx: pits ${r.state.pits}, expected $it" } }
        e["stores"]?.let { check(r.state.stores == it.ints()) { "$ctx: stores ${r.state.stores}, expected $it" } }
        e["to_move"]?.let {
            check(r.state.toMove.wire == it.jsonPrimitive.content) { "$ctx: to_move ${r.state.toMove}, expected $it" }
        }
        e["status"]?.let {
            val got = CoreJson.encodeToJsonElement(Status.serializer(), r.state.status).jsonPrimitive.content
            check(got == it.jsonPrimitive.content) { "$ctx: status $got, expected $it" }
        }
        e["events"]?.let { events ->
            val expected = events.jsonArray.map { MoveEvent.from(it.jsonObject) }
            check(r.events == expected) { "$ctx: events\n  got:      ${r.events}\n  expected: $expected" }
        }
    }

    /** `"initial"`, `{ "initial": true, "first_player": … }`, or an explicit position. */
    private fun setup(variant: VariantRef, config: VariantConfig, setup: JsonElement): GameState {
        if (setup is JsonPrimitive && setup.isString && setup.content == "initial") {
            return Core.newGame(variant, Player.SOUTH)
        }
        val o = setup.jsonObject
        if (o["initial"]?.jsonPrimitive?.booleanOrNull == true) {
            val first = o["first_player"]?.let { CoreJson.decodeFromJsonElement(Player.serializer(), it) }
            return Core.newGame(variant, first ?: Player.SOUTH)
        }
        // As the engine reads a setup: no moves since a capture and still playing unless given.
        val defaults = mapOf("moves_since_capture" to JsonPrimitive(0), "status" to JsonPrimitive("playing"))
        val state = CoreJson.decodeFromJsonElement(GameState.serializer(), JsonObject(defaults + o))
        check(state.pits.size == 2 * config.pitsPerSide) {
            "setup has ${state.pits.size} pits, ${config.id} has ${2 * config.pitsPerSide}"
        }
        conserve(config, state, "setup")
        return state
    }

    private fun conserve(config: VariantConfig, s: GameState, ctx: String) {
        val total = 2 * config.pitsPerSide * config.seedsPerPit
        val sum = s.pits.sum() + s.stores.sum()
        check(sum == total) { "$ctx: seeds not conserved: $sum != $total" }
    }

    private fun JsonElement.ints() = jsonArray.map { it.jsonPrimitive.int }
}
