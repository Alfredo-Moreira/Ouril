# ouril-ffi API (native contract)

> The Kotlin (and later Swift) API of the Rust core, generated with UniFFI, as used by `apps/android`. **Status:** Draft (implemented; the Android unit tests run every test vector through it). Source: [`src/lib.rs`](src/lib.rs). It mirrors the [web contract](../wasm/API.md) function for function, with the same JSON shapes.

## Building and loading

- `just bindings-android` builds everything into `core/ffi/generated/` ([ADR 0007](../../docs/decisions/0007-rust-core-with-generated-bindings.md)). It's build output: gitignored, never hand-edited, never committed ([ADR 0015](../../docs/decisions/0015-generated-bindings-not-committed.md)).
  - `jniLibs/<abi>/libouril_ffi.so` for `arm64-v8a`, `armeabi-v7a` and `x86_64`, from `cargo ndk` (set `OURIL_ANDROID_ABIS` to build fewer).
  - `kotlin/uniffi/ouril_ffi/ouril_ffi.kt`, from this crate's own `uniffi-bindgen` binary in library mode, so the generator always matches the `uniffi` version in `Cargo.toml`.
  - `host/libouril_ffi.so`, the same library for the build machine, which the JVM unit tests load through `jna.library.path`.
- It runs in the `android` container (`docker/android.Dockerfile`: JDK, Android SDK and NDK, Rust with the Android targets, cargo-ndk), the same locally and in CI ([ADR 0017](../../docs/decisions/0017-local-first-development.md)).
- The app's Gradle build adds the generated Kotlin and libraries to its source sets and fails early with a hint when they're missing. The bindings load the library with JNA (`net.java.dev.jna:jna`, the `@aar` on Android).
- Hard can take hundreds of milliseconds, as on the web: call `aiMove` off the main thread (the app uses `Dispatchers.Default`).

```kotlin
import uniffi.ouril_ffi.*

val state = newGameJson("cv.standard", 1, "south")
val moves = legalMoves("cv.standard", 1, state)              // [0, 1, 2, 3, 4, 5]
val result = applyMoveJson("cv.standard", 1, state, 2)       // {"state":{…},"events":[…]}
```

The app doesn't call these directly: `org.moreiralabs.ouril.core.Core` wraps them in typed Kotlin calls and data classes (`Core.kt`, `Models.kt`).

## Conventions

- **Values cross as JSON strings** with exactly the shapes of the [web contract's types](../wasm/API.md#types) and the [test vectors](../../docs/architecture/test-vectors.md): snake_case keys and enums, `GameState.history` opaque, unknown event types possible in later versions. Pits, seeds and versions are `Int` (`i32`), and AI seeds `Long` (`i64`, never negative).
- A variant is passed as its `id` and `version` (the web's `VariantRef`).
- Every `GameState` is checked before use (board size, seed conservation): malformed input is an `invalid_argument` error, never a crash.
- As on the web, never assume turns alternate: read `to_move` from the returned state.

## Functions

| Function | Kotlin signature | Web equivalent |
|---|---|---|
| `coreVersion` | `(): String` | `coreVersion` |
| `variantsJson` | `(): String` (a `VariantConfig[]`) | `variants` |
| `defaultVariantJson` | `(country: String): String?` (a `VariantConfig`) | `defaultVariant` |
| `newGameJson` | `(variantId: String, variantVersion: Int, first: String): String` (a `GameState`) | `newGame` |
| `legalMoves` | `(variantId: String, variantVersion: Int, stateJson: String): List<Int>` | `legalMoves` |
| `applyMoveJson` | `(variantId: String, variantVersion: Int, stateJson: String, pitIndex: Int): String` (a `MoveResult`) | `applyMove` |
| `aiMove` | `(variantId: String, variantVersion: Int, stateJson: String, level: String, seed: Long): Int?` | `aiMove` |
| `replayJson` | `(variantId: String, variantVersion: Int, first: String, moves: List<Int>): String` (a `Replay`) | `replay` |
| `deriveStatsJson` | `(recordsJson: String): String` (a `GameRecord[]` in, `Stats` out) | `deriveStats` |

### Errors

Every function that can fail throws a `CoreException` whose `message` is the same machine-readable code as on the web:

| `message` | Subclass | When |
|---|---|---|
| `not_own_pit`, `empty_pit`, `single_seed_rule`, `must_feed`, `grand_slam_forbidden`, `game_over` | `CoreException.Move` | `applyMoveJson` with an illegal move. A pit off the board, negative included, is `not_own_pit`. |
| `illegal_move:<ply>:<code>` | `CoreException.IllegalMove` | `replayJson` hit an illegal move at 0-based `ply` |
| `unknown_variant` | `CoreException.UnknownVariant` | The variant isn't in the registry |
| `invalid_argument: <detail>` | `CoreException.InvalidArgument` | A JSON value didn't match its shape, a `GameState` doesn't fit the variant, or a seed is negative |

## Open questions

- The Swift bindings (an XCFramework for iOS) come from the same crate when the iOS app starts.
- Typed records instead of JSON strings (UniFFI records and enums) would skip a parse on each call. JSON keeps one shape for every platform and the vectors; revisit if profiling shows the cost.
