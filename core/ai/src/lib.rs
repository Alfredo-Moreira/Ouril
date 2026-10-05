//! `ouril-ai`: the computer opponent (`docs/architecture/ai.md`).
//!
//! **Deterministic:** no clocks and no ambient randomness. The search is bounded by a
//! **node budget** per level (not a wall-clock time budget, so results are reproducible on
//! every platform), and any randomness comes from the caller's `seed` through a small,
//! self-contained PRNG. The same `(variant, state, level, seed)` always returns the same move.
//!
//! Search: iterative-deepening negamax with alpha-beta pruning, captures-first move ordering
//! and a small transposition table (best move per position, used for ordering only). It
//! only calls the engine (`legal_moves`, `apply_move_quiet`), so it works for any variant.
//! A grand-slam extra turn (same player to move again) is handled without negating.

use ouril_engine::{
    apply_move_quiet, legal_moves, position_hash, Direction, GameState, Move, Player, Status,
    VariantConfig,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

/// Difficulty level. JSON: `"easy" | "medium" | "hard"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Easy,
    Medium,
    Hard,
}

impl Level {
    /// Maximum search nodes (engine `apply_move` calls) per decision. Placeholder values,
    /// to be tuned by playtesting (Hard must answer in < 1 s on a mid-range phone).
    pub fn node_budget(self) -> u64 {
        self.params().node_budget
    }

    /// Search parameters for this level (`docs/architecture/ai.md#difficulty-levels`).
    pub fn params(self) -> SearchParams {
        match self {
            // Depth 1–2; random among moves within 1.5 seeds of the best; 1 in 5 moves random.
            Level::Easy => SearchParams {
                max_depth: 2,
                node_budget: 2_000,
                margin: 150,
                random_move_percent: 20,
                tt_bits: 10,
            },
            // Depth up to 6, small randomness among near-equal moves.
            Level::Medium => SearchParams {
                max_depth: 6,
                node_budget: 50_000,
                margin: 30,
                random_move_percent: 0,
                tt_bits: 14,
            },
            // Whole budget, no randomness.
            Level::Hard => SearchParams {
                max_depth: 64,
                node_budget: 1_000_000,
                margin: 0,
                random_move_percent: 0,
                tt_bits: 17,
            },
        }
    }
}

/// Tunable search settings. Scores are in hundredths of a seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchParams {
    /// Deepest iteration (plies).
    pub max_depth: u32,
    /// Engine calls allowed per decision. Depth 1 always completes, even over budget.
    pub node_budget: u64,
    /// Pick randomly among moves scoring within this margin of the best (0 = best only).
    pub margin: i32,
    /// Chance (0–100) of playing a uniformly random legal move instead of searching.
    pub random_move_percent: u8,
    /// Transposition table size: `2^tt_bits` entries.
    pub tt_bits: u8,
}

/// The outcome of a search, for tests, hints and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best: Move,
    /// Score of `best` from the mover's point of view (hundredths of a seed, or ±WIN).
    pub score: i32,
    /// Deepest fully completed iteration.
    pub depth: u32,
    /// Engine calls made.
    pub nodes: u64,
    /// `(move, score)` for every root move of the last completed iteration. Scores below
    /// `best - margin` are upper bounds, not exact.
    pub root_scores: Vec<(Move, i32)>,
}

/// Score of a won game (minus the plies needed, so faster wins score higher).
pub const WIN: i32 = 1_000_000;

/// Pick a move for the player to move in `state`.
///
/// Returns `None` only when there is no legal move (the game is over).
/// Never returns an illegal move.
pub fn choose_move(v: &VariantConfig, state: &GameState, level: Level, seed: u64) -> Option<Move> {
    choose_move_with(v, state, level.params(), seed)
}

/// [`choose_move`] with explicit parameters.
pub fn choose_move_with(
    v: &VariantConfig,
    state: &GameState,
    p: SearchParams,
    seed: u64,
) -> Option<Move> {
    let legal = legal_moves(v, state);
    match legal.len() {
        0 => return None,
        1 => return Some(legal[0]),
        _ => {}
    }
    // Mix the position into the seed so one game seed still varies from move to move.
    let mut rng = SplitMix64(seed ^ position_hash(state).rotate_left(17) ^ stores_key(state));
    if p.random_move_percent > 0 && rng.below(100) < p.random_move_percent as u64 {
        return Some(legal[rng.below(legal.len() as u64) as usize]);
    }
    let r = search(v, state, p)?;
    if p.margin == 0 {
        return Some(r.best);
    }
    let candidates: Vec<Move> = r
        .root_scores
        .iter()
        .filter(|(_, s)| *s >= r.score.saturating_sub(p.margin))
        .map(|(m, _)| *m)
        .collect();
    Some(candidates[rng.below(candidates.len() as u64) as usize])
}

/// Run the search. `None` when there is no legal move.
pub fn search(v: &VariantConfig, state: &GameState, p: SearchParams) -> Option<SearchResult> {
    let legal = legal_moves(v, state);
    if legal.is_empty() {
        return None;
    }
    let mut s = Searcher {
        v,
        nodes: 0,
        budget: p.node_budget,
        enforce_budget: false,
        aborted: false,
        tt: vec![TtEntry::default(); 1usize << p.tt_bits.clamp(4, 22)],
        tt_mask: (1u64 << p.tt_bits.clamp(4, 22)) - 1,
    };
    let me = state.to_move;

    // Root children, generated once.
    let mut root: Vec<(Move, GameState)> = legal
        .iter()
        .map(|&m| (m, apply_move_quiet(v, state, m).expect("legal move")))
        .collect();
    s.nodes += root.len() as u64;
    // Initial ordering: biggest immediate gain first, then pit order.
    root.sort_by_key(|(m, c)| (-(c.stores[me as usize] as i32), *m));

    let mut completed: Option<SearchResult> = None;
    for depth in 1..=p.max_depth.max(1) {
        s.enforce_budget = depth > 1;
        let mut best = -WIN - 1;
        let mut best_move = root[0].0;
        let mut scores: Vec<(Move, i32)> = Vec::with_capacity(root.len());
        for (m, child) in &root {
            let alpha = if best <= -WIN {
                -WIN - 1
            } else {
                best - p.margin - 1
            };
            let score = s.child_score(me, child, depth - 1, alpha, WIN + 1, 1);
            if s.aborted {
                break;
            }
            scores.push((*m, score));
            if score > best {
                best = score;
                best_move = *m;
            }
        }
        if s.aborted {
            break;
        }
        // Next iteration searches the best moves first (stable: ties keep pit order).
        let order: Vec<Move> = {
            let mut sorted = scores.clone();
            sorted.sort_by_key(|(m, sc)| (-*sc, *m));
            sorted.into_iter().map(|(m, _)| m).collect()
        };
        root.sort_by_key(|(m, _)| order.iter().position(|o| o == m));
        completed = Some(SearchResult {
            best: best_move,
            score: best,
            depth,
            nodes: s.nodes,
            root_scores: scores,
        });
        if best.abs() >= WIN - 1_000 {
            break; // forced result found
        }
    }
    let mut r = completed.expect("depth 1 always completes");
    r.nodes = s.nodes;
    Some(r)
}

#[derive(Clone, Copy, Default)]
struct TtEntry {
    key: u64,
    best: u8,
    used: bool,
}

struct Searcher<'a> {
    v: &'a VariantConfig,
    nodes: u64,
    budget: u64,
    enforce_budget: bool,
    aborted: bool,
    tt: Vec<TtEntry>,
    tt_mask: u64,
}

impl Searcher<'_> {
    /// Score of `child` (reached by a move of `me`) from `me`'s point of view.
    fn child_score(
        &mut self,
        me: Player,
        child: &GameState,
        depth: u32,
        alpha: i32,
        beta: i32,
        ply: i32,
    ) -> i32 {
        if child.status != Status::Playing {
            return terminal_score(child, me, ply);
        }
        if child.to_move == me {
            // Extra turn: same player, no sign change.
            self.negamax(child, depth, alpha, beta, ply)
        } else {
            -self.negamax(child, depth, -beta, -alpha, ply)
        }
    }

    /// Score of `s` for `s.to_move` (fail-hard alpha-beta).
    fn negamax(&mut self, s: &GameState, depth: u32, mut alpha: i32, beta: i32, ply: i32) -> i32 {
        if depth == 0 {
            return evaluate(self.v, s);
        }
        let me = s.to_move;
        let key = position_hash(s) ^ stores_key(s);
        let slot = (key & self.tt_mask) as usize;
        let tt_move = {
            let e = self.tt[slot];
            (e.used && e.key == key).then_some(e.best)
        };

        let moves = legal_moves(self.v, s);
        if moves.is_empty() {
            return evaluate(self.v, s); // unreachable while playing; defensive
        }
        let mut children: Vec<(Move, GameState)> = Vec::with_capacity(moves.len());
        for m in moves {
            if self.enforce_budget && self.nodes >= self.budget {
                self.aborted = true;
                return 0;
            }
            self.nodes += 1;
            children.push((m, apply_move_quiet(self.v, s, m).expect("legal move")));
        }
        // Order: transposition-table move, then captures (biggest gain), then pit order.
        children.sort_by_key(|(m, c)| (Some(*m) != tt_move, -(c.stores[me as usize] as i32), *m));

        let mut best_move = children[0].0;
        for (m, child) in &children {
            let score = self.child_score(me, child, depth - 1, alpha, beta, ply + 1);
            if self.aborted {
                return 0;
            }
            if score > alpha {
                alpha = score;
                best_move = *m;
                if alpha >= beta {
                    break;
                }
            }
        }
        self.tt[slot] = TtEntry {
            key,
            best: best_move,
            used: true,
        };
        alpha
    }
}

fn terminal_score(s: &GameState, me: Player, ply: i32) -> i32 {
    match s.status {
        Status::Won(p) if p == me => WIN - ply,
        Status::Won(_) => -(WIN - ply),
        _ => 0,
    }
}

fn stores_key(s: &GameState) -> u64 {
    (s.stores[0] as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (s.stores[1] as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
}

/// Static evaluation for `s.to_move`, in hundredths of a seed:
/// store difference, plus small terms for seeds on each side and capture threats
/// (opponent pits holding 1–2 seeds that a move would land on).
fn evaluate(v: &VariantConfig, s: &GameState) -> i32 {
    let me = s.to_move;
    let opp = me.opponent();
    let n = s.pits_per_side as usize;
    let side = |p: Player| match p {
        Player::South => 0..n,
        Player::North => n..2 * n,
    };
    let store = |p: Player| s.stores[p as usize] as i32;
    let on_side = |p: Player| side(p).map(|i| s.pits[i] as i32).sum::<i32>();
    let threats = |p: Player| -> i32 {
        let len = 2 * n;
        side(p)
            .filter(|&i| s.pits[i] > 0 && (s.pits[i] as usize) < len)
            .filter(|&i| {
                let c = s.pits[i] as usize;
                let land = match v.direction {
                    Direction::Ccw => (i + c) % len,
                    Direction::Cw => (i + len - c % len) % len,
                };
                !side(p).contains(&land) && (1..=2).contains(&s.pits[land])
            })
            .count() as i32
    };
    100 * (store(me) - store(opp))
        + 5 * (on_side(me) - on_side(opp))
        + 15 * (threats(me) - threats(opp))
}

/// SplitMix64: tiny, seedable, identical on every platform.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-enough value in `0..n` (n is tiny here, so modulo bias is negligible).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ouril_engine::{apply_move, new_game, variant_version, GameStateJson};

    fn cv() -> &'static VariantConfig {
        variant_version("cv.standard", 1).unwrap()
    }

    fn state(pits: [u8; 12], stores: [u8; 2], to_move: Player) -> GameState {
        GameState::try_from(GameStateJson {
            pits: pits.to_vec(),
            stores,
            to_move,
            moves_since_capture: 0,
            status: Status::Playing,
            history: vec![],
        })
        .unwrap()
    }

    #[test]
    fn returns_none_when_game_is_over() {
        let mut s = new_game(cv(), Player::South);
        s.status = Status::Draw;
        assert_eq!(choose_move(cv(), &s, Level::Hard, 1), None);
    }

    #[test]
    fn single_legal_move_is_returned_immediately() {
        let s = state(
            [0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0],
            [23, 22],
            Player::South,
        );
        for level in [Level::Easy, Level::Medium, Level::Hard] {
            assert_eq!(choose_move(cv(), &s, level, 7), Some(5));
        }
    }

    #[test]
    fn deterministic_for_same_inputs() {
        let s = new_game(cv(), Player::South);
        for level in [Level::Easy, Level::Medium] {
            for seed in [0, 1, 42, u64::MAX] {
                let a = choose_move(cv(), &s, level, seed);
                let b = choose_move(cv(), &s, level, seed);
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn takes_the_winning_capture() {
        // South at 23: pit 4 captures 2 in pit 7 and wins (threshold-win vector position).
        let s = state([0, 0, 0, 0, 3, 0, 4, 1, 4, 4, 4, 4], [23, 1], Player::South);
        for level in [Level::Medium, Level::Hard] {
            assert_eq!(choose_move(cv(), &s, level, 3), Some(4), "{level:?}");
        }
        let r = search(cv(), &s, Level::Medium.params()).unwrap();
        assert!(r.score >= WIN - 10, "found as a forced win: {r:?}");
    }

    #[test]
    fn node_budget_is_respected_after_depth_one() {
        let s = new_game(cv(), Player::South);
        let p = SearchParams {
            max_depth: 64,
            node_budget: 5_000,
            margin: 0,
            random_move_percent: 0,
            tt_bits: 10,
        };
        let r = search(cv(), &s, p).unwrap();
        // Overshoot is at most one node's children.
        assert!(r.nodes <= p.node_budget + 12, "{}", r.nodes);
        assert!(r.depth >= 2);
    }

    #[test]
    fn ai_vs_ai_games_only_play_legal_moves_and_finish() {
        for (i, (south, north)) in [
            (Level::Easy, Level::Medium),
            (Level::Medium, Level::Easy),
            (Level::Easy, Level::Easy),
        ]
        .into_iter()
        .enumerate()
        {
            let mut s = new_game(cv(), Player::South);
            let mut plies = 0;
            while s.status == Status::Playing {
                let level = if s.to_move == Player::South {
                    south
                } else {
                    north
                };
                let m = choose_move(cv(), &s, level, i as u64 * 1000 + plies).expect("a move");
                s = apply_move(cv(), &s, m).expect("legal").state;
                plies += 1;
                assert!(plies < 2_000);
            }
        }
    }

    #[test]
    fn medium_beats_easy_more_often_than_not() {
        let mut medium_wins = 0;
        let mut easy_wins = 0;
        for g in 0..4u64 {
            let medium_side = if g % 2 == 0 {
                Player::South
            } else {
                Player::North
            };
            let mut s = new_game(cv(), Player::South);
            let mut ply = 0u64;
            while s.status == Status::Playing {
                let level = if s.to_move == medium_side {
                    Level::Medium
                } else {
                    Level::Easy
                };
                let m = choose_move(cv(), &s, level, g * 7919 + ply).unwrap();
                s = apply_move(cv(), &s, m).unwrap().state;
                ply += 1;
            }
            match s.status {
                Status::Won(p) if p == medium_side => medium_wins += 1,
                Status::Won(_) => easy_wins += 1,
                _ => {}
            }
        }
        assert!(
            medium_wins > easy_wins,
            "medium {medium_wins} vs easy {easy_wins}"
        );
    }
}
