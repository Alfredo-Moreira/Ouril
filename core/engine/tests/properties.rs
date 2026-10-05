//! Property-based tests (`proptest`) for the engine invariants listed in
//! `docs/architecture/engine.md` "Testing strategy", checked over random legal games of
//! `cv.standard@1` and over arbitrary (possibly unreachable) positions:
//!
//! - seeds are conserved: `sum(pits) + stores = total`
//! - no pit goes negative (pits are `u8`; we also rebuild every board from the events, so a
//!   seed can't be removed from an empty pit without the check failing)
//! - `legal_moves` is never empty while a reachable game is playing
//! - captures only happen on the opponent's side
//! - every game terminates
//! - `replay(record)` reproduces every position and every event
//!
//! Every assertion here follows from the variant spec (`docs/game/variants/cape-verde/
//! standard.md`), `docs/game/rules.md` or `docs/architecture/test-vectors.md` ("Event order").

use ouril_engine::*;
use proptest::prelude::*;
use proptest::sample::Index;

const TOTAL: u32 = 48;

/// Upper bound on plies for a cv.standard game: at most 100 moves without a capture, and at
/// most 48 / 2 captures before the board can't hold another one. Generous on purpose.
const MAX_PLIES: usize = 5_000;

fn cv() -> &'static VariantConfig {
    variant_version("cv.standard", 1).unwrap()
}

/// Relay sowing (ADR 0023): the same invariants must hold for `cv.continuous@1`.
fn relay() -> &'static VariantConfig {
    variant_version("cv.continuous", 1).unwrap()
}

/// Capture across (ADR 0024), alone and with continuous sowing.
fn across_variants() -> [&'static VariantConfig; 2] {
    [
        variant_version("cv.across", 1).unwrap(),
        variant_version("cv.across-continuous", 1).unwrap(),
    ]
}

/// A grand-slam rule under which a qualifying capture may be cancelled.
fn grand_slam_blocked(v: &VariantConfig) -> bool {
    matches!(
        v.grand_slam,
        GrandSlam::AllowedNoCapture | GrandSlam::Forbidden
    )
}

fn n(v: &VariantConfig) -> usize {
    v.pits_per_side as usize
}

fn owner(v: &VariantConfig, pit: u8) -> Player {
    if (pit as usize) < n(v) {
        Player::South
    } else {
        Player::North
    }
}

fn side(v: &VariantConfig, p: Player) -> std::ops::Range<usize> {
    match p {
        Player::South => 0..n(v),
        Player::North => n(v)..2 * n(v),
    }
}

fn side_sum(v: &VariantConfig, s: &GameState, p: Player) -> u32 {
    s.board()[side(v, p)].iter().map(|&x| x as u32).sum()
}

fn seeds(s: &GameState) -> u32 {
    s.board().iter().map(|&p| p as u32).sum::<u32>() + s.stores[0] as u32 + s.stores[1] as u32
}

fn idx(p: Player) -> usize {
    match p {
        Player::South => 0,
        Player::North => 1,
    }
}

/// Check one move against everything the spec and the event-order contract say about it.
/// `before` must be a playing position and `m` a legal move in it.
fn check_move(v: &VariantConfig, before: &GameState, m: Move, r: &MoveResult) {
    let after = &r.state;
    let mover = before.to_move;
    let total = seeds(before);
    let ev = &r.events;

    // Determinism and purity.
    assert_eq!(
        apply_move(v, before, m).as_ref(),
        Ok(r),
        "apply_move is deterministic"
    );
    assert_eq!(
        apply_move_quiet(v, before, m).as_ref(),
        Ok(after),
        "quiet path agrees"
    );

    // Seed conservation and board shape.
    assert_eq!(seeds(after), total, "seeds conserved");
    assert_eq!(after.pits_per_side, before.pits_per_side);
    assert!(
        after.pits[2 * n(v)..].iter().all(|&p| p == 0),
        "unused pits stay 0"
    );

    // Event order (test-vectors.md): sow/skip_origin*, capture*, grand_slam?, then either
    // extra_turn or collect_remaining* + game_over, or nothing (game continues).
    let mut i = 0;
    let sows: Vec<&MoveEvent> = ev
        .iter()
        .take_while(|e| {
            matches!(
                e,
                MoveEvent::Sow { .. } | MoveEvent::SkipOrigin { .. } | MoveEvent::Relay { .. }
            )
        })
        .collect();
    i += sows.len();
    let captures: Vec<(u8, u8)> = ev[i..]
        .iter()
        .map_while(|e| match e {
            MoveEvent::Capture { pit, seeds } => Some((*pit, *seeds)),
            _ => None,
        })
        .collect();
    i += captures.len();
    let grand_slam = ev.get(i) == Some(&MoveEvent::GrandSlam);
    if grand_slam {
        i += 1;
    }
    let extra_turn = ev.get(i) == Some(&MoveEvent::ExtraTurn);
    if extra_turn {
        i += 1;
    }
    let collects: Vec<(Player, u8)> = ev[i..]
        .iter()
        .map_while(|e| match e {
            MoveEvent::CollectRemaining { player, seeds } => Some((*player, *seeds)),
            _ => None,
        })
        .collect();
    i += collects.len();
    let game_over = match ev.get(i) {
        Some(MoveEvent::GameOver { result, reason }) => {
            i += 1;
            Some((*result, *reason))
        }
        _ => None,
    };
    assert_eq!(i, ev.len(), "events out of order: {ev:?}");
    assert!(
        !(extra_turn && game_over.is_some()),
        "extra turn or game over, not both"
    );
    assert!(
        collects.is_empty() || game_over.is_some(),
        "collect_remaining only at game end"
    );
    assert!(
        extra_turn <= grand_slam,
        "extra turn only after a grand slam"
    );

    // Sowing: one seed per sow event, starting after the origin and never into the origin
    // when it is skipped. Rebuild the board from the events.
    let picked = before.board()[m as usize];
    assert!(picked > 0, "legal move from a non-empty pit");
    let n_sow = sows
        .iter()
        .filter(|e| matches!(e, MoveEvent::Sow { .. }))
        .count();
    let relayed: usize = sows
        .iter()
        .map(|e| match e {
            MoveEvent::Relay { seeds, .. } => *seeds as usize,
            _ => 0,
        })
        .sum();
    assert_eq!(
        n_sow,
        picked as usize + relayed,
        "one sow event per seed picked up (including relay pickups)"
    );
    let relay_on = v.relay_sowing == RelaySowing::OwnSideOccupied;
    let mut board = before.board().to_vec();
    board[m as usize] = 0;
    let mut last_sown = None;
    let mut lap_origin = m;
    for e in &sows {
        match e {
            MoveEvent::Sow { pit } => {
                board[*pit as usize] += 1;
                last_sown = Some(*pit);
            }
            MoveEvent::SkipOrigin { pit } => {
                assert_eq!(*pit, lap_origin, "only the lap's origin is skipped");
                assert!(v.skip_origin_on_lap);
                if !relay_on {
                    assert!(picked as usize >= 2 * n(v));
                }
            }
            MoveEvent::Relay { pit, seeds } => {
                // Relay: from the last pit sown, on the mover's side, which already held seeds.
                assert!(relay_on, "relay only with relay sowing");
                assert_eq!(Some(*pit), last_sown, "relay from the last pit sown");
                assert_eq!(owner(v, *pit), mover, "relay only on the mover's side");
                assert!(*seeds >= 2, "relay only from a pit that already held seeds");
                assert_eq!(board[*pit as usize], *seeds, "relay picks up the whole pit");
                board[*pit as usize] = 0;
                lap_origin = *pit;
            }
            _ => unreachable!(),
        }
    }
    if relay_on {
        // The move ended because the last seed landed in an empty own pit or on the
        // opponent's side.
        let last = last_sown.expect("a move sows at least one seed");
        assert!(
            owner(v, last) != mover || board[last as usize] == 1,
            "relay sowing stopped early at pit {last}"
        );
    }
    if v.skip_origin_on_lap && !relay_on {
        assert!(
            !sows.contains(&&MoveEvent::Sow { pit: m }),
            "origin never sown into"
        );
    }

    // Captures always take seeds from the opponent's side. Oware (`opponent_side`): only
    // counts the variant allows, starting at the last pit sown and going backwards without
    // gaps. Capture across (ADR 0024): from the pits across a backward chain of own pits that
    // each hold exactly 1 seed, starting at the last pit sown.
    let opp = mover.opponent();
    let len = 2 * n(v);
    let back = |p: u8| -> u8 {
        (match v.direction {
            Direction::Ccw => (p as usize + len - 1) % len,
            Direction::Cw => (p as usize + 1) % len,
        }) as u8
    };
    let across = |p: u8| -> u8 { (len - 1 - p as usize) as u8 };
    let mut captured = 0u32;
    let mut own = last_sown;
    for (k, &(pit, s)) in captures.iter().enumerate() {
        assert_eq!(owner(v, pit), opp, "captures only on the opponent's side");
        assert_eq!(board[pit as usize], s, "capture event matches the pit");
        match v.capture_mode {
            CaptureMode::OpponentSide => {
                assert!(v.capture_counts.contains(&s), "capture of {s} seeds");
                let expected = if k == 0 {
                    last_sown.unwrap()
                } else {
                    back(captures[k - 1].0)
                };
                assert_eq!(
                    pit, expected,
                    "captures are a backward chain from the last seed"
                );
            }
            CaptureMode::AcrossFromEmptyOwnPit => {
                let o = if k == 0 {
                    own.unwrap()
                } else {
                    back(own.unwrap())
                };
                assert_eq!(owner(v, o), mover, "across: from an own pit");
                assert_eq!(board[o as usize], 1, "across: the own pit was empty");
                assert_eq!(pit, across(o), "across: the pit opposite");
                own = Some(o);
            }
        }
        board[pit as usize] = 0;
        captured += s as u32;
    }
    if v.capture_mode == CaptureMode::AcrossFromEmptyOwnPit {
        // Mandatory: the chain stops only where the next pit doesn't qualify.
        let next = match (captures.is_empty(), own) {
            (true, Some(l)) => Some(l),
            (false, Some(o)) if v.chain_capture == ChainCapture::Backward => Some(back(o)),
            _ => None,
        };
        if let Some(q) = next {
            let qualifies = owner(v, q) == mover
                && board[q as usize] == 1
                && board[across(q) as usize] > 0
                && captures.len() < n(v);
            assert!(
                !qualifies || grand_slam_blocked(v),
                "across capture missed at pit {q}"
            );
        }
    }
    if grand_slam {
        assert!(!captures.is_empty());
        assert!(
            board[side(v, opp)].iter().all(|&x| x == 0),
            "grand slam empties the opponent's side"
        );
    }
    let mut stores = before.stores;
    stores[idx(mover)] += captured as u8;

    // Game end: remaining seeds collected (South first), then game_over.
    let mut last_collector = None;
    for &(p, s) in &collects {
        assert!(s > 0, "collect only from sides that still have seeds");
        assert!(
            last_collector < Some(idx(p)),
            "South collects first, once each"
        );
        last_collector = Some(idx(p));
        let sd = side(v, p);
        assert_eq!(
            board[sd.clone()].iter().map(|&x| x as u32).sum::<u32>(),
            s as u32
        );
        for x in &mut board[sd] {
            *x = 0;
        }
        // cv.standard: whoever collects keeps the seeds (each/mover takes own side).
        stores[idx(p)] += s;
    }
    assert_eq!(after.board(), board.as_slice(), "board rebuilt from events");
    assert_eq!(after.stores, stores, "stores rebuilt from events");

    // Counters and status.
    if captures.is_empty() {
        assert_eq!(after.moves_since_capture, before.moves_since_capture + 1);
    } else {
        assert_eq!(after.moves_since_capture, 0);
    }
    match game_over {
        None => {
            assert_eq!(after.status, Status::Playing);
            let next = if extra_turn { mover } else { opp };
            assert_eq!(after.to_move, next, "turn passes unless extra turn");
            assert!(
                after.stores.iter().all(|&s| (s as u32) * 2 <= total),
                "nobody has a majority while playing"
            );
            assert!(after.moves_since_capture < v.endless_cycle_move_limit.unwrap_or(u16::MAX));
        }
        Some((result, reason)) => {
            assert_ne!(after.status, Status::Playing);
            assert!(legal_moves(v, after).is_empty(), "no moves once over");
            assert_eq!(apply_move(v, after, m), Err(MoveError::GameOver));
            assert!(
                after.board().iter().all(|&x| x == 0),
                "cv: board empty at end"
            );
            let [south, north] = after.stores;
            let expected = match south.cmp(&north) {
                std::cmp::Ordering::Greater => GameResult::SouthWins,
                std::cmp::Ordering::Less => GameResult::NorthWins,
                std::cmp::Ordering::Equal => GameResult::Draw,
            };
            assert_eq!(result, expected, "more captured seeds wins ({reason:?})");
            let status = match result {
                GameResult::SouthWins => Status::Won(Player::South),
                GameResult::NorthWins => Status::Won(Player::North),
                GameResult::Draw => Status::Draw,
            };
            assert_eq!(after.status, status);
            if reason == EndReason::Threshold {
                assert!(after.stores.iter().any(|&s| s >= v.win_threshold));
            }
        }
    }
}

/// Check what `legal_moves` returns in a playing position, against `apply_move` for every
/// pit on the board (own and opponent's).
fn check_legal_moves(v: &VariantConfig, s: &GameState) -> Vec<Move> {
    let legal = legal_moves(v, s);
    assert!(
        legal.windows(2).all(|w| w[0] < w[1]),
        "sorted, no duplicates"
    );
    let own = side(v, s.to_move);
    for p in 0..=(2 * n(v) as u8 + 1) {
        let r = apply_move(v, s, p);
        if legal.contains(&p) {
            assert!(own.contains(&(p as usize)), "legal move is an own pit");
            assert!(s.board()[p as usize] > 0, "legal move is not empty");
            assert!(r.is_ok(), "legal move {p} rejected: {r:?}");
        } else {
            let e = r.expect_err("non-legal move must be rejected");
            if !own.contains(&(p as usize)) {
                assert_eq!(e, MoveError::NotOwnPit);
            } else if s.board()[p as usize] == 0 {
                assert_eq!(e, MoveError::EmptyPit);
            }
        }
    }
    // Must feed: if the opponent has no seeds, every legal move puts seeds on their side.
    let opp = s.to_move.opponent();
    if v.must_feed && side_sum(v, s, opp) == 0 {
        for &m in &legal {
            let r = apply_move(v, s, m).unwrap();
            assert!(
                r.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Sow { pit } if owner(v, *pit) == opp)),
                "move {m} doesn't feed"
            );
        }
    }
    // Single-seed rule: a single-seed pit is only legal when no own pit holds 2+, unless it
    // is needed to feed (feeding_overrides_single_seed_rule).
    if v.single_seed_rule == SingleSeedRule::OnlyIfNoLargerPit {
        let has_larger = own.clone().any(|p| s.board()[p] >= 2);
        let feeding_needed = v.must_feed && side_sum(v, s, opp) == 0;
        if has_larger && !feeding_needed {
            for &m in &legal {
                assert!(s.board()[m as usize] >= 2, "single-seed pit {m} played");
            }
        }
    }
    legal
}

/// Play a game, picking each move with the next `choices` index (the first legal move once
/// they run out). Checks every move; returns the moves and the final state.
fn play(v: &VariantConfig, mut s: GameState, choices: &[Index]) -> (Vec<Move>, GameState) {
    let mut moves = Vec::new();
    for ply in 0.. {
        assert!(ply < MAX_PLIES, "game did not terminate");
        if s.status != Status::Playing {
            assert!(legal_moves(v, &s).is_empty());
            break;
        }
        let legal = check_legal_moves(v, &s);
        assert!(
            !legal.is_empty(),
            "no legal move while playing at ply {ply}"
        );
        let m = choices
            .get(ply)
            .map(|i| legal[i.index(legal.len())])
            .unwrap_or(legal[0]);
        let before = s.clone();
        let r = apply_move(v, &s, m).unwrap();
        assert_eq!(s, before, "input state is not mutated");
        check_move(v, &s, m, &r);
        moves.push(m);
        s = r.state;
    }
    (moves, s)
}

fn any_player() -> impl Strategy<Value = Player> {
    prop_oneof![Just(Player::South), Just(Player::North)]
}

/// An arbitrary playing position: 48 seeds spread over the 12 pits and 2 stores, nobody at
/// the threshold, and the player to move has at least one seed. Possibly unreachable.
fn arbitrary_position() -> impl Strategy<Value = GameState> {
    (
        // Bucket per seed: 0..12 pits, 12 = south store, 13 = north store. Store buckets are
        // weighted so mid- and late-game positions show up.
        prop::collection::vec(
            prop_oneof![6 => 0usize..12, 1 => Just(12usize), 1 => Just(13usize)],
            TOTAL as usize,
        ),
        any_player(),
        0u16..99,
    )
        .prop_map(|(buckets, to_move, msc)| {
            let mut pits = vec![0u8; 12];
            let mut stores = [0u8; 2];
            for b in buckets {
                match b {
                    12 => stores[0] += 1,
                    13 => stores[1] += 1,
                    p => pits[p] += 1,
                }
            }
            GameState::try_from(GameStateJson {
                pits,
                stores,
                to_move,
                moves_since_capture: msc,
                status: Status::Playing,
                history: vec![],
            })
            .unwrap()
        })
        .prop_filter("playable position", |s| {
            let v = cv();
            s.stores.iter().all(|&x| x < v.win_threshold) && side_sum(v, s, s.to_move) > 0
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, ..ProptestConfig::default() })]

    /// Random legal games: every invariant holds after every move, the game terminates,
    /// and replaying the record rebuilds every position and event exactly.
    #[test]
    fn random_games_keep_invariants_and_replay(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..400),
    ) {
        let v = cv();
        let start = new_game(v, first);
        prop_assert_eq!(start.board(), &[4u8; 12][..]);
        prop_assert_eq!(start.to_move, first);
        let (moves, end) = play(v, start, &choices);
        prop_assert_ne!(end.status, Status::Playing);
        prop_assert_eq!(seeds(&end), TOTAL);

        let rep = replay(v, first, &moves).unwrap();
        prop_assert_eq!(rep.final_state(), &end);
        prop_assert_eq!(rep.steps.len(), moves.len());
        // Every step equals a fresh apply_move from the previous replayed state.
        let mut s = rep.initial.clone();
        for (step, &m) in rep.steps.iter().zip(&moves) {
            prop_assert_eq!(&apply_move(v, &s, m).unwrap(), step);
            s = step.state.clone();
        }
        // A move after the end is rejected at the right ply.
        let mut extra = moves.clone();
        extra.push(0);
        let err = replay(v, first, &extra).unwrap_err();
        prop_assert_eq!(err.ply, moves.len());
        prop_assert_eq!(err.error, MoveError::GameOver);
    }

    /// Every intermediate position of a game survives a JSON round trip (including the
    /// opaque repetition history), and the game continues identically from it.
    #[test]
    fn json_round_trip_mid_game(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..120),
    ) {
        let v = cv();
        let mut s = new_game(v, first);
        for c in &choices {
            if s.status != Status::Playing { break; }
            let json = serde_json::to_string(&s).unwrap();
            let back: GameState = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(&back, &s);
            prop_assert!(validate_state(v, &back).is_ok());
            let legal = legal_moves(v, &s);
            prop_assert_eq!(&legal_moves(v, &back), &legal);
            let m = legal[c.index(legal.len())];
            let a = apply_move(v, &s, m).unwrap();
            prop_assert_eq!(&apply_move(v, &back, m).unwrap(), &a);
            s = a.state;
        }
    }

    /// Arbitrary positions: every legal move keeps every per-move invariant (conservation,
    /// opponent-side captures, event order, rebuilt board and stores).
    #[test]
    fn arbitrary_positions_keep_per_move_invariants(s in arbitrary_position()) {
        let v = cv();
        prop_assert!(validate_state(v, &s).is_ok());
        let legal = check_legal_moves(v, &s);
        for m in legal {
            let r = apply_move(v, &s, m).unwrap();
            check_move(v, &s, m, &r);
        }
    }

    /// Arbitrary positions played out with random moves still terminate and conserve seeds.
    #[test]
    fn arbitrary_positions_terminate(
        s in arbitrary_position(),
        choices in prop::collection::vec(any::<Index>(), 0..300),
    ) {
        let v = cv();
        let mut s = s;
        for ply in 0.. {
            prop_assert!(ply < MAX_PLIES, "did not terminate");
            if s.status != Status::Playing { break; }
            let legal = legal_moves(v, &s);
            if legal.is_empty() {
                // Only possible in an unreachable start position (opponent empty and no
                // feeding move, which a real game would already have ended). Must be ply 0.
                prop_assert_eq!(ply, 0);
                break;
            }
            let m = choices.get(ply).map(|i| legal[i.index(legal.len())]).unwrap_or(legal[0]);
            let r = apply_move(v, &s, m).unwrap();
            check_move(v, &s, m, &r);
            s = r.state;
        }
    }

    /// Rejected moves never change anything and always carry a documented error code.
    #[test]
    fn rejected_moves_are_reported_not_applied(s in arbitrary_position(), m in any::<u8>()) {
        let v = cv();
        let legal = legal_moves(v, &s);
        match apply_move(v, &s, m) {
            Ok(_) => prop_assert!(legal.contains(&m)),
            Err(e) => {
                prop_assert!(!legal.contains(&m));
                prop_assert!(apply_move_quiet(v, &s, m).is_err());
                let code = serde_json::to_value(e).unwrap();
                prop_assert_eq!(code.as_str().unwrap(), e.code());
            }
        }
    }

    /// The position hash depends only on pits and side to move.
    #[test]
    fn position_hash_ignores_stores_and_counters(s in arbitrary_position(), d in 0u16..50) {
        let mut t = s.clone();
        t.moves_since_capture = d;
        t.history.clear();
        prop_assert_eq!(position_hash(&s), position_hash(&t));
        let mut u = s.clone();
        u.to_move = u.to_move.opponent();
        prop_assert_ne!(position_hash(&s), position_hash(&u));
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// Relay sowing: random legal games keep every invariant, terminate, and replay exactly.
    #[test]
    fn relay_random_games_keep_invariants_and_replay(
        first in any_player(),
        choices in prop::collection::vec(any::<Index>(), 0..400),
    ) {
        let v = relay();
        let start = new_game(v, first);
        let (moves, end) = play(v, start, &choices);
        prop_assert_ne!(end.status, Status::Playing);
        prop_assert_eq!(seeds(&end), TOTAL);
        let rep = replay(v, first, &moves).unwrap();
        prop_assert_eq!(rep.final_state(), &end);
    }

    /// Relay sowing: every legal move in arbitrary positions keeps the per-move invariants.
    #[test]
    fn relay_arbitrary_positions_keep_per_move_invariants(s in arbitrary_position()) {
        let v = relay();
        let legal = check_legal_moves(v, &s);
        for m in legal {
            let r = apply_move(v, &s, m).unwrap();
            check_move(v, &s, m, &r);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 384, ..ProptestConfig::default() })]

    /// Capture across: random legal games keep every invariant, terminate, and replay.
    #[test]
    fn across_random_games_keep_invariants_and_replay(
        first in any_player(),
        which in 0usize..2,
        choices in prop::collection::vec(any::<Index>(), 0..400),
    ) {
        let v = across_variants()[which];
        let (moves, end) = play(v, new_game(v, first), &choices);
        prop_assert_ne!(end.status, Status::Playing);
        prop_assert_eq!(seeds(&end), TOTAL);
        let rep = replay(v, first, &moves).unwrap();
        prop_assert_eq!(rep.final_state(), &end);
    }

    /// Capture across: every legal move in arbitrary positions keeps the invariants.
    #[test]
    fn across_arbitrary_positions_keep_per_move_invariants(
        s in arbitrary_position(),
        which in 0usize..2,
    ) {
        let v = across_variants()[which];
        for m in check_legal_moves(v, &s) {
            let r = apply_move(v, &s, m).unwrap();
            check_move(v, &s, m, &r);
        }
    }
}

/// Relay sowing really happens: South's opening from pit 0 ends in an occupied own pit.
#[test]
fn relay_sowing_relays_from_the_opening() {
    let v = relay();
    let r = apply_move(v, &new_game(v, Player::South), 0).unwrap();
    assert!(r
        .events
        .iter()
        .any(|e| matches!(e, MoveEvent::Relay { pit: 4, seeds: 5 })));
}

/// Guard against vacuous properties: the generators above must actually reach every kind
/// of event and every cv.standard end reason (otherwise the per-event checks prove little).
#[test]
fn generators_cover_every_event_and_end_reason() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    use std::collections::BTreeSet;

    let v = cv();
    let mut runner = TestRunner::deterministic();
    let pos = arbitrary_position();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for _ in 0..3_000 {
        let mut s = pos.new_tree(&mut runner).unwrap().current();
        let mut rng = runner.rng().clone();
        for _ in 0..MAX_PLIES {
            let legal = legal_moves(v, &s);
            if s.status != Status::Playing || legal.is_empty() {
                break;
            }
            let m = legal[(rand_index(&mut rng)) % legal.len()];
            let r = apply_move(v, &s, m).unwrap();
            for e in &r.events {
                let key = match e {
                    MoveEvent::GameOver { reason, .. } => format!("end:{reason:?}"),
                    other => format!("{other:?}")
                        .split([' ', '{'])
                        .next()
                        .unwrap()
                        .to_string(),
                };
                seen.insert(key);
            }
            s = r.state;
        }
    }
    for want in [
        "Sow",
        "SkipOrigin",
        "Capture",
        "GrandSlam",
        "ExtraTurn",
        "CollectRemaining",
        "end:Threshold",
        "end:NoFeed",
        "end:EndlessCycle",
    ] {
        assert!(seen.contains(want), "never generated {want}; saw {seen:?}");
    }
}

fn rand_index(rng: &mut proptest::test_runner::TestRng) -> usize {
    use proptest::prelude::RngCore;
    rng.next_u32() as usize
}
