//! Rough timing of each level from a few positions. Ignored by default; run with
//! `cargo test -p ouril-ai --release --test bench -- --ignored --nocapture`.

use ouril_ai::{search, Level};
use ouril_engine::{apply_move, new_game, variant_version, Player};

#[test]
#[ignore]
fn bench_levels() {
    let v = variant_version("cv.standard", 1).unwrap();
    let mut s = new_game(v, Player::South);
    for m in [2, 8, 4, 10] {
        s = apply_move(v, &s, m).unwrap().state;
    }
    for level in [Level::Easy, Level::Medium, Level::Hard] {
        let t = std::time::Instant::now();
        let r = search(v, &s, level.params()).unwrap();
        println!(
            "{level:?}: best {} score {} depth {} nodes {} in {:?}",
            r.best,
            r.score,
            r.depth,
            r.nodes,
            t.elapsed()
        );
    }
}
