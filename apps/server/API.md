# ouril-server HTTP API (v1)

> Endpoint contract for `apps/server`, the web app and later the native apps. **Status:** Draft (implemented for the MVP; Google/Apple are placeholders). Types: [`core/protocol`](../../core/protocol/src/lib.rs) (TypeScript via `just protocol-ts`). Design: [backend](../../docs/architecture/backend.md), [data and sync](../../docs/architecture/data-and-sync.md), [accounts](../../docs/product/features/accounts.md), [versioning](../../docs/architecture/versioning.md).

## Ground rules

- **Guests never call this API.** A guest makes zero requests (not even `/v1/meta`) unless they opted into telemetry, which doesn't use this API. Clients call it only after the player chooses to sign in.
- Base path `/v1`. JSON in and out (`Content-Type: application/json`), UTF-8, snake_case keys. Within `/v1` changes are additive only: clients ignore unknown fields and handle unknown enum values.
- Signed-in requests send `X-Ouril-Client: web/0.1.0 (1); core/0.1.0` (format `<platform>/<semver> (<build>); core/<semver>`).
- Max request body: 1 MiB. Times are RFC 3339 UTC strings. IDs are UUID strings.
- Implementation status: every endpoint below is implemented. Google and Apple answer `501 not_configured` until their credentials are set; their ID-token verification (JWKS, issuer, audience, expiry, nonce) is written but hasn't been tested against real tokens yet.
- `X-Ouril-Client` is optional (curl and health checks work without it). When it parses and its version is below `min_supported` for that platform, every `/v1` endpoint except `/v1/meta` answers `426 upgrade_required`.

## Authentication

| Item | Web | Native (later) |
|---|---|---|
| Access token | JSON body (`tokens.access_token`), kept **in memory** only. Sent as `Authorization: Bearer <jwt>`. | Same, stored in Keychain / Keystore |
| Lifetime | `ACCESS_TOKEN_TTL_SECS` (default 900 s) | same |
| Refresh token | **httpOnly cookie** `ouril_refresh`: `HttpOnly; Secure` (not `Secure` on `http://localhost`); `SameSite=Lax`; `Path=/v1/auth`; `Max-Age` = `REFRESH_TOKEN_TTL_DAYS`. Never in the JSON body. | `tokens.refresh_token` in the body |
| Rotation | Every refresh rotates the refresh token. Reusing an old one revokes the session. Stored hashed server-side. | same |

- The server decides whether a client is "web" by the presence of an `Origin` header that matches `ALLOWED_ORIGINS`; those clients get the cookie and no `refresh_token` in the body. A request without `Origin` is treated as native. On `/v1/auth/*`, an `Origin` that isn't allowed gets `403 forbidden`.
- Cookie-authenticated endpoints (`/v1/auth/refresh`, `/v1/auth/logout`) require an allowed `Origin` (CSRF defence, together with `SameSite=Lax`). A cookie sent without `Origin` is ignored. A `refresh_token` in the body takes precedence over the cookie.
- CORS: exact origins from `ALLOWED_ORIGINS`, credentials allowed. In local dev the Vite proxy makes requests same-origin.
- Access token (JWT, HS256 with `JWT_SECRET`): claims `sub` (user ID), `sid` (session ID), `iat`, `exp`, `iss = "ouril"`, `aud = "ouril-api"`. Every authenticated request also checks that the session isn't revoked or expired and the account isn't deleted, so logout and deletion take effect at once.
- Sessions slide: each refresh extends the session to `REFRESH_TOKEN_TTL_DAYS` from now. Presenting an already-rotated refresh token revokes the whole session (reuse detection), including when two tabs refresh at the same moment.

## Errors

Every non-2xx response has the body `ApiError`:

```json
{ "code": "not_configured", "message": "Google sign-in is not configured on this server" }
```

| HTTP | `code` | When |
|---|---|---|
| 400 | `bad_request` | Malformed JSON, missing or invalid field |
| 401 | `unauthorized` | Missing/invalid/expired access token, bad refresh token, ID token verification failed |
| 403 | `forbidden` | Origin not allowed for a cookie request |
| 404 | `not_found` | Unknown route (also `/v1/auth/dev` in builds without `dev-auth`) |
| 409 | `handle_taken` | `PATCH /v1/me` with a handle someone else has |
| 413 | `payload_too_large` | Body > 1 MiB or more than 100 mutations |
| 426 | `upgrade_required` | `X-Ouril-Client` version below `min_supported` |
| 429 | `rate_limited` | Too many requests (`Retry-After` header). In-memory, per server machine, per client network (IPv4 address or IPv6 /64): 60/min on sign-in (`/v1/auth/{dev,google,apple}`), 600/min on everything else including refresh and logout. Configurable with `RATE_LIMIT_AUTH_PER_MINUTE` / `RATE_LIMIT_DEFAULT_PER_MINUTE` |
| 501 | `not_configured` | Provider not configured (Google/Apple placeholders) |
| 503 | `provider_unavailable` | Google's or Apple's signing keys couldn't be fetched. Retry later |
| 500 | `internal` | Anything else. Clients treat unknown codes like `internal`. |

`message` is English for developers; UIs show localized text keyed by `code`.

## Endpoints

| Method | Path | Auth | Request | Response |
|---|---|---|---|---|
| `GET` | `/healthz` | none | — | `200 text/plain "ok"` (Fly health check) |
| `GET` | `/v1/meta` | none | — | `200 Meta` |
| `POST` | `/v1/auth/dev` | none | `DevSignInRequest` | `200 SignInResponse` (debug + `dev-auth` only; otherwise 404) |
| `POST` | `/v1/auth/google` | none | `GoogleSignInRequest` | `200 SignInResponse` · `501 not_configured` while `GOOGLE_CLIENT_ID` is empty |
| `POST` | `/v1/auth/apple` | none | `AppleSignInRequest` | `200 SignInResponse` · `501 not_configured` while any `APPLE_*` is empty |
| `POST` | `/v1/auth/refresh` | refresh token (cookie or body) | `RefreshRequest` | `200 TokenPair` (web: new cookie) |
| `POST` | `/v1/auth/logout` | refresh token (cookie or body) | `RefreshRequest` | `204` (clears the cookie). Idempotent: an unknown token is still `204`. |
| `GET` | `/v1/me` | Bearer | — | `200 Me` |
| `PATCH` | `/v1/me` | Bearer | `ProfilePatch` | `200 Me` · `409 handle_taken` |
| `DELETE` | `/v1/me` | Bearer | — | `204`. Deletes personal data, revokes all sessions (and the Apple token when configured), clears the cookie. |
| `POST` | `/v1/sync/push` | Bearer | `SyncPush` | `200 SyncPushResult` |
| `GET` | `/v1/sync/pull?cursor=N&limit=M` | Bearer | query | `200 SyncPull` |

### `GET /v1/meta`

```json
{
  "min_supported": { "ios": "0.1.0", "android": "0.1.0", "web": "0.1.0" },
  "recommended":   { "ios": "0.1.0", "android": "0.1.0", "web": "0.1.0" },
  "api": { "current": "v1", "deprecated": [] },
  "realtime_proto": { "current": 1, "min": 1 }
}
```

Values come from `MIN_SUPPORTED_*` and `RECOMMENDED_*`. Below `min_supported` the client disables online features; offline play is never blocked.

### Sign-in (`/v1/auth/dev`, `/google`, `/apple`)

Requests:

```json
// POST /v1/auth/dev            (all fields optional)
{ "user": "dev", "display_name": "Dev Player" }

// POST /v1/auth/google
{ "id_token": "<Google ID token>", "nonce": "<nonce the client generated>" }

// POST /v1/auth/apple          (names only on the first Apple sign-in)
{ "id_token": "<Apple identity token>", "nonce": "<nonce>", "given_name": "Ana", "family_name": "Lopes",
  "authorization_code": "<code from the same Apple sign-in>" }
```

- Dev: `user` matches `[a-z0-9_-]{1,32}` (default `"dev"`). Finds or creates the user with identity `(provider = "dev", provider_subject = user)`. Compiled only with the `dev-auth` feature in debug builds; release builds fail to compile with it.
- Google/Apple: the server verifies signature (JWKS, cached), issuer, audience (`GOOGLE_CLIENT_ID` / `APPLE_SERVICE_ID`, plus `APPLE_BUNDLE_ID` for native iOS tokens), expiry and nonce, then finds or creates the user by `(provider, provider_subject)`. The token's nonce may be the raw nonce or its SHA-256 hex (Apple's native flow). Email is optional and never used to merge accounts: Google's is kept only when verified, Apple's only when verified or a private relay address. If the keys can't be fetched the answer is `503 provider_unavailable`.
- Apple `authorization_code` (optional): the server exchanges it for Apple's refresh token, using a client secret signed with the `.p8` key, and keeps the token only to revoke it when the account is deleted (ADR 0009). A failed exchange is logged and doesn't fail the sign-in. If the `.p8` key can't be loaded, Apple sign-in stays off (`501`).

Response (`SignInResponse`):

```json
{
  "tokens": { "access_token": "<jwt>", "token_type": "Bearer", "expires_in": 900 },
  "user": {
    "id": "0192f1c2-…", "display_name": "Dev Player", "handle": null,
    "avatar_url": null, "locale": "en", "country": null, "created_at": "2026-10-04T10:00:00Z"
  },
  "is_new_user": true
}
```

Native clients also get `"refresh_token": "<opaque>"` inside `tokens`. Web clients get `Set-Cookie: ouril_refresh=…` instead.

### `POST /v1/auth/refresh` and `POST /v1/auth/logout`

Body `{}` on the web (cookie), `{ "refresh_token": "<opaque>" }` on native. Refresh returns a new `TokenPair` and rotates the refresh token. `401 unauthorized` when the token is missing, expired, revoked or reused.

### `PATCH /v1/me`

```json
{ "display_name": "Ana", "handle": "mindelo_master", "locale": "pt", "country": "cv" }
```

All fields optional; absent fields are unchanged. `display_name`: trimmed, 1 to 50 characters, no control characters, no invisible format characters (Unicode category Cf: bidi overrides, zero-width characters, BOM) and no line/paragraph separators. Validation rules for handles are an open question (see [accounts](../../docs/product/features/accounts.md#open-questions)); until decided: `[a-z0-9_]{3,20}`, case-insensitive unique.

### `POST /v1/sync/push`

```json
{
  "mutations": [
    {
      "id": "0192f1c3-…",
      "type": "game_finished",
      "schema": 1,
      "payload": { "format": 1, "id": "0192f1c2-…", "variant": { "id": "cv.standard", "version": 1 }, "first_player": "south", "moves": [2, 7, 4, 9], "core_version": "0.1.0", "result": { "outcome": "south_wins", "stores": [26, 18], "reason": "threshold" }, "started_at": "…", "ended_at": "…", "mode": "vs_ai", "ai_level": "easy", "human_player": "south" },
      "client_time": "2026-10-03T18:31:00Z"
    }
  ]
}
```

| `type` | `schema` | Payload | Merge rule |
|---|---|---|---|
| `game_finished` | 1 | `GameRecord` (see [core/wasm/API.md](../../core/wasm/API.md#game-records-and-stats)) | Append-only, union by game `id`. The server replays the moves with `ouril-engine` and rejects records that are illegal or whose `result` doesn't match (`illegal_game`). A forfeit (format 2, `reason: "resigned"`) must replay to a game still in progress, with `human_player` set and the other side winning ([ADR 0022](../../docs/decisions/0022-forfeit-and-record-format-2.md)). |
| `profile_updated` | 1 | `{ display_name?, avatar_url?, locale?, country? }` | Field-level last-writer-wins by the mutation's `client_time` (clamped to server time). A direct `PATCH /v1/me` counts as made now. An older change never overwrites a newer field; it's still `applied` |
| `settings_updated` | 1 | `{ sound?, language?, hints? }` | Field-level last-writer-wins by the mutation's `client_time` (clamped to server time; unparsable means now). An older change never overwrites a newer field; it's still `applied` |
| `handle_requested` | 1 | `{ handle }` | Applied only if free, else rejected `handle_taken`. Also ordered by `client_time`: a request older than the current handle's change is ignored (`applied`) |

Each result has a `status`:

| `status` | Meaning | Client action |
|---|---|---|
| `applied` | Applied now, or already applied earlier (repeat) | Remove it from the outbox |
| `rejected` | Permanently invalid; recorded under the mutation ID | Remove it from the outbox (a game stays in local history, marked "Not backed up") |
| `deferred` | Possibly valid, but this server can't process it yet (a client newer than the server, e.g. during a rolling deploy). **Nothing is recorded.** | Keep it in the outbox and push it again on a later sync |

Clients treat an unknown `status` from a newer server like `deferred`.

Response, one entry per mutation in request order:

```json
{ "results": [ { "id": "0192f1c3-…", "status": "applied" },
               { "id": "0192f1c4-…", "status": "rejected", "reason": "handle_taken" } ] }
```

- Repeating a mutation ID returns the outcome recorded the first time (`applied`, or the same rejection) without changing anything (safe retries). Deferrals are never recorded, so a deferred ID can be applied once the server supports it.
- Each mutation is applied in its own transaction, in request order. A server error stops the batch with `500`; mutations applied before it stay applied and come back as `applied` on retry.
- `game_finished`: a `format` newer than 2 or an unknown variant `id@version` is **deferred**. Otherwise `format` must be 1 or 2, `id` a UUID, `started_at <= ended_at` (RFC 3339), `ai_level` one of `easy`/`medium`/`hard`, at most 4096 moves; anything else is `invalid_payload`. A game ID that already belongs to another account is `invalid_payload`.
- Rejection `reason`s (permanent): `invalid_payload`, `illegal_game`, `handle_taken`.
- Deferral `reason`s (retry later): `unknown_type`, `unsupported_schema`, `unsupported_variant`, `unsupported_record_format`.
- At most 100 mutations per request (`413 payload_too_large` otherwise).

### `GET /v1/sync/pull?cursor=N&limit=M`

`cursor` defaults to 0 (full pull). `limit` 1–500, default 200.

```json
{
  "changes": [
    { "entity": "game", "id": "0192f1c2-…", "change_seq": 41, "deleted": false, "data": { "format": 1, "…": "…" } },
    { "entity": "profile", "id": "<user id>", "change_seq": 42, "deleted": false, "data": { "display_name": "Ana", "…": "…" } },
    { "entity": "settings", "id": "<user id>", "change_seq": 43, "deleted": false, "data": { "sound": true } }
  ],
  "cursor": 43,
  "has_more": false
}
```

- `change_seq` comes from one Postgres sequence and only goes up. Changes are ordered by `change_seq`.
- `deleted: true` is a deletion marker (no `data`).
- When `has_more` is true, pull again right away with the new cursor.

## Configuration

See [`.env.example`](../../.env.example). Required: `DATABASE_URL`, `JWT_SECRET` (≥ 32 bytes). Empty OAuth variables mean "not configured"; a partial `APPLE_*` set also counts as not configured. Optional extras with defaults: `DB_MAX_CONNECTIONS` (10) and `LOG_FORMAT` (`json` for JSON log lines; anything else is human-readable).

Release builds refuse to start when `JWT_SECRET` is the `.env.example` value or contains `dev-only` / `change-me`, or when `ALLOWED_ORIGINS` lists an `http://` origin other than loopback (`localhost`, `127.0.0.1`, `[::1]`). `FLY_APP_NAME` (set by Fly) makes the rate limiter trust the `Fly-Client-IP` header; elsewhere only the TCP peer address counts.

Every `/v1` request has a 15 s time limit; past it the server answers 500 `internal`. Release builds also send `Strict-Transport-Security: max-age=31536000` (without `includeSubDomains` until every subdomain is confirmed to serve HTTPS).

Debug builds apply pending migrations on startup. Release builds rely on `ouril-server migrate` (the Fly release command).

## Open questions

- Handle validation rules (length, characters, reserved words, change frequency).
- Rate limits per endpoint (sign-in, sync) and where to keep counters before Valkey exists.
- Should `/v1/meta` be cached at the edge, and should `426 upgrade_required` be enforced on every request or only on sign-in and sync?
- Pull cursors and concurrent writes: `change_seq` is taken when a row is written, not when the transaction commits, so a pull running while two pushes for the same user commit out of order could skip a row. Acceptable for one player's devices in the MVP; revisit (for example, only return rows older than the oldest open transaction) before multiplayer.
- Timeouts and database pool exhaustion currently surface as 500 `internal`. A retryable 503 (new `ErrorCode`, e.g. `unavailable`, with `Retry-After`) would be a protocol contract change.
- Refresh-token reuse detection is strict: two browser tabs refreshing at the same moment revoke the session. Add a short grace window, or have the web app serialize refreshes across tabs?
