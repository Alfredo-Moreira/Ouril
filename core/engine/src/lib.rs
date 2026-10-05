//! `ouril-engine`: the Ouril rules engine.
//!
//! Pure and deterministic: no I/O, randomness or clocks (see `docs/architecture/engine.md`).
//! Behaviour comes from a [`VariantConfig`], never from hard-coded rules (ADR 0004).
//! Rules come **only** from the variant specs in `docs/game/variants/` and the test vectors
//! in `core/test-vectors/`.
//!
//! JSON shapes (serde) match `docs/architecture/test-vectors.md`: snake_case enum values,
//! `pits` of length `2 × pits_per_side`, events tagged by `"type"`.
//!
//! Engine conventions for points no spec covers (see `docs/architecture/engine.md`):
//! - End checks after a move run in this order: win threshold, then "the next player can't
//!   move" (`no_feed`, or `no_moves` when they have no seeds at all), then endless cycle.
//! - A position without `history` (e.g. a test-vector setup) counts itself as the first
//!   position since the last capture.

mod registry;
mod rules;
mod types;

pub use registry::{default_variant, variant, variant_version, variants, variants_for_country};
pub use types::*;

/// Start a new game in the variant's initial position, with `first` to move.
pub fn new_game(v: &VariantConfig, first: Player) -> GameState {
    rules::new_game(v, first)
}

/// Every legal move for the player to move, sorted ascending. Empty when the game is over.
pub fn legal_moves(v: &VariantConfig, s: &GameState) -> Vec<Move> {
    rules::legal_moves(v, s)
}

/// Apply `m` to `s`. The input state is never mutated.
///
/// Clients must read `to_move` from the returned state: turns don't always alternate
/// (grand slam with an extra turn).
pub fn apply_move(v: &VariantConfig, s: &GameState, m: Move) -> Result<MoveResult, MoveError> {
    let mut events = Vec::with_capacity(16);
    let state = rules::apply(v, s, m, &mut events)?;
    Ok(MoveResult { state, events })
}

/// Like [`apply_move`] but without recording events. Same resulting state; for search.
pub fn apply_move_quiet(v: &VariantConfig, s: &GameState, m: Move) -> Result<GameState, MoveError> {
    rules::apply(v, s, m, &mut rules::NoEvents)
}

/// Replay a game record's moves from the initial position. Rebuilds every position and the
/// events of every move (used for history replays, sync validation and bug reports).
pub fn replay(v: &VariantConfig, first: Player, moves: &[Move]) -> Result<Replay, ReplayError> {
    let initial = new_game(v, first);
    let mut steps: Vec<MoveResult> = Vec::with_capacity(moves.len());
    for (ply, &m) in moves.iter().enumerate() {
        let current = steps.last().map(|r| &r.state).unwrap_or(&initial);
        let r = apply_move(v, current, m).map_err(|error| ReplayError { ply, error })?;
        steps.push(r);
    }
    Ok(Replay { initial, steps })
}

/// Zobrist hash of a position (pits and player to move), as stored in `GameState.history`.
pub fn position_hash(s: &GameState) -> PositionHash {
    rules::position_hash(s)
}

/// Check that a variant config is well-formed and uses only behaviour the engine
/// implements. Every embedded variant passes (checked by tests).
pub fn validate(v: &VariantConfig) -> Result<(), String> {
    rules::validate(v)
}

/// Check that a position fits a variant: board size and seed conservation
/// (`sum(pits) + sum(stores) == 2 × pits_per_side × seeds_per_pit`). Use it on untrusted input.
pub fn validate_state(v: &VariantConfig, s: &GameState) -> Result<(), String> {
    rules::validate_state(v, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_cv_standard_as_cape_verde_default() {
        let v = default_variant("cv").expect("cv default");
        assert_eq!(v.id, "cv.standard");
        assert_eq!(v.version, 1);
        assert_eq!(v.pits_per_side, 6);
        assert!(variant("cv.standard").is_some());
        assert!(variant_version("cv.standard", 1).is_some());
        assert!(variant("xx.unknown").is_none());
    }

    #[test]
    fn game_state_json_matches_test_vector_shape() {
        let json = r#"{"pits":[4,4,4,4,4,4,4,4,4,4,4,4],"stores":[0,0],"to_move":"south","moves_since_capture":0,"status":"playing"}"#;
        let s: GameState = serde_json::from_str(json).unwrap();
        assert_eq!(s.pits_per_side, 6);
        assert_eq!(s.status, Status::Playing);
        let back = serde_json::to_value(&s).unwrap();
        assert_eq!(back["pits"].as_array().unwrap().len(), 12);
        assert_eq!(back["status"], "playing");
    }

    #[test]
    fn event_json_shape() {
        let e = MoveEvent::GameOver {
            result: GameResult::SouthWins,
            reason: EndReason::Threshold,
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"game_over","result":"south_wins","reason":"threshold"}"#
        );
        let e = MoveEvent::Capture { pit: 7, seeds: 3 };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"capture","pit":7,"seeds":3}"#
        );
        assert_eq!(
            serde_json::to_string(&MoveError::MustFeed).unwrap(),
            r#""must_feed""#
        );
    }
}
