# JNA and the UniFFI bindings call into the Rust library by reflection: keep them.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class uniffi.** { *; }
-dontwarn java.awt.**

# Filament (the 3D board) calls back into its Java classes from native code.
-keep class com.google.android.filament.** { *; }
