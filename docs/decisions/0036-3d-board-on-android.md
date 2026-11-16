# 0036. A 3D board on Android that moves exactly as the web's

- **Status:** Proposed
- **Date:** 2026-11-15

## Context
The web shows a 3D board with a hand that plays each move ([ADR 0021](0021-3d-web-board.md), [web design](../architecture/web-design.md#the-3d-board)). The first Android app ([ADR 0035](0035-android-before-ios.md)) drew a flat Compose board instead: one seed flying from pit to pit, no hand, and frames 2.2 times faster than the web's. Players should see the same game on every platform, and the docs left the native boards open ([ADR 0008](0008-native-apps-per-platform.md)). The two boards' motion had also drifted with nothing to catch it.

## Decision
- **Android draws the web's 3D board with Filament**, Google's renderer for mobile. It uses the same procedural geometry, colours, lights, camera, hand and seeds. Its materials are compiled at build time by Filament's `matc`, pinned in the Android image to the app's Filament version.
- **The web's board is the reference for motion.** Its choreography lives in plain modules: frames and timing, seeds, the hand, the camera, the rings and the tones. Android ports each one line for line.
- **Motion vectors keep the platforms the same.** `shared/motion/vectors/` records the web board on a 60 Hz clock (`just motion-vectors`). Every platform's tests replay the vectors: the web's to catch unrecorded changes, Android's (and later iOS's) to match them.
- On Android, the board lies across as on the web: the upright portrait board goes. The 2D board stays as a fallback where OpenGL ES 3 isn't available, and for the JVM tests.
- The `animation-parity` skill checks the platforms against each other and guides a motion change through them.

See [board motion](../architecture/board-motion.md).

## Alternatives considered
- **A 2D board with the web's choreography:** the same motion, seen flat from above. It's smaller and simpler, but it doesn't look like the web's board.
- **The choreography in the Rust core:** one implementation for every platform. But it puts presentation code in the core and needs a call across the bindings on every frame. The vectors give the same guarantee for now.
- **The web's three.js board in a WebView:** identical by construction, but it's a web page inside the app, and slower than native rendering on phones.
- **SceneView (a Filament wrapper):** less setup, but another dependency between the app and Filament, with its own materials.

## Consequences
- Android's board looks and moves like the web's.
- The app grows by Filament's library, about 2.8 MB on an arm64 phone. The Android image downloads Filament's tools at a pinned version and checksum.
- The motion vectors become a shared contract. A motion change starts on the web, regenerates the vectors and reaches every platform in the same change. iOS will port the same modules and replay the same vectors.
- The 3D board itself can't be tested on the JVM: its geometry, light and camera math are, and its rendering is checked on a phone.
