//! Property-based tests for the AI (`docs/architecture/ai.md`, `docs/architecture/engine.md`
//! "Testing strategy"): at every level the AI returns a legal move whenever one exists, and
//! the same `(variant, state, level, seed)` always gives the same move.
//!
//! Positions come from random legal games of `cv.standard@1` (reachable positions only).
//! Hard searches 1M nodes, which is slow in debug builds, so it gets few cases and positions
//! from later in the game (fewer seeds, cheaper nodes).

use ouril_ai::{choose_move, choose_move_with, search, Level, SearchParams, WIN};
use ouril_engine::*;
use proptest::prelude::*;
use proptest::sample::Index;

fn cv() -> &'static VariantConfig {
    variant_version("cv.standard", 1).unwrap()
}

fn any_player() -> impl Strategy<Value = Player> {
    prop_oneof![Just(Player::South), Just(Player::North)]
}

/// A reachable playing position: `choices` random legal moves from a new game (stopping one
/// move early if the game would end).
fn reachable(first: Player, choices: &[Index]) -> GameState {
    let v = cv();
    let mut s = new_game(v, first);
    for c in choices {
        let legal = legal_moves(v, &s);
        let m = legal[c.index(legal.len())];
        let next = apply_move(v, &s, m).unwrap().state;
        if next.status != Status::Playing {
            break;
        }
        s = next;
    }
    s
}

fn any_level() -> impl Strategy<Value = Level> {
    prop_oneof![Just(Level::Easy), Just(Level::Medium)]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Easy and Medium: always a legal move, and deterministic for the same inputs.
    #[test]
    fn easy_and_medium_return_a_legal_move_deterministically(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..80),
        level in any_level(),
        seed in any::<u64>(),
    ) {
        let v = cv();
        let s = reachable(first, &choices);
        let legal = legal_moves(v, &s);
        prop_assert!(!legal.is_empty());
        let before = s.clone();
        let m = choose_move(v, &s, level, seed);
        prop_assert_eq!(&s, &before, "input state is not mutated");
        let m = m.expect("a move while playing");
        prop_assert!(legal.contains(&m), "{:?} chose illegal {} from {:?}", level, m, legal);
        prop_assert!(apply_move(v, &s, m).is_ok());
        prop_assert_eq!(choose_move(v, &s, level, seed), Some(m), "deterministic");
    }

    /// `search` is deterministic and its result is self-consistent: `best` is legal and has
    /// the highest root score, every root move is scored, and `score` matches `best`.
    #[test]
    fn search_is_deterministic_and_consistent(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..80),
        level in any_level(),
    ) {
        let v = cv();
        let s = reachable(first, &choices);
        let legal = legal_moves(v, &s);
        let p = level.params();
        let r = search(v, &s, p).unwrap();
        prop_assert_eq!(search(v, &s, p).unwrap(), r.clone(), "deterministic");
        prop_assert!(legal.contains(&r.best));
        prop_assert!(r.depth >= 1 && r.depth <= p.max_depth);
        prop_assert!(r.score.abs() <= WIN);
        let mut scored: Vec<Move> = r.root_scores.iter().map(|(m, _)| *m).collect();
        scored.sort();
        prop_assert_eq!(scored, legal, "every legal root move is scored once");
        let best_score = r.root_scores.iter().find(|(m, _)| *m == r.best).unwrap().1;
        prop_assert_eq!(best_score, r.score);
        prop_assert!(r.root_scores.iter().all(|(_, sc)| *sc <= r.score));
    }

    /// With a margin, the chosen move is one whose root score is within the margin of the
    /// best (no random-move escape hatch here).
    #[test]
    fn margin_choice_stays_within_margin(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..80),
        seed in any::<u64>(),
        margin in 0i32..300,
    ) {
        let v = cv();
        let s = reachable(first, &choices);
        let p = SearchParams { max_depth: 3, node_budget: 5_000, margin, random_move_percent: 0, tt_bits: 10 };
        let m = choose_move_with(v, &s, p, seed).unwrap();
        let legal = legal_moves(v, &s);
        prop_assert!(legal.contains(&m));
        if legal.len() > 1 {
            let r = search(v, &s, p).unwrap();
            let sc = r.root_scores.iter().find(|(x, _)| *x == m).unwrap().1;
            prop_assert!(sc >= r.score - margin, "chose {} ({}) vs best {} ({})", m, sc, r.best, r.score);
        }
    }

    /// AI-vs-AI games (any mix of Easy/Medium, any seeds) only play legal moves and finish.
    #[test]
    fn ai_vs_ai_games_finish_with_legal_moves(
        south in any_level(),
        north in any_level(),
        first in any_player(),
        seed in any::<u64>(),
    ) {
        let v = cv();
        let mut s = new_game(v, first);
        let mut ply = 0u64;
        while s.status == Status::Playing {
            let level = if s.to_move == Player::South { south } else { north };
            let m = choose_move(v, &s, level, seed.wrapping_add(ply)).expect("a move");
            prop_assert!(legal_moves(v, &s).contains(&m));
            s = apply_move(v, &s, m).unwrap().state;
            ply += 1;
            prop_assert!(ply < 5_000, "game did not finish");
        }
        prop_assert_eq!(choose_move(v, &s, south, seed), None, "no move once over");
    }
}

proptest! {
    // Hard searches up to 1M nodes per call: keep the case count small for debug builds.
    #![proptest_config(ProptestConfig { cases: 6, ..ProptestConfig::default() })]

    /// Hard: always a legal move, and it ignores the seed (no randomness at Hard).
    #[test]
    fn hard_returns_a_legal_move_and_ignores_the_seed(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 20..120),
        seed_a in any::<u64>(),
        seed_b in any::<u64>(),
    ) {
        let v = cv();
        let s = reachable(first, &choices);
        let legal = legal_moves(v, &s);
        let a = choose_move(v, &s, Level::Hard, seed_a).expect("a move");
        prop_assert!(legal.contains(&a));
        prop_assert_eq!(choose_move(v, &s, Level::Hard, seed_b), Some(a));
    }
}

#[test]
fn every_level_returns_none_when_no_move_exists() {
    let v = cv();
    // A finished game, and a playing position with no legal move (unreachable in a real
    // game: South must feed an empty North but can't).
    let mut over = new_game(v, Player::South);
    over.status = Status::Won(Player::North);
    let stuck = GameState::try_from(GameStateJson {
        pits: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        stores: [23, 24],
        to_move: Player::South,
        moves_since_capture: 0,
        status: Status::Playing,
        history: vec![],
    })
    .unwrap();
    for level in [Level::Easy, Level::Medium, Level::Hard] {
        assert_eq!(choose_move(v, &over, level, 9), None);
        assert!(legal_moves(v, &stuck).is_empty());
        assert_eq!(choose_move(v, &stuck, level, 9), None);
        assert!(search(v, &stuck, level.params()).is_none());
    }
}

#[test]
fn easy_varies_with_the_seed() {
    // Easy plays a random move 1 time in 5 and picks among moves within 1.5 seeds, so over
    // many seeds from the opening it must pick more than one distinct move.
    let v = cv();
    let s = new_game(v, Player::South);
    let easy: std::collections::BTreeSet<Move> = (0..200u64)
        .map(|seed| choose_move(v, &s, Level::Easy, seed).unwrap())
        .collect();
    assert!(easy.len() > 1, "Easy never varies: {easy:?}");
}

#[test]
fn a_seed_reproduces_a_whole_game() {
    // Same seeds per ply => identical game (bug-report reproducibility, ai.md).
    let v = cv();
    let play = || {
        let mut s = new_game(v, Player::North);
        let mut moves = vec![];
        let mut ply = 0u64;
        while s.status == Status::Playing {
            let level = if ply.is_multiple_of(2) {
                Level::Easy
            } else {
                Level::Medium
            };
            let m = choose_move(v, &s, level, 0xC0FFEE ^ ply).unwrap();
            moves.push(m);
            s = apply_move(v, &s, m).unwrap().state;
            ply += 1;
        }
        (moves, s)
    };
    let (a, sa) = play();
    let (b, sb) = play();
    assert_eq!(a, b);
    assert_eq!(sa, sb);
    // And the record replays to the same end.
    assert_eq!(replay(v, Player::North, &a).unwrap().final_state(), &sa);
}
