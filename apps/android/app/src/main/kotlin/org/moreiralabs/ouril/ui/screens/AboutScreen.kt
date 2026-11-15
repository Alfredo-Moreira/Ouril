package org.moreiralabs.ouril.ui.screens

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.moreiralabs.ouril.BuildConfig
import org.moreiralabs.ouril.R
import org.moreiralabs.ouril.core.Core
import org.moreiralabs.ouril.i18n.tr
import org.moreiralabs.ouril.i18n.trKey
import org.moreiralabs.ouril.ui.components.Body
import org.moreiralabs.ouril.ui.components.ButtonKind
import org.moreiralabs.ouril.ui.components.PanuButton
import org.moreiralabs.ouril.ui.components.PanuCard
import org.moreiralabs.ouril.ui.components.PanuStrip
import org.moreiralabs.ouril.ui.components.SectionTitle
import org.moreiralabs.ouril.ui.theme.PanuShapes
import org.moreiralabs.ouril.ui.theme.PanuType
import org.moreiralabs.ouril.ui.theme.panu

/**
 * Who makes Ouril and where to find him, as on the web (`content/developer.ts`), in the same
 * order. The web's "buy me a coffee" links aren't here: Google Play doesn't allow tips to the
 * developer outside its own billing.
 */
private object Developer {
    const val NAME = "Alfredo Moreira"
    val socials = listOf(
        "website" to "https://alfredomoreira.dev",
        "email" to "mailto:devops.moreira@gmail.com",
        "linkedin" to "https://www.linkedin.com/in/alfredomoreira/",
        "github" to "https://github.com/Alfredo-Moreira",
        "instagram" to "https://www.instagram.com/alfredo.r.moreira/",
        "facebook" to "https://www.facebook.com/alfredo.d.moreira",
        "twitch" to "https://www.twitch.tv/playbydev",
    )
}

@Composable
fun AboutScreen() {
    val c = panu
    val context = LocalContext.current
    fun open(url: String) {
        try {
            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        } catch (_: ActivityNotFoundException) {
            // No app to open it with: nothing to do.
        }
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
        Column(
            Modifier
                .fillMaxWidth()
                .background(androidx.compose.ui.graphics.Brush.linearGradient(listOf(c.night, c.night2)))
                .windowInsetsPadding(WindowInsets.statusBars),
        ) {
            Column(Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Image(
                    painterResource(R.drawable.developer),
                    contentDescription = tr(R.string.about_photo_alt, "name" to Developer.NAME),
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.size(132.dp).clip(CircleShape).border(4.dp, c.nightInk, CircleShape),
                )
                Text(tr(R.string.about_kicker), style = PanuType.small, color = c.star)
                Text(Developer.NAME, style = PanuType.title, color = c.nightInk, modifier = Modifier.semantics { heading() })
                Body(tr(R.string.about_intro), color = c.nightInk)
                Body(tr(R.string.about_more), color = c.nightMuted)
            }
            PanuStrip(Modifier.fillMaxWidth().height(18.dp))
        }
        Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            PanuCard {
                SectionTitle(tr(R.string.about_find_me))
                for ((kind, url) in Developer.socials) {
                    PanuButton({ open(url) }, Modifier.fillMaxWidth(), ButtonKind.OUTLINE) {
                        Text(trKey("about.social.$kind"))
                    }
                }
            }
            Body(
                "${BuildConfig.VERSION_NAME} · ${Core.version}",
                color = c.muted,
                style = PanuType.small,
                modifier = Modifier.clip(PanuShapes.button),
            )
        }
    }
}
