//! `ouril-ffi`: UniFFI exports of the Ouril core for the native apps (ADR 0007).
//!
//! Contract: `core/ffi/API.md`. The functions mirror `core/wasm` (the web contract) one for one,
//! and values cross the boundary as **JSON strings** with exactly the same shapes as the web
//! API and the test vectors, so every app reads the same documents and the Kotlin side parses
//! them into its own data classes. Pits and seeds are `i32` (a Kotlin `Int`).
//!
//! Built by `just bindings-android`: `cargo ndk` for the Android ABIs, then this crate's
//! `uniffi-bindgen` for the Kotlin bindings. Both are build output, gitignored and never
//! hand-edited (ADR 0015).
//!
//! Errors are a [`CoreError`] whose message is the same machine-readable code as on the web:
//! a move error code (`not_own_pit`, `empty_pit`, `single_seed_rule`, `must_feed`,
//! `grand_slam_forbidden`, `game_over`), `unknown_variant`, `invalid_argument: <detail>`, or
//! `illegal_move:<ply>:<code>` from `replay`.

use ouril_ai::Level;
use ouril_engine::{GameState, Move, Player, VariantConfig};
use ouril_sync::GameRecord;
use serde::{de::DeserializeOwned, Serialize};

uniffi::setup_scaffolding!();

/// A failed call. Its message (`toString()` in Kotlin, `message` on the exception) is a code.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum CoreError {
    /// An illegal move: the message is the move error code.
    #[error("{0}")]
    Move(String),
    #[error("unknown_variant")]
    UnknownVariant,
    #[error("invalid_argument: {0}")]
    InvalidArgument(String),
    /// `replay` hit an illegal move: `illegal_move:<ply>:<code>`.
    #[error("illegal_move:{0}:{1}")]
    IllegalMove(u32, String),
}

fn to_json<T: Serialize>(value: &T) -> String {
    // Every value here is plain data (no maps with non-string keys), so this can't fail.
    serde_json::to_string(value).expect("core values serialize to JSON")
}

fn from_json<T: DeserializeOwned>(json: &str, what: &str) -> Result<T, CoreError> {
    serde_json::from_str(json).map_err(|e| CoreError::InvalidArgument(format!("{what}: {e}")))
}

fn variant(id: &str, version: i32) -> Result<&'static VariantConfig, CoreError> {
    let version = u16::try_from(version).map_err(|_| CoreError::UnknownVariant)?;
    ouril_engine::variant_version(id, version).ok_or(CoreError::UnknownVariant)
}

/// Parse a `GameState` and check it fits the variant (board size, seed conservation), so
/// malformed input is an `invalid_argument` error, never a panic.
fn state(v: &VariantConfig, json: &str) -> Result<GameState, CoreError> {
    let s: GameState = from_json(json, "state")?;
    ouril_engine::validate_state(v, &s)
        .map_err(|e| CoreError::InvalidArgument(format!("state: {e}")))?;
    Ok(s)
}

fn player(name: &str) -> Result<Player, CoreError> {
    from_json(&format!("\"{name}\""), "player")
}

/// Any pit outside the board (negative or too large) is `not_own_pit`, as on the web.
fn pit(pit: i32) -> Result<Move, CoreError> {
    Move::try_from(pit)
        .map_err(|_| CoreError::Move(ouril_engine::MoveError::NotOwnPit.code().to_string()))
}

/// Version of the Rust core compiled into this library (for `GameRecord.core_version`).
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Every known variant (all versions), as a JSON array of `VariantConfig`.
#[uniffi::export]
pub fn variants_json() -> String {
    to_json(&ouril_engine::variants())
}

/// The default variant for a country code (`"cv"`) as a `VariantConfig` JSON object, or null.
#[uniffi::export]
pub fn default_variant_json(country: String) -> Option<String> {
    ouril_engine::default_variant(&country).map(to_json)
}

/// The initial position with `first` (`"south"` or `"north"`) to move, as `GameState` JSON.
#[uniffi::export]
pub fn new_game_json(
    variant_id: String,
    variant_version: i32,
    first: String,
) -> Result<String, CoreError> {
    let v = variant(&variant_id, variant_version)?;
    Ok(to_json(&ouril_engine::new_game(v, player(&first)?)))
}

/// Legal moves (pit indices, ascending) for the player to move. Empty when the game is over.
#[uniffi::export]
pub fn legal_moves(
    variant_id: String,
    variant_version: i32,
    state_json: String,
) -> Result<Vec<i32>, CoreError> {
    let v = variant(&variant_id, variant_version)?;
    let s = state(v, &state_json)?;
    Ok(ouril_engine::legal_moves(v, &s)
        .into_iter()
        .map(i32::from)
        .collect())
}

/// Apply a move: `MoveResult` JSON (`{ state, events }`), or a [`CoreError::Move`] if illegal.
#[uniffi::export]
pub fn apply_move_json(
    variant_id: String,
    variant_version: i32,
    state_json: String,
    pit_index: i32,
) -> Result<String, CoreError> {
    let v = variant(&variant_id, variant_version)?;
    let s = state(v, &state_json)?;
    let result = ouril_engine::apply_move(v, &s, pit(pit_index)?)
        .map_err(|e| CoreError::Move(e.code().to_string()))?;
    Ok(to_json(&result))
}

/// The computer's move (`level` is `"easy"`, `"medium"` or `"hard"`), or null when there is
/// no legal move. Deterministic for the same inputs; `seed` must not be negative. Hard can take
/// hundreds of milliseconds: call it off the main thread.
#[uniffi::export]
pub fn ai_move(
    variant_id: String,
    variant_version: i32,
    state_json: String,
    level: String,
    seed: i64,
) -> Result<Option<i32>, CoreError> {
    let v = variant(&variant_id, variant_version)?;
    let s = state(v, &state_json)?;
    let level: Level = from_json(&format!("\"{level}\""), "level")?;
    let seed = u64::try_from(seed).map_err(|_| CoreError::InvalidArgument("seed".into()))?;
    Ok(ouril_ai::choose_move(v, &s, level, seed).map(i32::from))
}

/// Replay a move list from the initial position: `Replay` JSON (`{ initial, steps }`), or
/// [`CoreError::IllegalMove`] at the first illegal move.
#[uniffi::export]
pub fn replay_json(
    variant_id: String,
    variant_version: i32,
    first: String,
    moves: Vec<i32>,
) -> Result<String, CoreError> {
    let v = variant(&variant_id, variant_version)?;
    let first = player(&first)?;
    let moves = moves.into_iter().map(pit).collect::<Result<Vec<_>, _>>()?;
    let r = ouril_engine::replay(v, first, &moves)
        .map_err(|e| CoreError::IllegalMove(e.ply as u32, e.error.code().to_string()))?;
    Ok(to_json(&r))
}

/// Stats derived from finished game records (a JSON array of `GameRecord`), as `Stats` JSON.
#[uniffi::export]
pub fn derive_stats_json(records_json: String) -> Result<String, CoreError> {
    let records: Vec<GameRecord> = from_json(&records_json, "records")?;
    Ok(to_json(&ouril_sync::derive_stats(&records)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_runs_through_json_like_on_the_web() {
        let s = new_game_json("cv.standard".into(), 1, "south".into()).unwrap();
        assert_eq!(
            legal_moves("cv.standard".into(), 1, s.clone()).unwrap(),
            vec![0, 1, 2, 3, 4, 5]
        );
        let r: serde_json::Value =
            serde_json::from_str(&apply_move_json("cv.standard".into(), 1, s, 2).unwrap()).unwrap();
        assert_eq!(
            r["events"][0],
            serde_json::json!({ "type": "sow", "pit": 3 })
        );
        assert_eq!(r["state"]["to_move"], "north");
        let next = r["state"].to_string();
        let reply = ai_move("cv.standard".into(), 1, next, "easy".into(), 7).unwrap();
        assert!(matches!(reply, Some(6..=11)));
    }

    #[test]
    fn errors_are_the_web_codes() {
        let s = new_game_json("cv.standard".into(), 1, "south".into()).unwrap();
        let code = |e: CoreError| e.to_string();
        assert_eq!(
            code(apply_move_json("cv.standard".into(), 1, s.clone(), 7).unwrap_err()),
            "not_own_pit"
        );
        assert_eq!(
            code(apply_move_json("cv.standard".into(), 1, s.clone(), -1).unwrap_err()),
            "not_own_pit"
        );
        assert_eq!(
            code(legal_moves("nope".into(), 1, s.clone()).unwrap_err()),
            "unknown_variant"
        );
        assert!(
            code(legal_moves("cv.standard".into(), 1, "{}".into()).unwrap_err())
                .starts_with("invalid_argument: state")
        );
        assert!(
            code(ai_move("cv.standard".into(), 1, s, "easy".into(), -1).unwrap_err())
                .starts_with("invalid_argument")
        );
        assert!(
            code(replay_json("cv.standard".into(), 1, "south".into(), vec![7]).unwrap_err())
                .starts_with("illegal_move:0:")
        );
    }

    #[test]
    fn variants_and_defaults_are_json() {
        let all: serde_json::Value = serde_json::from_str(&variants_json()).unwrap();
        assert!(all
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == "cv.standard"));
        let cv: serde_json::Value =
            serde_json::from_str(&default_variant_json("cv".into()).unwrap()).unwrap();
        assert_eq!(cv["country"], "cv");
        assert!(default_variant_json("zz".into()).is_none());
    }
}
