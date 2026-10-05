//! `ouril-wasm`: wasm-bindgen exports of the Ouril core for `apps/web`.
//!
//! Contract: `core/wasm/API.md`. Values cross the boundary as plain JS objects (via
//! serde-wasm-bindgen, JSON-compatible) with the same shapes as the test vectors.
//! Built by `just bindings` (`wasm-pack build core/wasm --target web --out-dir pkg`) into
//! `core/wasm/pkg/` (build output, gitignored, never hand-edited; ADR 0015).
//!
//! Errors are thrown as JS `Error`s whose `message` is a machine-readable code:
//! a move error code (`not_own_pit`, `empty_pit`, `single_seed_rule`, `must_feed`,
//! `grand_slam_forbidden`, `game_over`), `unknown_variant`, or `invalid_argument: <detail>`.

use ouril_ai::Level;
use ouril_engine::{GameState, Move, Player};
use ouril_sync::GameRecord;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export type Player = "south" | "north";
export type Status = "playing" | "south_wins" | "north_wins" | "draw";
export type Level = "easy" | "medium" | "hard";
export type GameResult = "south_wins" | "north_wins" | "draw";
export type EndReason = "threshold" | "no_feed" | "endless_cycle" | "no_moves";
/** Why a recorded game ended: an engine end reason, or a forfeit (record format 2). */
export type RecordEndReason = EndReason | "resigned";
export type MoveErrorCode =
  | "not_own_pit" | "empty_pit" | "single_seed_rule"
  | "must_feed" | "grand_slam_forbidden" | "game_over";

/** `id@version` of a variant. */
export interface VariantRef { id: string; version: number }

/** Resolved variant rules (keys from docs/game/variants/README.md). */
export interface VariantConfig {
  id: string; version: number; name: string; country: string; default_for_country: boolean;
  pits_per_side: number; seeds_per_pit: number; direction: "ccw" | "cw";
  skip_origin_on_lap: boolean; single_seed_rule: "allowed" | "only_if_no_larger_pit";
  capture_counts: number[]; capture_mandatory: boolean; chain_capture: "backward" | "none";
  grand_slam: "allowed_no_capture" | "forbidden" | "captures_all"
    | "captures_all_extra_turn_must_feed" | "loses";
  must_feed: boolean; feeding_overrides_single_seed_rule: boolean;
  no_feed_outcome: "mover_takes_own_side" | "remaining_not_scored" | "each_takes_own_side";
  win_threshold: number;
  endless_cycle_outcome: "each_takes_own_side" | "remaining_not_scored" | "draw";
  endless_cycle_move_limit: number | null;
  first_player: "random" | "loser_starts_next";
  match_scoring: { big_win_threshold: number; big_win_points: number } | null;
  /** ADR 0023: "own_side_occupied" keeps sowing from an occupied own pit. */
  relay_sowing: "none" | "own_side_occupied";
  /** ADR 0024: "across_from_empty_own_pit" captures across from an empty own pit. */
  capture_mode: "opponent_side" | "across_from_empty_own_pit";
}

/** A position. `pits.length === 2 * pits_per_side`. Treat `history` as opaque. */
export interface GameState {
  pits: number[];
  stores: [number, number];
  to_move: Player;
  moves_since_capture: number;
  status: Status;
  history?: string[];
}

export type MoveEvent =
  | { type: "sow"; pit: number }
  | { type: "skip_origin"; pit: number }
  /** Relay sowing: `seeds` picked up from `pit` and sown on (ADR 0023). */
  | { type: "relay"; pit: number; seeds: number }
  | { type: "capture"; pit: number; seeds: number }
  | { type: "grand_slam" }
  | { type: "extra_turn" }
  | { type: "collect_remaining"; player: Player; seeds: number }
  | { type: "game_over"; result: GameResult; reason: EndReason };

export interface MoveResult { state: GameState; events: MoveEvent[] }
export interface Replay { initial: GameState; steps: MoveResult[] }

export interface GameRecord {
  /** 1: ended by the rules; 2: may also be a forfeit (`reason: "resigned"`). */
  format: number;
  id: string;
  variant: VariantRef;
  first_player: Player;
  moves: number[];
  core_version: string;
  result: { outcome: GameResult; stores: [number, number]; reason: RecordEndReason };
  started_at: string;
  ended_at: string;
  mode?: "vs_ai";
  ai_level?: Level;
  human_player?: Player;
}

export interface WinLoss { played: number; wins: number; losses: number; draws: number }
export interface Stats {
  overall: WinLoss;
  by_level: Partial<Record<Level, WinLoss>>;
  current_streak: number;
  best_streak: number;
}
"#;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&ser)
        .map_err(|e| JsError::new(&format!("invalid_argument: {e}")))
}

fn from_js<T: for<'de> Deserialize<'de>>(value: JsValue, what: &str) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsError::new(&format!("invalid_argument: {what}: {e}")))
}

#[derive(Deserialize)]
struct VariantRefArg {
    id: String,
    version: u16,
}

fn resolve_variant(variant: JsValue) -> Result<&'static ouril_engine::VariantConfig, JsError> {
    let r: VariantRefArg = from_js(variant, "variant")?;
    ouril_engine::variant_version(&r.id, r.version).ok_or_else(|| JsError::new("unknown_variant"))
}

/// Parse a `GameState` from JS and check it fits the variant (board size, seed
/// conservation), so malformed input is an `invalid_argument` error, never a panic.
fn state_from_js(v: &ouril_engine::VariantConfig, state: JsValue) -> Result<GameState, JsError> {
    let s: GameState = from_js(state, "state")?;
    ouril_engine::validate_state(v, &s)
        .map_err(|e| JsError::new(&format!("invalid_argument: state: {e}")))?;
    Ok(s)
}

/// Version of the Rust core compiled into this package (for `GameRecord.core_version`).
#[wasm_bindgen(js_name = coreVersion)]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Every known variant (all versions).
#[wasm_bindgen(js_name = variants, unchecked_return_type = "VariantConfig[]")]
pub fn variants() -> Result<JsValue, JsError> {
    to_js(&ouril_engine::variants())
}

/// The default variant for a country code (`"cv"`), or `undefined`.
#[wasm_bindgen(js_name = defaultVariant, unchecked_return_type = "VariantConfig | undefined")]
pub fn default_variant(country: &str) -> Result<JsValue, JsError> {
    match ouril_engine::default_variant(country) {
        Some(v) => to_js(v),
        None => Ok(JsValue::UNDEFINED),
    }
}

/// Initial position with `first` to move.
#[wasm_bindgen(js_name = newGame, unchecked_return_type = "GameState")]
pub fn new_game(
    #[wasm_bindgen(unchecked_param_type = "VariantRef")] variant: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Player")] first: JsValue,
) -> Result<JsValue, JsError> {
    let v = resolve_variant(variant)?;
    let first: Player = from_js(first, "first")?;
    to_js(&ouril_engine::new_game(v, first))
}

/// Legal moves (pit indices, ascending) for the player to move. Empty when the game is over.
#[wasm_bindgen(js_name = legalMoves, unchecked_return_type = "number[]")]
pub fn legal_moves(
    #[wasm_bindgen(unchecked_param_type = "VariantRef")] variant: JsValue,
    #[wasm_bindgen(unchecked_param_type = "GameState")] state: JsValue,
) -> Result<JsValue, JsError> {
    let v = resolve_variant(variant)?;
    let s = state_from_js(v, state)?;
    to_js(&ouril_engine::legal_moves(v, &s))
}

/// Apply a move. Throws an `Error` whose message is a `MoveErrorCode` if it's illegal.
#[wasm_bindgen(js_name = applyMove, unchecked_return_type = "MoveResult")]
pub fn apply_move(
    #[wasm_bindgen(unchecked_param_type = "VariantRef")] variant: JsValue,
    #[wasm_bindgen(unchecked_param_type = "GameState")] state: JsValue,
    pit: f64,
) -> Result<JsValue, JsError> {
    let v = resolve_variant(variant)?;
    let s = state_from_js(v, state)?;
    // Any number outside the board (negative, fractional, too large) is `not_own_pit`,
    // the same code the engine gives for an out-of-range pit.
    let pit: Move = if pit.is_finite() && pit >= 0.0 && pit.fract() == 0.0 && pit <= 255.0 {
        pit as Move
    } else {
        return Err(JsError::new(ouril_engine::MoveError::NotOwnPit.code()));
    };
    let result = ouril_engine::apply_move(v, &s, pit).map_err(|e| JsError::new(e.code()))?;
    to_js(&result)
}

/// The AI's move, or `undefined` when there is no legal move. Deterministic for the same
/// inputs. `seed` must be a non-negative integer <= Number.MAX_SAFE_INTEGER.
/// Run it in a Web Worker: Hard can take hundreds of milliseconds.
#[wasm_bindgen(js_name = aiMove, unchecked_return_type = "number | undefined")]
pub fn ai_move(
    #[wasm_bindgen(unchecked_param_type = "VariantRef")] variant: JsValue,
    #[wasm_bindgen(unchecked_param_type = "GameState")] state: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Level")] level: JsValue,
    seed: f64,
) -> Result<JsValue, JsError> {
    let v = resolve_variant(variant)?;
    let s = state_from_js(v, state)?;
    let level: Level = from_js(level, "level")?;
    if !(seed.is_finite() && seed >= 0.0 && seed.fract() == 0.0 && seed <= 9_007_199_254_740_991.0)
    {
        return Err(JsError::new("invalid_argument: seed"));
    }
    match ouril_ai::choose_move(v, &s, level, seed as u64) {
        Some(m) => Ok(JsValue::from(m)),
        None => Ok(JsValue::UNDEFINED),
    }
}

/// Replay a move list from the initial position. Throws `illegal_move:<ply>:<code>` if a
/// move is illegal.
#[wasm_bindgen(js_name = replay, unchecked_return_type = "Replay")]
pub fn replay(
    #[wasm_bindgen(unchecked_param_type = "VariantRef")] variant: JsValue,
    #[wasm_bindgen(unchecked_param_type = "Player")] first: JsValue,
    #[wasm_bindgen(unchecked_param_type = "number[]")] moves: JsValue,
) -> Result<JsValue, JsError> {
    let v = resolve_variant(variant)?;
    let first: Player = from_js(first, "first")?;
    let moves: Vec<Move> = from_js(moves, "moves")?;
    let r = ouril_engine::replay(v, first, &moves)
        .map_err(|e| JsError::new(&format!("illegal_move:{}:{}", e.ply, e.error.code())))?;
    to_js(&r)
}

/// Stats derived from finished game records (order-independent, de-duplicated by `id`).
#[wasm_bindgen(js_name = deriveStats, unchecked_return_type = "Stats")]
pub fn derive_stats(
    #[wasm_bindgen(unchecked_param_type = "GameRecord[]")] records: JsValue,
) -> Result<JsValue, JsError> {
    let records: Vec<GameRecord> = from_js(records, "records")?;
    to_js(&ouril_sync::derive_stats(&records))
}
