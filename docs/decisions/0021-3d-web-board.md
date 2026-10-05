# 0021. 3D web board with React Three Fiber

- **Status:** Accepted
- **Date:** 2026-10-04

## Context
[ADR 0008](0008-native-apps-per-platform.md) says the web board is drawn "with SVG or Canvas". We want the web game to feel premium: a carved wooden board, seeds that fly from pit to pit, a camera that reacts to captures. Constraints:
- **Offline and private.** No runtime network fetches beyond our API: no CDN models, HDRIs or fonts ([data and sync](../architecture/data-and-sync.md), CSP `'self'`).
- **Accessibility.** Keyboard and screen-reader play must never depend on WebGL.
- **Low-end devices.** Many players are on mid-range Android phones and slow data.
- **The rules stay in the Rust core.** The board only draws engine events ([engine](../architecture/engine.md)).

## Decision
- Draw the board in **3D with React Three Fiber** (`three` + `@react-three/fiber` + `@react-three/drei`), in its own **lazily loaded chunk** (`src/board3d/`). The home page and the game load it on demand; a board-shaped loader shows until it's ready.
- Everything is **procedural**: the board is extruded geometry with lathe-turned bowls, the wood grain is a canvas texture, the lighting is local light-formers, and the 48 seeds are one instanced mesh. No asset files, no network.
- The scene renders **on demand** (`frameloop="demand"`): it draws only while seeds move or highlights fade, so an idle board costs nothing.
- Seeds follow the game's display frames (`game/animation.ts`). Between frames, the fewest seeds move from container to container (`board3d/seeds.ts`), on an arc with a strong ease-in-out.
- **The board is only 3D.** There's no other board in the web app: no fallback, no board style setting.
  - A browser without WebGL2 can't draw the board; the board area says so (the hidden controls below keep the game playable by keyboard).
  - If the 3D board fails to start (the chunk fails to load, a lost WebGL context), the board area says so and offers **Try again**.
  - With reduced motion, seeds snap into place and the camera stays still.
- **Accessibility layer:** `BoardControls` gives keyboard and screen-reader users the pits and stores as visually hidden controls with spoken labels. Focusing a pit lights the same pit in 3D.

## Alternatives considered
- **SVG/Canvas 2D only (ADR 0008 as written):** small and simple, but can't give depth, lighting or the "real board" feel we want.
- **Plain three.js without React Three Fiber:** a smaller dependency, but the scene would sit outside React's state, so syncing with game state, props and Suspense would be hand-written.
- **Babylon.js or PlayCanvas:** full engines; far larger bundles than we need for one board.
- **Pre-rendered or video assets:** heavy downloads, no interactivity, and breaks offline-first unless every asset is precached.
- **CSS 3D transforms:** cheap, but no real lighting or shadows, and 48 independently moving seeds get janky.

## Consequences
- **Bundle:** the 3D chunk is about 980 KB minified (about 265 KB gzipped). It's not on the critical path: it loads lazily and is precached for offline use by the service worker.
- **One board on the web.** jsdom has no WebGL, so component tests play through `BoardControls`; the 3D board itself is checked with screenshots in a real browser.
- **No board without WebGL2.** Such browsers see a message instead of the board; the hidden controls keep the game playable by keyboard.
- **Native apps are unaffected.** iOS and Android choose their own rendering under ADR 0008 (SceneKit/RealityKit or Filament are options if we want the same look).
- **This amends ADR 0008** only for the web board's rendering technology.
- Moves are played by a procedural **hand** that picks up, sows and carries captures to the store (no model files). While the 3D chunk loads, a small loading widget (seeds dropping into a bowl) holds its place.
- Motion follows the rules in [web design](../architecture/web-design.md).
