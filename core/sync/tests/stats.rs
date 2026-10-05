//! Tests for `derive_stats` (`docs/architecture/data-and-sync.md`: stats are derived from
//! game records, never synced as counters, so every device must compute the same numbers).
//!
//! Properties: order independence, idempotent de-duplication, count consistency, streak
//! semantics. Plus unit tests for the documented edge cases and the JSON shapes.

use ouril_engine::{GameResult, Player};
use ouril_sync::*;
use proptest::prelude::*;

fn rec(
    id: &str,
    ended: &str,
    outcome: GameResult,
    human: Option<Player>,
    level: Option<&str>,
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
            stores: [24, 24],
            reason: RecordEndReason::Threshold,
        },
        started_at: "2026-10-01T00:00:00Z".into(),
        ended_at: ended.into(),
        mode: GameMode::VsAi,
        ai_level: level.map(Into::into),
        human_player: human,
    }
}

fn wl(played: u32, wins: u32, losses: u32, draws: u32) -> WinLoss {
    WinLoss {
        played,
        wins,
        losses,
        draws,
    }
}

/// Is this record a win for the local player?
fn is_win(r: &GameRecord) -> bool {
    matches!(
        (r.result.outcome, r.human_player),
        (GameResult::SouthWins, Some(Player::South)) | (GameResult::NorthWins, Some(Player::North))
    )
}

fn arb_record() -> impl Strategy<Value = GameRecord> {
    (
        0u8..12, // small id space so duplicates are common
        0u32..30,
        prop_oneof![
            Just(GameResult::SouthWins),
            Just(GameResult::NorthWins),
            Just(GameResult::Draw)
        ],
        prop_oneof![
            4 => Just(Some(Player::South)),
            4 => Just(Some(Player::North)),
            1 => Just(None)
        ],
        prop_oneof![
            Just(Some("easy")),
            Just(Some("medium")),
            Just(Some("hard")),
            Just(None)
        ],
        0u8..3,
    )
        .prop_map(|(id, minute, outcome, human, level, start)| {
            let mut r = rec(
                &format!("id-{id:02}"),
                &format!("2026-10-01T10:{minute:02}:00Z"),
                outcome,
                human,
                level,
            );
            r.started_at = format!("2026-10-01T0{start}:00:00Z");
            r
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    #[test]
    fn order_independent(recs in prop::collection::vec(arb_record(), 0..40), perm in any::<proptest::sample::Index>()) {
        let base = derive_stats(&recs);
        let mut rev = recs.clone();
        rev.reverse();
        prop_assert_eq!(derive_stats(&rev), base.clone());
        let mut rot = recs.clone();
        if !rot.is_empty() {
            let k = perm.index(rot.len());
            rot.rotate_left(k);
        }
        prop_assert_eq!(derive_stats(&rot), base);
    }

    #[test]
    fn duplicating_every_record_changes_nothing(recs in prop::collection::vec(arb_record(), 0..40)) {
        let mut twice = recs.clone();
        twice.extend(recs.iter().cloned());
        prop_assert_eq!(derive_stats(&twice), derive_stats(&recs));
    }

    #[test]
    fn counts_are_consistent(recs in prop::collection::vec(arb_record(), 0..40)) {
        let s = derive_stats(&recs);
        let ids: std::collections::BTreeSet<&str> = recs.iter().map(|r| r.id.as_str()).collect();
        prop_assert_eq!(s.overall.played as usize, ids.len(), "one game per unique id");
        prop_assert!(s.overall.wins + s.overall.losses + s.overall.draws <= s.overall.played);
        let by_level_played: u32 = s.by_level.values().map(|w| w.played).sum();
        prop_assert!(by_level_played <= s.overall.played);
        for (k, w) in &s.by_level {
            prop_assert!(["easy", "medium", "hard"].contains(&k.as_str()));
            prop_assert!(w.played > 0);
            prop_assert!(w.wins + w.losses + w.draws <= w.played);
        }
        let sum = |f: fn(&WinLoss) -> u32| s.by_level.values().map(f).sum::<u32>();
        prop_assert!(sum(|w| w.wins) <= s.overall.wins);
        prop_assert!(sum(|w| w.losses) <= s.overall.losses);
        prop_assert!(sum(|w| w.draws) <= s.overall.draws);
        prop_assert!(s.current_streak <= s.best_streak);
        prop_assert!(s.best_streak <= s.overall.wins);
    }

    /// With unique ids, the streaks match a direct computation over records sorted by
    /// `(ended_at, id)`.
    #[test]
    fn streaks_match_a_reference_model(recs in prop::collection::vec(arb_record(), 0..40)) {
        let mut uniq: Vec<GameRecord> = vec![];
        for r in recs {
            if !uniq.iter().any(|u| u.id == r.id) {
                uniq.push(r);
            }
        }
        let s = derive_stats(&uniq);
        let mut sorted = uniq.clone();
        sorted.sort_by(|a, b| (&a.ended_at, &a.id).cmp(&(&b.ended_at, &b.id)));
        let (mut cur, mut best) = (0u32, 0u32);
        for r in &sorted {
            if is_win(r) { cur += 1; best = best.max(cur); } else { cur = 0; }
        }
        prop_assert_eq!(s.current_streak, cur);
        prop_assert_eq!(s.best_streak, best);
        prop_assert_eq!(s.overall.wins, sorted.iter().filter(|r| is_win(r)).count() as u32);
    }

    #[test]
    fn stats_json_round_trip(recs in prop::collection::vec(arb_record(), 0..20)) {
        let s = derive_stats(&recs);
        let json = serde_json::to_string(&s).unwrap();
        let back: Stats = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, s);
    }
}

#[test]
fn duplicate_id_keeps_the_earliest_ended_record() {
    // Same id, different results: the earliest (ended_at, started_at) wins, whatever the order.
    let early = rec(
        "x",
        "2026-10-01T10:00:00Z",
        GameResult::SouthWins,
        Some(Player::South),
        Some("easy"),
    );
    let late = rec(
        "x",
        "2026-10-01T11:00:00Z",
        GameResult::NorthWins,
        Some(Player::South),
        Some("hard"),
    );
    for recs in [
        vec![early.clone(), late.clone()],
        vec![late.clone(), early.clone()],
    ] {
        let s = derive_stats(&recs);
        assert_eq!(s.overall, wl(1, 1, 0, 0));
        assert_eq!(s.by_level.len(), 1);
        assert_eq!(s.by_level["easy"], wl(1, 1, 0, 0));
        assert_eq!(s.current_streak, 1);
    }
}

#[test]
fn north_player_wins_and_losses() {
    let recs = [
        rec(
            "a",
            "2026-10-01T10:00:00Z",
            GameResult::NorthWins,
            Some(Player::North),
            Some("medium"),
        ),
        rec(
            "b",
            "2026-10-01T10:01:00Z",
            GameResult::SouthWins,
            Some(Player::North),
            Some("medium"),
        ),
    ];
    let s = derive_stats(&recs);
    assert_eq!(s.overall, wl(2, 1, 1, 0));
    assert_eq!(s.current_streak, 0);
    assert_eq!(s.best_streak, 1);
}

#[test]
fn draws_and_unknown_sides_break_streaks() {
    use GameResult::*;
    let p = Some(Player::South);
    let recs = [
        rec("1", "2026-10-01T10:00:00Z", SouthWins, p, None),
        rec("2", "2026-10-01T10:01:00Z", SouthWins, p, None),
        rec("3", "2026-10-01T10:02:00Z", Draw, p, None),
        rec("4", "2026-10-01T10:03:00Z", SouthWins, p, None),
        rec("5", "2026-10-01T10:04:00Z", SouthWins, None, None), // side unknown
        rec("6", "2026-10-01T10:05:00Z", SouthWins, p, None),
    ];
    let s = derive_stats(&recs);
    assert_eq!(s.overall, wl(6, 4, 0, 1));
    assert!(s.by_level.is_empty(), "no ai_level => only overall");
    assert_eq!(s.best_streak, 2);
    assert_eq!(s.current_streak, 1);
}

#[test]
fn draw_without_human_player_still_counts_as_draw() {
    let s = derive_stats(&[rec(
        "d",
        "2026-10-01T10:00:00Z",
        GameResult::Draw,
        None,
        Some("hard"),
    )]);
    assert_eq!(s.overall, wl(1, 0, 0, 1));
    assert_eq!(s.by_level["hard"], wl(1, 0, 0, 1));
}

#[test]
fn stats_json_shape() {
    let s = derive_stats(&[rec(
        "a",
        "2026-10-01T10:00:00Z",
        GameResult::SouthWins,
        Some(Player::South),
        Some("easy"),
    )]);
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(
        v,
        serde_json::json!({
            "overall": { "played": 1, "wins": 1, "losses": 0, "draws": 0 },
            "by_level": { "easy": { "played": 1, "wins": 1, "losses": 0, "draws": 0 } },
            "current_streak": 1,
            "best_streak": 1
        })
    );
}

#[test]
fn game_record_optional_fields_round_trip_and_are_omitted_when_absent() {
    let r = rec("a", "2026-10-01T10:00:00Z", GameResult::Draw, None, None);
    let v = serde_json::to_value(&r).unwrap();
    assert!(v.get("ai_level").is_none());
    assert!(v.get("human_player").is_none());
    assert_eq!(v["mode"], "vs_ai");
    assert_eq!(v["result"]["outcome"], "draw");
    let back: GameRecord = serde_json::from_value(v).unwrap();
    assert_eq!(back, r);

    let full = rec(
        "b",
        "2026-10-01T10:00:00Z",
        GameResult::NorthWins,
        Some(Player::North),
        Some("hard"),
    );
    let v = serde_json::to_value(&full).unwrap();
    assert_eq!(v["ai_level"], "hard");
    assert_eq!(v["human_player"], "north");
    assert_eq!(serde_json::from_value::<GameRecord>(v).unwrap(), full);
}

#[test]
fn replaying_a_recorded_game_reproduces_its_result() {
    // A record's moves + variant are enough to rebuild the result (versioning.md).
    use ouril_engine::*;
    let v = variant_version("cv.standard", 1).unwrap();
    let mut s = new_game(v, Player::South);
    let mut moves = vec![];
    while s.status == Status::Playing {
        let m = legal_moves(v, &s)[0];
        moves.push(m);
        s = apply_move(v, &s, m).unwrap().state;
    }
    let end = replay(v, Player::South, &moves).unwrap();
    assert_eq!(end.final_state(), &s);
    let outcome = match s.status {
        Status::Won(Player::South) => GameResult::SouthWins,
        Status::Won(Player::North) => GameResult::NorthWins,
        _ => GameResult::Draw,
    };
    let mut r = rec(
        "r",
        "2026-10-01T10:00:00Z",
        outcome,
        Some(Player::South),
        Some("easy"),
    );
    r.moves = moves;
    r.result.stores = s.stores;
    let st = derive_stats(&[r]);
    assert_eq!(st.overall.played, 1);
}

#[test]
fn a_forfeit_counts_as_a_loss_and_breaks_the_streak() {
    let win = rec(
        "a",
        "2026-10-01T10:00:00Z",
        GameResult::SouthWins,
        Some(Player::South),
        Some("easy"),
    );
    let mut forfeit = rec(
        "b",
        "2026-10-01T11:00:00Z",
        GameResult::NorthWins,
        Some(Player::South),
        Some("easy"),
    );
    forfeit.format = GAME_RECORD_FORMAT;
    forfeit.result.reason = RecordEndReason::Resigned;
    let stats = derive_stats(&[win, forfeit]);
    assert_eq!(stats.overall.losses, 1);
    assert_eq!(stats.overall.wins, 1);
    assert_eq!(stats.current_streak, 0);
}

#[test]
fn a_forfeit_record_round_trips_as_json() {
    let json = r#"{"format":2,"id":"01890000-0000-7000-8000-000000000000","variant":{"id":"cv.standard","version":1},"first_player":"south","moves":[2],"core_version":"0.1.0","result":{"outcome":"north_wins","stores":[0,0],"reason":"resigned"},"started_at":"2026-10-01T00:00:00Z","ended_at":"2026-10-01T00:01:00Z","mode":"vs_ai","ai_level":"easy","human_player":"south"}"#;
    let r: GameRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.result.reason, RecordEndReason::Resigned);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&r).unwrap()).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}
