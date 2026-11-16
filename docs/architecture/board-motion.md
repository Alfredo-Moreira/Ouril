# Board motion

> How a move looks on every platform: the web's 3D board is the reference, and motion vectors make every other board play it exactly the same. **Status:** Draft ([ADR 0036](../decisions/0036-3d-board-on-android.md), Proposed)

The game's animation is the same everywhere:

- the frames of a move and their timing;
- the hand;
- how each seed lifts, falls and flies;
- the camera's push-ins;
- the pits' glow;
- the sounds that go with each frame.

Each platform draws it with its own renderer (three.js on the web, Filament on Android), but no platform decides the motion on its own.

## The reference

The web's 3D board (`apps/web/src/board3d/`, [web design](web-design.md#the-3d-board)) is the reference. Its motion lives in plain modules with no React or WebGL, so a test can run it on a clock:

| What | Web (the reference) | Android |
|---|---|---|
| A move's frames and their timing: base times, the 3D pace (2.2), reduced motion (0.4), the computer's pause (350 ms) | `game/animation.ts`, `board3d/pace.ts` | `game/Frames.kt` |
| Which seed is in which pit, store or the hand | `board3d/seeds.ts` | `ui/board3d/Seeds.kt` |
| The board's measurements and where each seed rests | `board3d/geometry.ts` | `ui/board3d/Geometry.kt` |
| The hand's keyframes; each seed's lift, carry, fall and flight | `board3d/choreography.ts` | `ui/board3d/Choreography.kt` |
| The camera: the fit, the push-in on captures, the end-of-game angle | `board3d/cameraRig.ts` | `ui/board3d/CameraRig.kt` |
| The pits' glow: colours, strength, a fast rise and a slow fade | `board3d/rings.ts` | `ui/board3d/Rings.kt` |
| The sounds, played as each frame starts | `game/sound.ts` | `game/Sounds.kt` |
| Each seed's orientation and colour | `board3d/choreography.ts` | `ui/board3d/Choreography.kt` |

Each Android file ports its web file line for line, with the same arithmetic in the same order, so the two agree to floating-point precision.

## Motion vectors

`shared/motion/vectors/` holds recordings of the web board, written by `apps/web/src/board3d/motionVectors.test.ts` (`just motion-vectors`):

- **One file per scenario**, from real moves played by the engine:
  - an opening move and the computer's answer;
  - a double capture on a tall phone screen;
  - a 13-seed lap that skips its own pit;
  - continuous sowing;
  - the game's last move, then the end-of-game camera;
  - reduced motion;
  - an undo.

  Each file holds:
  - the moves;
  - the board's inputs over time: display frames at the game's pace, the lit pits, the hint and the game's end;
  - what the board did on a 60 Hz clock, sampled every third frame: the hand (position, grip, shown, side), every moving seed (position, tilt), the camera's position and the glowing rings;
  - where every seed ended.
- **`timings.json`:** the frame times, the pace, the reduced-motion speed, the computer's pause and every tone.
- **`seed-looks.json`:** each seed's orientation and colour.

Numbers keep five decimals, and a platform must match them within 2 × 10⁻⁵. Two tests replay them:

- **The web's `motionVectors.test.ts`** runs the web's own code, so an unrecorded change to the web fails.
- **Android's `MotionVectorsTest`** runs the Android port.

Both tests also rebuild each move's frames from the engine's events and check their kinds, counts and times. CI and the git hooks run them.

## Changing the motion

1. Change the web board (the reference) and look at it in the browser. `/review-animations` reviews motion code.
2. Run `just motion-vectors`, and read the diff in `shared/motion/vectors/`.
3. Port the change to every other platform until its tests replay the vectors. `/animation-parity` checks the platforms against each other.

Never edit a vector by hand, and never change one platform's motion alone.

## What may differ

The renderers may differ, but the motion may not:

- **Light and colour:** each platform tone-maps differently:
  - the web uses three.js's ACES filmic tone mapping and an environment of light panels;
  - Android uses Filament's ACES (legacy), with the same panels turned into spherical harmonics and a small cubemap.
  - The cloth and the count chips aren't tone-mapped on either platform. Android draws them with Compose, under and over the 3D view.
- **The rings:** Android feeds them in brighter, so they read the same after tone mapping.
- **Haptics:** Android adds a tick for each seed sown and a confirm for each capture. The web has none.
- **Resolution:** both platforms cap the board at 1.75 pixels per CSS pixel or dp. Android adds 4× MSAA.
- **When 3D can't run:**
  - Android draws its 2D board (Compose) on devices without OpenGL ES 3 and in the JVM tests;
  - the web says it can't show the board without WebGL2.

## Open questions

- Should the choreography move into the Rust core, so iOS gets it without a third port? The vectors already keep the ports the same; the core would remove the duplication, at the cost of presentation code in the core and a call across the bindings every frame.
- Vectors for grand-slam and extra-turn frames, once a playable variant has them (no Cape Verdean rule set does).
- A tap to fast-forward a long move (a lap of 12 or more seeds takes about 5 s), on every platform at once.
