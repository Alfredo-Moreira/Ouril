-- Games against other players, played through the server turn by turn (ADR 0031). The server
-- owns the moves and validates each with the engine; a finished game is copied into both
-- players' `games` history (format-3 records) and `user_opponents`. Expand-only: a new table.
CREATE TABLE online_games (
    id               uuid        PRIMARY KEY,
    variant_id       text        NOT NULL,
    variant_version  integer     NOT NULL,
    first_player     text        NOT NULL,
    -- The inviter plays South.
    south_user       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Set when the game is accepted.
    north_user       uuid        REFERENCES users (id) ON DELETE CASCADE,
    -- Who was invited (by handle or from recent opponents); NULL for an invite link.
    invited_user     uuid        REFERENCES users (id) ON DELETE CASCADE,
    -- The invite link's code, while the link is open.
    invite_code      text        UNIQUE,
    status           text        NOT NULL DEFAULT 'invited',
    moves            integer[]   NOT NULL DEFAULT '{}',
    -- Active games: when the player to move loses on time.
    turn_deadline    timestamptz,
    result_outcome   text,
    result_reason    text,
    store_south      integer,
    store_north      integer,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    started_at       timestamptz,
    ended_at         timestamptz,
    CONSTRAINT online_games_first_player CHECK (first_player IN ('south', 'north')),
    CONSTRAINT online_games_status
        CHECK (status IN ('invited', 'active', 'finished', 'declined', 'cancelled', 'expired')),
    CONSTRAINT online_games_two_players CHECK (north_user IS NULL OR north_user <> south_user),
    CONSTRAINT online_games_not_self CHECK (invited_user IS NULL OR invited_user <> south_user),
    CONSTRAINT online_games_active_has_north CHECK (status <> 'active' OR north_user IS NOT NULL),
    CONSTRAINT online_games_finished_has_result
        CHECK (status <> 'finished' OR (result_outcome IS NOT NULL AND result_reason IS NOT NULL))
);

-- A player's games, newest first, from either seat or as the invited player.
CREATE INDEX online_games_south ON online_games (south_user, updated_at DESC);
CREATE INDEX online_games_north ON online_games (north_user, updated_at DESC);
CREATE INDEX online_games_invited ON online_games (invited_user, updated_at DESC);
