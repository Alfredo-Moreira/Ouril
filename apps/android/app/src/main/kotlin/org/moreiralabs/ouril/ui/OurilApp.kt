package org.moreiralabs.ouril.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import kotlinx.serialization.Serializable
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.core.Level
import org.moreiralabs.ouril.game.NewGameRequest
import org.moreiralabs.ouril.game.Starter
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.ui.components.OurilIcons
import org.moreiralabs.ouril.ui.screens.AboutScreen
import org.moreiralabs.ouril.ui.screens.GameScreen
import org.moreiralabs.ouril.ui.screens.HomeScreen
import org.moreiralabs.ouril.ui.screens.RulesScreen
import org.moreiralabs.ouril.ui.screens.SettingsScreen
import org.moreiralabs.ouril.ui.screens.TutorialScreen
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

@Serializable object HomeRoute
@Serializable object TutorialRoute
@Serializable data class RulesRoute(val variant: String = "cv.standard")
@Serializable object SettingsRoute
@Serializable object AboutRoute

/** The game screen: a new game with these choices, or (all null) the saved game. */
@Serializable data class GameRoute(val level: String? = null, val starter: String? = null, val variant: String? = null) {
    val request: NewGameRequest?
        get() = if (level == null) null else NewGameRequest(Level.valueOf(level), Starter.valueOf(starter ?: "RANDOM"), variant ?: "cv.standard")

    companion object {
        fun of(r: NewGameRequest) = GameRoute(r.level.name, r.starter.name, r.variant)
    }
}

private data class Tab(val route: Any, val label: Int, val icon: ImageVector)

private val TABS = listOf(
    Tab(HomeRoute, R.string.menu_game, OurilIcons.Play),
    Tab(TutorialRoute, R.string.menu_tutorial, OurilIcons.Learn),
    Tab(RulesRoute(), R.string.menu_rules, OurilIcons.Rules),
    Tab(SettingsRoute, R.string.menu_settings, OurilIcons.Settings),
)

/**
 * The app: four tabs (Play, Learn, Rules, Settings) in a bottom bar, as on a phone on the web,
 * and the game full screen without them.
 */
@Composable
fun OurilApp() {
    val nav = rememberNavController()
    val entry by nav.currentBackStackEntryAsState()
    val destination = entry?.destination
    val inGame = destination?.hasRoute<GameRoute>() == true
    val c = panu

    Column(Modifier.fillMaxSize().background(c.bg)) {
        NavHost(nav, startDestination = HomeRoute, modifier = Modifier.weight(1f)) {
            composable<HomeRoute> {
                HomeScreen(
                    onPlay = { request -> nav.navigate(if (request == null) GameRoute() else GameRoute.of(request)) },
                    onLearn = { nav.navigate(TutorialRoute) { launchSingleTop = true } },
                    onAbout = { nav.navigate(AboutRoute) },
                )
            }
            composable<GameRoute> { back ->
                GameScreen(
                    route = back.toRoute<GameRoute>(),
                    onExit = { nav.popBackStack(HomeRoute, inclusive = false) },
                    onAllRules = { variant -> nav.navigate(RulesRoute(variant)) },
                )
            }
            composable<TutorialRoute> { TutorialScreen(onFinish = { nav.popBackStack(HomeRoute, false) }) }
            composable<RulesRoute> { back ->
                RulesScreen(initial = back.toRoute<RulesRoute>().variant, onTutorial = { nav.navigate(TutorialRoute) })
            }
            composable<SettingsRoute> { SettingsScreen(onAbout = { nav.navigate(AboutRoute) }) }
            composable<AboutRoute> { AboutScreen() }
        }
        if (!inGame) {
            HorizontalDivider(color = c.line)
            Row(
                Modifier
                    .fillMaxWidth()
                    .background(c.surface)
                    .windowInsetsPadding(WindowInsets.navigationBars)
                    .padding(horizontal = 6.dp, vertical = 6.dp)
                    .selectableGroup(),
                horizontalArrangement = Arrangement.SpaceAround,
            ) {
                for (tab in TABS) {
                    val selected = destination?.hasRoute(tab.route::class) == true
                    Column(
                        Modifier
                            .weight(1f)
                            .clip(androidx.compose.foundation.shape.RoundedCornerShape(14.dp))
                            .selectable(selected = selected, role = Role.Tab) {
                                nav.navigate(tab.route) {
                                    popUpTo(nav.graph.findStartDestination().id) { saveState = true }
                                    launchSingleTop = true
                                    restoreState = true
                                }
                            }
                            .padding(vertical = 6.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Box(
                            Modifier
                                .clip(CircleShape)
                                .background(if (selected) c.indigoSoft else c.surface)
                                .padding(horizontal = 14.dp, vertical = 4.dp),
                        ) {
                            Icon(tab.icon, contentDescription = null, tint = if (selected) c.indigo else c.ink2, modifier = Modifier.size(22.dp))
                        }
                        Text(tr(tab.label), style = PanuType.tab, color = if (selected) c.indigo else c.ink2, modifier = Modifier.padding(top = 2.dp))
                    }
                }
            }
        }
    }
}
