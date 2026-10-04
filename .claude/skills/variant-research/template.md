---
id: <country>.<variant>                # e.g. cv.standard
name: <English display name>
local_names: [<names in local languages and spellings>]
country: <ISO 3166-1 alpha-2, lowercase>
country_name: <English country name>
default_for_country: <true|false>
extends: <parent id, usually base.oware or the country default>
version: <int>                        # 1 for new variants; bump when changing a released variant (docs/architecture/versioning.md)
status: researched                    # researched | implemented
researched_on: <YYYY-MM-DD>

# Values that differ from the parent. Keys/values ONLY from the parameters table
# in docs/game/variants/README.md. Use null for "unknown, needs confirmation".
overrides:
  <parameter>: <value>

# Full resolved config (parent values + overrides). Used directly by the engine.
resolved:
  pits_per_side: <int>
  seeds_per_pit: <int>
  direction: <ccw|cw>
  skip_origin_on_lap: <bool>
  single_seed_rule: <allowed|only_if_no_larger_pit>
  capture_counts: [<ints>]
  capture_mandatory: <bool>
  chain_capture: <backward|none>
  grand_slam: <allowed_no_capture|forbidden|captures_all|captures_all_extra_turn_must_feed|loses>
  must_feed: <bool>
  feeding_overrides_single_seed_rule: <bool>
  no_feed_outcome: <mover_takes_own_side|remaining_not_scored|each_takes_own_side>
  win_threshold: <int>
  endless_cycle_outcome: <each_takes_own_side|remaining_not_scored|draw>
  endless_cycle_move_limit: <int or null>
  first_player: <random|loser_starts_next>
  match_scoring: <null or {big_win_threshold: int, big_win_points: int}>

# Confidence for each parameter: high | medium | low | conflict | unknown | inherited | decision
# (decision = no source covers it; the project chose a default, recorded with a D<n> source)
confidence:
  <parameter>: <level>

# Rules found in sources that no parameter can express.
unsupported_rules:
  - <short description + source ref>

sources:
  - ref: S1
    title: <title>
    url: <url>
    kind: <academic|federation|ludii|wikipedia|book|implementation|blog>
    language: <en|pt|fr|…>
    accessed: <YYYY-MM-DD>
---

# <Name> (`<id>`)

> <One-line summary: where it's played and how it differs from its parent.> **Status:** Researched.

## Why this is the default for <country>
<Only when default_for_country is true: the reasoning.>

## Rules at a glance
<Short plain-language rules a player would recognize, 5–10 bullets.>

## Evidence by parameter

| Parameter | Value | Confidence | Evidence |
|---|---|---|---|
| <parameter> | <value> | <level> | S1: "<quote>" (<translation>); S2: … |

## Rule interactions
<How rules combine, e.g. single-seed rule vs feeding; what sources say or don't say.>

## Regional sub-variants found
<Leads for other variants in this country, to add to the catalog as `lead`.>

## Open questions
1. <Question for players from this community>
