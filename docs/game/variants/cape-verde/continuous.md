---
id: cv.continuous
name: Ouril (continuous sowing)
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
  skip_origin_on_lap: false
  single_seed_rule: allowed
  relay_sowing: own_side_occupied

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

confidence:
  skip_origin_on_lap: low
  pits_per_side: inherited
  seeds_per_pit: inherited
  direction: inherited
  single_seed_rule: low
  capture_counts: low
  capture_mandatory: inherited
  chain_capture: low
  grand_slam: inherited
  must_feed: low
  feeding_overrides_single_seed_rule: inherited
  no_feed_outcome: inherited
  win_threshold: inherited
  endless_cycle_outcome: inherited
  endless_cycle_move_limit: inherited
  first_player: inherited
  match_scoring: inherited
  relay_sowing: low

unsupported_rules:
  - >-
    No turn limit (P1: "no limit"). A simulation found no endless turn: every position with up
    to 11 seeds checked exhaustively (3.9 million moves), 1.3 million random 48-seed moves, and
    an adversarial search whose longest 48-seed turn took 15 laps. This is evidence, not a
    proof, so the engine caps a move at 1000 relay laps (engine convention; never reached in
    practice).

sources:
  - ref: P1
    title: Player report from the project owner, a family rule learned in Santiago
    url: null
    kind: player_report
    language: en
    accessed: 2026-10-04
  - ref: S1
    title: Ouri (jogo), Mancala wiki
    url: https://mancala.fandom.com/wiki/Ouri_(jogo)
    kind: wikipedia
    language: pt
    accessed: 2026-10-04
  - ref: S2
    title: Urim (português), Mancala wiki
    url: https://mancala.fandom.com/wiki/Urim_(portugu%C3%AAs)
    kind: wikipedia
    language: pt
    accessed: 2026-10-04
  - ref: S3
    title: Wuri (Tanji, Gambia)
    url: https://monauale.com/2025/03/26/wuri/
    kind: blog
    language: en
    accessed: 2026-10-04
  - ref: S4
    title: Mancala (relay sowing), Wikipedia
    url: https://en.wikipedia.org/wiki/Mancala
    kind: wikipedia
    language: en
    accessed: 2026-10-04
---

# Ouril with continuous sowing (`cv.continuous`)

> Standard Ouril, but sowing continues: a move keeps going while its last seed lands in an occupied pit on the mover's own side (relay sowing). Reported as a family rule from Santiago. **Status:** Implemented (one player report, a family's rule; no published source). [ADR 0023](../../../decisions/0023-relay-sowing.md).

## Rules at a glance

Everything is as in [standard Ouril](standard.md), except sowing:

- Pick up all the seeds of any of your pits (a lone seed is always allowed) and sow them counter-clockwise, one per pit. **No pit is skipped**, not even the one you started from, however many seeds you have.
- Look at the pit where your last seed landed:
  - **Your side, and it already had seeds:** pick up everything in it (this can be the pit you started from) and keep sowing from the next pit.
  - **Your side, and it was empty:** your move ends.
  - **Your opponent's side:** your move ends, and the normal capture rule applies: 2 or 3 seeds there are captured, and so are the pits before it with 2 or 3, going backwards on your opponent's side.
- There's no limit on how long a move can run.
- If your opponent has no seeds, your move must give them some; seeds dropped on their side during any lap count.

## Evidence by parameter

| Parameter | Value | Confidence | Evidence |
|---|---|---|---|
| `relay_sowing` | `own_side_occupied` | low | P1: "if you land with the last seed in your own side of the board and it has seeds in it, you keep going, grabbing all and seeding the following hole. You only stop if you land in the opposite side." Not found in any published Cape Verdean source: S1 and S2 describe single laps ("voltas simples"); S3 (Wuri, Ouril's Gambian ancestor) has "simple turn sowing", no relay. Relay sowing itself is common in mancala (S4). |
| `skip_origin_on_lap` | `false` | low | P1: "you do not skip [the pit] where you started", and no skip on the first lap either ("no skip"). Differs from `cv.standard` (`true`). |
| `capture_counts` | `[2, 3]` | low | P1: when the move ends on the opponent's side, it "still follows the same regular rules, you can capture". |
| `chain_capture` | `backward` | low | P1 (as above: the regular rules). |
| `single_seed_rule` | `allowed` | low | P1: a lone seed may be played any time, even when another pit holds more ("you can move a single seed into another hole to grab both and start going from there"). Differs from `cv.standard` (`only_if_no_larger_pit`). |
| `must_feed` | `true` | low | P1: you must feed an opponent with no seeds, and seeds dropped on their side during **any** relay lap count, even if the move then captures them back ("it counts as feeding"). |
| Other parameters | as `cv.standard` | inherited | Not discussed; inherited from the parent. |

## Rule interactions

- **Single seeds start relays:** with the single-seed restriction off, a lone seed played into an occupied pit on your own side is a common way to start a relay (P1).
- **Feeding:** a move feeds a starving opponent if any of its laps drops seeds on their side, **even if the move then captures them all back** (P1).
- **Grand slam:** inherited from `cv.standard` (captures all, extra turn, must feed). A feeding move that ends by capturing every seed it gave the opponent is a grand slam, so it captures them, gives an extra turn and the next move must feed again.
- **Endless-cycle rule:** inherited from `cv.standard`.
- **Termination:** see `unsupported_rules`; the engine caps a move at 1000 relay laps.

## Regional sub-variants found

None besides this one. Published sources say Ouril "varies slightly from island to island" (S1) but don't describe Santiago's rules.

## Open questions

1. Is the rule played beyond the family (other Santiago players)? Confirmation would raise the confidence.
