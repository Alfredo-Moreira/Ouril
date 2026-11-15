# Ouril for Android

> The Android app: Kotlin and Jetpack Compose, with the shared Rust core through UniFFI. **Status:** Draft (built for guest play; accounts and online play come next, [ADR 0035](../../docs/decisions/0035-android-before-ios.md), Proposed).

## What it does

- **Play against the computer:** three levels, the four Cape Verdean rule sets, who starts. Undo, a hint button (Settings), forfeit, and the game saved after every move, so it resumes after the app closes.
- **Learn:** the tutorial's ten lessons and the rules of each variant, with the same positions and strings as the web.
- **The Panu design** ([web design](../../docs/architecture/web-design.md)): the light (cotton) and dark (night) themes, following the system; one star-yellow action per screen; Bricolage Grotesque and Figtree; the seed ring; the panu strip; the board on a cloth.
- **The board** is drawn on a Canvas. When the screen is taller than it is wide, it stands upright: your pits are the left column and run down into your store, and the computer's run up the right. Otherwise it lies flat, as on the web. Every seed drop, capture and collection is animated from the core's move events. Sound effects (tones made on the device, as on the web) and haptics follow Settings.
- **Offline and private:** no network permission. A guest's data stays on the device.

It doesn't have the web's sign-in and sync, Stats, games against other players, leaderboards, profiles or seasons yet, nor its music. It has no "buy me a coffee" links: Google Play doesn't allow tips to the developer outside its own billing.

## Building and testing

Everything runs in the `android` container (`docker/android.Dockerfile`: JDK 21, the Android SDK and NDK, Rust with the Android targets), the same locally and in CI ([ADR 0017](../../docs/decisions/0017-local-first-development.md)). Android Studio and the emulator stay on the host.

| Task | With `just` | Without `just` on the host |
|---|---|---|
| Build the core's libraries and Kotlin bindings (`core/ffi/generated`, needed first) | `just bindings-android` | `docker compose --profile android run --rm android just bindings-android` |
| Build the debug APK (`app/build/outputs/apk/debug`) | `just android-build` | `docker compose --profile android run --rm android just android-build` |
| Unit tests | `just android-test` | `docker compose --profile android run --rm android just android-test` |
| Screenshots of every screen, light, dark and landscape (`app/build/outputs/roborazzi`) | `just android-screenshots` | `docker compose --profile android run --rm android just android-screenshots` |

To open the project in Android Studio, run `just bindings-android` first, then open `apps/android`. Install the debug APK on a phone or the emulator with `adb install app/build/outputs/apk/debug/app-debug.apk`. It holds the core for `arm64-v8a`, `armeabi-v7a` and `x86_64`.

## How it's put together

Package `org.moreiralabs.ouril`:

| Folder | What's in it |
|---|---|
| `core/` | `Core`: the generated bindings behind typed calls, and the core's values (`Models.kt`) with the web's JSON shapes ([core/ffi/API.md](../../core/ffi/API.md)) |
| `game/` | The game against the computer as data and pure helpers (`Session.kt`), its view model, the move animation (`Frames.kt`), the lessons and rules, and the sounds |
| `data/` | The local database (Room, [ADR 0010](../../docs/decisions/0010-postgres-and-local-sqlite.md)): `games`, `outbox` and `kv`, as on the web ([data and sync](../../docs/architecture/data-and-sync.md)) |
| `i18n/` | Strings: `shared/i18n/en.json` becomes `res/values/strings.xml` at build time (the `generateStrings` Gradle task), and messages are formatted with ICU, as on the web ([localization](../../docs/product/features/localization.md)) |
| `ui/` | The Panu theme, the components, the board, the screens and the navigation |

- **No rule logic in Kotlin:** legal moves, results and positions all come from the core, and the board only shows them.
- **Generated code isn't committed:** the bindings and libraries in `core/ffi/generated` and the strings ([ADR 0015](../../docs/decisions/0015-generated-bindings-not-committed.md)). Room's schema history (`app/schemas`) is committed, because migrations are written against it.

### Tests

| Test | Checks |
|---|---|
| `CoreVectorsTest` | Every [test vector](../../docs/architecture/test-vectors.md) through the Kotlin bindings, as the app calls the core. A vector that can't run fails. |
| `FramesTest`, `SessionTest`, `LessonsTest`, `SoundsTest` | The move animation lands on the engine's board in random games of every variant; undo, outcomes, saved games and records the core's stats read; every lesson plays through |
| `StringKeysTest` | Every key built at run time exists in `shared/i18n` |
| `StoreTest` | The database: the guest ID, settings and finished games with their outbox entries (Robolectric) |
| `ScreensTest` | A tour of the app on a phone, light, dark and landscape: the tabs, About, a game with a move, the computer's reply, the menu, a forfeit and the result (Robolectric, Roborazzi) |

Robolectric runs on API 36 (`robolectric.properties`): Espresso, which the Compose test rule uses, still calls a method that API 37 removed.

## Open questions

- Sign-in (Google with Credential Manager), the sync engine (WorkManager), then games against other players and leaderboards.
- The Play Store release: signing, the listing, the data safety form.
- Portuguese and Kriolu once their translations exist, through `tools/i18n-gen` ([ADR 0016](../../docs/decisions/0016-i18n-source-format.md)).
- Check the screenshots in CI against committed ones, instead of only recording them?
