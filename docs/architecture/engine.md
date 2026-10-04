# Rules engine

> Design of `core/engine` (Rust): game state, variant configuration, move logic, and how every platform uses it. **Status:** Accepted. See [ADR 0004](../decisions/0004-variant-configurable-rules-engine.md) and [ADR 0007](../decisions/0007-rust-core-with-generated-bindings.md).

## Requirements

- **Pure and deterministic:** the same state plus the same move always gives the same result. No I/O, randomness or clocks.
- **Variant-driven:** behaviour comes from a `VariantConfig`, not hard-coded rules.
- **Fast:** the AI calls it millions of times. State is a small fixed-size value that is cheap to copy.
- **Portable:** a single Rust crate, exposed to Swift and Kotlin via UniFFI, to the web via WebAssembly, and used directly by the server.

## Core types (sketch)

```rust
pub enum Player { South, North }      // South = pits 0–5, North = pits 6–11

// Fields mirror the parameters table in docs/game/variants/README.md (the source of truth).
pub struct VariantConfig {
    pub id: String,                   // "cv.standard", "cv.brava", "gh.abapa", …
    pub version: u16,                 // immutable once released (see versioning.md)
    pub country: String,              // ISO 3166-1 alpha-2: "cv"
    pub default_for_country: bool,
    pub pits_per_side: u8,            // 6 (7 on Brava)
    pub seeds_per_pit: u8,            // 4
    pub direction: Direction,         // Ccw | Cw
    pub skip_origin_on_lap: bool,
    pub single_seed_rule: SingleSeedRule, // Allowed | OnlyIfNoLargerPit
    pub capture_counts: Vec<u8>,      // [2, 3]
    pub capture_mandatory: bool,
    pub chain_capture: ChainCapture,  // Backward | None
    pub grand_slam: GrandSlam,        // AllowedNoCapture | Forbidden | CapturesAll
                                      // | CapturesAllExtraTurnMustFeed | Loses
    pub must_feed: bool,
    pub feeding_overrides_single_seed_rule: bool,
    pub no_feed_outcome: EndOutcome,  // MoverTakesOwnSide | RemainingNotScored | EachTakesOwnSide
    pub win_threshold: u8,            // 25
    pub endless_cycle_outcome: CycleOutcome, // EachTakesOwnSide | RemainingNotScored | Draw
    pub endless_cycle_move_limit: Option<u16>, // moves without capture before the cycle rule applies
    pub first_player: FirstPlayer,    // Random | LoserStartsNext
    pub match_scoring: Option<MatchScoring>, // e.g. 36+ captured = 2 points
}

pub struct GameState {
    pub pits: [u8; MAX_PITS],         // fixed-size for speed; first 2 * pits_per_side used
    pub stores: [u8; 2],
    pub to_move: Player,
    pub moves_since_capture: u16,
    pub status: Status,               // Playing | Won(Player) | Draw
}

pub type Move = u8;                   // pit index to sow from

pub fn new_game(v: &VariantConfig, first: Player) -> GameState;
pub fn legal_moves(v: &VariantConfig, s: &GameState) -> Vec<Move>;
pub fn apply_move(v: &VariantConfig, s: &GameState, m: Move) -> Result<MoveResult, MoveError>;

pub struct MoveResult {
    pub state: GameState,             // new state (input never mutated)
    pub events: Vec<MoveEvent>,       // Sow | SkipOrigin | Capture | GrandSlam | ExtraTurn
                                      // | CollectRemaining | GameOver
                                      // order within a move: see test-vectors.md "Event order"
}

pub enum MoveError {                  // codes as used in test vectors
    NotOwnPit,                        // not_own_pit
    EmptyPit,                         // empty_pit
    SingleSeedRule,                   // single_seed_rule
    MustFeed,                         // must_feed
    GrandSlamForbidden,               // grand_slam_forbidden
    GameOver,                         // game_over
}
```

### End of game

Whenever a game ends, the seeds left on the board go into the stores. After a failed feed or an endless cycle, they follow the variant's `no_feed_outcome` or `endless_cycle_outcome`. After a threshold win, each player takes their own side. That last rule is engine-wide, because it can't change the winner. The engine emits one `collect_remaining` event per side that still has seeds, then `game_over`.

### Endless cycles

The engine ends a game as an endless cycle when **a position repeats** (same pits and player to move) or after `endless_cycle_move_limit` moves without a capture, whichever comes first, then applies `endless_cycle_outcome`. Repetition is tracked with a position hash (Zobrist) of the positions since the last capture. Positions before a capture can never recur, because a capture removes seeds from the board for good.

### Variant registry

- Each variant spec (`docs/game/variants/<country>/<variant>.md`, produced by the `variant-research` skill) has a `resolved` config in its YAML front matter. The implemented ones are copied to `core/engine/variants/<id>.toml`, and the engine embeds them at build time.
- Variants form a tree: `base.oware` → country default → regional variants (`extends`). The build checks that every variant resolves to a complete config, and that each country has exactly one default.
- Public API: `variants()`, `variants_for_country(cc)`, `default_variant(cc)`, `variant(id)`.
- A grand slam with an extra turn (`CapturesAllExtraTurnMustFeed`) means `to_move` doesn't always alternate. Clients must read `to_move` from the returned state, never assume turns alternate.

`events` lets every UI (SwiftUI, Compose, web) animate each seed drop and capture exactly as the engine computed it. No platform re-implements any rule.

### How each platform sees it

| Platform | Access | Example |
|---|---|---|
| iOS | Generated Swift (UniFFI) | `let result = try applyMove(variant: v, state: s, move: 2)` |
| Android | Generated Kotlin (UniFFI) | `val result = applyMove(v, s, 2u)` |
| Web | WASM + generated TS types | `const result = applyMove(v, s, 2)` |
| Server | Rust crate directly | `ouril_engine::apply_move(&v, &s, 2)?` |

## Game record

A game is stored as `{ format, variant: {id, version}, first_player, moves: [Move] }` (full format in [versioning](versioning.md#game-records)). Variant configs are immutable for each `version`, so replays are always exact. Replaying the moves rebuilds every position. This format is used for:

- local history and replays (MVP)
- the server's source of truth (online)
- bug reports and test fixtures

## Testing strategy

- **Shared test vectors** (`core/test-vectors/`, format in [test-vectors.md](test-vectors.md)): language-neutral cases (variant, start position, moves) → expected board, stores, events and result. They run in Rust **and** through the Swift, Kotlin and WASM bindings in CI, which proves every platform behaves identically. They include every worked example in [rules.md](../game/rules.md#worked-examples).
- **Property-based tests** (`proptest`), checked over random legal games:
  - seeds are conserved: `sum(pits) + stores = total`
  - no pit goes negative
  - `legal_moves` is never empty while the game is playing
  - captures only happen on the opponent's side
  - every game terminates
- **Golden games:** recorded real games with known results.
- **Variant matrix:** shared scenarios run against all configs, with the expected result per variant.

## Open questions

- Do some variants need hooks beyond config flags (for example captures during sowing in Anan-anan, or multi-round pit loss)? Plan for an optional per-variant rule override trait.
