//! `ouril-sync`: game records, stats derived from them, and sync mutation payloads
//! (`docs/architecture/data-and-sync.md`, `docs/architecture/versioning.md#game-records`).
//!
//! Stats are **never** synced as counters: every platform and the server derive identical
//! numbers from the same list of game records.

use std::collections::BTreeMap;

use ouril_engine::{EndReason, GameResult, Move, Player};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Newest game record format this core reads and writes.
///
/// - **1:** a game the rules ended (`reason` is an engine end reason).
/// - **2:** adds `reason: "resigned"`, a player forfeited before the rules ended the game
///   (ADR 0022). Clients write format 2 only for forfeited games, so servers that only know
///   format 1 keep accepting every other game and defer forfeits until they're updated.
pub const GAME_RECORD_FORMAT: u16 = 2;

/// Format of a game the rules ended (still the format clients write for those games).
pub const GAME_RECORD_FORMAT_RULES: u16 = 1;

/// Why a recorded game ended: the engine's end reasons, plus a forfeit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum RecordEndReason {
    Threshold,
    NoFeed,
    EndlessCycle,
    NoMoves,
    /// The losing side forfeited (format 2). The engine never produces this.
    Resigned,
}

impl From<EndReason> for RecordEndReason {
    fn from(r: EndReason) -> Self {
        match r {
            EndReason::Threshold => Self::Threshold,
            EndReason::NoFeed => Self::NoFeed,
            EndReason::EndlessCycle => Self::EndlessCycle,
            EndReason::NoMoves => Self::NoMoves,
        }
    }
}

/// `id@version` of the variant a game was played with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct VariantRef {
    pub id: String,
    pub version: u16,
}

/// How a finished game ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GameRecordResult {
    pub outcome: GameResult,
    /// Final `[south, north]` stores.
    pub stores: [u8; 2],
    pub reason: RecordEndReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    VsAi,
}

/// A finished game: everything needed to replay it (format 1).
/// Also the payload of the `game_finished@1` sync mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GameRecord {
    pub format: u16,
    /// UUIDv7 generated on the device.
    pub id: String,
    pub variant: VariantRef,
    pub first_player: Player,
    pub moves: Vec<Move>,
    /// Diagnostics only; replay depends only on `variant` and `moves`.
    pub core_version: String,
    pub result: GameRecordResult,
    /// RFC 3339.
    pub started_at: String,
    /// RFC 3339. Stats order games by `ended_at`, then `id`.
    pub ended_at: String,
    /// Defaults to `vs_ai` (the only MVP mode).
    #[serde(default = "vs_ai")]
    pub mode: GameMode,
    /// `"easy" | "medium" | "hard"` for vs-AI games.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ai_level: Option<String>,
    /// The side the local player played (vs AI). Needed to count wins and losses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub human_player: Option<Player>,
}

fn vs_ai() -> GameMode {
    GameMode::VsAi
}

/// Win/loss/draw counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct WinLoss {
    pub played: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

/// Stats derived from game records (MVP: local stats screen).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Stats {
    pub overall: WinLoss,
    /// Keyed by AI level (`"easy"`, `"medium"`, `"hard"`).
    pub by_level: BTreeMap<String, WinLoss>,
    /// Consecutive wins ending with the most recent game.
    pub current_streak: u32,
    pub best_streak: u32,
}

/// Derive stats from game records. Order-independent input: records are de-duplicated by
/// `id` and sorted by `(ended_at, id)` first, so two devices with the same games always
/// compute the same stats.
///
/// - A win or loss is counted from `human_player`'s side. A record without `human_player`
///   counts as played (and as a draw if it was one) but never as a win or a loss.
/// - `by_level` is keyed by `ai_level` (records without one only count in `overall`).
/// - Streaks count consecutive wins; any other game (loss, draw, unknown side) breaks them.
/// - If two records share an `id`, the one with the earliest `(ended_at, started_at)` is
///   kept (ties broken by the record's full content), so the choice is order-independent.
pub fn derive_stats(records: &[GameRecord]) -> Stats {
    let mut sorted: Vec<&GameRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        (&a.id, &a.ended_at, &a.started_at)
            .cmp(&(&b.id, &b.ended_at, &b.started_at))
            .then_with(|| format!("{a:?}").cmp(&format!("{b:?}")))
    });
    sorted.dedup_by(|later, first| later.id == first.id);
    sorted.sort_by(|a, b| (&a.ended_at, &a.id).cmp(&(&b.ended_at, &b.id)));

    let mut stats = Stats::default();
    let mut streak = 0u32;
    for r in sorted {
        let outcome = Outcome::of(r);
        stats.overall.add(outcome);
        if let Some(level) = &r.ai_level {
            stats
                .by_level
                .entry(level.clone())
                .or_default()
                .add(outcome);
        }
        if outcome == Outcome::Win {
            streak += 1;
            stats.best_streak = stats.best_streak.max(streak);
        } else {
            streak = 0;
        }
    }
    stats.current_streak = streak;
    stats
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Win,
    Loss,
    Draw,
    /// A decisive game whose local side is unknown (`human_player` missing).
    Unknown,
}

impl Outcome {
    fn of(r: &GameRecord) -> Outcome {
        match (r.result.outcome, r.human_player) {
            (GameResult::Draw, _) => Outcome::Draw,
            (_, None) => Outcome::Unknown,
            (GameResult::SouthWins, Some(Player::South))
            | (GameResult::NorthWins, Some(Player::North)) => Outcome::Win,
            _ => Outcome::Loss,
        }
    }
}

impl WinLoss {
    fn add(&mut self, o: Outcome) {
        self.played += 1;
        match o {
            Outcome::Win => self.wins += 1,
            Outcome::Loss => self.losses += 1,
            Outcome::Draw => self.draws += 1,
            Outcome::Unknown => {}
        }
    }
}

/// `profile_updated@1` payload: fields changed on the device (absent = unchanged).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ProfileUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub country: Option<String>,
}

/// `settings_updated@1` payload: changed settings (absent = unchanged).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SettingsUpdate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sound: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hints: Option<bool>,
}

/// `handle_requested@1` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct HandleRequest {
    pub handle: String,
}

/// Mutation type constants, re-exported for convenience.
pub use ouril_protocol::mutation_types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_record_matches_versioning_doc_example() {
        let json = r#"{
          "format": 1,
          "id": "0192f1c2-0000-7000-8000-000000000000",
          "variant": { "id": "cv.standard", "version": 1 },
          "first_player": "south",
          "moves": [2, 7, 4, 9],
          "core_version": "0.7.2",
          "result": { "outcome": "south_wins", "stores": [26, 18], "reason": "threshold" },
          "started_at": "2026-10-03T18:20:00Z",
          "ended_at": "2026-10-03T18:31:00Z"
        }"#;
        let r: GameRecord = serde_json::from_str(json).unwrap();
        assert_eq!(r.mode, GameMode::VsAi);
        assert_eq!(r.result.outcome, GameResult::SouthWins);
    }

    fn rec(
        id: &str,
        ended: &str,
        outcome: GameResult,
        human: Option<Player>,
        level: &str,
    ) -> GameRecord {
        GameRecord {
            format: GAME_RECORD_FORMAT_RULES,
            id: id.into(),
            variant: VariantRef {
                id: "cv.standard".into(),
                version: 1,
            },
            first_player: Player::South,
            moves: vec![],
            core_version: "0.1.0".into(),
            result: GameRecordResult {
                outcome,
                stores: [25, 23],
                reason: RecordEndReason::Threshold,
            },
            started_at: "2026-10-01T00:00:00Z".into(),
            ended_at: ended.into(),
            mode: GameMode::VsAi,
            ai_level: Some(level.into()),
            human_player: human,
        }
    }

    use GameResult::*;
    use Player::*;

    fn sample() -> Vec<GameRecord> {
        vec![
            rec("a", "2026-10-01T10:00:00Z", SouthWins, Some(South), "easy"), // win
            rec("b", "2026-10-01T11:00:00Z", NorthWins, Some(North), "easy"), // win
            rec(
                "c",
                "2026-10-01T12:00:00Z",
                NorthWins,
                Some(South),
                "medium",
            ), // loss
            rec("d", "2026-10-01T13:00:00Z", SouthWins, Some(South), "hard"), // win
            rec("e", "2026-10-01T14:00:00Z", Draw, Some(South), "hard"),      // draw
            rec(
                "f",
                "2026-10-01T15:00:00Z",
                SouthWins,
                Some(South),
                "medium",
            ), // win
            rec(
                "g",
                "2026-10-01T16:00:00Z",
                NorthWins,
                Some(North),
                "medium",
            ), // win
        ]
    }

    #[test]
    fn derive_stats_counts_and_streaks() {
        let s = derive_stats(&sample());
        assert_eq!(
            s.overall,
            WinLoss {
                played: 7,
                wins: 5,
                losses: 1,
                draws: 1
            }
        );
        assert_eq!(
            s.by_level["easy"],
            WinLoss {
                played: 2,
                wins: 2,
                losses: 0,
                draws: 0
            }
        );
        assert_eq!(
            s.by_level["medium"],
            WinLoss {
                played: 3,
                wins: 2,
                losses: 1,
                draws: 0
            }
        );
        assert_eq!(
            s.by_level["hard"],
            WinLoss {
                played: 2,
                wins: 1,
                losses: 0,
                draws: 1
            }
        );
        assert_eq!(s.current_streak, 2);
        assert_eq!(s.best_streak, 2);
    }

    #[test]
    fn derive_stats_is_order_independent_and_deduplicates() {
        let mut recs = sample();
        let expected = derive_stats(&recs);
        recs.reverse();
        recs.push(recs[3].clone());
        recs.push(recs[0].clone());
        assert_eq!(derive_stats(&recs), expected);
    }

    #[test]
    fn derive_stats_empty_and_unknown_side() {
        assert_eq!(derive_stats(&[]), Stats::default());
        let mut r = rec("x", "2026-10-02T00:00:00Z", SouthWins, None, "easy");
        r.ai_level = None;
        let s = derive_stats(&[r]);
        assert_eq!(
            s.overall,
            WinLoss {
                played: 1,
                wins: 0,
                losses: 0,
                draws: 0
            }
        );
        assert!(s.by_level.is_empty());
        assert_eq!(s.current_streak, 0);
    }

    #[test]
    fn streak_sorted_by_ended_at_then_id() {
        // Same ended_at: "b" (loss) sorts after "a" (win), so the current streak is 0.
        let recs = vec![
            rec("b", "2026-10-01T10:00:00Z", NorthWins, Some(South), "easy"),
            rec("a", "2026-10-01T10:00:00Z", SouthWins, Some(South), "easy"),
        ];
        let s = derive_stats(&recs);
        assert_eq!(s.current_streak, 0);
        assert_eq!(s.best_streak, 1);
    }
}
