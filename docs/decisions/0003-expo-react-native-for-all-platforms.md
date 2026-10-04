# 0003. Expo / React Native for iOS, Android and web

- **Status:** Rejected — superseded by [0008](0008-native-apps-per-platform.md)
- **Date:** 2026-10-03

## Context
We need iOS, Android and web clients, and we're a small team. Ouril is a turn-based board game: it needs smooth animations of seeds, but not a 3D or physics engine.

## Decision
- Build **one app with Expo (React Native)**, using **expo-router**, and target the web through react-native-web.
- Render the board with **@shopify/react-native-skia** and animate with **Reanimated**. Both work on native and web.
- Use **EAS** for builds and store submission. Over-the-air updates handle JS-only fixes.

## Alternatives considered
- **Flutter:** an excellent cross-platform UI, but it uses Dart, so the engine couldn't be shared with a Node server. Flutter's web output is also heavier.
- **Unity or Godot:** great for action games, but overkill for a board game, with large web builds and weaker native UI for menus and accounts.
- **Web app wrapped with Capacitor:** simplest for the web, but less native feel and worse performance on low-end Android.
- **Separate web app (for example Next.js) plus React Native:** better SEO, but two UIs to maintain. We can still add a landing site later.

## Consequences
- One codebase for all three platforms, and the engine is shared with the server.
- The web build is good enough for the game, but less tuned than a dedicated web framework.
- Skia on web loads a WASM bundle, so we need to watch first-load size.
