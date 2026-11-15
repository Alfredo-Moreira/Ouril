package org.moreiralabs.ouril.game

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.core.MoveEvent

class LessonsTest {
    /** What each lesson's first move must show, from the engine's own events. */
    private val teaches: Map<String, (List<MoveEvent>) -> Boolean> = mapOf(
        "sowing" to { e -> e.all { it is MoveEvent.Sow } },
        "capture" to { e -> e.count { it is MoveEvent.Capture } == 1 },
        "chain" to { e -> e.count { it is MoveEvent.Capture } >= 2 },
        "lap" to { e -> e.any { it is MoveEvent.SkipOrigin } },
        "grand_slam" to { e -> e.any { it is MoveEvent.GrandSlam } && e.any { it is MoveEvent.ExtraTurn } },
        "continuous" to { e -> e.any { it is MoveEvent.Relay } },
        "continuous_capture" to { e -> e.any { it is MoveEvent.Relay } && e.any { it is MoveEvent.Capture } },
        "across" to { e -> e.any { it is MoveEvent.Capture } },
    )

    /**
     * Every lesson plays through on its own rules: each move it allows is legal, there's always
     * one to play, it's always your turn, and the first move shows what the lesson teaches.
     */
    @Test
    fun everyLessonPlaysThrough() {
        assertEquals(TUTORIAL_STEPS.size, TUTORIAL_STEPS.map { it.id }.toSet().size)
        for (step in TUTORIAL_STEPS) {
            val variant = variantRef(step.variant)
            assertEquals(step.id, step.variant, variant.id)
            var state = step.position
            for ((i, allowed) in step.moves.withIndex()) {
                val where = "${step.id}, move ${i + 1}"
                assertEquals(where, HUMAN, state.toMove)
                val legal = Core.legalMoves(variant, state)
                assertTrue("$where: the lesson allows $allowed, the rules $legal", allowed.orEmpty().all { it in legal })
                val pit = (allowed ?: legal).firstOrNull() ?: error("$where: nothing to play")
                val result = Core.applyMove(variant, state, pit)
                if (i == 0) teaches[step.id]?.let { assertTrue("$where: ${result.events}", it(result.events)) }
                state = result.state
            }
        }
    }

    @Test
    fun everyPlayableVariantHasItsRules() {
        assertEquals(PLAYABLE_VARIANTS.toSet(), VARIANT_RULES.keys)
        for ((id, rules) in VARIANT_RULES) {
            rules.basedOn?.let { assertTrue("$id is based on $it", it in PLAYABLE_VARIANTS) }
            assertTrue(id, rules.sections.isNotEmpty() && rules.sections.all { it.paragraphs > 0 })
        }
    }
}
