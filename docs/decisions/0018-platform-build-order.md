# 0018. Build order: core, web, iOS, Android; launch all three together

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
Native apps mean building the UI three times ([ADR 0008](0008-native-apps-per-platform.md)). The MVP and roadmap left open whether to launch all platforms together or staggered. Development is local-first ([ADR 0017](0017-local-first-development.md)).

## Decision
- **Build order:**
  1. **Rust core:** engine, AI and test vectors, plus the WASM and UniFFI bindings
  2. **Web app:** the fastest loop to iterate on board UI, animation, tutorial flow and sign-in with the local server. It becomes the reference implementation for the design spec.
  3. **iOS app**
  4. **Android app**

  The server's MVP scope (auth, profile, sync) is built alongside step 2, when the web app first needs it.
- **Each platform follows the design spec and passes the same test vectors.** A feature is only "done" for the MVP when it exists on all three platforms.
- **Launch all three platforms together** for the MVP, matching the vision of playing anywhere and avoiding a launch where some players are left out. Before launch, the web build is used for internal playtesting.
- If Android lags far behind, a staggered launch needs a new ADR that supersedes this one.

## Alternatives considered
- **iOS first:** best-known platform for polish, but slower to iterate on than the web, and it needs Apple tooling for every UI experiment.
- **All three in parallel from the start:** fastest wall-clock time with several developers, but triples the cost of every UI change while the design is still moving.
- **Staggered launch (web + iOS first):** ships sooner, but splits the first audience. It stays available as a fallback.

## Consequences
- The web app shapes the UI first, and iOS and Android follow a more settled design, so less rework.
- Launch waits for the slowest platform.
- The web build must stay feature-complete, because it's the reference and the internal playtest build.
