# CLAUDE.md

Guidance for AI agents (and humans) working in this repo.

## What this is

**Ouril** is a digital version of the Cape Verdean seed-sowing game (Oware family), shipping native apps for iOS (SwiftUI) and Android (Jetpack Compose), a web app (TypeScript + React), and a Rust server. A shared **Rust core** (rules engine, AI, sync rules) is compiled into every app and the server.

**Current state:** Phase 0 (documentation) is done; Phase 1 (MVP) is starting with the Rust engine. `core/test-vectors/` holds the rule test cases; there's no other code yet. Development is **local-first and Docker-first** (ADR 0017): run things through `just` / `docker compose`, not host-installed toolchains, except for iOS/Xcode and the Android Emulator. Build order: core → web → iOS → Android (ADR 0018). See the [roadmap](docs/product/roadmap.md).

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

**Hook:** after every edit to a Markdown doc, `.claude/hooks/docs-check.sh` runs the docs checker and reports errors in that file. Fix them before moving on.

## Conventions

- Conventional Commits: `feat(engine): …`, `fix(ios): …`, `docs: …`.
- Rust crates are named `ouril-*`. Variant IDs are `<iso2>.<slug>` (`cv.standard`).
- Docs: each one starts with a purpose line and a **Status**, and ends with **Open questions**. Use relative links and Mermaid diagrams.
- English is the default language. All UI strings live in `shared/i18n/<locale>.json` (ICU subset, ADR 0016), never hard-coded.

## Commands

None yet. Build, test and run commands go here once the monorepo is scaffolded (a `justfile` at the root, see [monorepo.md](docs/architecture/monorepo.md)).
