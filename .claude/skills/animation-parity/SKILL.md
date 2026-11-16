---
name: animation-parity
description: Keeps Ouril's game animation the same on every platform. The web's 3D board is the reference; Android (and later iOS) must play its exact motion, checked by the motion vectors in shared/motion/vectors. Checks the platforms against each other (timings, tones, the ported motion code, the vectors, the tests) and reports, or carries a motion change from the web to every platform. Use after any change to how a move looks (frames, timing, the hand, seeds, camera, rings, sounds) on any platform, when adding a platform, before a release, or when the user asks whether the animations match across web and the apps.
argument-hint: "[check (default) | change <what to change> | an area: frames, hand, seeds, camera, rings, tones, looks]"
allowed-tools:
  - Bash(python3 -I ${CLAUDE_PROJECT_DIR}/.claude/skills/animation-parity/scripts/check_motion.py *)
---

# Animation parity

A move must look the same on the web, Android and, later, iOS. That covers:

- the frames and their pace;
- the hand reaching in, gripping, sowing and capturing;
- each seed's lift, fall and bounce;
- the camera's push-in;
- the pits' glow;
- the sounds.

Read [board motion](../../../docs/architecture/board-motion.md) first. It holds the file map, the vector format and what may differ.

**The rule:** the web's board is the reference. Motion changes start there, re-record the vectors (`just motion-vectors`) and reach every platform in the same change. No platform decides motion on its own, and nobody edits a vector by hand.

## Input

`$ARGUMENTS`:

- empty or `check`: audit and report (read-only, no edits);
- `change <description>`: carry a motion change through every platform;
- an area (`frames`, `hand`, `seeds`, `camera`, `rings`, `tones`, `looks`): audit only that, with its files.

## Check (report only)

1. **Static check**, which builds nothing:
   ```bash
   python3 -I ${CLAUDE_PROJECT_DIR}/.claude/skills/animation-parity/scripts/check_motion.py ${CLAUDE_PROJECT_DIR}
   ```
   It checks that:
   - the vectors are there and cover every frame kind a playable variant has;
   - the frame times, the 3D pace, reduced motion, the computer's pause and every tone agree across the web, each port and `timings.json`;
   - each ported file keeps its web original's numbers;
   - the docs name every pair.

   The script lists numbers a port legitimately needs that the web gets elsewhere (`EXPECTED`, with reasons). Any other number difference is a lead: read both files at that spot.
2. **The vectors on every platform** are the proof:
   - web: `docker compose run --rm -T web sh -c "pnpm install --frozen-lockfile && pnpm --filter web test src/board3d/motionVectors.test.ts"` (build the WASM package first with `just bindings` if `core/wasm/pkg` is missing);
   - Android: `docker compose --profile android run --rm -T android sh -c "cd apps/android && ./gradlew --no-daemon --no-watch-fs testDebugUnitTest --tests 'org.moreiralabs.ouril.ui.board3d.*'"` (run `just bindings-android` first if `core/ffi/generated` is missing).

   A web failure means the web changed without new vectors. A port failure means the port drifted; the message names the sample time and the value.
3. **Off the clock:** some parts aren't in the vectors yet, so compare them by reading the code:
   - where each platform fires the sounds and haptics (the web plays tones as a frame starts; Android adds haptics, which is allowed);
   - the reduced-motion paths (the web's `prefers-reduced-motion`, Android's system animations switch);
   - the speed each screen passes to the board (game, tutorial, online games).
4. **Report** a table: area × platform → matches / differs (file:line, what) / not covered. Then list the fixes in order. Don't edit anything in this mode.

## Change

1. Change the web first (`apps/web/src/board3d/`, `game/animation.ts`, `game/sound.ts`, `board3d/pace.ts`). Review the motion with `/review-animations` and look at it in the browser (`just dev`).
2. Re-record: `just motion-vectors`. Read the diff in `shared/motion/vectors/`; only the scenarios your change touches should move.
3. Port the same lines to each platform (Android: `ui/board3d/*.kt`, `game/Frames.kt`, `game/Sounds.kt`), keeping the arithmetic in the same order. Run each platform's test until it replays the vectors.
4. A new scenario (for example a new frame kind) goes in `scenarios()` in `motionVectors.test.ts`, then is re-recorded.
5. Run the static check again. Update [board motion](../../../docs/architecture/board-motion.md) and, for rules about how motion should feel, the motion rules in [web design](../../../docs/architecture/web-design.md#motion-rules).

## Adding a platform (iOS)

- Port each pair in the doc's table: the same functions and the same order of operations.
- Add the platform's files to `PAIRS` and `PLATFORM_ROOTS` in `scripts/check_motion.py`, and its root to the timings parser.
- Add a test that replays every vector the way `MotionVectorsTest.kt` does, wired into CI and the git hooks.
- Add the platform's column to the doc's table.

## What may differ

Only rendering: tone mapping, light approximations, anti-aliasing, haptics and the fallback when 3D can't run (the doc lists them).

Never different between platforms:

- frame times and pace;
- keyframe fractions;
- easing curves;
- seed timings;
- camera and ring rates;
- tone choices.

Each of these is either in the vectors or checked by the script.
