# Ouril task runner (ADR 0017). Every recipe runs through Docker, so the host needs only
# Docker + just. Inside the toolbox container (OURIL_IN_TOOLBOX=1, also CI) the same recipes
# run natively: `docker compose run --rm toolbox just test-core`.

set shell := ["bash", "-euo", "pipefail", "-c"]

in_toolbox := env_var_or_default("OURIL_IN_TOOLBOX", "")
dc := "docker compose"
tb := if in_toolbox == "1" { "" } else { "docker compose run --rm toolbox" }
web := if in_toolbox == "1" { "" } else { "docker compose run --rm web" }
# Inside the toolbox the db must already be running (compose network host `db`).
db_up := if in_toolbox == "1" { "true" } else { "docker compose up -d --wait db" }
migrations := "apps/server/migrations"
ts_out := "/workspace/apps/web/src/generated/protocol"

# List recipes
default:
    @just --list

# Create .env from .env.example if it doesn't exist
env:
    @test -f .env || (cp .env.example .env && echo "created .env from .env.example")

# Run the local stack: db, server (dev-auth, hot reload) and web (Vite on :5173)
dev: env bindings
    {{dc}} up db server web

# All tests: core, server, web
test: test-core test-server test-web

# Rust core tests (engine, ai, sync, protocol, wasm) incl. test vectors
test-core:
    {{tb}} cargo test -p ouril-engine -p ouril-ai -p ouril-sync -p ouril-protocol -p ouril-wasm

# Server tests (starts the db; dev-auth enabled as in local dev)
test-server:
    {{db_up}}
    {{tb}} cargo test -p ouril-server --features dev-auth
    # Release-like build too: proves /v1/auth/dev is absent without the feature.
    {{tb}} cargo test -p ouril-server

# Web tests (Vitest) against a fresh WASM build
test-web: bindings
    {{web}} sh -c "pnpm install --frozen-lockfile && pnpm --filter web test"

# Generate all bindings (MVP: WASM only; iOS/Android come later, ADR 0018)
bindings: bindings-wasm

# Build core/wasm/pkg (gitignored build output, ADR 0015)
bindings-wasm:
    {{tb}} wasm-pack build core/wasm --target web --out-dir pkg --out-name ouril_wasm --release
    {{tb}} wasm-opt -Oz --strip-debug core/wasm/pkg/ouril_wasm_bg.wasm -o core/wasm/pkg/ouril_wasm_bg.wasm

# Export TypeScript types for API/core types to apps/web/src/generated/protocol (gitignored)
protocol-ts:
    {{tb}} sh -c "rm -rf {{ts_out}} && TS_RS_EXPORT_DIR={{ts_out}} cargo test -p ouril-engine -p ouril-ai -p ouril-sync -p ouril-protocol --features ts export_bindings"

# Generate per-platform strings from shared/i18n (tools/i18n-gen, not built yet)
i18n:
    @echo "tools/i18n-gen is not implemented yet (ADR 0016). The web app reads shared/i18n/en.json directly for now."

# Drop, recreate and migrate the local database
db-reset:
    {{db_up}}
    {{tb}} sqlx database reset -y --source {{migrations}}

# Apply pending migrations to the local database
migrate:
    {{db_up}}
    {{tb}} sqlx migrate run --source {{migrations}}

# Open a shell in the toolbox container
shell:
    {{dc}} run --rm toolbox bash

# Format Rust and web code
fmt:
    {{tb}} cargo fmt --all
    {{web}} sh -c "pnpm install --frozen-lockfile && pnpm --filter web format"

# Lint: rustfmt check, clippy (warnings are errors), ESLint, Prettier check, tsc
lint:
    {{tb}} cargo fmt --all -- --check
    {{tb}} cargo clippy --workspace --all-targets -- -D warnings
    {{web}} sh -c "pnpm install --frozen-lockfile && pnpm --filter web lint && pnpm --filter web format:check && pnpm --filter web typecheck"

# Build the production server image (never includes dev-auth)
build-server-image:
    docker build -f apps/server/Dockerfile -t ouril-server:local .
