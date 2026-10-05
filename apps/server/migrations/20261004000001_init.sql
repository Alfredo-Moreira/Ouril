-- Ouril MVP schema: accounts, sessions, settings, game records and offline-first sync.
-- Design: docs/architecture/backend.md (data model), docs/architecture/data-and-sync.md.
-- Changes after release follow expand/contract (ADR 0013): never edit this file once applied
-- anywhere shared; add a new migration instead.

-- One sequence for every synced row: pull returns rows with change_seq > cursor, in order.
CREATE SEQUENCE change_seq AS bigint START 1;

-- Users are our own records; providers are linked identities (ADR 0009).
CREATE TABLE users (
    id            uuid        PRIMARY KEY,
    display_name  text        NOT NULL,
    -- Lowercase, [a-z0-9_]{3,20} until the handle rules are decided (API.md).
    handle        text,
    avatar_url    text,
    locale        text,
    country       text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    -- Set on account deletion: the row stays as an anonymized "Deleted player".
    deleted_at    timestamptz,
    change_seq    bigint      NOT NULL DEFAULT nextval('change_seq')
);
CREATE UNIQUE INDEX users_handle_lower_key ON users (lower(handle)) WHERE handle IS NOT NULL;

CREATE TABLE auth_identities (
    id                      uuid        PRIMARY KEY,
    user_id                 uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- "google" | "apple" | "dev" (debug builds only)
    provider                text        NOT NULL,
    provider_subject        text        NOT NULL,
    -- Optional everywhere and never used to merge accounts.
    email                   text,
    email_is_private_relay  boolean     NOT NULL DEFAULT false,
    linked_at               timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT auth_identities_provider_subject_key UNIQUE (provider, provider_subject)
);
CREATE INDEX auth_identities_user_idx ON auth_identities (user_id);

-- A signed-in device. Refresh tokens rotate within a session (session_refresh_tokens).
CREATE TABLE sessions (
    id            uuid        PRIMARY KEY,
    user_id       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- From X-Ouril-Client, informational only.
    device_name   text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    last_used_at  timestamptz NOT NULL DEFAULT now(),
    expires_at    timestamptz NOT NULL,
    revoked_at    timestamptz
);
CREATE INDEX sessions_user_idx ON sessions (user_id);

-- Every refresh token ever issued for a session, stored as SHA-256 (never the token itself).
-- The current one has rotated_at NULL. Presenting a rotated token again is reuse: the whole
-- session is revoked.
CREATE TABLE session_refresh_tokens (
    token_hash  bytea       PRIMARY KEY,
    session_id  uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    issued_at   timestamptz NOT NULL DEFAULT now(),
    rotated_at  timestamptz
);
CREATE INDEX session_refresh_tokens_session_idx ON session_refresh_tokens (session_id);

-- Field-level last-writer-wins settings (sound, language, hints).
CREATE TABLE user_settings (
    user_id     uuid        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    settings    jsonb       NOT NULL DEFAULT '{}'::jsonb,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    change_seq  bigint      NOT NULL DEFAULT nextval('change_seq')
);

-- Finished games. MVP: games vs AI (mode = 'vs_ai'), ID generated on the device (UUIDv7).
-- Append-only, union by id. `record` is the validated GameRecord returned by pull.
CREATE TABLE games (
    id               uuid        PRIMARY KEY,
    user_id          uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    variant_id       text        NOT NULL,
    variant_version  integer     NOT NULL,
    first_player     text        NOT NULL,
    moves            integer[]   NOT NULL,
    mode             text        NOT NULL DEFAULT 'vs_ai',
    ai_level         text,
    human_player     text,
    result_outcome   text        NOT NULL,
    result_reason    text        NOT NULL,
    store_south      integer     NOT NULL,
    store_north      integer     NOT NULL,
    core_version     text        NOT NULL,
    record_format    integer     NOT NULL,
    record           jsonb       NOT NULL,
    started_at       timestamptz NOT NULL,
    ended_at         timestamptz NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    change_seq       bigint      NOT NULL DEFAULT nextval('change_seq'),
    deleted_at       timestamptz
);
CREATE INDEX games_user_change_seq_idx ON games (user_id, change_seq);

-- Applied (or rejected) mutation IDs, so repeated pushes are ignored (safe retries).
CREATE TABLE sync_mutation (
    user_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    id          uuid        NOT NULL,
    type        text        NOT NULL,
    -- 'applied' | 'rejected'
    outcome     text        NOT NULL,
    reason      text,
    applied_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, id)
);
