//! Public engine types. Field names and enum values mirror the parameters table in
//! `docs/game/variants/README.md` (source of truth) and the JSON format in
//! `docs/architecture/test-vectors.md`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Largest board supported: 2 × 7 pits (Brava).
pub const MAX_PITS: usize = 14;

/// A move is the index of the pit to sow from (South `0..n-1`, North `n..2n-1`).
pub type Move = u8;

/// Zobrist hash of a position (pits + player to move), used for repetition detection.
pub type PositionHash = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Player {
    /// Pits `0..pits_per_side`.
    South,
    /// Pits `pits_per_side..2*pits_per_side`.
    North,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::South => Player::North,
            Player::North => Player::South,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Ccw,
    Cw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum SingleSeedRule {
    Allowed,
    OnlyIfNoLargerPit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ChainCapture {
    Backward,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum GrandSlam {
    AllowedNoCapture,
    Forbidden,
    CapturesAll,
    CapturesAllExtraTurnMustFeed,
    Loses,
}

/// What happens to the seeds left on the board when a player can't be fed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum EndOutcome {
    MoverTakesOwnSide,
    RemainingNotScored,
    EachTakesOwnSide,
}

/// What happens when an endless cycle is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum CycleOutcome {
    EachTakesOwnSide,
    RemainingNotScored,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum FirstPlayer {
    Random,
    LoserStartsNext,
}

/// Where captures happen (ADR 0024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    /// On the opponent's side: the last pit sown holds one of `capture_counts`, with
    /// `chain_capture` (Oware).
    #[default]
    OpponentSide,
    /// Across: the last seed lands in an empty pit on the mover's side and the opposite pit
    /// has seeds; all of them are captured (one pit, no limit). `capture_counts` and
    /// `chain_capture` don't apply (`cv.across`).
    AcrossFromEmptyOwnPit,
}

/// Relay sowing (ADR 0023): whether a move continues after its last seed lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum RelaySowing {
    /// One lap: the move ends where the last seed lands.
    #[default]
    None,
    /// When the last seed of a lap lands in an occupied pit on the mover's own side, pick
    /// up every seed there and keep sowing. Stop on an empty own pit or on the opponent's
    /// side (`cv.continuous`).
    OwnSideOccupied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MatchScoring {
    pub big_win_threshold: u8,
    pub big_win_points: u8,
}

/// A variant's complete, resolved rules. Immutable per `(id, version)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct VariantConfig {
    /// `<iso2>.<slug>`, e.g. `cv.standard`.
    pub id: String,
    pub version: u16,
    /// Human-readable English name (UI uses i18n keys where available).
    pub name: String,
    /// ISO 3166-1 alpha-2, lowercase.
    pub country: String,
    pub default_for_country: bool,
    pub pits_per_side: u8,
    pub seeds_per_pit: u8,
    pub direction: Direction,
    pub skip_origin_on_lap: bool,
    pub single_seed_rule: SingleSeedRule,
    pub capture_counts: Vec<u8>,
    pub capture_mandatory: bool,
    pub chain_capture: ChainCapture,
    pub grand_slam: GrandSlam,
    pub must_feed: bool,
    pub feeding_overrides_single_seed_rule: bool,
    pub no_feed_outcome: EndOutcome,
    pub win_threshold: u8,
    pub endless_cycle_outcome: CycleOutcome,
    pub endless_cycle_move_limit: Option<u16>,
    pub first_player: FirstPlayer,
    pub match_scoring: Option<MatchScoring>,
    /// Absent in variant files = `none` (every variant before ADR 0023).
    #[serde(default)]
    pub relay_sowing: RelaySowing,
    /// Absent in variant files = `opponent_side` (every variant before ADR 0024).
    #[serde(default)]
    pub capture_mode: CaptureMode,
}

/// Game status. JSON: `"playing" | "south_wins" | "north_wins" | "draw"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Playing,
    Won(Player),
    Draw,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Playing => "playing",
            Status::Won(Player::South) => "south_wins",
            Status::Won(Player::North) => "north_wins",
            Status::Draw => "draw",
        }
    }
}

impl Serialize for Status {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "playing" => Ok(Status::Playing),
            "south_wins" => Ok(Status::Won(Player::South)),
            "north_wins" => Ok(Status::Won(Player::North)),
            "draw" => Ok(Status::Draw),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["playing", "south_wins", "north_wins", "draw"],
            )),
        }
    }
}

/// A position. Small and cheap to clone; fixed-size pits for speed.
///
/// JSON (via [`GameStateJson`]): `pits` has exactly `2 × pits_per_side` entries, and
/// `pits_per_side` is inferred from its length. `history` (positions since the last capture,
/// for repetition detection) is opaque to clients: round-trip it unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "GameStateJson", try_from = "GameStateJson")]
pub struct GameState {
    /// Only the first `2 * pits_per_side` entries are used; the rest stay 0.
    pub pits: [u8; MAX_PITS],
    pub pits_per_side: u8,
    /// `[south, north]` captured seeds.
    pub stores: [u8; 2],
    pub to_move: Player,
    pub moves_since_capture: u16,
    pub status: Status,
    /// Position hashes since the last capture (repetition detection). Engine-internal.
    pub history: Vec<PositionHash>,
}

impl GameState {
    /// The used part of the board, `2 × pits_per_side` pits.
    pub fn board(&self) -> &[u8] {
        &self.pits[..2 * self.pits_per_side as usize]
    }
}

/// Serialized form of [`GameState`] (the shape used in test vectors and the WASM API).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename = "GameState"))]
pub struct GameStateJson {
    pub pits: Vec<u8>,
    pub stores: [u8; 2],
    pub to_move: Player,
    #[serde(default)]
    pub moves_since_capture: u16,
    #[serde(default = "playing")]
    #[cfg_attr(
        feature = "ts",
        ts(type = "\"playing\" | \"south_wins\" | \"north_wins\" | \"draw\"")
    )]
    pub status: Status,
    /// Opaque (hex-encoded position hashes). Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub history: Vec<String>,
}

fn playing() -> Status {
    Status::Playing
}

impl From<GameState> for GameStateJson {
    fn from(s: GameState) -> Self {
        GameStateJson {
            pits: s.board().to_vec(),
            stores: s.stores,
            to_move: s.to_move,
            moves_since_capture: s.moves_since_capture,
            status: s.status,
            history: s.history.iter().map(|h| format!("{h:016x}")).collect(),
        }
    }
}

impl TryFrom<GameStateJson> for GameState {
    type Error = String;
    fn try_from(j: GameStateJson) -> Result<Self, Self::Error> {
        let n = j.pits.len();
        if n == 0 || !n.is_multiple_of(2) || n > MAX_PITS {
            return Err(format!(
                "pits must have an even length between 2 and {MAX_PITS}, got {n}"
            ));
        }
        let mut pits = [0u8; MAX_PITS];
        pits[..n].copy_from_slice(&j.pits);
        let history = j
            .history
            .iter()
            .map(|h| {
                u64::from_str_radix(h, 16).map_err(|e| format!("invalid history entry {h:?}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GameState {
            pits,
            pits_per_side: (n / 2) as u8,
            stores: j.stores,
            to_move: j.to_move,
            moves_since_capture: j.moves_since_capture,
            status: j.status,
            history,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    SouthWins,
    NorthWins,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Threshold,
    NoFeed,
    EndlessCycle,
    NoMoves,
}

/// One step of a move, for animation. Order within a move: see test-vectors.md "Event order".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveEvent {
    Sow {
        pit: u8,
    },
    SkipOrigin {
        pit: u8,
    },
    /// Relay sowing: the last seed landed in an occupied own pit, whose `seeds` (including
    /// that last seed) are picked up and sown on (ADR 0023).
    Relay {
        pit: u8,
        seeds: u8,
    },
    Capture {
        pit: u8,
        seeds: u8,
    },
    GrandSlam,
    ExtraTurn,
    CollectRemaining {
        player: Player,
        seeds: u8,
    },
    GameOver {
        result: GameResult,
        reason: EndReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveResult {
    /// The new state (the input is never mutated).
    pub state: GameState,
    pub events: Vec<MoveEvent>,
}

/// Error codes exactly as used in test vectors (`expect_error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum MoveError {
    NotOwnPit,
    EmptyPit,
    SingleSeedRule,
    MustFeed,
    GrandSlamForbidden,
    GameOver,
}

impl MoveError {
    pub fn code(self) -> &'static str {
        match self {
            MoveError::NotOwnPit => "not_own_pit",
            MoveError::EmptyPit => "empty_pit",
            MoveError::SingleSeedRule => "single_seed_rule",
            MoveError::MustFeed => "must_feed",
            MoveError::GrandSlamForbidden => "grand_slam_forbidden",
            MoveError::GameOver => "game_over",
        }
    }
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for MoveError {}

/// Result of [`crate::replay`]: the initial position and every move's result, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub initial: GameState,
    pub steps: Vec<MoveResult>,
}

impl Replay {
    /// The final position (the initial one when no moves were played).
    pub fn final_state(&self) -> &GameState {
        self.steps.last().map(|s| &s.state).unwrap_or(&self.initial)
    }
}

/// A move in a replayed record was illegal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayError {
    /// 0-based index into the move list.
    pub ply: usize,
    pub error: MoveError,
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal move at ply {}: {}", self.ply, self.error)
    }
}

impl std::error::Error for ReplayError {}
