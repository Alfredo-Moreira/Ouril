package org.moreiralabs.ouril.ui

import android.os.Looper
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.performClick
import com.github.takahirom.roborazzi.ExperimentalRoborazziApi
import com.github.takahirom.roborazzi.captureScreenRoboImage
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.moreiralabs.ouril.MainActivity
import org.moreiralabs.ouril.R
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import java.time.Duration

private const val PHONE = "w412dp-h915dp-xxhdpi"
private const val PHONE_NIGHT = "w412dp-h915dp-night-xxhdpi"

/**
 * The app on a phone, as a player goes through it: every tab, About, then a game with a move,
 * the computer's reply, the menu and a forfeit. It runs with the unit tests; with
 * `just android-screenshots` it also saves a picture of each step (Roborazzi).
 */
abstract class AppTour {
    @get:Rule
    val compose = createAndroidComposeRule<MainActivity>()

    private fun text(id: Int) = compose.activity.getString(id)

    @OptIn(ExperimentalRoborazziApi::class)
    protected fun shot(name: String) {
        settle()
        captureScreenRoboImage("build/outputs/roborazzi/$name.png")
    }

    /** Lets the game's coroutines (timers on the main looper, the core off it) move on. */
    private fun settle(ms: Long = 400) {
        repeat((ms / 100).toInt()) {
            shadowOf(Looper.getMainLooper()).idleFor(Duration.ofMillis(100))
            compose.waitForIdle()
            Thread.sleep(5)
        }
    }

    private fun waitFor(what: String, matcher: SemanticsMatcher): SemanticsNodeInteraction {
        val end = System.currentTimeMillis() + 30_000
        while (compose.onAllNodes(matcher).fetchSemanticsNodes().isEmpty()) {
            check(System.currentTimeMillis() < end) { "timed out waiting for $what" }
            settle(100)
        }
        return compose.onAllNodes(matcher)[0]
    }

    private fun tab(label: Int) =
        waitFor("tab", hasText(text(label)) and SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab))

    private fun button(label: Int) = waitFor(text(label), hasText(text(label)) and hasClickAction())

    /** A pit of yours you can play now: it's your turn and the board takes taps. */
    private val playablePit by lazy {
        hasContentDescription(text(R.string.board_pit_yours).substringBefore(" {"), substring = true) and hasClickAction()
    }

    protected fun tourTabs(prefix: String) {
        shot("${prefix}home")
        tab(R.string.menu_tutorial).performClick()
        shot("${prefix}tutorial")
        tab(R.string.menu_rules).performClick()
        shot("${prefix}rules")
        tab(R.string.menu_settings).performClick()
        shot("${prefix}settings")
        button(R.string.menu_about).performClick()
        shot("${prefix}about")
        tab(R.string.menu_game).performClick()
    }

    protected fun playAndForfeit(prefix: String) {
        button(R.string.home_cta_play).performClick()
        waitFor("your turn", playablePit).performClick()
        settle(1_200)
        shot("${prefix}game_sowing")
        waitFor("your next turn", playablePit)
        shot("${prefix}game")
        waitFor("the menu", hasContentDescription(text(R.string.game_menu_open)) and hasClickAction()).performClick()
        shot("${prefix}game_menu")
        button(R.string.game_menu_forfeit).performClick()
        shot("${prefix}forfeit")
        button(R.string.game_forfeit_confirm).performClick()
        waitFor("the result", hasText(text(R.string.game_over_loss)))
        shot("${prefix}result")
    }
}

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = PHONE)
class ScreensTest : AppTour() {
    @Test
    fun aTourOfTheApp() {
        tourTabs("")
        playAndForfeit("")
    }
}

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = PHONE_NIGHT)
class NightScreensTest : AppTour() {
    @Test
    fun aTourOfTheAppAtNight() {
        tourTabs("night_")
        playAndForfeit("night_")
    }
}

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = "w915dp-h412dp-land-xxhdpi")
class LandscapeScreensTest : AppTour() {
    @Test
    fun aGameOnAPhoneHeldSideways() {
        shot("land_home")
        playAndForfeit("land_")
    }
}
