-- The seasonal points board (ADR 0034, "later"): points for activity and wins, per calendar
-- month (UTC). Design and scoring: docs/product/features/leaderboards.md.

CREATE TABLE season_points (
    -- `YYYY-MM`.
    season       text        NOT NULL,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    points       integer     NOT NULL DEFAULT 0,
    online_games integer     NOT NULL DEFAULT 0,
    ai_wins      integer     NOT NULL DEFAULT 0,
    updated_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (season, user_id),
    CONSTRAINT season_points_season CHECK (season ~ '^[0-9]{4}-(0[1-9]|1[0-2])$'),
    CONSTRAINT season_points_counts CHECK (points >= 0 AND online_games >= 0 AND ai_wins >= 0)
);
CREATE INDEX season_points_board ON season_points (season, points DESC);

-- Points from games against the computer per player and day (UTC), for the daily cap: those
-- games are played on the device and can't be fully verified.
CREATE TABLE ai_points_daily (
    user_id uuid    NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    day     date    NOT NULL,
    points  integer NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, day)
);
