# 0025. Launch the web app first, as a static site; accounts and native apps after

- **Status:** Accepted
- **Date:** 2026-10-04

## Context
[ADR 0018](0018-platform-build-order.md) planned to launch web, iOS and Android together, and [ADR 0009](0009-oauth-accounts-google-apple.md) put optional Google and Apple sign-in in the MVP. The web app is now feature-complete for single-player play, with a 3D board, four rule sets, a tutorial and offline support, while the native apps haven't started and sign-in still needs provider keys. Everything a guest does already runs in the browser with no server ([ADR 0011](0011-offline-first-sync.md), [ADR 0014](0014-telemetry-consent.md)).

## Decision
- **The MVP is the web app alone**, deployed as a **static site** (Vercel, [deployment](../architecture/deployment.md)). It needs **no server and no database**: the rules engine (WASM), AI, storage (IndexedDB) and music all run in the browser, and it works offline once loaded.
- **Sign-in, sync and the Stats page are built but switched off** (`apps/web/src/features.ts`). They ship later, with the server, in their own release. The accounts design in ADR 0009 stands; only its timing moves to after the MVP.
- **No consent prompt while no telemetry service is configured**: there is nothing to consent to. It appears once a service is set up (ADR 0014).
- **Build order stays** core → web → iOS → Android. iOS and Android launch after the web, each when it reaches parity with the web reference; their order and timing are decided on the [roadmap](../product/roadmap.md).
- This supersedes ADR 0018 (its build order is restated here; "launch all three together" is replaced).

## Alternatives considered
- **Launch all three together (ADR 0018):** one launch moment, but the finished web game would wait months for the native apps.
- **Web with sign-in from day one:** backs up progress, but needs the server, the database, provider keys, a privacy policy and account deletion before anyone can play.

## Consequences
- The launch needs only static hosting; there's no server to run or pay for until accounts ship.
- Progress lives in each browser. Players who clear their data lose their history (sign-in later adds backup and claims guest data).
- The server, sync and account code stay maintained and tested in CI while switched off.
