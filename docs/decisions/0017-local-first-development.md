# 0017. Local-first, Docker-first development; hosting decided before the first deployment

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
We start by building and running everything on a developer machine. Hosting and the choice of managed vs self-hosted Postgres ([data and sync](../architecture/data-and-sync.md), [backend](../architecture/backend.md)) don't need to be settled to write the core, the apps or the server. The project spans several toolchains: Rust with cross-compile targets, wasm-pack, cargo-ndk with the Android NDK, Node with pnpm, and Postgres. Installing all of them on every machine is slow and drifts between contributors. Sign-in with Google and Apple normally needs real credentials and public HTTPS URLs, which is awkward locally.

## Decision

### Docker is the default way to run everything locally
- One `compose.yaml` at the root defines the local stack. The host needs only **Docker** (Docker Desktop or OrbStack) and **`just`**. `just` recipes wrap `docker compose`, so you can also run the compose commands directly.

| Service | Image | Purpose | Port |
|---|---|---|---|
| `db` | `postgres:18` | Local database, data in a named volume, with a healthcheck | 5432 |
| `server` | `docker/server.dev.Dockerfile` (Rust + `cargo-watch`) | `ouril-server` with hot reload and the `dev-auth` feature; starts after `db` is healthy | 8080 |
| `web` | `node:lts` + pnpm | Vite dev server (`--host`), proxying `/v1` to `server:8080` | 5173 |
| `toolbox` | `docker/toolbox.Dockerfile` | Rust + wasm-pack + cargo-ndk + Android NDK + `just`: tests, test vectors, WASM and Android bindings, `i18n-gen`, migrations | — |
| `valkey` | `valkey/valkey` | Added with multiplayer (compose profile `multiplayer`) | 6379 |

- **Fast rebuilds:** the cargo registry, the `target/` directory and `node_modules` live in **named volumes**, not bind mounts, to avoid slow file I/O on macOS. Source code is bind-mounted.
- **Main commands:**
  - `just dev`: `docker compose up db server web`
  - `just test`: core tests and vectors in `toolbox`
  - `just bindings`: WASM and Android in `toolbox`, plus iOS on the host (see below)
  - `just db-reset`: recreate the database with migrations and seed data
  - `just shell`: a shell in `toolbox`
- **Dev Container:** `.devcontainer/devcontainer.json` reuses the `toolbox` image, so VS Code or Codespaces get the same environment.
- **CI uses the same `toolbox` image**, so local and CI builds behave the same.
- **Production image:** `apps/server/Dockerfile` is a multi-stage build producing a small runtime image. It's the same artifact we'll deploy, which keeps us cloud-agnostic.

### What runs on the host
Apple and Android tooling can't run in Linux containers:
- **iOS:** Xcode, the Simulator, and the Swift/XCFramework bindings (`just bindings-ios` runs on macOS). The Simulator reaches the server at `http://localhost:8080`.
- **Android:** Android Studio and the Emulator. The bindings are built in `toolbox`, and the Emulator reaches the server at `http://10.0.2.2:8080`.

### Configuration, sign-in and telemetry
- The server reads config from `.env`, loaded by compose. The template is `.env.example`; never commit `.env`.
- **Dev sign-in:** debug builds of the server and apps include a **development identity provider** (`POST /v1/auth/dev`). It signs in as a seeded test user without contacting Google or Apple. It's compiled in only with the `dev-auth` feature and debug app builds, and **never** in release builds (CI checks the production image and release apps). Real Google/Apple sign-in is tested with local credentials when needed.
- **Telemetry** is off in local builds (no Sentry DSN), so local testing never sends data.

### Hosting
- Hosting, managed vs self-hosted Postgres, and environments (staging, prod) are decided in a new ADR **before the first deployment** (internal beta). Until then the server stays cloud-agnostic: 12-factor config, one container image, standard Postgres, no provider-specific services.

## Alternatives considered
- **Native toolchains on the host:** fastest for compiling, but every contributor installs and maintains Rust targets, the NDK, Node and Postgres, and versions drift. It still works for anyone who prefers it, since the `just` recipes can run natively. Docker is just the default.
- **Docker only for the database:** less setup, but the toolchain drift problem remains.
- **Hosted dev database** (for example a Neon branch): needs a network connection and an account for every contributor, which goes against offline-friendly development.
- **Pick hosting now:** premature. It costs money and attention before there's anything to deploy.

## Consequences
- A new contributor goes from clone to a running stack with `just dev`. Only iOS work needs extra host tooling.
- The first container build is large: the toolbox includes the Android NDK. Named volumes keep later builds fast.
- On macOS, Docker's VM needs enough memory and disk (Rust builds are heavy). CONTRIBUTING lists recommended settings.
- The dev identity provider is a security-sensitive feature flag, so release-build checks must prove it's absent.
- A later ADR must pick hosting, plus a staging environment, before the beta.
