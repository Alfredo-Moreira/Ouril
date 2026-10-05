//! Move generation and move application. Every behaviour is driven by [`VariantConfig`]
//! (ADR 0004). Rules come only from `docs/game/rules.md`, the variant specs and the test
//! vectors; points they don't cover are marked **Engine convention** below and listed in
//! the engine's open questions.

use crate::types::*;

/// Receives the events of a move. `Vec<MoveEvent>` records them; [`NoEvents`] drops them
/// (used by the AI search, which calls the engine millions of times).
pub(crate) trait Sink {
    fn push(&mut self, e: MoveEvent);
}

impl Sink for Vec<MoveEvent> {
    #[inline]
    fn push(&mut self, e: MoveEvent) {
        Vec::push(self, e);
    }
}

pub(crate) struct NoEvents;

impl Sink for NoEvents {
    #[inline]
    fn push(&mut self, _e: MoveEvent) {}
}

/// Board geometry for one variant.
#[derive(Clone, Copy)]
struct Board {
    /// Pits per side.
    n: usize,
    ccw: bool,
}

impl Board {
    fn new(v: &VariantConfig) -> Board {
        Board {
            // Clamp so a malformed config can never index out of bounds.
            n: (v.pits_per_side as usize).clamp(1, MAX_PITS / 2),
            ccw: v.direction == Direction::Ccw,
        }
    }

    #[inline]
    fn len(self) -> usize {
        2 * self.n
    }

    #[inline]
    fn owner(self, pit: usize) -> Player {
        if pit < self.n {
            Player::South
        } else {
            Player::North
        }
    }

    #[inline]
    fn side(self, p: Player) -> std::ops::Range<usize> {
        match p {
            Player::South => 0..self.n,
            Player::North => self.n..2 * self.n,
        }
    }

    /// The next pit in sowing order.
    #[inline]
    fn next(self, pit: usize) -> usize {
        if self.ccw {
            (pit + 1) % self.len()
        } else {
            (pit + self.len() - 1) % self.len()
        }
    }

    /// The previous pit in sowing order (chain captures walk this way).
    #[inline]
    fn prev(self, pit: usize) -> usize {
        if self.ccw {
            (pit + self.len() - 1) % self.len()
        } else {
            (pit + 1) % self.len()
        }
    }

    /// The pit directly across the board (South pit `i` faces North pit `2n-1-i`).
    #[inline]
    fn opposite(self, pit: usize) -> usize {
        self.len() - 1 - pit
    }

    fn side_sum(self, pits: &[u8; MAX_PITS], p: Player) -> u16 {
        self.side(p).map(|i| pits[i] as u16).sum()
    }

    /// Does sowing `seeds` from `pit` put at least one seed on the other side?
    fn reaches_opponent(self, pit: usize, seeds: u8) -> bool {
        let me = self.owner(pit);
        let mut p = pit;
        for step in 1..=self.len() {
            p = self.next(p);
            if self.owner(p) != me {
                return seeds as usize >= step;
            }
        }
        false
    }
}

/// Engine convention (ADR 0023): a relay-sowing move stops after this many relay laps.
/// Simulation found no endless turn (the longest 48-seed turn found took 15 laps), so this
/// is a safeguard only.
pub(crate) const MAX_RELAY_LAPS: u16 = 1000;

/// Outcome of sowing one pit (before captures are applied).
struct Sowing {
    pits: [u8; MAX_PITS],
    /// At least one seed was dropped on the opponent's side (in any lap). This is what
    /// "feeding" means, also under relay sowing.
    reached_opponent: bool,
    /// Pits that would be captured, in capture order, with their seed counts.
    captures: [(u8, u8); MAX_PITS / 2],
    n_captures: usize,
    /// The captures would take every seed on the opponent's side.
    grand_slam: bool,
}

fn sow<S: Sink>(v: &VariantConfig, b: Board, s: &GameState, origin: usize, ev: &mut S) -> Sowing {
    let me = s.to_move;
    let opp = me.opponent();
    let mut pits = s.pits;
    let mut seeds = pits[origin];
    pits[origin] = 0;
    let mut pit = origin;
    // The pit the current lap started from (the origin, or the last relay pickup).
    let mut lap_origin = origin;
    let mut reached_opponent = false;
    let mut relays: u16 = 0;
    loop {
        while seeds > 0 {
            pit = b.next(pit);
            if pit == lap_origin && v.skip_origin_on_lap {
                ev.push(MoveEvent::SkipOrigin { pit: pit as u8 });
                continue;
            }
            pits[pit] = pits[pit].saturating_add(1);
            seeds -= 1;
            reached_opponent |= b.owner(pit) == opp;
            ev.push(MoveEvent::Sow { pit: pit as u8 });
        }
        // Relay sowing: the last seed landed in an own pit that already held seeds.
        let relay = v.relay_sowing == RelaySowing::OwnSideOccupied
            && b.owner(pit) == me
            && pits[pit] > 1
            && relays < MAX_RELAY_LAPS;
        if !relay {
            break;
        }
        relays += 1;
        seeds = pits[pit];
        pits[pit] = 0;
        lap_origin = pit;
        ev.push(MoveEvent::Relay {
            pit: pit as u8,
            seeds,
        });
    }

    let mut captures = [(0u8, 0u8); MAX_PITS / 2];
    let mut n_captures = 0;
    let mut captured_total: u16 = 0;
    match v.capture_mode {
        CaptureMode::OpponentSide => {
            let mut p = pit;
            while b.owner(p) == opp && n_captures < b.n && v.capture_counts.contains(&pits[p]) {
                captures[n_captures] = (p as u8, pits[p]);
                captured_total += pits[p] as u16;
                n_captures += 1;
                if v.chain_capture == ChainCapture::None {
                    break;
                }
                p = b.prev(p);
            }
        }
        CaptureMode::AcrossFromEmptyOwnPit => {
            // The last seed landed in an own pit that was empty (it now holds 1) and the pit
            // across has seeds: capture them all (ADR 0024); the landing seed stays. With
            // `chain_capture: backward`, the own pits sown just before it are checked the same
            // way, stopping at the first that doesn't qualify or at the end of the own row.
            let mut p = pit;
            while b.owner(p) == me && n_captures < b.n {
                let across = b.opposite(p);
                if pits[p] != 1 || pits[across] == 0 {
                    break;
                }
                captures[n_captures] = (across as u8, pits[across]);
                captured_total += pits[across] as u16;
                n_captures += 1;
                if v.chain_capture == ChainCapture::None {
                    break;
                }
                p = b.prev(p);
            }
        }
    }
    let grand_slam = n_captures > 0 && captured_total == b.side_sum(&pits, opp);
    Sowing {
        pits,
        reached_opponent,
        captures,
        n_captures,
        grand_slam,
    }
}

/// Legal moves split by stage, so errors can name the rule that excluded a move.
struct Legality {
    /// Feeding is required (the opponent has no seeds).
    feeding_required: bool,
    /// After the single-seed and feeding rules.
    before_grand_slam: Vec<Move>,
    /// Final legal moves.
    legal: Vec<Move>,
}

/// Does sowing `pit` put at least one seed on the opponent's side? With relay sowing, any
/// lap counts, even if the move then captures those seeds back (ADR 0023).
fn feeds(v: &VariantConfig, b: Board, s: &GameState, pit: usize) -> bool {
    match v.relay_sowing {
        RelaySowing::None => b.reaches_opponent(pit, s.pits[pit]),
        RelaySowing::OwnSideOccupied => sow(v, b, s, pit, &mut NoEvents).reached_opponent,
    }
}

fn feeding_required(v: &VariantConfig, b: Board, s: &GameState) -> bool {
    // The grand-slam extra turn "must feed" even if a variant had must_feed = false.
    (v.must_feed || v.grand_slam == GrandSlam::CapturesAllExtraTurnMustFeed)
        && b.side_sum(&s.pits, s.to_move.opponent()) == 0
}

fn legality(v: &VariantConfig, s: &GameState) -> Legality {
    let b = Board::new(v);
    let me = s.to_move;
    let mut out = Legality {
        feeding_required: false,
        before_grand_slam: Vec::new(),
        legal: Vec::new(),
    };
    if s.status != Status::Playing {
        return out;
    }
    let candidates: Vec<usize> = b.side(me).filter(|&i| s.pits[i] > 0).collect();
    if candidates.is_empty() {
        return out;
    }

    // Single-seed rule: no 1-seed pit while any own pit holds 2 or more.
    let single_rule_applies = v.single_seed_rule == SingleSeedRule::OnlyIfNoLargerPit
        && candidates.iter().any(|&i| s.pits[i] >= 2);
    let single_ok = |i: &usize| !single_rule_applies || s.pits[*i] >= 2;

    out.feeding_required = feeding_required(v, b, s);
    let stage: Vec<usize> = if out.feeding_required {
        let feeds = |i: &usize| feeds(v, b, s, *i);
        let f: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|i| single_ok(i) && feeds(i))
            .collect();
        if f.is_empty() && v.feeding_overrides_single_seed_rule {
            // Feeding beats the single-seed rule when only single-seed pits can feed
            // (cv.standard app decision 1).
            candidates.iter().copied().filter(feeds).collect()
        } else {
            f
        }
    } else {
        candidates.iter().copied().filter(single_ok).collect()
    };
    out.before_grand_slam = stage.iter().map(|&i| i as Move).collect();

    out.legal = if v.grand_slam == GrandSlam::Forbidden {
        let ok: Vec<Move> = stage
            .iter()
            .filter(|&&i| !sow(v, b, s, i, &mut NoEvents).grand_slam)
            .map(|&i| i as Move)
            .collect();
        // Engine convention: if every move would be a grand slam, they stay legal and
        // capture nothing (see `apply`).
        if ok.is_empty() {
            out.before_grand_slam.clone()
        } else {
            ok
        }
    } else {
        out.before_grand_slam.clone()
    };
    out
}

pub(crate) fn legal_moves(v: &VariantConfig, s: &GameState) -> Vec<Move> {
    legality(v, s).legal
}

fn check_move(v: &VariantConfig, s: &GameState, m: Move) -> Result<(), MoveError> {
    let b = Board::new(v);
    if s.status != Status::Playing {
        return Err(MoveError::GameOver);
    }
    let pit = m as usize;
    if pit >= b.len() || b.owner(pit) != s.to_move {
        return Err(MoveError::NotOwnPit);
    }
    if s.pits[pit] == 0 {
        return Err(MoveError::EmptyPit);
    }
    let l = legality(v, s);
    if l.legal.contains(&m) {
        return Ok(());
    }
    if l.feeding_required && !feeds(v, b, s, pit) {
        return Err(MoveError::MustFeed);
    }
    if !l.before_grand_slam.contains(&m) {
        return Err(MoveError::SingleSeedRule);
    }
    Err(MoveError::GrandSlamForbidden)
}

/// Mix function (SplitMix64 finalizer): derives the Zobrist key of each `(pit, seeds)` and
/// of the side to move, so no key table or random generator is needed.
#[inline]
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Zobrist hash of a position: the pits and the player to move (stores can't change
/// without a capture, and history is reset on every capture).
pub(crate) fn position_hash(s: &GameState) -> PositionHash {
    let mut h = 0u64;
    for (i, &c) in s.board().iter().enumerate() {
        if c != 0 {
            h ^= mix(((i as u64) << 16) | c as u64);
        }
    }
    if s.to_move == Player::North {
        h ^= mix(0xFFFF_FFFF);
    }
    h
}

/// Collect remaining seeds per `outcome`, set the status and emit `game_over`.
fn finish<S: Sink>(
    b: Board,
    st: &mut GameState,
    collect: Collect,
    reason: EndReason,
    threshold_winner: Option<Player>,
    draw: bool,
    ev: &mut S,
) {
    let players: &[Player] = match collect {
        Collect::Each => &[Player::South, Player::North],
        Collect::Only(Player::South) => &[Player::South],
        Collect::Only(Player::North) => &[Player::North],
        Collect::Nothing => &[],
    };
    for &p in players {
        let seeds = b.side_sum(&st.pits, p);
        if seeds > 0 {
            for i in b.side(p) {
                st.pits[i] = 0;
            }
            let idx = p as usize;
            st.stores[idx] = st.stores[idx].saturating_add(seeds as u8);
            ev.push(MoveEvent::CollectRemaining {
                player: p,
                seeds: seeds as u8,
            });
        }
    }
    let result = if let Some(w) = threshold_winner {
        w.into()
    } else if draw {
        GameResult::Draw
    } else {
        match st.stores[0].cmp(&st.stores[1]) {
            std::cmp::Ordering::Greater => GameResult::SouthWins,
            std::cmp::Ordering::Less => GameResult::NorthWins,
            std::cmp::Ordering::Equal => GameResult::Draw,
        }
    };
    st.status = result.into();
    ev.push(MoveEvent::GameOver { result, reason });
}

#[derive(Clone, Copy)]
enum Collect {
    Each,
    Only(Player),
    Nothing,
}

pub(crate) fn apply<S: Sink>(
    v: &VariantConfig,
    s: &GameState,
    m: Move,
    ev: &mut S,
) -> Result<GameState, MoveError> {
    check_move(v, s, m)?;
    let b = Board::new(v);
    let me = s.to_move;
    let opp = me.opponent();

    let sowing = sow(v, b, s, m as usize, ev);
    let mut st = s.clone();
    st.pits = sowing.pits;

    // Grand slam handling. `CapturesAll` and `CapturesAllExtraTurnMustFeed` keep the
    // captures; the others capture nothing (`Forbidden` only gets here when every legal
    // move is a grand slam; `Loses` is unsupported, see `validate`).
    let keep_captures = !sowing.grand_slam
        || matches!(
            v.grand_slam,
            GrandSlam::CapturesAll | GrandSlam::CapturesAllExtraTurnMustFeed
        );
    let captured = sowing.n_captures > 0 && keep_captures;
    if captured {
        for &(pit, seeds) in &sowing.captures[..sowing.n_captures] {
            st.pits[pit as usize] = 0;
            st.stores[me as usize] = st.stores[me as usize].saturating_add(seeds);
            ev.push(MoveEvent::Capture { pit, seeds });
        }
        if sowing.grand_slam {
            ev.push(MoveEvent::GrandSlam);
        }
        st.moves_since_capture = 0;
    } else {
        st.moves_since_capture = st.moves_since_capture.saturating_add(1);
    }

    // 1. Win threshold: the mover wins at once; each player takes their own side
    //    (engine-wide convention, cv.standard app decision 5).
    if captured && st.stores[me as usize] >= v.win_threshold {
        finish(
            b,
            &mut st,
            Collect::Each,
            EndReason::Threshold,
            Some(me),
            false,
            ev,
        );
        return Ok(st);
    }

    let extra_turn =
        captured && sowing.grand_slam && v.grand_slam == GrandSlam::CapturesAllExtraTurnMustFeed;
    st.to_move = if extra_turn { me } else { opp };

    // Repetition bookkeeping: hashes of the positions since the last capture.
    let h = position_hash(&st);
    let mut repeated = false;
    if captured {
        st.history.clear();
    } else {
        if st.history.is_empty() {
            // A position given without history (e.g. a test-vector setup) still counts.
            st.history.push(position_hash(s));
        }
        repeated = st.history.contains(&h);
    }
    st.history.push(h);

    // 2. The next player can't move: they can't feed a starving opponent (no_feed, also
    //    when they have no seeds themselves, e.g. after a grand slam that emptied both
    //    sides), or they have no seeds while the opponent has some (no_moves; engine
    //    convention: each takes own side).
    if legality(v, &st).legal.is_empty() {
        let next = st.to_move;
        if b.side_sum(&st.pits, next) == 0 && b.side_sum(&st.pits, next.opponent()) > 0 {
            finish(
                b,
                &mut st,
                Collect::Each,
                EndReason::NoMoves,
                None,
                false,
                ev,
            );
        } else {
            let collect = match v.no_feed_outcome {
                EndOutcome::MoverTakesOwnSide => Collect::Only(next),
                EndOutcome::EachTakesOwnSide => Collect::Each,
                EndOutcome::RemainingNotScored => Collect::Nothing,
            };
            finish(b, &mut st, collect, EndReason::NoFeed, None, false, ev);
        }
        return Ok(st);
    }

    if extra_turn {
        ev.push(MoveEvent::ExtraTurn);
    }

    // 3. Endless cycle: a repeated position, or the move limit without a capture.
    let limit_hit = v
        .endless_cycle_move_limit
        .is_some_and(|l| st.moves_since_capture >= l);
    if !captured && (repeated || limit_hit) {
        let (collect, draw) = match v.endless_cycle_outcome {
            CycleOutcome::EachTakesOwnSide => (Collect::Each, false),
            CycleOutcome::RemainingNotScored => (Collect::Nothing, false),
            CycleOutcome::Draw => (Collect::Nothing, true),
        };
        finish(b, &mut st, collect, EndReason::EndlessCycle, None, draw, ev);
    }
    Ok(st)
}

pub(crate) fn new_game(v: &VariantConfig, first: Player) -> GameState {
    let b = Board::new(v);
    let mut pits = [0u8; MAX_PITS];
    for p in pits.iter_mut().take(b.len()) {
        *p = v.seeds_per_pit;
    }
    let mut s = GameState {
        pits,
        pits_per_side: b.n as u8,
        stores: [0, 0],
        to_move: first,
        moves_since_capture: 0,
        status: Status::Playing,
        history: Vec::new(),
    };
    s.history.push(position_hash(&s));
    s
}

/// Check that a config is complete and uses only behaviour the engine implements.
pub(crate) fn validate(v: &VariantConfig) -> Result<(), String> {
    if v.pits_per_side == 0 || v.pits_per_side as usize > MAX_PITS / 2 {
        return Err(format!("pits_per_side must be 1..={}", MAX_PITS / 2));
    }
    if v.seeds_per_pit == 0 {
        return Err("seeds_per_pit must be at least 1".into());
    }
    let total = 2 * v.pits_per_side as u32 * v.seeds_per_pit as u32;
    if total > u8::MAX as u32 {
        return Err(format!("total seeds {total} exceed {}", u8::MAX));
    }
    if v.capture_counts.is_empty() {
        return Err("capture_counts must not be empty".into());
    }
    if v.win_threshold == 0 || v.win_threshold as u32 > total {
        return Err("win_threshold must be 1..=total seeds".into());
    }
    if !v.capture_mandatory {
        // A move is only a pit index, so an optional capture can't be expressed yet.
        return Err("capture_mandatory = false is not supported by the engine yet".into());
    }
    if v.grand_slam == GrandSlam::Loses {
        return Err("grand_slam = loses is not supported by the engine yet".into());
    }
    Ok(())
}

/// Check that a position is consistent with a variant: board size and seed conservation.
pub(crate) fn validate_state(v: &VariantConfig, s: &GameState) -> Result<(), String> {
    if s.pits_per_side != v.pits_per_side {
        return Err(format!(
            "state has {} pits per side, variant {} has {}",
            s.pits_per_side, v.id, v.pits_per_side
        ));
    }
    let total = 2 * v.pits_per_side as u32 * v.seeds_per_pit as u32;
    let sum: u32 = s.board().iter().map(|&p| p as u32).sum::<u32>()
        + s.stores.iter().map(|&p| p as u32).sum::<u32>();
    if sum != total {
        return Err(format!(
            "seeds on board and in stores sum to {sum}, expected {total}"
        ));
    }
    if s.history.len() > 4096 {
        return Err("history too long".into());
    }
    Ok(())
}

impl From<Player> for GameResult {
    fn from(p: Player) -> GameResult {
        match p {
            Player::South => GameResult::SouthWins,
            Player::North => GameResult::NorthWins,
        }
    }
}

impl From<GameResult> for Status {
    fn from(r: GameResult) -> Status {
        match r {
            GameResult::SouthWins => Status::Won(Player::South),
            GameResult::NorthWins => Status::Won(Player::North),
            GameResult::Draw => Status::Draw,
        }
    }
}
