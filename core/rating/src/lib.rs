//! Skill ratings for games against other players: Glicko-2 (Glickman, "Example of the
//! Glicko-2 system", 2012) with fractional rating periods, as Lichess uses it (ADR 0034).
//!
//! - Ratings update right after every game. Instead of waiting for a rating period to close,
//!   a player's deviation first grows by the time that passed since their last rated game,
//!   measured in rating periods (a fraction, from [`periods`]).
//! - With exactly one elapsed period, [`update`] is Glickman's algorithm step for step; the
//!   tests check his worked example.
//! - Pure: no clocks or I/O. The caller passes elapsed time.
//!
//! Values are on the familiar scale (rating 1500, deviation 350); the Glicko-2 scale
//! (μ, φ) is internal.

/// Converts between the rating scale and Glicko-2's μ/φ scale (Glickman, step 2).
const SCALE: f64 = 173.7178;
/// Convergence tolerance for the volatility iteration (Glickman, step 5).
const EPSILON: f64 = 0.000_001;
/// One rating period, in seconds (ADR 0034: one day).
pub const PERIOD_SECS: f64 = 86_400.0;

/// A player's rating in one pool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    /// The estimate of skill (starts at 1500).
    pub rating: f64,
    /// Rating deviation (RD): how unsure the estimate is. Smaller is surer.
    pub deviation: f64,
    /// How erratic the player's results are (σ).
    pub volatility: f64,
}

/// System constants (ADR 0034; tune with real games).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// τ: how fast volatility can change. Glickman suggests 0.3 to 1.2.
    pub tau: f64,
    /// A new player's rating.
    pub initial: Rating,
    /// The deviation never drops below this...
    pub min_deviation: f64,
    /// ...or grows above this (also a new player's deviation).
    pub max_deviation: f64,
    /// Above this deviation a rating is provisional (shown with a `?`).
    pub provisional_deviation: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tau: 0.5,
            initial: Rating {
                rating: 1500.0,
                deviation: 350.0,
                volatility: 0.06,
            },
            min_deviation: 45.0,
            max_deviation: 350.0,
            provisional_deviation: 110.0,
        }
    }
}

impl Config {
    /// Whether `deviation` (already grown for time, see [`deviation_after`]) is provisional.
    pub fn is_provisional(&self, deviation: f64) -> bool {
        deviation > self.provisional_deviation
    }
}

/// A game's result from one player's side: 1 for a win, 0.5 for a draw, 0 for a loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    Win,
    Draw,
    Loss,
}

impl Score {
    fn value(self) -> f64 {
        match self {
            Score::Win => 1.0,
            Score::Draw => 0.5,
            Score::Loss => 0.0,
        }
    }

    /// The same game from the other side.
    pub fn flip(self) -> Score {
        match self {
            Score::Win => Score::Loss,
            Score::Draw => Score::Draw,
            Score::Loss => Score::Win,
        }
    }
}

/// Rating periods in `elapsed_secs` (negative times count as none).
pub fn periods(elapsed_secs: f64) -> f64 {
    (elapsed_secs / PERIOD_SECS).max(0.0)
}

/// The deviation after `elapsed_periods` without rated games: it grows with the volatility
/// (Glickman, step 6, applied for a fraction of a period), up to the maximum. Use it to
/// show a rating's current uncertainty and to decide leaderboard eligibility.
pub fn deviation_after(cfg: &Config, r: &Rating, elapsed_periods: f64) -> f64 {
    let phi = r.deviation / SCALE;
    let grown = (phi * phi + elapsed_periods.max(0.0) * r.volatility * r.volatility).sqrt();
    (grown * SCALE).clamp(cfg.min_deviation, cfg.max_deviation)
}

fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (std::f64::consts::PI * std::f64::consts::PI)).sqrt()
}

fn expected(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
    1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp())
}

/// The player's expected score (0 to 1) against `opponent`.
pub fn expected_score(player: &Rating, opponent: &Rating) -> f64 {
    expected(
        (player.rating - 1500.0) / SCALE,
        (opponent.rating - 1500.0) / SCALE,
        opponent.deviation / SCALE,
    )
}

/// The new volatility σ' (Glickman, step 5: the Illinois algorithm).
fn new_volatility(tau: f64, phi: f64, sigma: f64, v: f64, delta: f64) -> f64 {
    let a = (sigma * sigma).ln();
    let f = |x: f64| {
        let ex = x.exp();
        let denom = 2.0 * (phi * phi + v + ex).powi(2);
        ex * (delta * delta - phi * phi - v - ex) / denom - (x - a) / (tau * tau)
    };
    let mut big_a = a;
    let mut big_b = if delta * delta > phi * phi + v {
        (delta * delta - phi * phi - v).ln()
    } else {
        let mut k = 1.0;
        while f(a - k * tau) < 0.0 {
            k += 1.0;
        }
        a - k * tau
    };
    let mut f_a = f(big_a);
    let mut f_b = f(big_b);
    // Bounded, in case of pathological input; convergence normally takes a handful of steps.
    for _ in 0..100 {
        if (big_b - big_a).abs() <= EPSILON {
            break;
        }
        let big_c = big_a + (big_a - big_b) * f_a / (f_b - f_a);
        let f_c = f(big_c);
        if f_c * f_b <= 0.0 {
            big_a = big_b;
            f_a = f_b;
        } else {
            f_a /= 2.0;
        }
        big_b = big_c;
        f_b = f_c;
    }
    (big_a / 2.0).exp()
}

/// The player's rating after `results` (opponents' ratings as they were, with their scores),
/// `elapsed_periods` after the player's last update. With no results, only the deviation
/// grows. The opponents' deviations should already include their own elapsed time
/// ([`deviation_after`]).
pub fn update(
    cfg: &Config,
    player: &Rating,
    elapsed_periods: f64,
    results: &[(Rating, Score)],
) -> Rating {
    if results.is_empty() {
        return Rating {
            deviation: deviation_after(cfg, player, elapsed_periods),
            ..*player
        };
    }
    let mu = (player.rating - 1500.0) / SCALE;
    let phi = player.deviation / SCALE;
    let sigma = player.volatility;

    // Steps 3 and 4: estimated variance v and improvement Δ.
    let mut inv_v = 0.0;
    let mut sum = 0.0;
    for (opp, score) in results {
        let mu_j = (opp.rating - 1500.0) / SCALE;
        let phi_j = opp.deviation / SCALE;
        let e = expected(mu, mu_j, phi_j);
        let gj = g(phi_j);
        inv_v += gj * gj * e * (1.0 - e);
        sum += gj * (score.value() - e);
    }
    let v = 1.0 / inv_v;
    let delta = v * sum;

    // Step 5: the new volatility.
    let sigma_new = new_volatility(cfg.tau, phi, sigma, v, delta);
    // Step 6: the pre-period deviation, for the elapsed fraction of a period (1 = Glickman).
    let phi_star = (phi * phi + elapsed_periods.max(0.0) * sigma_new * sigma_new).sqrt();
    // Step 7: the new deviation and rating.
    let phi_new = 1.0 / (1.0 / (phi_star * phi_star) + 1.0 / v).sqrt();
    let mu_new = mu + phi_new * phi_new * sum;

    Rating {
        rating: mu_new * SCALE + 1500.0,
        deviation: (phi_new * SCALE).clamp(cfg.min_deviation, cfg.max_deviation),
        volatility: sigma_new,
    }
}

/// One game between `a` and `b`, each with the rating periods since their last update.
/// Returns both new ratings. `score_a` is `a`'s result.
pub fn rate_game(
    cfg: &Config,
    a: (&Rating, f64),
    b: (&Rating, f64),
    score_a: Score,
) -> (Rating, Rating) {
    // Each side sees the other as they are now, uncertainty grown for time.
    let a_now = Rating {
        deviation: deviation_after(cfg, a.0, a.1),
        ..*a.0
    };
    let b_now = Rating {
        deviation: deviation_after(cfg, b.0, b.1),
        ..*b.0
    };
    (
        update(cfg, a.0, a.1, &[(b_now, score_a)]),
        update(cfg, b.0, b.1, &[(a_now, score_a.flip())]),
    )
}

/// What `player` would gain or lose against `opponent` for each result, rounded to whole
/// points: `[win, draw, loss]`. For showing the stakes before a rated game.
pub fn stakes(cfg: &Config, player: (&Rating, f64), opponent: (&Rating, f64)) -> [i32; 3] {
    [Score::Win, Score::Draw, Score::Loss].map(|s| {
        let (after, _) = rate_game(cfg, player, opponent, s);
        (after.rating - player.0.rating).round() as i32
    })
}
