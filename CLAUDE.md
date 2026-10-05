# CLAUDE.md

Guidance for AI agents (and humans) working in this repo.

## What this is

**Ouril** is a digital version of the Cape Verdean seed-sowing game (Oware family), shipping native apps for iOS (SwiftUI) and Android (Jetpack Compose), a web app (TypeScript + React), and a Rust server. A shared **Rust core** (rules engine, AI, sync rules) is compiled into every app and the server.

**Current state:** Phase 0 (documentation) is done. Phase 1, **the MVP, is the web app alone as a static site with no server** ([ADR 0025](docs/decisions/0025-web-first-static-mvp.md), [deployment](docs/architecture/deployment.md)):
- **Built:** the Rust core (`core/engine`, `core/ai`, `core/sync`, `core/protocol`, `core/wasm`; `cv.standard@1` passes every test vector), the server (`apps/server`: dev sign-in, profile, account deletion, sync push and pull), and the guest-first web app (`apps/web`: play vs AI, tutorial, rules, settings, About; a 3D board, ADR 0021; sign-in with sync and Stats are built but off for the MVP, `apps/web/src/features.ts`).
- **Placeholders:** Google and Apple sign-in answer `501 not_configured` until client IDs exist, and the web buttons for them are disabled. Use dev sign-in (`POST /v1/auth/dev`) locally. `just i18n` and `core/ffi` (UniFFI) aren't built yet.
- **Not started:** iOS and Android.

Development is **local-first and Docker-first** (ADR 0017): run things through `just` / `docker compose`, not host-installed toolchains, except for iOS/Xcode and the Android Emulator. Build order: core → web → iOS → Android (ADR 0018). See the [roadmap](docs/product/roadmap.md).

## Where things are decided

| Topic | Source of truth |
|---|---|
| Index of all docs | [docs/README.md](docs/README.md) |
| Game rules (MVP default) | [docs/game/rules.md](docs/game/rules.md) + spec [docs/game/variants/cape-verde/standard.md](docs/game/variants/cape-verde/standard.md) |
| Variant parameters and catalog | [docs/game/variants/README.md](docs/game/variants/README.md) |
| Architecture | [docs/architecture/](docs/architecture/overview.md) |
| Rule test format | [docs/architecture/test-vectors.md](docs/architecture/test-vectors.md) |
| Versioning and compatibility | [docs/architecture/versioning.md](docs/architecture/versioning.md) |
| Decisions | [docs/decisions/](docs/decisions/README.md) (ADRs) |
| Workflow and conventions | [CONTRIBUTING.md](CONTRIBUTING.md) |
| PR checklist and issue forms | [.github/](.github/pull_request_template.md) |
| Labels | [.github/labels.yml](.github/labels.yml) |

## Rules for agents

1. **Read the relevant docs before changing anything.** If the code and docs disagree, stop and ask; don't silently pick one.
2. **Game rules come only from variant specs.** Never implement or "fix" a rule from memory or from general Oware knowledge. If a spec says `null` or **To confirm**, ask the user.
3. **New or unverified variants:** use the `variant-research` skill (`.claude/skills/variant-research/`). Only use config keys from the parameters table in `docs/game/variants/README.md`.
4. **Every rule change needs test vectors** ([format](docs/architecture/test-vectors.md)), and changing a released variant means bumping its version ([versioning](docs/architecture/versioning.md)).
5. **Keep `core/*` pure:** no I/O, networking, storage, clocks or randomness without an injected seed. Networking, storage and sign-in are native in each app.
6. **Offline-first:** gameplay never depends on the network. **Guests make no network requests** unless they have opted into telemetry ([ADR 0014](docs/decisions/0014-telemetry-consent.md)).
7. **Significant decisions get an ADR** in `docs/decisions/` (template in its README). Update the docs in the same change as the code they describe.
8. **Never hand-edit or commit generated code** (UniFFI and wasm-bindgen output, generated string files). Regenerate it with `just bindings` / `just i18n` (ADRs 0015, 0016).
9. Don't commit, push, or create releases unless asked.

## Project skills and hooks

| Skill | Use it to |
|---|---|
| `/variant-research <country or variant>` | Research a variant's rules and write its spec |
| `/rule-change <variant> <rule>` | Apply a confirmed rule answer: spec, rules.md, vectors, version |
| `/test-vectors <variant> <area>` | Write or update JSON rule test vectors and check them |
| `/rules-audit [variant]` | Check that the parameters, specs, rules.md, vectors and engine agree (report only) |
| `/adr <title \| accept/reject/supersede NNNN>` | Write or change an ADR and propagate it through the docs |
| `/docs-audit [area]` | Check links, structure, index or catalog drift, and contradictions between docs |
| `/review-animations` | Review UI motion against the motion rules in [web design](docs/architecture/web-design.md). The `emil-design-eng` and `emilkowalski-motion` skills guide motion while you build |

**Hook:** after every edit to a Markdown doc, `.claude/hooks/docs-check.sh` runs the docs checker and reports errors in that file. Fix them before moving on.

## Conventions

- Conventional Commits: `feat(engine): …`, `fix(ios): …`, `docs: …`.
- Rust crates are named `ouril-*`. Variant IDs are `<iso2>.<slug>` (`cv.standard`).
- Docs: each one starts with a purpose line and a **Status**, and ends with **Open questions**. Use relative links and Mermaid diagrams.
- English is the default language. All UI strings live in `shared/i18n/<locale>.json` (ICU subset, ADR 0016), never hard-coded: `apps/web/src/i18n/hardcoded.test.ts` fails on literal text or labels in components, and `keys.test.ts` on missing keys.

## Commands

The host needs only Docker, plus `just` if you want it. Don't use host Rust or Node: everything runs in containers. The `toolbox` image includes `just`, so without `just` on the host, run any recipe as `docker compose run --rm toolbox just <recipe>`. Recipes that start other containers are the exception (`dev`, `test-web`, `fmt`, `lint`, `build-server-image`); use the plain `docker compose` command for those.

| Task | With `just` | Without `just` on the host |
|---|---|---|
| Create `.env` from `.env.example` | `just env` | `cp .env.example .env` |
| Build the WASM package (`core/wasm/pkg`, needed before `pnpm install`) | `just bindings` | `docker compose run --rm toolbox just bindings` |
| Run the stack (db, server with dev-auth and hot reload on :8080, web on :5173) | `just dev` | build the WASM package, then `docker compose up db server web` |
| Rust core tests (engine, AI, sync, protocol, wasm, test vectors) | `just test-core` | `docker compose run --rm toolbox just test-core` |
| Server tests (needs `db`) | `just test-server` | `docker compose up -d --wait db && docker compose run --rm toolbox just test-server` |
| Web tests (Vitest) | `just test-web` | build the WASM package, then `docker compose run --rm web sh -c "pnpm install --frozen-lockfile && pnpm --filter web test"` |
| Everything | `just test` | the three rows above |
| Lint (rustfmt, clippy `-D warnings`, ESLint, Prettier, tsc) / format | `just lint` / `just fmt` | see the `lint` and `fmt` recipes in the `justfile` |
| TS types for the API (`apps/web/src/generated/protocol`) | `just protocol-ts` | `docker compose run --rm toolbox just protocol-ts` |
| Migrations / reset the db | `just migrate` / `just db-reset` | `docker compose run --rm toolbox just migrate` |
| Production server image (never includes `dev-auth`) | `just build-server-image` | `docker build -f apps/server/Dockerfile -t ouril-server:local .` |
| Shell with the toolchain | `just shell` | `docker compose run --rm toolbox bash` |

- Generated, gitignored output: `core/wasm/pkg/`, `apps/web/src/generated/` (protocol types and i18n), and `core/*/bindings/` (ts-rs output from `cargo test --all-features`). `pnpm --filter web dev|build|test|typecheck` regenerates the web strings from `shared/i18n/` (`apps/web/scripts/gen-i18n.mjs`). `just i18n` is a stub until `tools/i18n-gen` exists.
- Published ports bind to `127.0.0.1`. For LAN or phone testing: `docker compose -f compose.yaml -f docker/compose.lan.yaml up db server web`.
- Docker Desktop: don't run two heavy cargo builds at once. The `server` service has its own target volume, so `cargo-watch` and toolbox builds don't block each other.

Contracts: [core/wasm/API.md](core/wasm/API.md) (web ↔ core) and [apps/server/API.md](apps/server/API.md) (HTTP API).
