package org.moreiralabs.ouril.game

import org.moreiralabs.ouril.core.GameState
import org.moreiralabs.ouril.core.Player
import org.moreiralabs.ouril.core.Status

/**
 * The tutorial and the rules text, as on the web (`tutorial/steps.ts`, `content/rules.ts`): the
 * same positions from docs/game/rules.md and the same string keys in shared/i18n. The core
 * decides every outcome; these only set up each lesson. `LessonsTest` checks every position.
 */
data class TutorialStep(
    /** `tutorial.<id>.title|intro|done` (and `.next` between moves). */
    val id: String,
    /** The rules it's played with. */
    val variant: String = "cv.standard",
    val position: GameState,
    /** One entry per move: the pits the lesson allows, or null for any legal pit. */
    val moves: List<List<Int>?>,
)

private fun pos(south: List<Int>, north: List<Int>, stores: List<Int>) =
    GameState(south + north, stores, Player.SOUTH, 0, Status.PLAYING)

val TUTORIAL_STEPS = listOf(
    // Example 1: simple sowing.
    TutorialStep("sowing", position = pos(listOf(4, 4, 4, 4, 4, 4), listOf(4, 4, 4, 4, 4, 4), listOf(0, 0)), moves = listOf(listOf(2))),
    // A single capture: the last seed makes 2 in North's pit 6.
    TutorialStep("capture", position = pos(listOf(4, 4, 4, 4, 2, 3), listOf(1, 4, 4, 4, 4, 4), listOf(3, 3)), moves = listOf(listOf(4))),
    // Example 2: capture with a chain.
    TutorialStep("chain", position = pos(listOf(4, 4, 4, 4, 3, 0), listOf(1, 2, 4, 4, 4, 4), listOf(5, 5)), moves = listOf(listOf(4))),
    // Example 3: a lap of 12+ seeds skips the origin pit.
    TutorialStep("lap", position = pos(listOf(12, 0, 0, 0, 0, 0), listOf(9, 9, 9, 9, 0, 0), listOf(0, 0)), moves = listOf(listOf(0))),
    // Example 4: the single-seed rule.
    TutorialStep("single_seed", position = pos(listOf(1, 0, 3, 1, 0, 2), listOf(4, 4, 4, 4, 4, 4), listOf(9, 8)), moves = listOf(null)),
    // Example 6: must feed a starving opponent.
    TutorialStep("feeding", position = pos(listOf(0, 2, 0, 0, 0, 1), listOf(0, 0, 0, 0, 0, 0), listOf(21, 24)), moves = listOf(null)),
    // Example 5: grand slam, then the extra move must feed.
    TutorialStep("grand_slam", position = pos(listOf(0, 0, 0, 1, 3, 0), listOf(1, 2, 0, 0, 0, 0), listOf(18, 23)), moves = listOf(listOf(4), null)),
    // Continuous sowing: the opening move carries on from an occupied own pit.
    TutorialStep("continuous", "cv.continuous", pos(listOf(4, 4, 4, 4, 4, 4), listOf(4, 4, 4, 4, 4, 4), listOf(0, 0)), listOf(listOf(0))),
    // Continuous sowing: a lone seed starts a relay that ends with a capture.
    TutorialStep("continuous_capture", "cv.continuous", pos(listOf(2, 3, 4, 1, 1, 3), listOf(1, 4, 4, 4, 4, 4), listOf(6, 7)), listOf(listOf(3))),
    // Capture across: three seeds land in empty pits 2-4 and capture across, with the chain.
    TutorialStep("across", "cv.across", pos(listOf(3, 0, 0, 0, 4, 4), listOf(4, 4, 4, 4, 4, 4), listOf(6, 7)), listOf(listOf(0))),
)

/** A rules section: `rules.<id>.title` and `rules.<id>.1..paragraphs`. */
data class RulesSection(val id: String, val paragraphs: Int)

data class VariantRules(val intro: String, val basedOn: String? = null, val sections: List<RulesSection>)

val VARIANT_RULES = mapOf(
    "cv.standard" to VariantRules(
        intro = "rules.intro",
        sections = listOf(
            RulesSection("setup", 3), RulesSection("sowing", 3), RulesSection("single_seed", 1),
            RulesSection("lap", 1), RulesSection("capture", 3), RulesSection("feeding", 3),
            RulesSection("grand_slam", 4), RulesSection("end", 5),
        ),
    ),
    "cv.continuous" to VariantRules(
        intro = "rules.continuous.intro",
        basedOn = "cv.standard",
        sections = listOf(RulesSection("continuous_relay", 2), RulesSection("continuous_stop", 1), RulesSection("continuous_other", 3)),
    ),
    "cv.across" to VariantRules(
        intro = "rules.across.intro",
        basedOn = "cv.standard",
        sections = listOf(RulesSection("across_capture", 3), RulesSection("across_strategy", 1)),
    ),
    "cv.across-continuous" to VariantRules(
        intro = "rules.across_continuous.intro",
        basedOn = "cv.continuous",
        sections = listOf(RulesSection("across_capture", 3), RulesSection("across_continuous_end", 1), RulesSection("across_strategy", 1)),
    ),
)
