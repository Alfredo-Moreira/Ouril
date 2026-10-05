# Contributing to Ouril

> How we work: workflow, conventions, and what "done" means. **Status:** Draft. Local setup covers the web app, the server and the Rust core. iOS and Android setup comes when those apps start.

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Workflow

1. **Docs first.** For anything beyond a small fix, check the relevant doc in [`docs/`](docs/README.md). If the change alters a decision, write or update an [ADR](docs/decisions/README.md) first.
2. **Branch** from `main`: `feat/<short-name>`, `fix/<short-name>`, `docs/<short-name>`.
3. **Commit** with [Conventional Commits](https://www.conventionalcommits.org/). Use the scope for the area: `engine`, `ai`, `sync`, `protocol`, `ios`, `android`, `web`, `server`, `docs`, `ci`.
4. **Open a PR.** The [PR template](.github/pull_request_template.md) includes the definition of done below. CI must pass. Keep PRs focused, with one concern each.
5. **Merge to `main`**, which is always releasable.

## Issues

Use the [issue forms](https://github.com/Alfredo-Moreira/Ouril/issues/new/choose): bug report, **rule or variant report** ("that's not how we play it here"), feature request, or docs issue. Labels are defined in [`.github/labels.yml`](.github/labels.yml). Report security issues privately ([SECURITY.md](SECURITY.md)).

## When to write an ADR

Write one when a change:

- adds or replaces a language, framework, database or external service
- changes a public contract: the API, realtime protocol, sync format, game record or test-vector format
- changes how data is stored, synced or retained, or what personal data is collected
- adds a variant parameter to the engine

## Game rules and variants

- Rules are implemented **only** from variant specs in [`docs/game/variants/`](docs/game/variants/README.md).
- Player reports from the **rule or variant report** form are evidence for the `variant-research` skill. Link the issue in the spec's sources.
- To add or verify a variant, run the `variant-research` skill, review the spec, and resolve the open questions with players from that community.
- Every rule needs [test vectors](docs/architecture/test-vectors.md). Every behaviour that is still "To confirm" gets a vector tagged `to-confirm`.
- Changing a released variant's behaviour means a **new variant version** ([versioning](docs/architecture/versioning.md)).

## Definition of done

- [ ] Tests added or updated. For rule changes, test vectors pass in Rust **and** through every binding (Swift, Kotlin, WASM).
- [ ] Docs updated in the same PR (including the ADR if needed).
- [ ] User-facing strings added to `shared/i18n/` (English at minimum). Nothing hard-coded.
- [ ] Accessibility checked: screen-reader labels, contrast, tap target size.
- [ ] Works offline where applicable. Guests make no network requests without telemetry consent.
- [ ] No secrets in code or config. Generated bindings regenerated, not hand-edited.

## Local setup: Docker first

Development is **local-first and Docker-first** ([ADR 0017](docs/decisions/0017-local-first-development.md)). The database, server, web dev server and Rust/WASM toolchain run in containers defined in `compose.yaml`. You don't need Rust or Node on your machine.

| Need it for | Install on your machine |
|---|---|
| Everything | `git`, **Docker** (Docker Desktop or OrbStack). [`just`](https://github.com/casey/just) is optional: the `toolbox` container includes it. |
| iOS (later) | macOS, Xcode (latest stable). The iOS bindings and the Simulator can't run in Docker. |
| Android (later) | Android Studio, for the Emulator and device debugging. Bindings will be built in the `toolbox` container. |

### First run

```bash
just env        # creates .env from .env.example (or: cp .env.example .env)
just bindings   # builds core/wasm/pkg; the web app needs it before `pnpm install`
just dev        # db (Postgres 18), server (dev-auth, hot reload) and web (Vite)
```

Without `just` on the host:

```bash
cp .env.example .env
docker compose run --rm toolbox just bindings
docker compose up db server web
```

- Open the web app at <http://localhost:5173>. It starts as a guest: once the page has loaded, a guest makes no network requests. The Vite dev server proxies `/v1` to the server, so signed-in requests are same-origin.
- The server listens on <http://localhost:8080> (`/healthz`, `/v1/meta`). Debug builds apply migrations on startup.
- **Sign-in locally:** use **Dev sign-in** on the web Sign-in screen (shown only in dev builds), or `POST /v1/auth/dev`. Google and Apple are placeholders: their buttons are disabled, and the endpoints answer `501 not_configured` while `GOOGLE_CLIENT_ID` / `APPLE_*` in `.env` are empty.
- Published ports bind to `127.0.0.1` only, because the dev server lets anyone sign in. For testing from a phone on your LAN: `docker compose -f compose.yaml -f docker/compose.lan.yaml up db server web`.

### Everyday commands

| Command | What it does |
|---|---|
| `just dev` | Build the WASM package, then start `db`, `server` and `web` |
| `just test` | All tests: `test-core`, `test-server`, `test-web` |
| `just test-core` | Rust core tests, including every test vector and the property tests |
| `just test-server` | Server unit and integration tests against the compose Postgres (with `dev-auth`) |
| `just test-web` | Rebuild the WASM package, then run the Vitest suite |
| `just lint` / `just fmt` | rustfmt, clippy (warnings are errors), ESLint, Prettier and tsc / format Rust and web code |
| `just bindings` | Build `core/wasm/pkg` (WASM only for now; UniFFI comes with iOS/Android) |
| `just protocol-ts` | Export the API and core TypeScript types to `apps/web/src/generated/protocol/` |
| `just migrate` / `just db-reset` | Apply migrations / drop, recreate and migrate the local database |
| `just build-server-image` | Build the production server image (`apps/server/Dockerfile`, never with `dev-auth`) |
| `just shell` | Open a shell in the `toolbox` container |

Without `just` on the host, run any toolbox recipe as `docker compose run --rm toolbox just <recipe>` (for example `just test-core`). For `test-server`, start the database first with `docker compose up -d --wait db`. Web commands run in the `web` container: `docker compose run --rm web sh -c "pnpm install --frozen-lockfile && pnpm --filter web test"`.

- Never commit `.env`. Every OAuth, Sentry and JWT value in it is a local placeholder.
- Generated files are build output, gitignored and never hand-edited: `core/wasm/pkg/`, `apps/web/src/generated/` (protocol types and i18n strings) and `core/*/bindings/`. The web strings are generated from `shared/i18n/` every time you run the web `dev`, `build`, `test` or `typecheck` scripts. `just i18n` is a stub until `tools/i18n-gen` exists ([ADR 0016](docs/decisions/0016-i18n-source-format.md)).
- Docker Desktop on macOS: give the VM at least 8 GB of memory and 60 GB of disk, and avoid running two heavy cargo builds at once.
- Native toolchains (Rust via `rustup`, `wasm-pack`, Node LTS + pnpm) still work if you prefer them: set `OURIL_IN_TOOLBOX=1` so the `just` recipes run commands directly. Docker is the supported default.
- VS Code users can open the repo in the Dev Container (`.devcontainer/`), which uses the same `toolbox` image.

### Claude Code skills

Our own skills in `.claude/skills/` are committed. Third-party skills (Rust, Swift, Kotlin/Compose, Postgres, security, architecture) are listed in `skills-lock.json` but **not committed**, because they ship without licence files. Restore them with:

```bash
npx skills experimental_install
```

When you add or remove a third-party skill, keep its folder in `.gitignore` in sync with `skills-lock.json`.

## Code style

- Rust: `rustfmt` + `clippy` (warnings are errors in CI).
- Swift: SwiftFormat/SwiftLint. Kotlin: ktlint. TypeScript: ESLint + Prettier.
- Prefer small, pure functions. All game logic lives in `core/`; apps contain no rule logic.
