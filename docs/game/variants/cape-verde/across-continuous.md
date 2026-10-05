---
id: cv.across-continuous
name: Ouril (capture across, continuous sowing)
local_names: [Ouril, Uril, Ouri]
country: cv
country_name: Cape Verde
default_for_country: false
extends: cv.continuous
version: 1
status: implemented
researched_on: 2026-10-04

# Values that differ from the parent. Keys/values ONLY from the parameters table
# in docs/game/variants/README.md. Use null for "unknown, needs confirmation".
overrides:
  capture_mode: across_from_empty_own_pit

# Full resolved config (parent values + overrides). Used directly by the engine.
resolved:
  pits_per_side: 6
  seeds_per_pit: 4
  direction: ccw
  skip_origin_on_lap: false
  single_seed_rule: allowed
  capture_counts: [2, 3]
  capture_mandatory: true
  chain_capture: backward
  grand_slam: captures_all_extra_turn_must_feed
  must_feed: true
  feeding_overrides_single_seed_rule: true
  no_feed_outcome: mover_takes_own_side
  win_threshold: 25
  endless_cycle_outcome: each_takes_own_side
  endless_cycle_move_limit: 100
  first_player: loser_starts_next
  match_scoring: { big_win_threshold: 36, big_win_points: 2 }
  relay_sowing: own_side_occupied
  capture_mode: across_from_empty_own_pit

confidence:
  capture_mode: low
  chain_capture: low
  capture_mandatory: low
  must_feed: low
  win_threshold: low
  pits_per_side: inherited
  seeds_per_pit: inherited
  direction: inherited
  skip_origin_on_lap: inherited
  single_seed_rule: inherited
  capture_counts: inherited
  grand_slam: inherited
  feeding_overrides_single_seed_rule: inherited
  no_feed_outcome: inherited
  endless_cycle_outcome: inherited
  endless_cycle_move_limit: inherited
  first_player: inherited
  match_scoring: inherited

unsupported_rules: []

sources:
  - ref: P1
    title: Player report from the project owner (a family rule)
    url: null
    kind: player_report
    language: en
    accessed: 2026-10-04
---

# Ouril, capture across, continuous sowing (`cv.across-continuous`)

> Capture across combined with continuous sowing: sowing carries on from occupied own pits, and the move ends with a capture across when the last seed lands in an empty pit of your own. **Status:** Implemented (one player report; no published source). [ADR 0024](../../../decisions/0024-capture-across.md).

## Rules at a glance

- **Captures happen across, from your own side.** When your last seed lands in an empty pit on your side and the pit directly across has seeds, you capture all of them, however many there are. The seed you dropped stays.
- **Chain:** then look at the pit you sowed just before it. If it was empty too (it now holds 1) and the pit across has seeds, capture those as well, and so on backwards, until a pit doesn't qualify or your row ends.
- **No captures on your opponent's side:** landing there never captures, not even on 2 or 3.
- Capturing is required when it's possible.
- **Strategy:** don't pile many seeds into one pit (they can be taken in one go), and keep your opponent's pits from running empty.
- Sowing is continuous, as in [continuous sowing](continuous.md): when your last seed lands in one of your pits that already had seeds, pick them up and keep sowing. A move therefore ends either in an empty pit of yours (where you may capture across) or on your opponent's side (no capture).

## Evidence by parameter

| Parameter | Value | Confidence | Evidence |
|---|---|---|---|
| `capture_mode` | `across_from_empty_own_pit` | low | P1: "you can only eat if the seed falls in an empty socket on your side and the opposite side has seeds"; "there is no limit to how much you can eat". The landing seed stays (P1 confirmed). |
| `chain_capture` | `backward` | low | P1: "there is chain": each own pit sown just before that now holds 1 seed, with seeds across, is captured from too (P1 confirmed the worked example). |
| `capture_mandatory` | `true` | low | P1 confirmed (required, as in standard Ouril). |
| `must_feed`, `no_feed_outcome` | as `cv.standard` | low | P1 confirmed: a player with no seeds must be fed; if they can't be, the game ends and the mover keeps the seeds on their side. |
| `win_threshold` | `25` | low | P1 confirmed. |
| Other parameters | as the parent | inherited | P1: "the rest of the standard rules stay".
| `relay_sowing` | `own_side_occupied` | low | P1: "you can also play this variant with the continuous"; inherited from `cv.continuous`. |

## Rule interactions

- **Grand slam:** inherited (captures all, extra turn, must feed). A chain of across captures that takes every seed on the opponent's side is a grand slam.
- **Feeding:** unchanged; a move feeds if it drops a seed on the opponent's side.
- **Relay and capture:** a relay always ends in an empty own pit (capture across possible) or on the opponent's side (no capture), so the two rules never conflict.

## Regional sub-variants found

None.

## Open questions

1. Is this combination played beyond the family? Confirmation would raise the confidence.
