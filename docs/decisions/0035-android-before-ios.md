# 0035. Build the Android app before iOS

- **Status:** Proposed
- **Date:** 2026-11-14

## Context
[ADR 0025](0025-web-first-static-mvp.md) kept the build order core → web → iOS → Android and left the native apps' order and timing to the [roadmap](../product/roadmap.md). The web app is live with accounts, games against other players and leaderboards, and the native apps are next. The developer chose to start with Android. Android builds and tests run entirely in Docker on any machine ([ADR 0017](0017-local-first-development.md)); iOS needs Xcode on a Mac.

## Decision
- **Build the Android app before the iOS app.** Its first version is guest play at parity with the web's guest features: play against the computer (three levels, four rule sets), the tutorial, the rules, settings and About, offline, with no network permission.
- The core reaches Android through `core/ffi` (UniFFI, [ADR 0007](0007-rust-core-with-generated-bindings.md)), with the same JSON values as the web contract ([core/ffi/API.md](../../core/ffi/API.md)). The same crate gives iOS its Swift bindings later.
- Accounts and sync, games against other players and leaderboards come to Android next, then the iOS app. Each app launches when it reaches parity with the web, as ADR 0025 says.

See [apps/android/README.md](../../apps/android/README.md).

## Alternatives considered
- **iOS first (ADR 0025's order):** the order on paper, but it needs a Mac with Xcode for every build and test, and nothing in it is easier to start first.
- **Both at once:** every native screen built twice while the native design is still settling.

## Consequences
- CI gets an Android job (bindings for every ABI, unit tests, a debug build); the native contract (`core/ffi`) is designed and tested now, before Swift needs it.
- iOS waits for Android; the installable web app keeps covering iPhones.
- When this is accepted, ADR 0025's build-order bullet reads as amended by this ADR, and the roadmap's phase 5 lists Android first.
