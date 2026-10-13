-- Ratings and leaderboards (ADR 0034): Glicko-2 per pool, updated when a rated game against
-- another player finishes. Design: docs/product/features/leaderboards.md.

-- Games against other players: their mode (a pool is `<variant_id>:<mode>`), whether they
-- were created as rated, and whether they counted (for the per-pair daily limit).
-- Existing games stay unrated.
ALTER TABLE online_games
    ADD COLUMN mode           text    NOT NULL DEFAULT 'async',
    ADD COLUMN rated          boolean NOT NULL DEFAULT false,
    ADD COLUMN rating_applied boolean NOT NULL DEFAULT false,
    ADD CONSTRAINT online_games_mode CHECK (mode IN ('async', 'live')),
    ADD CONSTRAINT online_games_applied_if_rated CHECK (NOT rating_applied OR rated);

-- One row per player and pool (`overall` or `<variant_id>:<mode>`). Ratings are on the
-- familiar scale (1500 ± 350); `deviation` is as of `last_rated_at` and grows with time
-- when read (ouril-rating `deviation_after`).
CREATE TABLE ratings (
    user_id       uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    pool          text             NOT NULL,
    rating        double precision NOT NULL,
    deviation     double precision NOT NULL,
    volatility    double precision NOT NULL,
    games         integer          NOT NULL DEFAULT 0,
    wins          integer          NOT NULL DEFAULT 0,
    draws         integer          NOT NULL DEFAULT 0,
    losses        integer          NOT NULL DEFAULT 0,
    peak          double precision NOT NULL,
    last_rated_at timestamptz,
    PRIMARY KEY (user_id, pool),
    CONSTRAINT ratings_counts CHECK (
        games >= 0 AND wins >= 0 AND draws >= 0 AND losses >= 0 AND wins + draws + losses = games
    ),
    CONSTRAINT ratings_values CHECK (deviation > 0 AND volatility > 0)
);
-- Leaderboards: a pool's players by rating.
CREATE INDEX ratings_pool_rating ON ratings (pool, rating DESC);

-- Every change, for history and for showing a finished game's result.
CREATE TABLE rating_changes (
    game_id          uuid             NOT NULL REFERENCES online_games (id) ON DELETE CASCADE,
    user_id          uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    pool             text             NOT NULL,
    rating_before    double precision NOT NULL,
    rating_after     double precision NOT NULL,
    deviation_before double precision NOT NULL,
    deviation_after  double precision NOT NULL,
    at               timestamptz      NOT NULL,
    PRIMARY KEY (game_id, user_id, pool)
);
CREATE INDEX rating_changes_user ON rating_changes (user_id, at DESC);
