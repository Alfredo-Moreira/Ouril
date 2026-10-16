-- Rank movement and best ranks (ADR 0034, later work), written by scheduled jobs on the
-- homelab (ADR 0032): POST /v1/internal/jobs/leaderboard-snapshot.

-- Each board's ranks, once per day (UTC). A job run during the day replaces that day's rows,
-- so the latest run wins. Boards compare today's live ranks with the latest earlier day.
-- `scope` is `world` or a country code.
CREATE TABLE leaderboard_snapshots (
    taken_on date             NOT NULL,
    pool     text             NOT NULL,
    scope    text             NOT NULL,
    user_id  uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    rank     integer          NOT NULL,
    rating   double precision NOT NULL,
    PRIMARY KEY (taken_on, pool, scope, user_id),
    CONSTRAINT leaderboard_snapshots_rank CHECK (rank > 0)
);
CREATE INDEX leaderboard_snapshots_user ON leaderboard_snapshots (user_id, taken_on DESC);

-- Each player's best rank on each board, kept when old snapshots are pruned (profiles, badges).
CREATE TABLE rank_bests (
    user_id     uuid    NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    pool        text    NOT NULL,
    scope       text    NOT NULL,
    best_rank   integer NOT NULL,
    achieved_on date    NOT NULL,
    PRIMARY KEY (user_id, pool, scope),
    CONSTRAINT rank_bests_rank CHECK (best_rank > 0)
);

-- When each scheduled job last ran, and how it went (for checking that the homelab is alive).
CREATE TABLE job_runs (
    name             text        PRIMARY KEY,
    last_finished_at timestamptz NOT NULL,
    last_rows        bigint      NOT NULL,
    last_error       text
);
