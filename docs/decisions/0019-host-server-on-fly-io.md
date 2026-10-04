# 0019. Host the Rust server and Postgres on Fly.io

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
[ADR 0017](0017-local-first-development.md) deferred hosting to a later decision. The Rust server (`apps/server`) is a single container image. It needs long-running processes, **WebSockets** for live multiplayer (Phase 4), a **Postgres** database close by, Valkey/Redis later, and background jobs ([backend](../architecture/backend.md)). Players are mostly in Cape Verde, Europe and the US diaspora. The web app is a static Vite build hosted separately; Vercel is the current candidate ([overview](../architecture/overview.md#open-questions)).

## Decision
- **Fly.io** hosts the Rust server, deployed from the same production image (`apps/server/Dockerfile`) that CI builds.
- **Postgres:** Fly's **managed Postgres**, in the **same region** as the server.
- **Region:** start in **one European region** (Frankfurt or Amsterdam, chosen at setup) for good latency to Cape Verde and the European diaspora. Add a US East region later if US latency matters.
- **Environments:** two Fly apps, `ouril-server-staging` and `ouril-server-prod`, each with its own Postgres cluster and secrets. Staging deploys from `main` and production from release tags.
- **Deploys:** GitHub Actions builds the image with the `toolbox` setup and runs `flyctl deploy`. The config lives in `apps/server/fly.toml`.
  - Database migrations (`sqlx migrate run`) run as Fly's **release command**, before new machines take traffic.
  - At least **2 machines** in production, for zero-downtime rolling deploys, plus health checks on `/healthz`.
- **Secrets** (database URL, Apple sign-in key, OAuth client secrets, Sentry DSN, later push credentials) live in `fly secrets`, never in the repo or the image.
- **Backups:** besides the managed Postgres backups, a scheduled job writes logical dumps (`pg_dump`) to S3-compatible object storage. Restores are tested before launch.
- **Release image safety:** CI checks that the production image is built **without** the `dev-auth` feature ([ADR 0017](0017-local-first-development.md)).
- **Later (Phase 4):** live games are pinned to one machine using Fly's request routing (`fly-replay`), with Valkey/Redis for presence, queues and pub/sub between machines ([ADR 0010](0010-postgres-and-local-sqlite.md)).
- The server stays **cloud-agnostic** (one image, 12-factor config, standard Postgres), so moving hosts remains possible.

## Alternatives considered
- **Render:** the simplest managed option, with Postgres, a Valkey-compatible store, workers and cron. It has fewer regions, and Fly fits a future multi-region setup and live-game pinning better.
- **Railway:** very easy to start with, but fewer regions and less control.
- **Google Cloud Run:** scales to zero, which is cheap for the MVP, but its autoscaling and connection time limits suit in-memory live games poorly.
- **AWS (ECS Fargate + RDS + ElastiCache):** the most control, but much more to set up and operate for a small team.
- **A VPS with Docker Compose:** cheapest, but we'd own backups, failover, TLS and patching.

## Consequences
- One provider for the server and database keeps latency low and operations simple.
- Usage-based pricing: costs must be watched as traffic grows. Set alerts on the Fly organisation.
- Moving off Fly later is possible but would need a database migration, so keep backups portable (`pg_dump`).
- A staging environment exists from the first deployment, so release checks can run against real infrastructure.
