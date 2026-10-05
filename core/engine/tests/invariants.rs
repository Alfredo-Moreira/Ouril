//! Engine invariants over many random legal games (seeded, reproducible), plus engine
//! behaviour that the vector format can't express yet (repetition needs `setup.history`).

use ouril_engine::*;

/// SplitMix64: a tiny seeded PRNG so the "random" games are reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn cv() -> &'static VariantConfig {
    variant_version("cv.standard", 1).unwrap()
}

fn state(pits: [u8; 12], stores: [u8; 2], to_move: Player) -> GameState {
    let j = GameStateJson {
        pits: pits.to_vec(),
        stores,
        to_move,
        moves_since_capture: 0,
        status: Status::Playing,
        history: vec![],
    };
    GameState::try_from(j).unwrap()
}

fn seeds(s: &GameState) -> u32 {
    s.board().iter().map(|&p| p as u32).sum::<u32>() + s.stores[0] as u32 + s.stores[1] as u32
}

#[test]
fn random_games_keep_every_invariant() {
    let v = cv();
    let n = v.pits_per_side as usize;
    let mut reasons = std::collections::BTreeMap::new();
    for game in 0..500u64 {
        let mut rng = Rng(game);
        let first = if game % 2 == 0 {
            Player::South
        } else {
            Player::North
        };
        let mut s = new_game(v, first);
        let mut moves = Vec::new();
        for ply in 0.. {
            assert!(ply < 5_000, "game {game} did not terminate");
            let legal = legal_moves(v, &s);
            if s.status != Status::Playing {
                assert!(legal.is_empty());
                break;
            }
            assert!(
                !legal.is_empty(),
                "game {game}: no legal move while playing"
            );
            assert!(legal.windows(2).all(|w| w[0] < w[1]), "legal moves sorted");
            // Every own pit not in `legal` is rejected with an error.
            let own = match s.to_move {
                Player::South => 0..n,
                Player::North => n..2 * n,
            };
            for p in own {
                let r = apply_move(v, &s, p as u8);
                assert_eq!(r.is_ok(), legal.contains(&(p as u8)), "game {game} pit {p}");
            }
            let m = legal[(rng.next() % legal.len() as u64) as usize];
            let before = s.clone();
            let r = apply_move(v, &s, m).unwrap();
            assert_eq!(s, before, "input state is not mutated");
            assert_eq!(
                apply_move_quiet(v, &s, m).unwrap(),
                r.state,
                "quiet path agrees"
            );
            assert_eq!(seeds(&r.state), 48, "seeds conserved");
            let mover = s.to_move;
            for e in &r.events {
                if let MoveEvent::Capture { pit, .. } = e {
                    let owner = if (*pit as usize) < n {
                        Player::South
                    } else {
                        Player::North
                    };
                    assert_ne!(owner, mover, "captures only on the opponent's side");
                }
                if let MoveEvent::GameOver { reason, .. } = e {
                    *reasons.entry(format!("{reason:?}")).or_insert(0) += 1;
                }
            }
            assert_eq!(
                r.events
                    .iter()
                    .filter(|e| matches!(e, MoveEvent::GameOver { .. }))
                    .count(),
                usize::from(r.state.status != Status::Playing)
            );
            moves.push(m);
            s = r.state;
        }
        // Replaying the record rebuilds the same final position.
        let rep = replay(v, first, &moves).unwrap();
        assert_eq!(rep.final_state(), &s);
        assert_eq!(rep.steps.len(), moves.len());
    }
    println!("end reasons over 500 games: {reasons:?}");
    // cv.standard always feeds (or ends with no_feed), so nobody is left without a move.
    assert!(!reasons.contains_key("NoMoves"), "{reasons:?}");
}

#[test]
fn new_game_and_json_round_trip() {
    let v = cv();
    let s = new_game(v, Player::North);
    assert_eq!(s.board(), &[4; 12]);
    assert_eq!(s.to_move, Player::North);
    assert_eq!(s.history.len(), 1);
    let json = serde_json::to_string(&s).unwrap();
    let back: GameState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
    assert!(validate_state(v, &s).is_ok());
}

#[test]
fn repeated_position_ends_as_endless_cycle() {
    // Two single seeds walk round the board without ever meeting; after 12 plies the
    // starting position (pits 5 and 11, South to move) recurs.
    let v = cv();
    let mut s = state(
        [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
        [23, 23],
        Player::South,
    );
    let mut last = None;
    for ply in 0..12 {
        assert_eq!(s.status, Status::Playing, "ply {ply}");
        let legal = legal_moves(v, &s);
        assert_eq!(legal.len(), 1, "forced line at ply {ply}");
        let r = apply_move(v, &s, legal[0]).unwrap();
        s = r.state;
        last = Some(r.events);
    }
    assert_eq!(s.status, Status::Draw);
    assert!(s.moves_since_capture < 100);
    assert_eq!(
        last.unwrap().last(),
        Some(&MoveEvent::GameOver {
            result: GameResult::Draw,
            reason: EndReason::EndlessCycle
        })
    );
    assert_eq!(s.stores, [24, 24]);
}

#[test]
fn history_round_trips_through_json_and_still_detects_repetition() {
    let v = cv();
    let mut s = state(
        [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
        [23, 23],
        Player::South,
    );
    for _ in 0..11 {
        let m = legal_moves(v, &s)[0];
        let json = serde_json::to_string(&apply_move(v, &s, m).unwrap().state).unwrap();
        s = serde_json::from_str(&json).unwrap();
    }
    let r = apply_move(v, &s, legal_moves(v, &s)[0]).unwrap();
    assert_eq!(r.state.status, Status::Draw);
}

#[test]
fn errors_take_precedence_in_a_fixed_order() {
    let v = cv();
    let s = new_game(v, Player::South);
    assert_eq!(apply_move(v, &s, 12), Err(MoveError::NotOwnPit));
    assert_eq!(apply_move(v, &s, 255), Err(MoveError::NotOwnPit));
    let mut over = s.clone();
    over.status = Status::Draw;
    assert_eq!(apply_move(v, &over, 0), Err(MoveError::GameOver));
    assert!(legal_moves(v, &over).is_empty());
}

#[test]
fn single_seed_rule_still_applies_among_feeding_moves() {
    // North is empty. Pit 4 (2 seeds) and pit 5 (1 seed) both feed: the single-seed rule
    // keeps only pit 4, because a feeding move from a larger pit exists.
    let v = cv();
    let s = state(
        [0, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0],
        [23, 22],
        Player::South,
    );
    assert_eq!(legal_moves(v, &s), vec![4]);
    assert_eq!(apply_move(v, &s, 5), Err(MoveError::SingleSeedRule));
}

#[test]
fn grand_slam_variants_behave_as_named() {
    let base = cv().clone();
    let s = state(
        [0, 0, 0, 1, 3, 0, 1, 2, 0, 0, 0, 0],
        [18, 23],
        Player::South,
    );

    // allowed_no_capture: the move is legal but captures nothing.
    let mut v = base.clone();
    v.grand_slam = GrandSlam::AllowedNoCapture;
    let r = apply_move(&v, &s, 4).unwrap();
    assert_eq!(r.state.stores, [18, 23]);
    assert_eq!(r.state.to_move, Player::North);
    assert!(!r
        .events
        .iter()
        .any(|e| matches!(e, MoveEvent::Capture { .. } | MoveEvent::GrandSlam)));

    // captures_all: captures, then the (now empty) opponent can't move: no_moves.
    v.grand_slam = GrandSlam::CapturesAll;
    let r = apply_move(&v, &s, 4).unwrap();
    assert!(r.events.contains(&MoveEvent::GrandSlam));
    assert!(!r.events.contains(&MoveEvent::ExtraTurn));
    assert_eq!(
        r.events.last(),
        Some(&MoveEvent::GameOver {
            result: GameResult::SouthWins,
            reason: EndReason::NoMoves
        })
    );

    // forbidden: the grand slam is illegal while another move exists.
    v.grand_slam = GrandSlam::Forbidden;
    let s2 = state(
        [0, 0, 2, 1, 3, 0, 1, 2, 0, 0, 0, 0],
        [16, 23],
        Player::South,
    );
    assert_eq!(legal_moves(&v, &s2), vec![2]);
    assert_eq!(apply_move(&v, &s2, 4), Err(MoveError::GrandSlamForbidden));
}

#[test]
fn clockwise_sowing_goes_to_lower_indices() {
    let mut v = cv().clone();
    v.direction = Direction::Cw;
    let s = new_game(&v, Player::South);
    let r = apply_move(&v, &s, 2).unwrap();
    assert_eq!(
        r.events,
        vec![
            MoveEvent::Sow { pit: 1 },
            MoveEvent::Sow { pit: 0 },
            MoveEvent::Sow { pit: 11 },
            MoveEvent::Sow { pit: 10 }
        ]
    );
}

#[test]
fn validate_rejects_unsupported_and_inconsistent_configs() {
    let mut v = cv().clone();
    assert!(validate(&v).is_ok());
    v.grand_slam = GrandSlam::Loses;
    assert!(validate(&v).is_err());
    let mut v = cv().clone();
    v.capture_mandatory = false;
    assert!(validate(&v).is_err());
    let mut v = cv().clone();
    v.pits_per_side = 8;
    assert!(validate(&v).is_err());

    let s = state([4; 12], [0, 1], Player::South);
    assert!(validate_state(cv(), &s).is_err(), "49 seeds");
}
