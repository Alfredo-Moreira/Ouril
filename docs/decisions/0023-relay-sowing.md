# 0023. Relay sowing as a variant parameter, for Ouril of Santiago

- **Status:** Accepted
- **Date:** 2026-10-04

## Context
A family rule from Santiago, Cape Verde ([`cv.santiago-relay`](../game/variants/cape-verde/continuous.md), since renamed `cv.continuous`) uses **relay sowing**: when the last seed of a lap lands in an occupied pit on the mover's own side, the mover picks up its seeds and keeps sowing. The move stops on an empty own pit or anywhere on the opponent's side. Variants are configurations of one engine ([ADR 0004](0004-variant-configurable-rules-engine.md)), and no parameter could express this. Relay sowing is common across the mancala family, so other variants will need it too.

## Decision
- Add the variant parameter **`relay_sowing`**: `none` (default, every existing variant) or `own_side_occupied` (the Santiago rule). Other relay styles become new values when a variant needs them.
- A relay pickup is a new move event, **`relay { pit, seeds }`**, between the `sow` events of consecutive laps, so every UI can animate it. Test vectors and the WASM types gain it.
- **Feeding** under relay sowing: a move feeds the opponent if **any** lap drops a seed on their side, even when the move then captures those seeds back. The engine simulates the laps to decide this.
- `skip_origin_on_lap` refers to the pit the current lap started from.
- **Engine convention:** a move stops after 1000 relay laps. Simulation found no endless turn (the longest 48-seed turn found took 15 laps), so the cap is a safeguard only.
- Ship `cv.santiago-relay@1` (extends `cv.standard`): `relay_sowing: own_side_occupied`, `skip_origin_on_lap: false`, `single_seed_rule: allowed`. Players choose it when starting a game; `cv.standard` stays the default.

## Alternatives considered
- **A per-variant code hook instead of a parameter:** more flexible, but rules would move out of the variant data and into code, against ADR 0004.
- **A boolean `relay: true`:** relay games differ in where the relay happens and when it stops; an enum leaves room for those without a breaking change.

## Consequences
- Older apps don't know `relay` events or `cv.santiago-relay@1`. Records of that variant from a newer client are `deferred` by an older server until it's updated ([versioning](../architecture/versioning.md)).
- The variant rests on one family's report (confidence: low). Its vectors are tagged `to-confirm`.
- AI search handles relay moves through the same engine; relay moves are slightly more expensive to evaluate.

## Notes
- 2026-10-04: before any release, the variant was renamed **`cv.continuous`** ("Continuous sowing") so that players see a generic name. Its rules are unchanged, and its spec still records where the rule came from. No released games used the old ID.
