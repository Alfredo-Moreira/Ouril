# ouril-wasm API (web contract)

> The JavaScript/TypeScript API of the Rust core as used by `apps/web`. **Status:** Draft (implemented; `cv.standard@1` passes every test vector). Source: [`src/lib.rs`](src/lib.rs). Shapes match the [test vector format](../../docs/architecture/test-vectors.md) and the [engine design](../../docs/architecture/engine.md).

## Building and loading

- `just bindings` (or `docker compose run --rm toolbox wasm-pack build core/wasm --target web --out-dir pkg --out-name ouril_wasm --release`) writes the package to `core/wasm/pkg/`. It is build output: gitignored, never hand-edited, never committed ([ADR 0015](../../docs/decisions/0015-generated-bindings-not-committed.md)).
- The package is named `ouril-wasm` and is a **pnpm workspace package** (`pnpm-workspace.yaml` lists `core/wasm/pkg`). `apps/web/package.json` depends on it with `"ouril-wasm": "workspace:*"`, so `core/wasm/pkg` must exist **before** `pnpm install`.
- Target `web`: the module must be initialised once before any call.
  - In the app: `apps/web/src/engine/index.ts` exports `loadCore()`, which calls the default export `init()`. Vite serves `ouril_wasm_bg.wasm` as a same-origin static asset, so guests make no other network requests.
  - In Vitest (Node): `apps/web/src/test/wasm.ts` exports `loadCoreSync()`, which reads the `.wasm` file from disk and calls `initSync({ module })`.
- The AI can take hundreds of milliseconds on Hard (about 350 ms for its 1,000,000-node budget in Node on a laptop; Easy and Medium take a few ms). Call `aiMove` from a **Web Worker** (load the module inside the worker with the same `init()`).
- `wasm-opt` is disabled for now (no network download at build time). See Open questions.

```ts
import init, { newGame, legalMoves, applyMove, aiMove } from 'ouril-wasm';

await init();
const variant = { id: 'cv.standard', version: 1 };
let state = newGame(variant, 'south');
const moves = legalMoves(variant, state);          // [0, 1, 2, 3, 4, 5]
const { state: next, events } = applyMove(variant, state, 2);
const reply = aiMove(variant, next, 'medium', 42); // number | undefined
```

## Conventions

- Every value crossing the boundary is a **plain JS object** (JSON-compatible): snake_case keys, snake_case string enums, `number`s. No classes, no BigInt.
- A variant is always passed as a `VariantRef` `{ id, version }`. New games use the latest version (`variants()` / `defaultVariant('cv')`). Replays use the recorded version.
- `GameState.pits` has exactly `2 × pits_per_side` entries: South `0..n-1`, then North `n..2n-1`. Sowing is counter-clockwise by increasing index for `ccw` variants.
- `GameState.history` is opaque (position hashes since the last capture, used for repetition). Store and pass it back unchanged; it is omitted when empty.
- **Never assume turns alternate**: read `to_move` from the returned state (a grand slam can give an extra turn).
- Unknown enum values (new event types, error codes) may appear in later versions. Handle them gracefully ([versioning](../../docs/architecture/versioning.md)).

## Functions

| Function | Signature | Notes |
|---|---|---|
| `coreVersion` | `() => string` | Workspace version of the Rust core. Put it in `GameRecord.core_version`. |
| `variants` | `() => VariantConfig[]` | Every known variant, every version. |
| `defaultVariant` | `(country: string) => VariantConfig \| undefined` | Latest default variant for an ISO 3166-1 alpha-2 code (`"cv"`). |
| `newGame` | `(variant: VariantRef, first: Player) => GameState` | Initial position, `first` to move. The app picks `first` (player choice or random, see Open questions). |
| `legalMoves` | `(variant: VariantRef, state: GameState) => number[]` | Sorted ascending. `[]` when the game is over. Use it for highlighting; illegal pits can't be tapped. |
| `applyMove` | `(variant: VariantRef, state: GameState, pit: number) => MoveResult` | Throws `Error(message = MoveErrorCode)` if illegal. A `pit` that isn't an integer on the board (negative, fractional, too large) is `not_own_pit`. Input is not mutated. |
| `aiMove` | `(variant: VariantRef, state: GameState, level: Level, seed: number) => number \| undefined` | Deterministic for the same inputs. `seed`: integer `0..=Number.MAX_SAFE_INTEGER`. `undefined` only when there is no legal move. |
| `replay` | `(variant: VariantRef, first: Player, moves: number[]) => Replay` | Rebuilds every position and event. Throws `Error("illegal_move:<ply>:<code>")`. |
| `deriveStats` | `(records: GameRecord[]) => Stats` | Order-independent; de-duplicates by `id`; sorts by `(ended_at, id)` for streaks. |

### Errors

Every function throws a JS `Error` on failure. `error.message` is machine-readable:

| `message` | When |
|---|---|
| `not_own_pit`, `empty_pit`, `single_seed_rule`, `must_feed`, `grand_slam_forbidden`, `game_over` | `applyMove` with an illegal move (same codes as test vectors) |
| `illegal_move:<ply>:<code>` | `replay` hit an illegal move at 0-based `ply` |
| `unknown_variant` | The `VariantRef` isn't in the registry |
| `invalid_argument: <detail>` | A value didn't match the expected shape, or a `GameState` doesn't fit the variant (wrong board size, or `sum(pits) + sum(stores)` isn't the variant's total seeds) |

## Types

These are emitted into `ouril_wasm.d.ts` by the build (from a `typescript_custom_section` in `src/lib.rs`). `just protocol-ts` also exports the same core types, plus the API types, to `apps/web/src/generated/protocol/`.

```ts
type Player = 'south' | 'north';
type Status = 'playing' | 'south_wins' | 'north_wins' | 'draw';
type Level = 'easy' | 'medium' | 'hard';
type GameResult = 'south_wins' | 'north_wins' | 'draw';
type EndReason = 'threshold' | 'no_feed' | 'endless_cycle' | 'no_moves';
type RecordEndReason = EndReason | 'resigned'; // records only (format 2, ADR 0022)
type MoveErrorCode =
  | 'not_own_pit' | 'empty_pit' | 'single_seed_rule'
  | 'must_feed' | 'grand_slam_forbidden' | 'game_over';

interface VariantRef { id: string; version: number }

interface GameState {
  pits: number[];               // length 2 × pits_per_side
  stores: [number, number];     // [south, north]
  to_move: Player;
  moves_since_capture: number;
  status: Status;
  history?: string[];           // opaque
}

type MoveEvent =
  | { type: 'sow'; pit: number }
  | { type: 'skip_origin'; pit: number }
  | { type: 'capture'; pit: number; seeds: number }
  | { type: 'grand_slam' }
  | { type: 'extra_turn' }
  | { type: 'collect_remaining'; player: Player; seeds: number }
  | { type: 'game_over'; result: GameResult; reason: EndReason };

interface MoveResult { state: GameState; events: MoveEvent[] }
interface Replay { initial: GameState; steps: MoveResult[] }   // steps[i] = result of moves[i]
```

`VariantConfig` has exactly the keys of the [parameters table](../../docs/game/variants/README.md#parameters) plus `id`, `version`, `name`, `country`, `default_for_country`. `endless_cycle_move_limit` and `match_scoring` may be `null`.

### Event order within one move

Exactly as in [test-vectors.md](../../docs/architecture/test-vectors.md#event-types):

1. `sow` and `skip_origin`, in sowing order
2. `capture`, in capture order (last pit sown first, then backwards)
3. `grand_slam`, if the move captured every opponent seed
4. either `extra_turn`, or the end of the game: `collect_remaining` (South first, only sides with seeds > 0), then `game_over`

Example (`cv.standard/end/threshold-win`): `sow 5, sow 6, sow 7, capture 7 (2), collect_remaining south 1, collect_remaining north 21, game_over south_wins threshold`.

### Game records and stats

```ts
interface GameRecord {
  format: 1 | 2;                 // 2 only for forfeits (reason 'resigned', ADR 0022)
  id: string;                    // UUIDv7, generated on the device
  variant: VariantRef;
  first_player: Player;
  moves: number[];
  core_version: string;          // coreVersion(); diagnostics only
  result: { outcome: GameResult; stores: [number, number]; reason: RecordEndReason };
  started_at: string;            // RFC 3339
  ended_at: string;              // RFC 3339
  mode?: 'vs_ai';                // default 'vs_ai'
  ai_level?: Level;
  human_player?: Player;         // side the local player played; needed for wins/losses
}

interface WinLoss { played: number; wins: number; losses: number; draws: number }
interface Stats {
  overall: WinLoss;
  by_level: Partial<Record<Level, WinLoss>>;
  current_streak: number;        // consecutive wins ending with the latest game
  best_streak: number;
}
```

`GameRecord` is also the payload of the `game_finished@1` sync mutation ([server API](../../apps/server/API.md)).

## Open questions

- `wasm-opt` is off. Install binaryen in the toolbox and enable it for production builds?
- `history` (repetition detection) isn't in the test-vector `setup` format yet (see [test-vectors.md](../../docs/architecture/test-vectors.md#open-questions)).
- `GameRecord.mode`, `ai_level` and `human_player` extend the documented format 1 record; confirm before the format is frozen.
