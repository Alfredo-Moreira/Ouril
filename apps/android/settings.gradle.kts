// Ouril for Android (ADR 0008): Kotlin + Jetpack Compose, with the shared Rust core through
// UniFFI (core/ffi, ADR 0007). Build with `just android-build`; see apps/android/README.md.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Ouril"
include(":app")
