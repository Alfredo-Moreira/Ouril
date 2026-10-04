# Monorepo layout

> Folder structure, tooling and boundaries. **Status:** Accepted. See [ADR 0007](../decisions/0007-rust-core-with-generated-bindings.md) and [ADR 0008](../decisions/0008-native-apps-per-platform.md).

## Layout

```
ouril/
├── core/                   # Rust workspace members shared by every platform
│   ├── engine/             # ouril-engine: rules + variant configs        (Phase 1)
│   ├── ai/                 # ouril-ai: computer opponent                   (Phase 1)
│   ├── protocol/           # ouril-protocol: API + realtime message types  (Phase 1: auth; later: games)
│   ├── sync/               # ouril-sync: stats derived from game records, merge rules, mutation types (Phase 1)
│   ├── ffi/                # UniFFI bindings → Swift + Kotlin              (Phase 1)
│   ├── wasm/               # wasm-bindgen bindings → TypeScript/WASM       (Phase 1)
│   └── test-vectors/       # language-neutral JSON rule tests              (Phase 1)
├── apps/
│   ├── ios/                # SwiftUI app (Xcode); uses the core as an XCFramework via SPM
│   ├── android/            # Kotlin + Jetpack Compose (Gradle); uses the core .so + Kotlin bindings
│   ├── web/                # TypeScript + React (Vite); uses the core WASM package
│   └── server/             # Rust (Axum): auth + accounts (Phase 1), multiplayer (later); Dockerfile + fly.toml (ADR 0019)
├── tools/
│   └── i18n-gen/           # Rust CLI: shared/i18n → .xcstrings, strings.xml, i18next JSON (ADR 0016)
├── shared/
│   ├── i18n/               # source translations (ICU messages in JSON) → generated per-platform files
│   └── assets/             # board/seed art, sounds, icons
├── docs/                   # this documentation
├── docker/                 # toolbox.Dockerfile (Rust, wasm-pack, cargo-ndk, NDK) + server.dev.Dockerfile (ADR 0017)
├── .devcontainer/          # Dev Container using the toolbox image
├── compose.yaml            # local stack: db (Postgres 18), server, web, toolbox; later valkey
├── .env.example            # server config template; copy to .env (never committed)
├── Cargo.toml              # Rust workspace root (core/* + apps/server + tools/*)
├── justfile                # cross-language task runner (build-core, bindings, test-all…)
└── pnpm-workspace.yaml     # web tooling
```

## Tooling

| Area | Tools |
|---|---|
| Rust | Cargo workspace, `clippy`, `rustfmt`, `cargo test`/`nextest`, `proptest` for property tests |
| Bindings | **UniFFI** (Swift and Kotlin), `cargo-ndk` (Android ABIs), XCFramework packaging for iOS, **wasm-bindgen**/`wasm-pack` (web) |
| iOS | Xcode, Swift Package Manager, SwiftUI, GRDB (SQLite), XCTest |
| Android | Gradle, Kotlin, Jetpack Compose, Room (SQLite), WorkManager, JUnit |
| Web | pnpm, Vite, React, TypeScript (strict), Dexie (IndexedDB), Vitest, ESLint, Prettier |
| Server | Axum, Tokio, sqlx (Postgres), Redis client (later) |
| Local environment | **Docker Compose** (`compose.yaml`): `db`, `server`, `web`, `toolbox`. Only Xcode, the iOS Simulator and the Android Emulator run on the host ([ADR 0017](../decisions/0017-local-first-development.md)) |
| Tasks | `just`: wraps `docker compose` and runs the same recipes locally and in CI (`just dev`, `just test`, `just bindings`, `just i18n`) |
| CI | GitHub Actions, using the same `toolbox` image as local development: Rust tests and lints on Linux; iOS build on macOS runners; Android and web builds; test vectors on every platform |
| Releases | fastlane (or Xcode Cloud) for App Store; Gradle Play Publisher for Google Play; static hosting + CDN for web |

## Dependency rules

```mermaid
flowchart LR
  ios[apps/ios] --> ffi
  android[apps/android] --> ffi
  web[apps/web] --> wasm
  server[apps/server] --> engine
  server --> protocol
  ffi[core/ffi] --> engine
  ffi --> ai
  ffi --> protocol
  ffi --> sync
  wasm[core/wasm] --> engine
  wasm --> ai
  wasm --> protocol
  wasm --> sync
  server --> sync
  sync[core/sync] --> protocol
  ai[core/ai] --> engine[core/engine]
  protocol[core/protocol] --> engine
```

- `core/*` crates are **pure**: no I/O, no networking, no storage, no platform APIs.
- Apps never call the engine directly. They go through the generated bindings (`ffi` or `wasm`).
- Apps never import from each other. Shared logic belongs in `core/`, and shared resources in `shared/`.
- **Networking, sign-in and storage are native** in each app, using platform SDKs.

## Conventions

- Rust crates are prefixed `ouril-` (`ouril-engine`, `ouril-ai`, …).
- Generated bindings and generated string files are **build output**: gitignored, regenerated with `just bindings` / `just i18n`, never hand-edited ([ADR 0015](../decisions/0015-generated-bindings-not-committed.md)).
- Commits: [Conventional Commits](https://www.conventionalcommits.org/) (`feat(engine): …`, `fix(ios): …`).
- Branches: `main` is always releasable. Feature branches merge through PRs.

## Open questions

- Should a marketing/landing site live in `apps/site`, or be part of `apps/web`?
