package org.moreiralabs.ouril.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.collapse
import androidx.compose.ui.semantics.expand
import androidx.compose.ui.unit.dp
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.game.PLAYABLE_VARIANTS
import org.moreiralabs.ouril.game.VARIANT_RULES
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.i18n.trKey
import org.moreiralabs.ouril.ui.components.Body
import org.moreiralabs.ouril.ui.components.ChoiceChips
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/** The rules, one tab per playable variant, each topic a section that opens and closes. */
@Composable
fun RulesScreen(initial: String, onTutorial: () -> Unit) {
    var variant by rememberSaveable { mutableStateOf(initial) }
    val c = panu
    Column(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars).verticalScroll(rememberScrollState()).padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Text(tr(R.string.rules_title), style = PanuType.title, color = c.ink, modifier = Modifier.semantics { heading() })
        Text(
            tr(R.string.rules_try_tutorial),
            style = PanuType.label,
            color = c.indigo,
            modifier = Modifier.clickable(onClick = onTutorial).padding(vertical = 6.dp),
        )
        ChoiceChips(PLAYABLE_VARIANTS, variant, { trKey("variant.$it.short") }, { variant = it })
        RulesContent(variant)
    }
}

/** A variant's rules: its intro, what it's based on, and its sections (all open at first). */
@Composable
fun RulesContent(variant: String) {
    val rules = VARIANT_RULES[variant] ?: return
    val c = panu
    val closed = remember(variant) { mutableStateMapOf<String, Boolean>() }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Body(trKey(rules.intro), style = PanuType.bodyLarge)
        rules.basedOn?.let { Body(tr(R.string.rules_based_on, "name" to trKey("variant.$it.short")), color = c.muted) }
        for (section in rules.sections) {
            val open = closed[section.id] != true
            Column(
                Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(c.surface)
                    .border(1.dp, c.line, RoundedCornerShape(16.dp)),
            ) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .defaultMinSize(minHeight = 52.dp)
                        .clickable { closed[section.id] = open }
                        .padding(horizontal = 16.dp, vertical = 12.dp)
                        // TalkBack announces expanded/collapsed from these actions.
                        .semantics {
                            if (open) collapse { closed[section.id] = true; true } else expand { closed[section.id] = false; true }
                        },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(trKey("rules.${section.id}.title"), style = PanuType.labelStrong, color = c.ink, modifier = Modifier.weight(1f).semantics { heading() })
                    Text(if (open) "−" else "+", style = PanuType.section, color = c.muted, modifier = Modifier.clearAndSetSemantics { })
                }
                AnimatedVisibility(open) {
                    Column(Modifier.padding(start = 16.dp, end = 16.dp, bottom = 14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (n in 1..section.paragraphs) Body(trKey("rules.${section.id}.$n"))
                    }
                }
            }
        }
    }
}
