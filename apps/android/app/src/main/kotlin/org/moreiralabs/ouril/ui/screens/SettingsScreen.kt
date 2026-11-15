package org.moreiralabs.ouril.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.data.Settings
import org.moreiralabs.ouril.data.Store
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.ui.components.ButtonKind
import org.moreiralabs.ouril.ui.components.OurilIcons
import org.moreiralabs.ouril.ui.components.PanuButton
import org.moreiralabs.ouril.ui.components.PanuCard
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/** Settings: sound (with haptics) and the hint button, kept on this device. */
@Composable
fun SettingsScreen(onAbout: () -> Unit) {
    val context = LocalContext.current
    val store = remember { Store(context) }
    val settings by store.settings.collectAsState(initial = Settings())
    val scope = rememberCoroutineScope()
    val c = panu
    Column(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars).verticalScroll(rememberScrollState()).padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(tr(R.string.settings_title), style = PanuType.title, color = c.ink, modifier = Modifier.semantics { heading() })
        PanuCard {
            Text(tr(R.string.settings_game), style = PanuType.card, color = c.ink, modifier = Modifier.semantics { heading() })
            ToggleRow(tr(R.string.settings_sound), settings.sound) { scope.launch { store.setSound(it) } }
            ToggleRow(tr(R.string.settings_hints), settings.hints) { scope.launch { store.setHints(it) } }
        }
        PanuButton(onAbout, Modifier.fillMaxWidth(), ButtonKind.OUTLINE) {
            Icon(OurilIcons.Info, null, Modifier.size(20.dp)); Text(tr(R.string.menu_about))
        }
    }
}

/** A setting that's on or off: the whole row toggles the switch (one target, one label). */
@Composable
private fun ToggleRow(label: String, on: Boolean, onChange: (Boolean) -> Unit) {
    val c = panu
    Row(
        Modifier.fillMaxWidth().defaultMinSize(minHeight = 52.dp).toggleable(on, role = Role.Switch, onValueChange = onChange),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = PanuType.label, color = c.ink, modifier = Modifier.weight(1f))
        Switch(
            checked = on,
            onCheckedChange = null,
            colors = SwitchDefaults.colors(
                checkedTrackColor = c.selectedBg,
                checkedThumbColor = c.selectedInk,
                uncheckedTrackColor = c.surface3,
                uncheckedThumbColor = c.muted,
                uncheckedBorderColor = c.muted,
            ),
        )
    }
}
