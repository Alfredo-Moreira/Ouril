package org.moreiralabs.ouril

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import org.moreiralabs.ouril.ui.OurilApp
import org.moreiralabs.ouril.ui.theme.OurilTheme

/** The one activity: Compose draws the whole app, edge to edge. */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            OurilTheme {
                OurilApp()
            }
        }
    }
}
