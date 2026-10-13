-- Live games (ADR 0033): clocks computed from timestamps, aborted games, and seeks for
-- matchmaking. Design: docs/architecture/backend.md (Realtime).

-- A live game's clock. `clock_*_ms` is each side's time left when the current turn started
-- (`turn_started_at`); the side to move loses the time since then. Clocks only run once both
-- players have moved. `turn_deadline` stays "when time decides this game" for live games
-- too (a flag fall, or the first-move window), so the games list can settle overdue games.
ALTER TABLE online_games
    ADD COLUMN initial_ms      bigint,
    ADD COLUMN increment_ms    bigint,
    ADD COLUMN clock_south_ms  bigint,
    ADD COLUMN clock_north_ms  bigint,
    ADD COLUMN turn_started_at timestamptz,
    DROP CONSTRAINT online_games_status,
    ADD CONSTRAINT online_games_status CHECK (
        status IN ('invited', 'active', 'finished', 'declined', 'cancelled', 'expired', 'aborted')
    ),
    ADD CONSTRAINT online_games_live_clock CHECK (
        mode <> 'live' OR (initial_ms > 0 AND increment_ms >= 0)
    );

-- Players looking for a live game. One row per player: a new seek replaces the old one.
-- When two seeks pair up, both rows get the new game's ID until the players pick it up.
CREATE TABLE seeks (
    user_id      uuid             PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    variant_id   text             NOT NULL,
    initial_ms   bigint           NOT NULL,
    increment_ms bigint           NOT NULL,
    rated        boolean          NOT NULL,
    -- The player's rating in the game's live pool when they started seeking.
    rating       double precision NOT NULL,
    created_at   timestamptz      NOT NULL DEFAULT now(),
    heartbeat_at timestamptz      NOT NULL DEFAULT now(),
    game_id      uuid             REFERENCES online_games (id) ON DELETE SET NULL
);
-- Pairing: open seeks for the same game settings, oldest first.
CREATE INDEX seeks_open ON seeks (variant_id, initial_ms, increment_ms, rated, created_at)
    WHERE game_id IS NULL;
