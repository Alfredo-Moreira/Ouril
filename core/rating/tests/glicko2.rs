//! Glicko-2 against Glickman's worked example, plus properties every update must keep.

use ouril_rating::{
    deviation_after, periods, rate_game, update, Config, Rating, Score, PERIOD_SECS,
};
use proptest::prelude::*;

fn r(rating: f64, deviation: f64, volatility: f64) -> Rating {
    Rating {
        rating,
        deviation,
        volatility,
    }
}

/// Glickman's example: τ = 0.5; a 1500/200/0.06 player beats 1400/30 and loses to 1550/100
/// and 1700/300 in one rating period, giving 1464.06 / 151.52 / 0.05999.
#[test]
fn glickmans_worked_example() {
    let cfg = Config {
        min_deviation: 0.0,
        ..Config::default()
    };
    let after = update(
        &cfg,
        &r(1500.0, 200.0, 0.06),
        1.0,
        &[
            (r(1400.0, 30.0, 0.06), Score::Win),
            (r(1550.0, 100.0, 0.06), Score::Loss),
            (r(1700.0, 300.0, 0.06), Score::Loss),
        ],
    );
    assert!((after.rating - 1464.06).abs() < 0.01, "{after:?}");
    assert!((after.deviation - 151.52).abs() < 0.01, "{after:?}");
    assert!((after.volatility - 0.05999).abs() < 0.00001, "{after:?}");
}

#[test]
fn equal_players_drawing_keep_their_rating_and_get_surer() {
    let cfg = Config::default();
    let p = r(1600.0, 120.0, 0.06);
    let (a, b) = rate_game(&cfg, (&p, 1.0), (&p, 1.0), Score::Draw);
    assert!((a.rating - 1600.0).abs() < 1e-9);
    assert!((b.rating - 1600.0).abs() < 1e-9);
    assert!(a.deviation < 120.0 + 1.0);
}

#[test]
fn a_win_between_equals_moves_both_by_the_same_amount() {
    let cfg = Config::default();
    let p = r(1500.0, 350.0, 0.06);
    let (w, l) = rate_game(&cfg, (&p, 0.0), (&p, 0.0), Score::Win);
    assert!(w.rating > 1500.0 && l.rating < 1500.0);
    assert!(((w.rating - 1500.0) - (1500.0 - l.rating)).abs() < 1e-9);
}

#[test]
fn new_players_move_more_than_established_ones() {
    let cfg = Config::default();
    let new = r(1500.0, 350.0, 0.06);
    let old = r(1500.0, 60.0, 0.06);
    let (new_after, old_after) = rate_game(&cfg, (&new, 0.0), (&old, 0.0), Score::Win);
    assert!(new_after.rating - 1500.0 > 1500.0 - old_after.rating);
}

#[test]
fn deviation_grows_with_time_and_is_capped() {
    let cfg = Config::default();
    let p = r(1800.0, 60.0, 0.06);
    let a_month = deviation_after(&cfg, &p, periods(30.0 * PERIOD_SECS));
    let a_year = deviation_after(&cfg, &p, periods(365.0 * PERIOD_SECS));
    assert!(60.0 < a_month && a_month < a_year);
    assert!(deviation_after(&cfg, &p, 1e9) <= cfg.max_deviation);
    assert_eq!(update(&cfg, &p, 2.0, &[]).rating, 1800.0);
}

#[test]
fn time_never_runs_backwards() {
    let cfg = Config::default();
    let p = r(1500.0, 80.0, 0.06);
    assert_eq!(periods(-10.0), 0.0);
    assert_eq!(deviation_after(&cfg, &p, -3.0), 80.0);
}

#[test]
fn provisional_above_the_threshold() {
    let cfg = Config::default();
    assert!(cfg.is_provisional(cfg.initial.deviation));
    assert!(!cfg.is_provisional(100.0));
}

#[test]
fn a_long_absence_never_moves_a_rating_more_than_a_new_players_first_game() {
    let cfg = Config::default();
    let opponent = r(1500.0, 60.0, 0.06);
    // A volatile player back after a year: their grown deviation is capped at 350.
    let returning = r(1500.0, 290.0, 0.1);
    let (back, _) = rate_game(&cfg, (&returning, 365.0), (&opponent, 0.0), Score::Win);
    let new = cfg.initial;
    let (first, _) = rate_game(&cfg, (&new, 0.0), (&opponent, 0.0), Score::Win);
    assert!(
        back.rating - 1500.0 <= first.rating - 1500.0 + 1e-6,
        "{back:?} vs {first:?}"
    );
}

proptest! {
    #[test]
    fn updates_stay_finite_and_in_range(
        ra in 100.0f64..3000.0, da in 45.0f64..350.0, va in 0.01f64..0.2,
        rb in 100.0f64..3000.0, db in 45.0f64..350.0, vb in 0.01f64..0.2,
        ta in 0.0f64..400.0, tb in 0.0f64..400.0, s in 0u8..3,
    ) {
        let cfg = Config::default();
        let score = [Score::Win, Score::Draw, Score::Loss][s as usize];
        let a = r(ra, da, va);
        let b = r(rb, db, vb);
        let (a2, b2) = rate_game(&cfg, (&a, ta), (&b, tb), score);
        for x in [a2, b2] {
            prop_assert!(x.rating.is_finite() && x.volatility.is_finite() && x.volatility > 0.0);
            prop_assert!(x.deviation >= cfg.min_deviation && x.deviation <= cfg.max_deviation);
        }
        // Winning never lowers a rating, losing never raises it.
        match score {
            Score::Win => prop_assert!(a2.rating >= a.rating && b2.rating <= b.rating),
            Score::Loss => prop_assert!(a2.rating <= a.rating && b2.rating >= b.rating),
            Score::Draw => {}
        }
    }
}
