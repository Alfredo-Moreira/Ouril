# Human review: Phase 1 build

> Open questions and items that need a human decision or check after the multi-agent build of the Rust core and WASM, the Axum server and the React/Vite web app (iOS and Android are out of scope). **Status:** Draft

Collected on 2026-10-04 from the reports of the build, test, QA, review, fix, verify and docs agents, plus a scan of the repo. Each item names its area, the agents that raised it and the files involved. Duplicates are merged and every source is listed. Items that later agents fixed and that `verify` confirmed are left out. Each section is ordered by importance.

Agent labels: `scaffold`, `build:core`, `build:server`, `build:web`, `tests:core`, `tests:server`, `tests:web`, `qa:integrate` (qa), `review:security` (sec), `review:architecture` (arch), `apply:infra`, `apply:web`, `apply:server`, `verify`, `docs-keeper` (docs), and `scan` for this curator's own repo scan.

> **Update (ADR 0025):** the MVP launches as the web app alone, a static site with no server. Items about the server, sign-in, sync and their hosting don't block that launch; they block the accounts release that follows.

## Blocking before launch

1. ✅ **Fixed 2026-10-04: "not supported yet" sync results are now `deferred`, not permanent rejections.** New `MutationStatus::Deferred` with `deferral_reasons` (`unknown_type`, `unsupported_schema`, `unsupported_variant`, `unsupported_record_format`) in `core/protocol`. The server answers these before recording anything, so the mutation ID stays usable after an upgrade (`apps/server/src/sync.rs`). The web keeps deferred mutations (and unknown statuses) in the outbox, skips them for the rest of that sync so they can't block the rest, and pushes them again later (`apps/web/src/sync/syncEngine.ts`). Docs: API.md, data-and-sync.md, versioning.md. Tests: server unit and integration (`records_from_newer_clients_are_deferred_then_applied_once_supported`), web (`keeps deferred mutations queued without blocking the ones behind them`). Follow-up fixed the same day: settings now merge by device time (`client_time`, clamped to server time) with per-field times in `user_settings.field_times` (migration `20261004000002`), so a late-applied older change can't overwrite a newer one (tests: `settings_merge_by_device_time_not_arrival`, `settings_from_a_future_clock_are_clamped_to_server_time`). Profile fields (display name, avatar, locale, country, handle) got the same fix the same day: per-field times in `users.profile_field_times` (migration `20261004000003`), synced changes ordered by clamped `client_time`, direct `PATCH /v1/me` edits by server time (tests: `profile_merges_by_device_time_not_arrival`, `a_direct_edit_beats_older_queued_profile_and_handle_changes`). Area: server, web, core. Source: arch (high).
2. ✅ **Fixed 2026-10-04 (code); decision pending: production topology for web and API.** The web client takes `VITE_API_BASE_URL` and sends `credentials: 'include'` to a separate API origin (same-origin stays the default for local development). [ADR 0020](../decisions/0020-web-and-api-on-one-site.md) (**Proposed**) recommends the web app on Vercel at the main domain and the API on Fly at `api.<domain>`, the same site, so the `SameSite=Lax` refresh cookie is sent and Fly still sees players' real IPs (a Vercel `/v1` proxy would hide them from the rate limiter). `fly.toml` now sets `ALLOWED_ORIGINS`. Still needed: accept or change ADR 0020, choose the domain, and replace the placeholders (see Placeholders). Area: infra, web, server. Files: `apps/web/src/api/client.ts`, `apps/server/fly.toml`, `apps/web/vercel.json`.
3. ✅ **Fixed 2026-10-04: settings and the in-progress game are now scoped by owner on the web.** They're stored as `settings:<owner>` and `current_game:<owner>`, with a Dexie v2 upgrade that moves the old global keys to the active owner. Signing in claims the guest's copies unless the device already holds the account's own; a different account starts from its own (default) data, and the previous user's stays aside. Tests: `apps/web/src/storage/repo.test.ts`. Decided 2026-10-04: a guest's queued settings changes are claimed and uploaded on sign-in, like games (recorded in data-and-sync.md). Area: web. Source: arch. Files: `apps/web/src/storage/repo.ts`, `db.ts`.
4. **Google and Apple sign-in still need real credentials (code side fixed 2026-10-04).** Done:
   - `APPLE_BUNDLE_ID` accepted as an audience (native iOS tokens)
   - Apple email stored only when verified or a private relay address
   - `503 provider_unavailable` when the signing keys can't be fetched
   - nonce: the raw nonce or its SHA-256 hex is accepted (Google and Apple's native flow)
   - **Apple token revocation on account deletion** (ADR 0009): the server exchanges the `authorization_code` at sign-in, keeps Apple's refresh token (`auth_identities.provider_refresh_token`, migration `20261004000004`) and revokes it on `DELETE /v1/me`; Apple sign-in stays off if the `.p8` key can't be loaded (`apps/server/src/auth/apple.rs`, tested against a local mock)

   Still needed before enabling: real client IDs and keys (Google, Apple), testing with real tokens, the web sign-in flows (Google Identity Services and Sign in with Apple JS, then their hosts in the CSP), and a decision on a server-issued single-use nonce. Failed revocations are only logged; decide whether to retry them. Area: server, web. Files: `apps/server/src/auth/oidc.rs`, `apps/server/src/auth/apple.rs`, `apps/web/src/routes/SignIn.tsx`.
5. ✅ **Fixed 2026-10-04: rate limiter hardened.** Keys are IPv4 addresses or IPv6 /64 networks; only sign-in is in the strict class (refresh and logout need a 256-bit token, so players behind carrier-grade NAT no longer lock each other out); limits are configurable (`RATE_LIMIT_AUTH_PER_MINUTE`=60, `RATE_LIMIT_DEFAULT_PER_MINUTE`=600); the map is capped at 50,000 windows and pruned at most every 10 s, and new clients pass untracked when it's full. Still a decision: the final limits and when to move counters to Valkey (Decisions 9). Area: server. File: `apps/server/src/ratelimit.rs`.
6. ✅ **Fixed 2026-10-04: missing MVP web features added.**
   - profile editing in Settings: display name (applied right away) and handle (queued as `handle_requested`, shown as pending until the server confirms; "taken" is reported)
   - the `/v1/meta` version check after sign-in (never for guests) and handling of `426 upgrade_required` from sync, with an "update required" or "update available" banner; offline play is never blocked
   - replay of past games from Stats (`/replay/:id`), stepping through positions rebuilt by the engine

   Area: web. Files: `apps/web/src/routes/Settings.tsx`, `apps/web/src/routes/Replay.tsx`, `apps/web/src/state/AppProvider.tsx`.
7. ✅ **Fixed 2026-10-04: CI added** (`.github/workflows/ci.yml`): docs and test-vector checks; Rust fmt, clippy, all tests with Postgres, the server tests without `dev-auth`, and `cargo audit`; the WASM build and web lint, typecheck, tests and build, plus `pnpm audit`; and the release image built and checked for the dev sign-in route. `just test-server` now also runs the plain build. Not yet run on GitHub: check the first run. Area: infra.
8. ✅ **Fixed 2026-10-04: web hosting headers.** `apps/web/vercel.json` serves SPA routing, a strict CSP (`script-src 'self' 'wasm-unsafe-eval'`, `worker-src 'self'`, `style-src 'self'`, `frame-ancestors 'none'`, …), HSTS, nosniff, `Referrer-Policy`, `X-Frame-Options`, COOP and `Permissions-Policy`, plus immutable caching for hashed assets. The build has no inline scripts or styles, so the CSP needs no exceptions. Still needed: the real API host in `connect-src` (placeholder), and Sentry's host only once a DSN is set. Area: web, infra.
9. ✅ **Fixed 2026-10-04: wasm-opt enabled.** The toolbox installs a pinned, checksum-verified binaryen (version 133, x86_64 and aarch64), and `just bindings-wasm` and CI run `wasm-opt -Oz` after wasm-pack: the engine went from 502 KB to 372 KB (155 KB gzipped). The web tests run against the optimised build. Area: core, infra.
10. ✅ **Fixed 2026-10-04: HSTS no longer includes subdomains.** Release builds send `Strict-Transport-Security: max-age=31536000` without `includeSubDomains`, so nothing is pinned for subdomains by accident. Launch checklist: once every subdomain serves HTTPS, consider adding `includeSubDomains` (and `preload`). Area: infra. File: `apps/server/src/routes/mod.rs`.

## Decisions needed

1. **Refresh-token reuse grace window (about 10 s) on the server.** The web now serializes refreshes across tabs with Web Locks. Browsers without Web Locks, and separate browser profiles that share a cookie jar, can still trigger a false reuse revocation. Once this is decided, close the API.md open question and update the concurrent-refresh test. Area: server. Source: build:server, tests:server, sec, apply:web, docs. Files: `apps/server/src/auth/session.rs`, `apps/server/API.md`.
2. **What happens locally to a rejected `game_finished`?** The web keeps the game in history marked "Not backed up". data-and-sync.md (Accepted) says rejected mutations are rolled back locally. Also decide whether rejected games count in local stats. Area: web, docs. Source: build:web, docs. Files: `apps/web/src/sync/`, `docs/architecture/data-and-sync.md`.
3. **One source of TypeScript types.** Today three overlap: the hand-written `apps/web/src/api/types.ts`, the ts-rs output from `just protocol-ts`, and the hand-written `TS_TYPES` in `core/wasm/src/lib.rs`. They already differ on `MutationResult.status`. Either import the generated types (so the web build depends on `protocol-ts`), or keep the mirror and add a type-level drift check to `just test-web`. Area: web, core. Source: build:web, qa, arch, docs.
4. **One owner for finished-game records.** The server (`validate_game_record`) and the web (`apps/web/src/game/session.ts`, which hard-codes `format: 1`) each build or check records, and iOS and Android would need two more copies. Option: `verify_record` and `finish_record` in ouril-sync, exposed through WASM. This changes the `core/wasm/API.md` contract. Area: core. Source: arch. File: `core/sync/src/lib.rs`.
5. **Pull cursor race.** Concurrent per-user writes can commit out of order, so a pull can skip a row. Fix now with a per-user `pg_advisory_xact_lock` before `nextval('change_seq')`, or accept it for the MVP as API.md does. Area: server. Source: build:server, arch. File: `apps/server/src/sync.rs`.
6. **Retryable error code.** Timeouts, database pool exhaustion and JWKS failures return 500 `internal`. Add 503 `unavailable` with `Retry-After` to ouril-protocol and handle it in the web client? Area: server, core, web. Source: arch, sec, apply:server, verify. Files: `core/protocol`, `apps/server/API.md`.
7. **Data retention and database integrity.**
   - `sessions`, `session_refresh_tokens` and `sync_mutation` are never pruned (about 35k token rows per device-year).
   - Account deletion keeps revoked `sessions` rows, including `device_name`.
   - Enum-like text columns have no CHECK constraints. These can still go into the init migration, because it hasn't been applied anywhere shared.

   Decide the retention policy and record it in data-and-sync.md. Area: server. Source: sec, arch. Files: `apps/server/src/users.rs`, `apps/server/migrations/20261004000001_init.sql`.
8. **`DELETE /v1/me` with only an access token.** Require a fresh sign-in or a confirmation token, or rely on the UI confirmation. Document the choice in accounts.md. Area: server. Source: sec. File: `apps/server/src/routes/me.rs`.
9. **Display names and handles.** Should display names be NFC-normalized? Should U+200D be allowed again? It is rejected now, which blocks family emoji. The handle rule `[a-z0-9_]{3,20}` is provisional. Area: server. Source: scaffold, build:server, apply:server, verify, sec. Files: `apps/server/src/validate.rs`, `docs/product/features/accounts.md`.
10. **Guest zero-network guarantee.** Confirm it means "no API, telemetry or third-party requests", so that same-origin asset and service-worker requests are allowed. Record this in ADR 0014 and data-and-sync.md. A guest feature that needs the network, such as `/v1/meta`, would need an ADR. Area: web, docs. Source: sec, tests:web. Files: `apps/web/src/main.tsx`, `apps/web/src/routes/GuestNetwork.test.tsx`.
11. **Sign-out and account deletion on the device.**
    - After sign-out, guest games go into the last user's outbox. Should sign-out switch to a fresh guest profile instead?
    - Confirm that deleting the account wipes local games and starts a new guest profile.

    Area: web. Source: build:web.
12. **Who moves first.** For a new vs-AI game, and for "Play again": random (current), the loser starts as in a series (rules.md, low confidence), or reuse the last choice? Area: core, web. Source: scaffold, build:web.
13. **GameRecord format 1, before it is frozen.**
    - Confirm the optional `mode`, `ai_level` and `human_player` fields.
    - Decide whether to add the AI seed so bug reports can be replayed (a format change).

    Area: core, docs. Source: scaffold, build:core, build:web, docs. Files: `core/sync`, `docs/architecture/versioning.md`, `core/wasm/API.md`.
14. **Engine conventions with no spec.** Should these go into the `cv.standard` spec?
    - The end-check order: threshold, then can't move, then endless cycle.
    - `no_feed` versus `no_moves`.
    - A state with no history counts its own position as the first since the last capture.
    - Feeding overrides the single-seed rule only when single-seed pits are the only way to feed. Is that reading right?

    Area: core, docs. Source: build:core, tests:core, docs. Files: `docs/architecture/engine.md`, `docs/game/variants/cape-verde/standard.md`.
15. **Test-vector format (a public contract).**
    - Choose the `first_player` shape: `"initial"` versus `{"initial": true, "first_player": …}`.
    - Add `setup.history` so repeated-position vectors are possible.

    Area: core. Source: build:core, tests:core. Files: `core/test-vectors/schema/`, `docs/architecture/test-vectors.md`.
16. **AI budgets.**
    - Confirm node budgets instead of the ~500 ms time budget in ai.md.
    - Should there be a wall-clock cap? Hard takes about 345 ms in WASM on a laptop, which may be over 1 s on low-end phones.
    - Should Hard allow undo? Undo is currently allowed at every level.

    Area: core, web. Source: scaffold, build:core, build:web, docs. File: `docs/architecture/ai.md`.
17. **`/healthz` is liveness only.** Add a readiness check that touches the database for Fly? Area: infra. Source: build:server. File: `apps/server/fly.toml`.
18. **Avatars.** `avatar_url` accepts any https URL, which forces `img-src https:` in the CSP. Proxy or restrict avatars? Area: server, web. Source: sec. File: `apps/server/src/validate.rs`.
19. **426 `upgrade_required` scope.** It currently applies to every `/v1` route except `/v1/meta`, including unknown routes, rather than only sign-in and sync. `/v1/meta` is still rate limited. Intended? Area: server. Source: build:server, tests:server. Files: `apps/server/src/client.rs`, `apps/server/src/ratelimit.rs`.
20. **Sessions have only a sliding expiry.** Add an absolute maximum session lifetime? Area: server. Source: build:server.
21. **Wrong-owner game ID.** A `game_finished` whose game ID belongs to another account is rejected as `invalid_payload`. Keep that, or add a dedicated reason? Area: server. Source: build:server. File: `apps/server/src/sync.rs`.

## Placeholders to fill (OAuth, secrets, contacts)

1. **OAuth credentials.** These are empty in `.env.example`: `GOOGLE_CLIENT_ID`, `APPLE_SERVICE_ID`, `APPLE_TEAM_ID`, `APPLE_KEY_ID`, `APPLE_PRIVATE_KEY_PATH` (.p8). `APPLE_BUNDLE_ID` still needs adding. Until all are set, `/v1/auth/google` and `/v1/auth/apple` answer 501 `not_configured` and the web buttons stay disabled. Area: server, web, infra. Source: scaffold, build:server, build:web, scan. Files: `.env.example`, `apps/server/src/routes/auth.rs`, `apps/web/src/routes/SignIn.tsx`.
2. **`JWT_SECRET`.** `.env.example` and the local `.env` hold the dev-only value. Staging and production need a real secret via `fly secrets set` (for example `openssl rand -base64 48`). Release builds, including `just docker-build`, refuse to start with the placeholder. Area: infra. Source: scaffold, sec, apply:server, verify.
3. **`fly.toml`.**
   - `ALLOWED_ORIGINS` is a commented TODO (`https://staging.ouril.example`). It must be `https://`, or release builds won't start.
   - `LOG_FORMAT=json` is missing.
   - A production copy is needed (`ouril-server-prod`, `min_machines_running = 2`).
   - Region `fra` is a placeholder, and nothing is deployed yet.

   Area: infra. Source: scaffold, build:server, sec, apply:server, scan. File: `apps/server/fly.toml`.
4. **Telemetry.** Usage statistics are wired to self-hosted Matomo ([ADR 0026](../decisions/0026-usage-statistics-with-matomo.md), **Proposed**), with the offline queue. Still TODO: `SENTRY_DSN` is empty and Sentry lazy-loading after consent isn't written, so crash reports send nothing; accept ADR 0026; confirm the Matomo instance's IP anonymization and geolocation settings. Area: web, server. Source: build:web, scan. File: `apps/web/src/telemetry/index.ts`.
5. **`.env.example` is missing documented server keys:** `DB_MAX_CONNECTIONS` (default 10) and `LOG_FORMAT` (json|text). Area: infra. Source: build:server, scan.
6. **Project contacts.** `[INSERT CONTACT METHOD]` in `CODE_OF_CONDUCT.md`. In `SECURITY.md`, enable GitHub private vulnerability reporting and replace `security@TODO.example`. Area: docs. Source: scan.
7. **App versions.** `MIN_SUPPORTED_*` and `RECOMMENDED_*` are all `0.1.0`. Area: server. Source: scaffold. File: `.env.example`.
8. **AI tuning.** Node budgets (Easy 2k, Medium 50k, Hard 1M) and margins (Easy 1.5 seeds, Medium 0.3 seeds) need tuning by playtesting. Area: core. Source: scaffold, build:core.
9. **Rate limits.** Defaults 60/min on sign-in and 600/min elsewhere (configurable since 2026-10-04), with per-machine in-memory counters. Decide the production values and when to move counters to Valkey. Area: server. Source: build:server. File: `apps/server/src/ratelimit.rs`.
10. **PWA artwork.** The icons in `apps/web/public/icons` are programmatic placeholders. The manifest colours and description are placeholders too. Area: web. Source: build:web.
11. **Rules and tutorial text.** `rules.*` and `tutorial.*` in `shared/i18n/en.json` paraphrase rules.md and need proofreading against the spec. The tutorial adds one position that is not a rules.md example. Area: web, docs. Source: build:web.
12. **`just i18n` is a stub.** Its message also says the web "reads shared/i18n/en.json directly", but the web now runs `apps/web/scripts/gen-i18n.mjs`. Area: infra. Source: scaffold, build:web, scan. File: `justfile`.

- **Domain (ADR 0020):** `https://api.ouril.example` in the CSP `connect-src` (`apps/web/vercel.json`), `ALLOWED_ORIGINS = "https://staging.ouril.example"` (`apps/server/fly.toml`), and `VITE_API_BASE_URL` for production web builds. Area: infra, web, server.
- **Apple:** `APPLE_BUNDLE_ID` (once the iOS app exists) and `APPLE_REDIRECT_URI` (web flow) in `.env.example` / `fly secrets`. Area: server.
## Deviations from docs/ADRs

1. **[ADR 0016](../decisions/0016-i18n-source-format.md) (i18n).** The ADR says strings come from `tools/i18n-gen`, a Rust CLI. Instead, the web uses the interim `apps/web/scripts/gen-i18n.mjs`, run by every pnpm script, and `tools/i18n-gen` doesn't exist. Amend the ADR, or build the generator before iOS. Area: web, infra. Source: scaffold, build:web, docs, scan.
2. **[ADR 0007](../decisions/0007-rust-core-with-generated-bindings.md) and [ADR 0010](../decisions/0010-postgres-and-local-sqlite.md) (generated types).** The ADRs say data shapes are defined once in `core/protocol` and generated for each platform. The web instead hand-writes `apps/web/src/api/types.ts`. This is Decision 3 above. Area: web. Source: build:web, qa, arch, docs.
3. **[ADR 0010](../decisions/0010-postgres-and-local-sqlite.md) (sqlx).** The ADR says queries are checked at compile time. The server uses runtime `sqlx::query()` everywhere: 0 `query!` macros, 25 runtime calls. Amend the ADR, or switch to the macros with an offline `.sqlx` cache. Area: server. Source: docs.
4. **[ADR 0015](../decisions/0015-generated-bindings-not-committed.md) (bindings freshness).** The ADR says each build checks that the bindings exist and are newer than the core. The web build has no freshness check: it fails only at `pnpm install` when `core/wasm/pkg` is missing. Area: web, infra. Source: docs.
5. **[ADR 0017](../decisions/0017-local-first-development.md) (local development).**
   - `db-reset` has no seed data.
   - `just test` runs core, server and web tests.
   - The dev sign-in user is found or created by name.
   - The web image is `docker/web.dev.Dockerfile`.
   - The Android NDK is optional in the toolbox (off by default, x86_64 only).
   - Published ports are loopback-only, with an opt-in `docker/compose.lan.yaml` that exposes dev-auth to the LAN.

   Amend the ADR text? Area: infra, docs. Source: scaffold, apply:infra, docs.
6. **ai.md (Accepted).** It describes a time-bounded AI, but the code uses fixed node budgets for cross-platform determinism. This is Decision 16 above. Area: core, docs. Source: scaffold, build:core, docs.
7. **data-and-sync.md (Accepted).** Two mismatches: rejected mutations are not rolled back locally (Decision 2), and a different account does not start with an empty local cache (Blocking 3). Area: web, docs. Source: build:web, arch, docs.
8. **versioning.md (Accepted).** Format-1 game records gained the optional `mode`, `ai_level` and `human_player` fields. A note was added, and they need confirming before the format is frozen (Decision 13). Area: core, docs. Source: scaffold, build:core, docs.
9. **monorepo.md (Accepted).** The dependency diagram was changed to match the code: protocol has no dependencies, sync depends on engine and protocol, and the server depends on sync. `GameRecord` lives in ouril-sync, and protocol payloads are opaque `serde_json::Value`. Confirm this boundary is intended. Area: core, docs. Source: scaffold, docs.
10. **ADR 0009 and accounts.md.** Account deletion doesn't revoke the Apple token yet (Blocking 4). Area: server. Source: build:server, docs.
11. **Error code name.** The build task asked for `provider_not_configured`, but the code and API.md use `not_configured`. Confirm `not_configured`, or rename it everywhere. Area: server. Source: build:server. File: `apps/server/src/error.rs`.

## Known failing or unverified

1. ✅ **Fixed 2026-10-04: `is_local_http` now handles `http://[::1]` and `http://[::1]:PORT`.** Bracketed IPv6 hosts are parsed up to `]`, with tests for lookalikes (`[::2]`, `[::1]evil.example`, unterminated brackets). Area: server. Source: tests:server, qa, docs, scan. File: `apps/server/src/auth/cookie.rs`.
2. **Never checked in a real browser:**
   - the Worker AI on Hard
   - service worker and offline play after first load
   - the httpOnly cookie through the Vite proxy
   - the cross-tab Web Locks refresh (sign in, open two tabs, let the token expire)
   - phone and desktop layout, and sounds

   The Playwright and `flow.mjs` scripts were one-offs in the scratchpad. Should they become a checked-in `apps/web/e2e` suite in CI? Area: web. Source: build:web, tests:web, qa, apply:web.
3. **Endless cycle by repeated position has no JSON test vector.** It is covered only by `core/engine/tests/invariants.rs`, so the web, iOS and Android runners won't cover it until `setup.history` exists. Area: core. Source: build:core, tests:core, qa.
4. **Only `cv.standard` has test vectors.** The other grand-slam modes (`allowed_no_capture`, `forbidden`, `captures_all`), clockwise sowing, `remaining_not_scored` and the other endings were implemented from parameter names alone, with light unit tests. `grand_slam = loses` and `capture_mandatory = false` are rejected as unsupported. Area: core. Source: build:core, tests:core.
5. **ouril-wasm has no Rust-side tests.** It is covered only by a Node smoke test and the web Vitest suite. Area: core. Source: build:core, tests:core.
6. **Untested server paths:**
   - the database failing partway through a push batch
   - an engine panic turning into a 500
   - a real TCP peer IP (every test request shares the key `unknown`)
   - the live JWKS fetch

   Area: server. Source: tests:server.
7. **No web UI tests** for sign-out, account deletion or the "Session expired" state. The production-mode Dev sign-in test re-imports modules rather than inspecting the real bundle. Area: web. Source: tests:web.
8. **Hard AI is property-tested on only 6 cases in debug builds.** The test also assumes Hard is deterministic: it must change if Hard gains randomness. Area: core. Source: tests:core.
9. **The applied `game_finished` path was not hit in the curl smoke run.** It is covered by `apps/server/tests/sync.rs`. Area: server. Source: verify.
10. **Edits made outside an agent's own area need review:**
    - tests:server changed `apps/server/src/client.rs` (`full_path()`) and `ratelimit.rs`. Middleware on the nested `/v1` router must not trust `req.uri().path()`.
    - New dependencies changed the root `Cargo.lock`: jsonschema and proptest (test only), plus the server crates.
    - rustls compiles in both ring and aws-lc-rs, and relies on `install_crypto_provider()`.
    - `/core/*/bindings/` was added to `.gitignore`.

    Area: server, core, infra. Source: tests:server, tests:core, build:core, build:server, qa.
11. **Changed Mermaid diagrams not rendered** in `docs/architecture/backend.md` and `monorepo.md`. Area: docs. Source: docs.
12. **Vite warnings and logs.**
    - The main chunk is 504 kB, over the 500 kB limit. Consider route-level code splitting.
    - Vite blocks Host `web` (`server.allowedHosts`), so container-to-container tests fail.
    - tower-http logs 501 `not_configured` at ERROR level.

    Area: web, server. Source: build:web, verify, qa, build:server.
13. **Visual nits.**
    - "Your game is saved automatically" still shows after the game ends.

    Area: web. Source: qa, tests:web. File: `apps/web/src/routes/Game.tsx`.
14. **Dev environment housekeeping.**
    - The first `just dev` builds from scratch into the new `cargo-target-server` volume (a few GB more disk).
    - `compose.lan.yaml` needs Compose 2.24 or later.
    - The `pgdata` volume holds test users; run `just db-reset`.

    Area: infra. Source: apply:infra, qa.

## Questions

1. **Secret scanners.** `apps/server/tests/fixtures/oidc_test_only_rsa.pem` is a committed test-only key. Add an allowlist entry for gitleaks and GitHub push protection? Area: server, infra. Source: tests:server.
2. **ts-rs output.** Should `cargo test --all-features` keep writing to the gitignored `core/*/bindings`? Or should `TS_RS_EXPORT_DIR` in `.cargo/config.toml` point at `apps/web/src/generated/protocol`? Area: core. Source: qa.
3. **Proptest.** Commit `proptest-regressions/*.txt` files, the usual practice? Should a nightly CI run use more cases? That would need `PROPTEST_CASES` to be honoured. Area: core. Source: tests:core.
4. **Pit numbering wording.** Each player's pits are 1–6 from their own left. Confirm iOS and Android will use the same wording. Area: web, docs. Source: build:web.
5. **Dev-only strings in production.** The Dev sign-in strings are still in the production i18n bundle (harmless). Drop them? Area: web. Source: qa.
6. **Return value of `choose_move`.** It returns `Option<Move>` (None when there is no legal move) so it never panics in WASM. OK? Area: core. Source: scaffold.
7. **Repeated mutation IDs.** They return the outcome stored the first time, and API.md was updated to match. OK? Area: server. Source: build:server.
8. **Repetition history in `GameState`.** It is kept as an opaque hex `history` field that clients send back unchanged, capped at 4,096 entries. Acceptable, or track repetition through replay only? Area: core. Source: scaffold, build:core.
9. **Access-token signing.** Access tokens use HS256 with a shared secret. Move to asymmetric keys later? Area: server. Source: scaffold.
10. **Unicode list for display names.** It is a hard-coded list of Unicode 16 format characters, chosen to avoid a new dependency. Revisit when upgrading Unicode or adding a normalization crate? Area: server. Source: apply:server. File: `apps/server/src/validate.rs`.
11. **A less brittle i18n test.** Export `RULES_SECTIONS` from `Rules.tsx`, so the test doesn't have to parse the source with a regex? Area: web. Source: tests:web.

## Suggestions not applied (from reviews)

1. **Dependency audit.** Add a `just audit` recipe (`cargo deny check advisories licenses` and `pnpm audit --prod`) and run it in CI before release. Area: infra. Source: sec. File: `justfile`.
2. **Supply-chain pinning.**
   - Pin the Rust toolchain: `channel = "stable"` in `rust-toolchain.toml`, and `rust:1-bookworm` in every Dockerfile.
   - Pin base images by digest for the production image.
   - Install cargo-binstall from a release tag, not `curl … main | bash`.
   - Note that bumping wasm-bindgen means editing both `core/wasm/Cargo.toml` (`=0.2.129`) and `WASM_BINDGEN_VERSION`.

   Area: infra. Source: sec, arch, scaffold. Files: `rust-toolchain.toml`, `docker/toolbox.Dockerfile`, `docker/server.dev.Dockerfile`, `apps/server/Dockerfile`.
3. **Typed mutation handling.** Add `MutationType` and `RejectionReason` enums in ouril-protocol, return a single `MutationOutcome`, and type `ai_level`. The JSON stays the same. Area: server, core. Source: arch. File: `apps/server/src/sync.rs`.
4. **Self-describing engine events.** Give `capture` and `collect_remaining` a target store and a pit list, so UIs animate from data instead of inferring rules. Area: core, web. Source: arch. File: `apps/web/src/game/animation.ts`.
5. **Retryable 503 for timeouts and pool timeouts.** This part of the timeout suggestion was skipped because it is a contract change; see Decision 6. The 15 s timeout itself was applied. Area: server. Source: arch, apply:server.
6. **Slimmer `server.dev.Dockerfile`.** Dropping wasm32, clippy and rustfmt was tried and reverted, because `rust-toolchain.toml` requires them. Area: infra. Source: arch, apply:infra.
7. **Also not applied, tracked above:**
   - permanent sync rejections (Blocking 1)
   - production topology (Blocking 2)
   - owner-scoped web storage (Blocking 3)
   - OAuth gaps (Blocking 4)
   - rate limiter (Blocking 5)
   - web CSP (Blocking 8)
   - single owner for records (Decision 4)
   - pull race (Decision 5)
   - retention and CHECK constraints (Decision 7)
   - `DELETE /v1/me` (Decision 8)
   - guest network definition (Decision 10)

   The applied review fixes were confirmed by `verify`: the cross-tab refresh lock, loopback ports, the separate server target volume, the release JWT and origin checks, HSTS, display-name Cf rejection, validation before the transaction, `trust_fly_client_ip`, the JWKS re-check and the 15 s timeout. The docs drift (arch) was fixed by `docs-keeper`.

## Open questions

- Who owns this list from now on, and should each item become a GitHub issue so it can be tracked and closed?
- When should this document move to `Proposed`: once every "Blocking before launch" item has an owner?
- Should resolved items be deleted from here, or moved to a short "Resolved" log with a link to the change?
