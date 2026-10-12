# 0027. Postgres on Neon's free plan, the server stays on Fly.io

- **Status:** Accepted
- **Date:** 2026-10-11
- **Supersedes:** [0019](0019-host-server-on-fly-io.md) (the database only)

## Context
[ADR 0019](0019-host-server-on-fly-io.md) put the server and Postgres on Fly.io, with one managed Postgres cluster per environment. Fly's managed Postgres has no free tier: its smallest plan is about $38 a month per cluster, so staging and production would cost about $76 a month before accounts have a single player ([roadmap](../product/roadmap.md), Phase 2). The server needs plain Postgres ([ADR 0010](0010-postgres-and-local-sqlite.md)) with TLS, close to its Fly region (`fra`).

## Decision
- **Postgres runs on Neon's free plan:** one Neon project for production (there is no hosted staging, [ADR 0028](0028-production-only-local-staging.md)), in AWS Frankfurt (`aws-eu-central-1`), next to the Fly region.
- **The server stays on Fly.io** as in ADR 0019: release-command migrations, 2 production machines, deploys from GitHub Actions.
- **`DATABASE_URL` is Neon's direct (non-pooled) connection string**, with `sslmode=require`, set with `fly secrets set`. Neon's pooler runs in transaction mode, which breaks sqlx's named prepared statements and the migrator's advisory lock.
- **The server uses only the primary compute.** The project has a read replica, but every signed-in request checks its session on the primary (logout and deletion take effect at once), and sync pull runs right after a push, so replica lag would hide a player's own changes. `ouril-server migrate` refuses a read-only `DATABASE_URL`, so a replica's connection string stops the deploy.
- **Let the database sleep:** Fly's health check is `/healthz`, which doesn't touch the database, and the server's pool closes connections idle for 4 minutes, before Neon suspends an idle compute (5 minutes). `/readyz` stays available for manual checks.
- **Revisit** when a project nears the free plan's limits (0.5 GB of storage, 100 compute-hours a month), or when production needs longer point-in-time restore. The next steps are Neon's paid plan or Fly managed Postgres, both a `DATABASE_URL` change plus a `pg_dump` restore.

See [deployment](../architecture/deployment.md#the-server-on-flyio).

## Alternatives considered
- **Fly managed Postgres for both environments (ADR 0019 as written):** about $76 a month with no players yet.
- **Fly managed Postgres for production only, a free or unmanaged database for staging:** still $38 a month before launch, and two database providers to operate.
- **Fly's unmanaged Postgres (about $2 a month):** unsupported by Fly, so backups, failover and upgrades would be ours.
- **Supabase's free plan:** also free Postgres, but its free projects pause after a week without activity and need a manual restore, and it bundles auth and APIs we don't use.

## Consequences
- The database costs nothing until traffic grows. The server stays about $8–11 a month on Fly.
- A first query after 5 idle minutes waits for Neon to wake (a few hundred milliseconds).
- **The free limits are hard:** when a project uses its 100 compute-hours, Neon suspends its compute until the next month, and the API fails until then. Watch usage in Neon's console and upgrade before production has steady daily players.
- The read replica scales to zero while nothing connects to it. Running computes, replicas included, are assumed to draw on the project's compute-hours, so keep anything from connecting to it.
- The free plan keeps only a short point-in-time restore window, so the scheduled `pg_dump` backups from ADR 0019 matter more.
- The database is a second provider with its own account, outside Fly's private network. Traffic to it goes over TLS on the public internet.
