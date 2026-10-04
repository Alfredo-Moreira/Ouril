---
name: test-vectors
description: Writes or updates Ouril's language-neutral JSON rule test vectors in core/test-vectors/ from a variant spec or a worked example in docs/game/rules.md, then checks them for seed conservation and bookkeeping errors. Use when adding or changing a game rule, implementing a variant, settling a "To confirm" rule, turning a bug report or game record into a regression test, or when the user asks for test vectors, rule tests or golden games.
argument-hint: "[variant-id[@version]] [area | rules.md example | spec parameter]"
allowed-tools: Bash(python3 ${CLAUDE_SKILL_DIR}/scripts/check_vectors.py *)
---

# Rule test vectors

Vectors are the executable form of the variant specs and the contract every platform must pass. **Every expected value must trace to a doc** (a worked example or a spec evidence row), never to general Oware knowledge and never to what the engine currently outputs. If the spec says `null`, unknown or **To confirm**, stop and ask, unless the user agrees to a `to-confirm` vector using the documented `base.oware` default.

## Input

`$ARGUMENTS`: a variant ID (with an optional `@version`) and what to cover, for example `cv.standard capture`, `cv.standard rules.md example 5`, `cv.standard feeding_overrides_single_seed_rule`. If it's empty, cover whatever rule the conversation is changing.

## Read first

1. `docs/architecture/test-vectors.md`: format, event types and order, error codes and the rules for writing vectors. It's authoritative. If anything here disagrees with it, follow that doc.
2. The variant spec `docs/game/variants/<country>/<variant>.md`: `resolved`, `confidence`, the evidence table and **Rule interactions**. For `cv.standard`, also read `docs/game/rules.md` and its worked examples.
3. `docs/architecture/versioning.md#variant-versions`.
4. The existing vectors for that variant: `ls -R core/test-vectors/<id>/`. Extend rather than duplicate.

## Released or not?

Vectors for a **released** variant version are never edited. A version is released once an app build using it has shipped. Ask the user if you're unsure. Before the MVP ships, `cv.standard@1` is unreleased and may change.

For a released version, the rule change needs a new version first (`rule-change` skill). Then copy the affected vectors, set `variants` to the new version, and change only the copies.

## Plan coverage first

Before writing, list `source → vector path → tags`, and show the list to the user if it has more than about 5 entries. Coverage rules:

- every worked example in `rules.md`: at least one vector
- every evidence-table row in the spec: at least one vector
- every case in the spec's **Rule interactions**: at least one vector
- every error code the variant can produce: at least one `expect_error` vector
- confidence `low`, `unknown` or `conflict`, or text marked **To confirm**: tag `to-confirm`

## Write each vector

1. **Path:** `core/test-vectors/<variant-id>/<area>/<kebab-scenario>.json`. Areas: `sowing`, `capture`, `single-seed`, `feeding`, `grand-slam`, `end`, and `games/` for recorded full games only. Use `_common/` only when the behaviour is identical for every variant listed in `variants`.
2. **`id`** is the path under `core/test-vectors/` without `.json`. **`description`** is one sentence a human can check against the source. **`source`** is the repo-relative doc path plus heading anchor, for example `docs/game/rules.md#5-grand-slam-with-extra-turn`.
3. **Setup:** the smallest explicit position that isolates the rule. Pits are indexed South `0..n-1`, then North `n..2n-1`. Sowing goes to the next index, wrapping around. `stores` is always `[south, north]`. Fill unused pits and stores so `sum(pits) + sum(stores) = 2 × pits_per_side × seeds_per_pit`.
4. **Derive the result by hand**, step by step, in your reasoning (not in the file):
   - one `sow` per seed, in order
   - on a lap, a `skip_origin` event instead of sowing into the origin pit
   - capture check on the last pit, then backwards while on the opponent's side and the count is in `capture_counts`
   - then grand slam, feeding and end-of-game rules from the spec
   - `to_move` comes from the rules: an extra turn keeps the same player
5. **Assert** what the vector is about: `expect_legal_moves` (sorted, before the move) when legality is the point; the complete, ordered `events` (order defined in test-vectors.md); and `pits`, `stores`, `to_move`, `status` after the move. Use `expect_error` instead of `expect` for illegal moves.
6. Keep each vector to one rule and 1–3 steps.

## Check

Run the checker on the files you wrote and fix every problem until it's clean:

```bash
python3 ${CLAUDE_SKILL_DIR}/scripts/check_vectors.py "$(git rev-parse --show-toplevel)" <files or dirs>
```

The checker isn't a rules engine. It catches the following, but not wrong rule logic:

- format and id/path mismatches
- unknown events or error codes
- wrong board size and seed-conservation errors
- sow counts that don't match the moved pit
- store gains that don't match captures
- legal moves on the wrong side
- bad `source` anchors

When the engine exists, also run `cargo test -p ouril-engine`, and run the platform runners when they exist. **If the engine disagrees with a vector, don't edit the vector to match.** Re-derive it from the spec: either the engine has a bug, or the spec is ambiguous and you ask the user.

## Report

- vectors added or changed, each with its source and tags
- how many are tagged `to-confirm`
- spec rows or examples still without vectors
- any ambiguity you found, phrased as an open question to add to the spec

## Pitfalls

- With 12+ seeds the origin is skipped, so the number of `sow` events still equals the seeds picked up, plus one `skip_origin` per lap. See rules.md example 3.
- Chain captures stop at the mover's own side or at the first pit not in `capture_counts`. List captures in capture order: the last pit first.
- Legal moves must apply the single-seed rule **and** the feeding obligation together (rules.md examples 5–6).
- Grand slam (`captures_all_extra_turn_must_feed`) emits `grand_slam` after the captures, then `extra_turn`, and `to_move` stays with the mover. If the store reaches `win_threshold`, the end-of-game events replace the extra turn.
- Never fill in expectations from engine output ("golden master") for rule vectors. Only `games/` replays of real recorded games may use known results.
- There's no JSON Schema file yet (`core/test-vectors/schema/`). Create it with the engine scaffolding, not ad hoc.
