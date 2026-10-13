-- Players each account has played against: the foundation for "people you've played against"
-- and friends (roadmap Phase 4). One row per player per opponent, stored in both directions, so
-- a player's list is one indexed lookup. Empty until games between players exist: nothing writes
-- to it yet (`opponents::record_game_between`). Expand-only: a new table.
CREATE TABLE user_opponents (
    user_id         uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    opponent_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    games_played    integer     NOT NULL DEFAULT 1,
    first_played_at timestamptz NOT NULL,
    last_played_at  timestamptz NOT NULL,
    PRIMARY KEY (user_id, opponent_id),
    CONSTRAINT user_opponents_not_self CHECK (user_id <> opponent_id),
    CONSTRAINT user_opponents_games_positive CHECK (games_played > 0),
    CONSTRAINT user_opponents_times_ordered CHECK (first_played_at <= last_played_at)
);

-- A player's opponents, most recent first.
CREATE INDEX user_opponents_recent ON user_opponents (user_id, last_played_at DESC);
-- The other side of the foreign key (cascades, "who has played against me").
CREATE INDEX user_opponents_opponent ON user_opponents (opponent_id);
