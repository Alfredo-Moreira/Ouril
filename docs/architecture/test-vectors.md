# Test vectors

> Language-neutral JSON test cases that prove every platform plays the rules identically. **Status:** Accepted

## Purpose

The Rust core is used on iOS, Android, web and the server through different bindings. **Test vectors** are JSON files describing a position, the moves played, and the expected outcome. They run:

- in Rust (`cargo test`), directly against `ouril-engine`
- through the **Swift** and **Kotlin** bindings (UniFFI) in the iOS and Android CI jobs
- through the **WASM/TypeScript** bindings in the web CI job

If a vector passes everywhere, every platform agrees on the rules. Vectors are also the executable form of the variant specs and of the examples in [rules.md](../game/rules.md#worked-examples).

## Layout

```
core/test-vectors/
├── schema/
│   └── test-vector.schema.json     # JSON Schema for the format below
├── _common/                        # mechanics shared by several variants (run against each listed variant)
│   └── sowing/basic-opening.json
└── cv.standard/                    # one folder per variant ID
    ├── sowing/
    ├── capture/chain-basic.json
    ├── single-seed/
    ├── feeding/
    ├── grand-slam/
    └── end/
```

File names are kebab-case and describe the scenario. Each vector's `id` is its path without `.json`.

## Format

```json
{
  "id": "cv.standard/capture/chain-basic",
  "description": "Last seed makes 3 in pit 7; chain captures 2 in pit 6; stops at own side.",
  "variants": [{ "id": "cv.standard", "version": 1 }],
  "source": "docs/game/rules.md#2-capture-with-a-chain",
  "tags": ["capture", "chain"],
  "setup": {
    "pits":   [4, 4, 4, 4, 3, 0,   1, 2, 4, 4, 4, 4],
    "stores": [5, 5],
    "to_move": "south",
    "moves_since_capture": 0
  },
  "steps": [
    {
      "expect_legal_moves": [0, 1, 2, 3, 4],
      "move": 4,
      "expect": {
        "pits":   [4, 4, 4, 4, 0, 1,   0, 0, 4, 4, 4, 4],
        "stores": [10, 5],
        "to_move": "north",
        "status": "playing",
        "events": [
          { "type": "sow", "pit": 5 },
          { "type": "sow", "pit": 6 },
          { "type": "sow", "pit": 7 },
          { "type": "capture", "pit": 7, "seeds": 3 },
          { "type": "capture", "pit": 6, "seeds": 2 }
        ]
      }
    }
  ]
}
```

### Fields

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Unique, equal to the file path under `core/test-vectors/` without `.json` |
| `description` | yes | One sentence a human can check against the rules |
| `variants` | yes | List of `{id, version}` this vector applies to. `_common` vectors list several. |
| `source` | yes | The doc (and anchor) that justifies the expected behaviour: a rules example or a spec row |
| `tags` | no | Free labels. Two have fixed meanings: `to-confirm` marks behaviour based on a low-confidence or unconfirmed rule, and `app-decision` marks behaviour based on a project decision where no source covers the rule (confidence `decision` in the spec). |
| `setup` | yes | `"initial"` for the variant's starting position (with an optional `first_player`), or an explicit state |
| `setup.pits` | — | Seeds per pit in engine index order (South `0..n-1`, then North), length `2 × pits_per_side` |
| `setup.stores` | — | `[south, north]` captured seeds |
| `setup.to_move` | — | `"south"` \| `"north"` |
| `steps` | yes | Moves applied in order. Each step may assert before and/or after the move. |
| `steps[].expect_legal_moves` | no | Exact sorted list of legal moves **before** this step's move |
| `steps[].move` | no | Pit index to play. Omit it to only assert legal moves. |
| `steps[].expect_error` | no | Instead of `expect`: the error code the move must produce (see below) |
| `steps[].expect` | no | Assertions **after** the move. Only the fields present are checked. |

### Event types

| Event | Fields | Meaning |
|---|---|---|
| `sow` | `pit` | One seed dropped into `pit` |
| `skip_origin` | `pit` | The origin pit was skipped during a lap |
| `capture` | `pit`, `seeds` | Seeds captured from `pit`, listed in capture order |
| `grand_slam` | — | The move captured every opponent seed |
| `extra_turn` | — | The same player moves again (`captures_all_extra_turn_must_feed`) |
| `collect_remaining` | `player`, `seeds` | End-of-game collection of the seeds on one side into that player's store. Emitted only when `seeds > 0` |
| `game_over` | `result`, `reason` | `result`: `south_wins` \| `north_wins` \| `draw`. `reason`: `threshold` \| `no_feed` \| `endless_cycle` \| `no_moves` |

**Event order** within one move:

1. `sow` and `skip_origin` events, in sowing order
2. `capture` events, in capture order (the last pit sown first, then backwards)
3. `grand_slam`, if the move captured every opponent seed
4. then **either** `extra_turn` **or** the end of the game: `collect_remaining` events (South first, only for sides that still have seeds), then `game_over`. Every game end, including a threshold win, collects the remaining seeds this way.

`expect.status` is one of `playing`, `south_wins`, `north_wins` or `draw`.

### Error codes

`not_own_pit`, `empty_pit`, `single_seed_rule`, `must_feed`, `grand_slam_forbidden`, `game_over`.

## Rules for writing vectors

- **Every worked example** in `rules.md` and **every row** in a variant spec's evidence table has at least one vector.
- Behaviour based on a **To confirm** or low-confidence rule is tagged `to-confirm`, and behaviour based on an app decision is tagged `app-decision`. When the rule is confirmed or the decision changes, these are the vectors to review.
- Vectors are **immutable for a released variant version**. A rule change means a new variant version and new or copied vectors ([versioning](versioning.md)).
- Check seed conservation by hand: `sum(pits) + sum(stores)` must equal `2 × pits_per_side × seeds_per_pit` in every state. The runner also asserts this.
- Prefer small, focused vectors over long games. Golden full games go under `<variant>/games/`.

## Runners

| Platform | Runner | CI job |
|---|---|---|
| Rust | `cargo test -p ouril-engine`, which loads every file and validates it against the schema | `core` |
| iOS | XCTest suite that loads the vectors bundled as test resources | `ios` |
| Android | JUnit suite that loads the vectors as test resources | `android` |
| Web | Vitest suite using the WASM package | `web` |

Each runner must report vectors that were **skipped** (for example an unknown variant version) as failures, so nothing is silently skipped.

## Later: sync vectors

The same idea will cover `ouril-sync` (`core/test-vectors/sync/`): game histories as input, expected derived stats and merge results as output. The format will be defined when `ouril-sync` is built.

## Open questions

- Should we also generate random vectors (via property testing) and commit the ones that find bugs as regression tests?
- Format extensions still needed, each one an ADR since this format is a public contract:
  - `setup.history` (positions since the last capture), so endless cycles by **repetition** can be tested. The 100-move limit can already be tested with `setup.moves_since_capture`.
  - `setup.status`, so the `game_over` error can be tested.
  - game- and match-level vectors, for `first_player` and `match_scoring` (tag them `to-confirm`: low confidence in `cv.standard`).
