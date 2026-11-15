package org.moreiralabs.ouril.i18n

import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertTrue
import org.junit.Test
import org.moreiralabs.ouril.core.CoreJson
import org.moreiralabs.ouril.game.PLAYABLE_VARIANTS
import org.moreiralabs.ouril.game.TUTORIAL_STEPS
import org.moreiralabs.ouril.game.VARIANT_RULES
import java.io.File

/**
 * Keys the app builds at run time (`trKey`) exist in shared/i18n/en.json. Keys written as
 * `R.string.…` are checked by the compiler already.
 */
class StringKeysTest {
    private val messages: Map<String, String> by lazy {
        val file = File(System.getProperty("ouril.repo"), "shared/i18n/en.json")
        CoreJson.parseToJsonElement(file.readText()).jsonObject
            .mapValues { it.value.jsonObject.getValue("message").jsonPrimitive.content }
    }
    private val keys get() = messages.keys

    private fun assertKeys(wanted: List<String>) {
        val missing = wanted.filter { it !in keys }
        assertTrue("missing from shared/i18n/en.json: $missing", missing.isEmpty())
    }

    @Test
    fun lessonsHaveTheirStrings() = assertKeys(
        TUTORIAL_STEPS.flatMap { step ->
            listOf("title", "intro", "done").map { "tutorial.${step.id}.$it" } +
                (if (step.moves.size > 1) listOf("tutorial.${step.id}.next") else emptyList())
        },
    )

    /** The phone's board stands upright: a lesson that says "bottom row" needs its upright wording. */
    @Test
    fun lessonsSayWhereYourPitsAreOnTheUprightBoard() = assertKeys(
        TUTORIAL_STEPS.map { "tutorial.${it.id}.intro" }
            .filter { messages.getValue(it).contains("bottom row") }
            .map { "${it}_upright" },
    )

    @Test
    fun rulesHaveTheirStrings() = assertKeys(
        VARIANT_RULES.values.flatMap { rules ->
            listOf(rules.intro) + rules.sections.flatMap { s ->
                listOf("rules.${s.id}.title") + (1..s.paragraphs).map { "rules.${s.id}.$it" }
            }
        },
    )

    @Test
    fun screensHaveTheirStrings() = assertKeys(
        PLAYABLE_VARIANTS.flatMap { listOf("variant.$it.short", "variant.$it.about") } +
            listOf("win", "loss", "draw").flatMap { listOf("game.over.$it", "game.over.tagline.$it") } +
            listOf("threshold", "no_feed", "endless_cycle", "no_moves", "resigned").map { "game.over.reason.$it" } +
            (1..4).flatMap { listOf("home.steps.$it.title", "home.steps.$it.body") } +
            listOf("offline", "levels", "guest").flatMap { listOf("home.features.$it.title", "home.features.$it.body") } +
            listOf("website", "email", "linkedin", "github", "instagram", "facebook", "twitch").map { "about.social.$it" },
    )
}
