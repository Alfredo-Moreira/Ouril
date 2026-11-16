package org.moreiralabs.ouril.ui.board3d

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.double
import kotlinx.serialization.json.int
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.moreiralabs.ouril.core.CoreJson
import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.core.MoveEvent
import org.moreiralabs.ouril.game.AI_PAUSE_MS
import org.moreiralabs.ouril.game.END_TONES
import org.moreiralabs.ouril.game.FRAME_BASE_MS
import org.moreiralabs.ouril.game.FRAME_TONES
import org.moreiralabs.ouril.game.FrameKind
import org.moreiralabs.ouril.game.PACE_3D
import org.moreiralabs.ouril.game.REDUCED_MOTION_SPEED
import org.moreiralabs.ouril.game.Tone
import org.moreiralabs.ouril.game.buildFrames
import org.moreiralabs.ouril.game.frameDelay
import java.io.File
import kotlin.math.abs

/**
 * The motion vectors (`shared/motion/vectors`, docs/architecture/board-motion.md): recordings of
 * the web's 3D board, the reference for how a move looks. Android must play every one the same:
 * the same frames at the same pace, and on a 60 Hz clock the same hand, seeds, camera and rings.
 * Also the same timings, tones and seed looks.
 */
class MotionVectorsTest {
    private val dir = File(System.getProperty("ouril.repo"), "shared/motion/vectors")

    private fun read(name: String): JsonObject = CoreJson.parseToJsonElement(File(dir, name).readText()).jsonObject

    @Test
    fun timingsAndTonesMatchTheWeb() {
        val v = read("timings.json")
        val base = v.getValue("frameBaseMs").jsonObject
        assertEquals(FrameKind.entries.size, base.size)
        for (kind in FrameKind.entries) {
            assertEquals("frame base of $kind", base.getValue(kind.name.lowercase()).jsonPrimitive.int, FRAME_BASE_MS.getValue(kind))
        }
        assertEquals(v.getValue("pace3d").jsonPrimitive.double, PACE_3D, 0.0)
        assertEquals(v.getValue("reducedMotionSpeed").jsonPrimitive.double, REDUCED_MOTION_SPEED, 0.0)
        assertEquals(v.getValue("aiPauseMs").jsonPrimitive.int, AI_PAUSE_MS)

        val tones = v.getValue("frameTones").jsonObject
        assertEquals(tones.keys, FRAME_TONES.keys.map { it.name.lowercase() }.toSet())
        for ((kind, tone) in FRAME_TONES) assertTone("$kind", tones.getValue(kind.name.lowercase()).jsonObject, tone)
        val ends = v.getValue("endTones").jsonObject
        assertTone("win", ends.getValue("win").jsonObject, END_TONES.getValue(true))
        assertTone("loss", ends.getValue("loss").jsonObject, END_TONES.getValue(false))
    }

    private fun assertTone(name: String, web: JsonObject, tone: Tone) {
        assertEquals("$name freq", web.getValue("freq").jsonPrimitive.double, tone.freq, 0.0)
        assertEquals("$name jitter", web.getValue("jitter").jsonPrimitive.double, tone.jitter, 0.0)
        assertEquals("$name ms", web.getValue("ms").jsonPrimitive.int, tone.ms)
        assertEquals("$name wave", web.getValue("wave").jsonPrimitive.content, tone.wave.name.lowercase())
        assertEquals("$name gain", web.getValue("gain").jsonPrimitive.double, tone.gain, 0.0)
    }

    @Test
    fun seedsLookTheSame() {
        val v = read("seed-looks.json")
        val total = v.getValue("total").jsonPrimitive.int
        val orientations = seedOrientations(total)
        val colors = seedColors(total)
        v.getValue("orientations").jsonArray.forEachIndexed { i, q ->
            val o = orientations[i]
            assertNear("orientation $i", q.numbers(), listOf(o.x, o.y, o.z, o.w))
        }
        v.getValue("colors").jsonArray.forEachIndexed { i, c ->
            assertNear("colour $i", c.numbers(), listOf(colors[i * 3], colors[i * 3 + 1], colors[i * 3 + 2]))
        }
    }

    @Test
    fun everyVectorPlaysTheSame() {
        val files = dir.listFiles { f -> f.extension == "json" && f.name !in setOf("timings.json", "seed-looks.json") }
            .orEmpty().sortedBy { it.name }
        assertTrue("no motion vectors in $dir", files.size >= 7)
        val failures = mutableListOf<String>()
        for (file in files) {
            val v = CoreJson.parseToJsonElement(file.readText()).jsonObject
            val vector = Vector(v)
            try {
                checkFrames(vector)
                checkTotals(vector)
                checkPlay(vector)
            } catch (e: AssertionError) {
                failures += "${file.name}: ${e.message}"
            }
        }
        assertTrue(failures.joinToString("\n"), failures.isEmpty())
    }

    // --- The vectors ----------------------------------------------------------------------------

    private class Input(o: JsonObject) {
        val t = o.getValue("t").jsonPrimitive.double
        val move = o["move"]?.jsonPrimitive?.intOrNull
        val frame = o["frame"]?.jsonPrimitive?.intOrNull
        val pits = o.getValue("pits").ints()
        val stores = o.getValue("stores").ints()
        val highlight = o["highlight"]?.takeIf { it !is JsonNull }?.jsonObject?.let {
            Highlight(it["pit"]?.jsonPrimitive?.intOrNull, kindOf(it.getValue("kind").jsonPrimitive.content))
        }
        val total = o.getValue("total").jsonPrimitive.int
        val legal = o.getValue("legal").ints()
        val hintPit = o["hintPit"]?.jsonPrimitive?.intOrNull
        val finished = o.getValue("finished").jsonPrimitive.boolean
    }

    private class Move(o: JsonObject) {
        val before: GameState = CoreJson.decodeFromJsonElement(GameState.serializer(), o.getValue("before"))
        val pit = o.getValue("pit").jsonPrimitive.int
        val events = o.getValue("events").jsonArray.map { MoveEvent.from(it.jsonObject) }
        val after: GameState = CoreJson.decodeFromJsonElement(GameState.serializer(), o.getValue("after"))
        val at = o.getValue("at").jsonPrimitive.double
        val byAi = o.getValue("by").jsonPrimitive.content == "ai"
    }

    private class Vector(o: JsonObject) {
        val aspect = o.getValue("aspect").jsonPrimitive.double
        val frameSpeed = o.getValue("frameSpeed").jsonPrimitive.double
        val boardSpeed = o.getValue("boardSpeed").jsonPrimitive.double
        val tick = o.getValue("tick").jsonPrimitive.double
        val every = o.getValue("every").jsonPrimitive.int
        val until = o.getValue("until").jsonPrimitive.double
        val moves = o.getValue("moves").jsonArray.map { Move(it.jsonObject) }
        val inputs = o.getValue("inputs").jsonArray.map { Input(it.jsonObject) }
        val samples = o.getValue("samples").jsonArray.map { it.jsonObject }
        val final = o.getValue("final").jsonObject
    }

    /** Each move's frames, kinds, counts and times, at the game's pace (and the computer's pause). */
    private fun checkFrames(v: Vector) {
        var end = 0.0
        v.moves.forEachIndexed { i, m ->
            if (m.byAi && i > 0) assertNear("move $i: the computer's pause", m.at - end, AI_PAUSE_MS * v.frameSpeed / 1000, 1e-4)
            val frames = buildFrames(m.before, m.pit, m.events, m.after)
            val shown = v.inputs.filter { it.move == i }
            assertEquals("move $i: frames", frames.indices.toList(), shown.map { it.frame })
            var t = m.at
            frames.forEachIndexed { j, f ->
                val x = shown[j]
                assertNear("move $i frame $j: time", x.t, t, 1e-4)
                assertEquals("move $i frame $j: pits", f.pits, x.pits)
                assertEquals("move $i frame $j: stores", f.stores, x.stores)
                assertEquals("move $i frame $j: highlight", if (f.kind != FrameKind.END) Highlight(f.pit, f.kind) else null, x.highlight)
                t += frameDelay(f.kind, v.frameSpeed) / 1000
            }
            end = t
        }
    }

    /** The board keeps the same seed total as the web's scene while a move plays. */
    private fun checkTotals(v: Vector) {
        val total = SeedTotal()
        v.inputs.forEachIndexed { i, x -> assertEquals("input $i: total", x.total, total.of(x.pits, x.stores, x.highlight != null)) }
    }

    /** The hand, the moving seeds, the camera and the rings, sample by sample, then every seed's rest. */
    private fun checkPlay(v: Vector) {
        val layout = BoardLayout.forPits(v.inputs.first().pits.size)
        val choreo = Choreography()
        val rig = CameraRig()
        val rings = createRings(layout.n * 2)
        val camera = Vec3()
        var current: Input? = null
        var next = 0
        var sample = 0
        var tick = 0
        while (tick * v.tick <= v.until + 1e-9) {
            val now = tick * v.tick
            while (next < v.inputs.size && v.inputs[next].t <= now + 1e-9) {
                val x = v.inputs[next++]
                current = x
                applyFrame(choreo, layout, x.total, BoardFrame(x.pits, x.stores, x.highlight, v.boardSpeed), x.t)
                rig.setInputs(CameraInputs(x.finished, x.highlight?.kind, v.boardSpeed), x.t)
            }
            val dt = if (tick == 0) 0.0 else v.tick
            stepChoreography(choreo, now, dt)
            rig.step(now, dt, v.aspect, camera)
            current?.let { stepRings(rings, ringTargets(layout.n, it.legal, it.highlight, it.hintPit), dt, v.boardSpeed) }
            if (tick % v.every == 0) {
                val s = v.samples[sample++]
                val at = "t=${s.getValue("t").jsonPrimitive.double}"
                val h = choreo.hand
                val hand = s.getValue("hand").numbers()
                assertNear("$at hand", hand.take(4), listOf(h.pos.x, h.pos.y, h.pos.z, h.grip))
                assertEquals("$at hand shown", hand[4].toInt(), if (now < h.hideAt) 1 else 0)
                assertEquals("$at hand side", hand[5].toInt(), h.side)
                assertNear("$at camera", s.getValue("camera").numbers(), listOf(camera.x, camera.y, camera.z))

                val seeds = s.getValue("seeds").jsonArray.map { it.numbers() }
                val moving = choreo.motions.withIndex().filter { it.value.mode != SeedMode.REST }.map { it.index }
                assertEquals("$at moving seeds", seeds.map { it[0].toInt() }, moving)
                for (seed in seeds) {
                    val m = choreo.motions[seed[0].toInt()]
                    assertNear("$at seed ${seed[0].toInt()}", seed.drop(1), listOf(m.pos.x, m.pos.y, m.pos.z, m.tumble))
                }

                val lit = s.getValue("rings").jsonArray.map { it.numbers() }.associateBy { it[0].toInt() }
                rings.forEachIndexed { i, ring ->
                    val web = lit[i]
                    if (web != null) {
                        assertNear("$at ring $i", web.drop(1), listOf(ring.opacity) + ring.color.toList())
                    } else {
                        assertTrue("$at ring $i glows (${ring.opacity}) but not on the web", ring.opacity <= 1e-3 + TOL)
                    }
                }
            }
            tick++
        }
        assertEquals("samples", v.samples.size, sample)

        val placement = v.final.getValue("placement").jsonArray.map { it.ints() }
        assertEquals("final placement", placement, choreo.placement.orEmpty().map { listOf(it.container, it.slot) })
        v.final.getValue("positions").jsonArray.forEachIndexed { i, p ->
            val m = choreo.motions[i]
            assertNear("final position of seed $i", p.numbers(), listOf(m.pos.x, m.pos.y, m.pos.z))
        }
    }

    private companion object {
        /** The vectors keep five decimals. */
        const val TOL = 2e-5

        fun kindOf(name: String) = FrameKind.valueOf(name.uppercase())

        fun JsonElement.ints(): List<Int> = jsonArray.map { it.jsonPrimitive.int }

        fun JsonElement.numbers(): List<Double> = (this as JsonArray).map { it.jsonPrimitive.double }

        fun assertNear(what: String, expected: List<Double>, actual: List<Double>, tol: Double = TOL) {
            assertEquals("$what: length", expected.size, actual.size)
            expected.indices.forEach { assertNear("$what[$it]", expected[it], actual[it], tol) }
        }

        fun assertNear(what: String, expected: Double, actual: Double, tol: Double = TOL) {
            if (abs(expected - actual) > tol) throw AssertionError("$what: expected $expected, got $actual")
        }
    }
}
