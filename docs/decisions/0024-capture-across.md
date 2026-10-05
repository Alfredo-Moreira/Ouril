# 0024. Capture across as a variant parameter

- **Status:** Accepted
- **Date:** 2026-10-04

## Context
A family rule reported by the project owner captures **across the board** instead of on the opponent's side: when the last seed lands in an empty pit on the mover's side and the pit opposite has seeds, all of them are captured, with a backward chain through the mover's own pits. It can be played on its own or with continuous sowing ([ADR 0023](0023-relay-sowing.md)). No parameter could express it ([ADR 0004](0004-variant-configurable-rules-engine.md)), and across captures are common in the wider mancala family (Kalah, for example).

## Decision
- Add the variant parameter **`capture_mode`**: `opponent_side` (default, Oware as before) or `across_from_empty_own_pit`.
- Across captures reuse `capture` events (the captured pit is the one across) and `chain_capture`: `backward` checks each own pit sown just before; it continues while that pit holds exactly 1 seed (it was empty) and the pit across has seeds. `capture_counts` doesn't apply in this mode; any number of seeds is captured. The landing seed stays.
- Ship two variants, both choosable when starting a game: **`cv.across@1`** ("Capture across", extends `cv.standard`) and **`cv.across-continuous@1`** ("Capture across + continuous", extends `cv.continuous`). Everything else is inherited.

## Alternatives considered
- **One variant plus a "continuous sowing" switch:** fewer list entries, but every combination would need its own records and rules page anyway; separate variants keep each one a fixed, named ruleset.
- **A boolean `capture_across`:** an enum leaves room for other capture styles (Kalah also takes the landing seed; some games capture on both sides) without a breaking change.

## Consequences
- Older apps don't know `cv.across@1` or `cv.across-continuous@1`; their records are `deferred` by an older server until it's updated.
- The rules rest on one family's report (confidence: low); their test vectors are tagged `to-confirm`.
