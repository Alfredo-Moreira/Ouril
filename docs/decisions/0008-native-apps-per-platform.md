# 0008. Native apps per platform

- **Status:** Accepted
- **Date:** 2026-10-03
- **Supersedes:** [0003](0003-expo-react-native-for-all-platforms.md)

## Context
We want the best possible look, feel and performance on each platform, with full access to native features: sign-in, push notifications, haptics, widgets and Game Center/Play Games later. The game logic is already shared through the Rust core ([ADR 0007](0007-rust-core-with-generated-bindings.md)), so only the UI differs per platform.

## Decision
- **iOS:** Swift + **SwiftUI**. The board is drawn with SwiftUI Canvas/animations, or SpriteKit if needed.
- **Android:** Kotlin + **Jetpack Compose**. The board is drawn with Compose Canvas/animations.
- **Web:** **TypeScript + React (Vite)**. The board is drawn with SVG or Canvas. Installable as a PWA for offline play.
- All three call the Rust core through generated bindings, and handle networking, storage and sign-in natively.

## Alternatives considered
- **Expo / React Native:** one UI codebase, but it doesn't feel as native, and you depend on cross-platform layers for platform features.
- **Flutter:** one UI codebase, but uses Dart and is less native-feeling on iOS.
- **Compose Multiplatform:** shares UI across Android/iOS/web, but iOS and web support are less mature.

## Consequences
- The **UI is built three times**: board, animations, tutorial, menus and settings. This is the main cost of the decision.
- Mitigations: keep UI logic thin (the core produces the move events used for animation), share assets and translations in `shared/`, and write one design spec for all three.
- We may stagger platform launches if needed (open question in the [roadmap](../product/roadmap.md)).
