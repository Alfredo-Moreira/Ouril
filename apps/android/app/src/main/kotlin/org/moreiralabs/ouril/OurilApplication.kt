package org.moreiralabs.ouril

import android.app.Application
import org.moreiralabs.ouril.data.OurilDatabase

class OurilApplication : Application() {
    /** The local database (ADR 0010), opened on first use. */
    val database: OurilDatabase by lazy { OurilDatabase.open(this) }
}
