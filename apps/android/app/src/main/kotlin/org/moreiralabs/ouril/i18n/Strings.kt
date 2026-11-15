package org.moreiralabs.ouril.i18n

import android.content.Context
import android.icu.text.MessageFormat
import android.icu.util.ULocale
import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import java.util.Locale

/**
 * Every string comes from shared/i18n (ADR 0016), compiled into `res/values/strings.xml` by the
 * `generateStrings` Gradle task with the ICU messages unchanged. They're formatted here with
 * ICU's MessageFormat, so arguments and plurals (`{count, plural, one {# seed} …}`) work exactly
 * as on the web.
 */
fun formatMessage(pattern: String, locale: Locale, args: Map<String, Any>): String {
    if (args.isEmpty() && '{' !in pattern) return pattern
    return MessageFormat(pattern, ULocale.forLocale(locale)).format(args)
}

/** A string by resource id, with named ICU arguments: `tr(R.string.tutorial_progress, "step" to 1)`. */
@Composable
fun tr(@StringRes id: Int, vararg args: Pair<String, Any>): String {
    val context = LocalContext.current
    val locale = LocalConfiguration.current.locales[0]
    val pattern = remember(id, locale) { context.getString(id) }
    return remember(pattern, locale, args.toList()) { formatMessage(pattern, locale, args.toMap()) }
}

/** The same outside composition. */
fun Context.tr(@StringRes id: Int, vararg args: Pair<String, Any>): String =
    formatMessage(getString(id), resources.configuration.locales[0], args.toMap())

/** A string by its shared key (`"tutorial.sowing.title"`), for keys built at run time. */
@Composable
fun trKey(key: String, vararg args: Pair<String, Any>): String {
    val context = LocalContext.current
    val id = remember(key) { context.stringId(key) }
    return if (id == 0) key else tr(id, *args)
}

/** The resource id of a shared key (`home.cta.play` → `R.string.home_cta_play`), or 0. */
@Suppress("DiscouragedApi") // Keys built at run time (tutorial steps, rules sections).
fun Context.stringId(key: String): Int =
    resources.getIdentifier(key.replace('.', '_').replace('-', '_'), "string", packageName)
