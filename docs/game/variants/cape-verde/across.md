---
id: cv.across
name: Ouril (capture across)
local_names: [Ouril, Uril, Ouri]
country: cv
country_name: Cape Verde
default_for_country: false
extends: cv.standard
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
  skip_origin_on_lap: true
  single_seed_rule: only_if_no_larger_pit
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

# Ouril, capture across (`cv.across`)

> Standard Ouril, but captures happen across the board: from the pit opposite an empty pit of your own where your last seed lands. **Status:** Implemented (one player report; no published source). [ADR 0024](../../../decisions/0024-capture-across.md).

## Rules at a glance

- **Captures happen across, from your own side.** When your last seed lands in an empty pit on your side and the pit directly across has seeds, you capture all of them, however many there are. The seed you dropped stays.
- **Chain:** then look at the pit you sowed just before it. If it was empty too (it now holds 1) and the pit across has seeds, capture those as well, and so on backwards, until a pit doesn't qualify or your row ends.
- **No captures on your opponent's side:** landing there never captures, not even on 2 or 3.
- Capturing is required when it's possible.
- **Strategy:** don't pile many seeds into one pit (they can be taken in one go), and keep your opponent's pits from running empty.
- Everything else is as in [standard Ouril](standard.md).

## Evidence by parameter

| Parameter | Value | Confidence | Evidence |
|---|---|---|---|
| `capture_mode` | `across_from_empty_own_pit` | low | P1: "you can only eat if the seed falls in an empty socket on your side and the opposite side has seeds"; "there is no limit to how much you can eat". The landing seed stays (P1 confirmed). |
| `chain_capture` | `backward` | low | P1: "there is chain": each own pit sown just before that now holds 1 seed, with seeds across, is captured from too (P1 confirmed the worked example). |
| `capture_mandatory` | `true` | low | P1 confirmed (required, as in standard Ouril). |
| `must_feed`, `no_feed_outcome` | as `cv.standard` | low | P1 confirmed: a player with no seeds must be fed; if they can't be, the game ends and the mover keeps the seeds on their side. |
| `win_threshold` | `25` | low | P1 confirmed. |
| Other parameters | as the parent | inherited | P1: "the rest of the standard rules stay".

## Rule interactions

- **Grand slam:** inherited (captures all, extra turn, must feed). A chain of across captures that takes every seed on the opponent's side is a grand slam.
- **Feeding:** unchanged; a move feeds if it drops a seed on the opponent's side.

## Regional sub-variants found

None.

## Open questions

1. Is this rule played beyond the family? Confirmation would raise the confidence.
