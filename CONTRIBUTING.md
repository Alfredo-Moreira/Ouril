# Contributing to Ouril

> How we work: workflow, conventions, and what "done" means. **Status:** Draft. Toolchain sections will be filled in when the monorepo is scaffolded.

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

## Local setup (planned): Docker first

Development is **local-first and Docker-first** ([ADR 0017](docs/decisions/0017-local-first-development.md)). The database, server, web dev server and all Rust/WASM/Android toolchains run in containers defined in `compose.yaml`.

| Need it for | Install on your machine |
|---|---|
| Everything | `git`, **Docker** (Docker Desktop or OrbStack), [`just`](https://github.com/casey/just) |
| iOS | macOS, Xcode (latest stable). The iOS bindings and the Simulator can't run in Docker. |
| Android | Android Studio, for the Emulator and device debugging. Bindings are built in the `toolbox` container. |

Planned commands (exact usage will be documented once the scaffolding exists):

| Command | What it does |
|---|---|
| `just dev` | Start `db` (PostgreSQL 18), `server` (hot reload, dev sign-in) and `web` (Vite) |
| `just test` | Core tests and test vectors in the `toolbox` container |
| `just bindings` | WASM and Android bindings in `toolbox`, iOS bindings on the macOS host |
| `just i18n` | Generate platform string files from `shared/i18n/` |
| `just db-reset` | Recreate the local database with migrations and seed data |
| `just shell` | Open a shell in the `toolbox` container |

- Copy `.env.example` to `.env`, and never commit `.env`.
- Docker Desktop on macOS: give the VM at least 8 GB of memory and 60 GB of disk (Rust and the Android NDK are heavy).
- Native toolchains (Rust via `rustup`, `wasm-pack`, `cargo-ndk`, Node LTS + pnpm) still work if you prefer them. The `just` recipes can run natively, but Docker is the supported default and what CI uses.
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
