---
id: cv.standard
name: Ouril (Cape Verde, standard)
local_names: [Ouril, Ouri, Uril, Urim, Oril, Ori, Urinca]
country: cv
country_name: Cape Verde
default_for_country: true
extends: base.oware
version: 1                            # unreleased: may still change until first shipped
status: researched
researched_on: 2026-10-03

overrides:
  single_seed_rule: only_if_no_larger_pit
  grand_slam: captures_all_extra_turn_must_feed
  first_player: loser_starts_next
  match_scoring: { big_win_threshold: 36, big_win_points: 2 }

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
  relay_sowing: none # parameter added later (ADR 0023); single-lap sowing as always

confidence:
  pits_per_side: high
  seeds_per_pit: high
  direction: high
  skip_origin_on_lap: high
  single_seed_rule: high
  capture_counts: high
  capture_mandatory: medium
  chain_capture: high
  grand_slam: medium
  must_feed: high
  feeding_overrides_single_seed_rule: decision
  no_feed_outcome: medium
  win_threshold: high
  endless_cycle_outcome: medium
  endless_cycle_move_limit: decision
  first_player: low              # first game: random (decision)
  match_scoring: low

unsupported_rules: []

sources:
  - ref: S1
    title: "How to Play Ouril: Rules of The Traditional Game (Sal Cabo Verde)"
    url: https://salcaboverde.com/how-to-play-ouril-cape-verdes-traditional-mancala-game/
    kind: blog
    language: en
    accessed: 2026-10-03
  - ref: S2
    title: "Uril (Ludii Portal), citing Braz Dias & Braz Dias 2020, ethnography of Uril on São Vicente"
    url: https://ludii.games/details.php?keyword=Uril
    kind: ludii
    language: en
    accessed: 2026-10-03
  - ref: S3
    title: "OMerkel/Oware: Oware and Ouril with MCTS AI (rule notes)"
    url: https://github.com/OMerkel/Oware
    kind: implementation
    language: en
    accessed: 2026-10-03
  - ref: S4
    title: "Ouri (Wikipédia em português)"
    url: https://pt.wikipedia.org/wiki/Ouri
    kind: wikipedia
    language: pt
    accessed: 2026-10-03
  - ref: S5
    title: "MatheusMáthica: O jogo do Ouri"
    url: http://matheusmathica.blogspot.com/2011/07/o-jogo-do-ouri.html
    kind: blog
    language: pt
    accessed: 2026-10-03
  - ref: S7
    title: "Oware (English Wikipedia): endless cycles and grand slam variants"
    url: https://en.wikipedia.org/wiki/Oware
    kind: wikipedia
    language: en
    accessed: 2026-10-03
  - ref: S8
    title: "Ouri (Portugal), Mancala Wiki (search excerpt only; page returned 402): Portuguese school/tournament rules derived from Cape Verde"
    url: https://mancala.fandom.com/wiki/Ouri_(Portugal)
    kind: wikipedia
    language: pt
    accessed: 2026-10-03
  - ref: D1
    title: "Project decision, 2026-10-03: app defaults where no source covers the rule (see 'Decisions without a source')"
    url: docs/game/variants/cape-verde/standard.md
    kind: decision
    language: en
    accessed: 2026-10-03
  - ref: S6
    title: "Ouri (jogo), Mancala Wiki (search excerpt only; page returned 402)"
    url: https://mancala.fandom.com/wiki/Ouri_(jogo)
    kind: wikipedia
    language: pt
    accessed: 2026-10-03
---

# Ouril: Cape Verde, standard (`cv.standard`)

> The traditional Cape Verdean game, played across the islands and the diaspora. It's close to Abapa but adds a **single-seed restriction** and a **grand slam with an extra turn**. **Status:** Researched.

## Why this is the default for Cape Verde

Every source describes one common Cape Verdean ruleset. S1 notes that Sal "does not have any unique rules of its own, distinct from the standard Cape Verdean Ouril". The academic ethnography from São Vicente (S2) matches it. Island differences (Brava's 2×7 board, chains of four) are regional variants that extend this one.

## Rules at a glance

- 2 × 6 pits, 4 seeds each (48 total). Sow counter-clockwise, starting from one of your own pits.
- With 12 or more seeds, skip the pit you started from.
- **You may not play a pit holding a single seed while any of your pits holds two or more.**
- If your last seed makes 2 or 3 in an opponent's pit, capture it, then keep capturing backwards while the opponent's pits hold 2 or 3.
- If your opponent has no seeds, you must give them some. If you can't, the game ends and you take the seeds on your side.
- **Grand slam** (capturing every seed on the opponent's side) is allowed. You then play again immediately and must leave the opponent at least one seed. The exception is when the grand slam already gives you the win (25+).
- 25 or more seeds wins at once (seeds left on the board go to whoever's side they're on). 24–24 is a draw. In an endless cycle (a repeated position, or 100 moves without a capture), each player takes the seeds on their own side.
- In a series of games, the loser of the last game usually starts the next one. Capturing 36 or more counts as a 2–0 win in match play.

## Evidence by parameter

| Parameter | Value | Confidence | Evidence |
|---|---|---|---|
| `pits_per_side`, `seeds_per_pit` | 6, 4 | high | S1: "two rows of six small pits… four seeds in each"; S2: "2x6 board… four counters in each hole"; S4: "6 compartimentos… 4 sementes" |
| `direction` | ccw | high | S1: "Anti-clockwise"; S2: "anti-clockwise"; S4: "sentido contrário aos ponteiros do relógio" |
| `skip_origin_on_lap` | true | high | S1: "do not drop a seed into that original pit"; S2: "this hole is skipped"; S5: "saltando a casa de onde partir" |
| `single_seed_rule` | only_if_no_larger_pit | high | S1: cannot pick a single-seed pit "if you have any pits containing more than one seed"; S3: "allowed only as long as none other own bowl is holding more than single seeds"; S4: "enquanto houver buracos com mais sementes, não pode mexer nos que contenham apenas uma semente"; S5: same |
| `capture_counts` | [2, 3] | high | S1, S2, S4, S5: last seed in an opponent pit making 2 or 3 |
| `capture_mandatory` | true | medium | S4: earlier pits "podem e devem ser capturadas" ("may and must be captured"); the others describe capture as automatic |
| `chain_capture` | backward | high | S1: "continue capturing backwards"; S2: "in an unbroken line"; S4, S5: "casa(s) anterior(es)" |
| `grand_slam` | captures_all_extra_turn_must_feed | medium | S8 (excerpt): the grand slam "is permitted, and after this move, the player must make a play to feed their opponent"; S3: the player "must take another turn skipping the opponent's turn" and leave the opponent "one seed at least", unless the grand slam wins the game; S1: "you may take them, but you must then feed the opponent a seed on your extra turn" (described as regional) |
| `must_feed` | true | high | S1, S2: "the opponent must play so as to place counters in their row"; S4: "é obrigado a efectuar um movimento que introduza sementes" |
| `feeding_overrides_single_seed_rule` | true | decision | No source covers a case where the only feeding move is from a single-seed pit. D1: feeding wins (see below) |
| `no_feed_outcome` | mover_takes_own_side | medium | S5: "O adversário recolhe as sementes que estão nas suas casas para o seu depósito" ("the opponent collects the seeds in their own pits into their store"); S6 (excerpt): same |
| `win_threshold` | 25 | high | S1, S2, S4: 25 or more; S1: a 24–24 draw is possible |
| `endless_cycle_outcome` | each_takes_own_side | medium | S5: "Cada jogador recolhe as sementes que se encontram nas suas casas" ("each player collects the seeds in their own pits") |
| `endless_cycle_move_limit` | 100 | decision | No source gives a number. S7: an endless cycle is when the same position recurs (players then agree to stop). D1: the app ends the game on a **repeated position** (same pits, same player to move), with a 100-move no-capture limit as a backstop |
| `first_player` | loser_starts_next | low | S1: the winner "often lets the loser begin first in the next round". Nothing covers the first game. D1: the first game is random |
| `match_scoring` | 36 → 2 points | low | S1: capturing 36+ of 48 counts as a "2–0 victory" in tournament scoring |

**Note on S5 and S4:** these Portuguese sources may describe the **Portuguese school "Ouri"** (`pt.ouri`), which comes from Cape Verdean Ouril. They agree with the Cape Verdean sources on everything they cover, but should be re-checked when `pt.ouri` is researched.

## Rule interactions

- **Single-seed rule vs feeding:** if the opponent has no seeds and the only move that feeds them is from a single-seed pit, while your other pits (holding 2+) can't reach them, no source says which rule wins. App decision D1: **feeding wins** (`feeding_overrides_single_seed_rule: true`), so a legal move exists whenever feeding is possible.
- **Grand slam extra turn that can't feed:** S3 says that when this is unavoidable, "the players harvest the remaining seeds on their own rows". The opponent has no seeds at that point, so this matches `no_feed_outcome: mover_takes_own_side`.
- **Grand slam that reaches 25+:** the game ends immediately by `win_threshold`, so no extra turn is needed (S3).
- **Single-seed rule when every pit holds 0 or 1:** single-seed pits are then playable (S1, S4).

## Regional sub-variants found

- **`cv.brava`:** Brava plays on a **2×7 board** (14 pits) (S1). Seed count and win threshold unknown.
- **Chains of four:** "some" places also capture "chains of four seeds" (S1). The island and the exact rule are unknown.
- **`cv.sao-vicente`:** the S2 ethnography studies São Vicente play. Its local rules may differ in details.
- S4/S6: "As regras do jogo Ouri variam ligeiramente de ilha para ilha" (the rules "vary slightly from island to island").

## Decisions without a source

No source covers these points. They're **app decisions** (D1, 2026-10-03), chosen to keep the game playable and consistent. If players from a community confirm a different practice, apply it with `/rule-change`.

1. **Feeding beats the single-seed rule.** If the opponent has no seeds and the only feeding moves are from single-seed pits, those moves are legal. Rationale: every source makes feeding an obligation ("é obrigado"), while the single-seed rule is a preference among your own moves. This also guarantees a legal move exists whenever feeding is possible.
2. **Endless cycle:** the game ends when a position repeats (same pits and player to move; stores can't differ without a capture), or after **100 moves in a row without a capture**, whichever comes first. Then each player collects the seeds on their own side. Rationale: S7 defines a cycle as a repeated position, and the limit protects against long, pointless endgames, especially against the AI.
3. **First game:** the first player is chosen at random. In a series, the loser of the previous game starts (S1).
4. **Match scoring** (36+ = 2 points) applies only in match mode. Single games just record a win, loss or draw.
5. **Remaining seeds on a threshold win:** when a player reaches 25, the game ends at once and the seeds left on the board go to the player on whose side they are. Rationale: it matches the endless-cycle and no-feed outcomes, gives a complete final score, and can't change the winner (25 is already more than half of 48). This is an engine-wide convention, not a variant parameter.

## Open questions

Community confirmation is welcome on these, but none of them blocks implementation:

1. Is the grand slam extra turn played on every island, or regional, as S1 suggests? (S3 and S8 support it as the default.)
2. Is capture always mandatory (S4: "podem e devem"), or optional anywhere?
3. Do players recognise the app decisions above, or do they settle them differently?
4. Brava (`cv.brava`): is the board really 2×7 (14 pits)? S8 and the Portuguese school rules count the 2 stores in a "14-hole" board, so S1's claim may be the same mix-up. Seed count and win threshold unknown either way.
