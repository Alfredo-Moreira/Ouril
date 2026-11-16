import groovy.json.JsonSlurper
import org.gradle.process.ExecOperations
import java.text.SimpleDateFormat
import java.util.Date
import javax.inject.Inject

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ksp)
    alias(libs.plugins.room)
    alias(libs.plugins.roborazzi)
}

// The repository root (this file is apps/android/app/build.gradle.kts).
val repoRoot: File = rootProject.projectDir.parentFile.parentFile
// The core's Kotlin bindings and libraries: build output of `just bindings-android` (ADR 0015).
val coreBindings: File = repoRoot.resolve("core/ffi/generated")
val i18nSource: File = repoRoot.resolve("shared/i18n/en.json")

android {
    namespace = "org.moreiralabs.ouril"
    compileSdk = 37
    // The build tools in the Android image (docker/android.Dockerfile).
    buildToolsVersion = "37.0.0"

    defaultConfig {
        applicationId = "org.moreiralabs.ouril"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        // The ABIs the core is built for (`just bindings-android`): the app ships no other.
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }

    buildTypes {
        // Test builds say what they are in About's version line.
        debug {
            versionNameSuffix = "-debug"
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
        // The release build signed with the debug key, to try on a phone at full speed
        // (`just android-preview`). Never published.
        create("preview") {
            initWith(getByName("release"))
            // With the build's time, so About shows which test build is installed.
            versionNameSuffix = "-preview." + SimpleDateFormat("MMdd.HHmm").format(Date())
            signingConfig = signingConfigs.getByName("debug")
            matchingFallbacks += listOf("release")
        }
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            kotlin.directories.add(coreBindings.resolve("kotlin").path)
            jniLibs.directories.add(coreBindings.resolve("jniLibs").path)
        }
    }

    testOptions {
        unitTests.all {
            // JVM unit tests load the core built for the host (`just bindings-android`), and
            // read the test vectors from the repository.
            it.systemProperty("jna.library.path", coreBindings.resolve("host").path)
            it.systemProperty("ouril.repo", repoRoot.path)
        }
        unitTests.isReturnDefaultValues = true
        // Robolectric tests (the database, the screens) need the app's resources.
        unitTests.isIncludeAndroidResources = true
    }
}

// The database schema of each version, kept with the code for migrations (Room).
room {
    schemaDirectory("$projectDir/schemas")
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.room.runtime)
    implementation(libs.androidx.room.ktx)
    ksp(libs.androidx.room.compiler)
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.ui.graphics)
    implementation(libs.compose.ui.tooling.preview)
    implementation(libs.compose.foundation)
    implementation(libs.compose.material3)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    // The 3D board (ADR 0036).
    implementation(libs.filament.android)
    // The core's bindings load the Rust library through JNA (UniFFI's Kotlin backend).
    implementation("${libs.jna.get()}@aar")
    debugImplementation(libs.compose.ui.tooling)
    debugImplementation(libs.compose.ui.test.manifest)

    testImplementation(libs.junit)
    testImplementation(libs.jna)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.androidx.test.junit)
    testImplementation(libs.compose.ui.test.junit4)
    testImplementation(libs.roborazzi)
    testImplementation(libs.roborazzi.compose)
}

// The core's bindings must exist before anything compiles: say how to build them.
val checkCoreBindings = tasks.register("checkCoreBindings") {
    val kotlinDir = coreBindings.resolve("kotlin")
    val libsDir = coreBindings.resolve("jniLibs")
    doLast {
        if (!kotlinDir.isDirectory || !libsDir.isDirectory) {
            throw GradleException(
                "The core's Android bindings are missing ($coreBindings). " +
                    "Run `just bindings-android` from the repository root first (ADR 0015).",
            )
        }
    }
}
tasks.named("preBuild") { dependsOn(checkCoreBindings) }

/**
 * Strings: shared/i18n/en.json → res/values/strings.xml (interim, until tools/i18n-gen, ADR
 * 0016). Keys become resource names (`home.cta.play` → `home_cta_play`), and the messages keep
 * their ICU syntax: the app formats them with android.icu.text.MessageFormat (`Strings.kt`),
 * so plurals and arguments behave exactly as on the web.
 */
abstract class GenerateStrings : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val source: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        @Suppress("UNCHECKED_CAST")
        val entries = JsonSlurper().parse(source.get().asFile) as Map<String, Map<String, Any>>
        val seen = mutableMapOf<String, String>()
        val body = StringBuilder()
        for ((key, entry) in entries.toSortedMap()) {
            val name = key.replace('.', '_').replace('-', '_')
            seen.put(name, key)?.let { throw GradleException("i18n keys $it and $key collide as $name") }
            body.append("    <string name=\"").append(name).append("\" formatted=\"false\">")
                .append(escape(entry["message"] as String)).append("</string>\n")
        }
        val out = outputDir.get().asFile.resolve("values/strings.xml")
        out.parentFile.mkdirs()
        out.writeText(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n" +
                "<!-- Generated from shared/i18n/en.json by the generateStrings task. Do not edit. -->\n" +
                "<resources>\n$body</resources>\n",
        )
    }

    /** A message as Android string-resource text: XML-escaped, quotes and leading @/? escaped. */
    private fun escape(message: String): String {
        val xml = message.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
        val escaped = xml.replace("\\", "\\\\").replace("'", "\\'").replace("\"", "\\\"").replace("\n", "\\n")
        return if (escaped.startsWith("@") || escaped.startsWith("?")) "\\$escaped" else escaped
    }
}

val generateStrings = tasks.register<GenerateStrings>("generateStrings") {
    source.set(i18nSource)
}

/**
 * The 3D board's materials: each `.mat` in src/main/materials becomes a `.filamat` in the assets'
 * materials folder, compiled by Filament's matc (in the Android image: FILAMENT_MATC). Build output,
 * never committed (ADR 0015).
 */
abstract class CompileMaterials : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Input
    abstract val filamentVersion: Property<String>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val exec: ExecOperations

    @TaskAction
    fun compile() {
        val matc = System.getenv("FILAMENT_MATC") ?: "matc"
        val out = outputDir.get().asFile.resolve("materials")
        out.deleteRecursively()
        out.mkdirs()
        for (source in sources.files.sortedBy { it.name }) {
            val target = out.resolve(source.nameWithoutExtension + ".filamat")
            val result = try {
                exec.exec {
                    commandLine(matc, "-p", "mobile", "-a", "opengl", "-o", target.path, source.path)
                    isIgnoreExitValue = true
                }
            } catch (e: Exception) {
                throw GradleException(
                    "Filament's material compiler isn't here ($matc). Build in Docker (`just android-build`), " +
                        "or set FILAMENT_MATC to matc from Filament ${filamentVersion.get()}.",
                    e,
                )
            }
            if (result.exitValue != 0) throw GradleException("matc couldn't compile ${source.name}")
        }
    }
}

val compileMaterials = tasks.register<CompileMaterials>("compileMaterials") {
    sources.from(fileTree("src/main/materials") { include("*.mat") })
    filamentVersion.set(libs.versions.filament.get())
}

androidComponents {
    onVariants { variant ->
        variant.sources.res?.addGeneratedSourceDirectory(generateStrings, GenerateStrings::outputDir)
        variant.sources.assets?.addGeneratedSourceDirectory(compileMaterials, CompileMaterials::outputDir)
    }
}
