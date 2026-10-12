# 0028. One deployed environment: production; local Docker is staging

- **Status:** Accepted
- **Date:** 2026-10-11
- **Supersedes:** [0019](0019-host-server-on-fly-io.md) (the staging environment only)

## Context
[ADR 0019](0019-host-server-on-fly-io.md) planned two Fly apps, `ouril-server-staging` and `ouril-server-prod`, each with its own database, and [ADR 0017](0017-local-first-development.md) asked for a staging environment before the first deployment. Ouril has one maintainer and no players on accounts yet. The local Docker stack (`just dev`, `just build-server-image`) already runs the same server, migrations and Postgres version as production. A hosted staging environment would add a second Fly app, a second Neon project ([ADR 0027](0027-postgres-on-neon-free-plan.md)), and secrets to keep in step.

## Decision
- **Deploy one environment, production:** the Fly app `ouril-server-prod` (`apps/server/fly.toml`) and one Neon project.
- **Local Docker is the staging environment:** check changes with `just dev` and `just test`, and the release image with `just build-server-image`, before deploying.
- **Production deploys are manual:** `deploy-server.yml` deploys a release tag by hand, only if CI passed on its commit.
- **Revisit** when real Google and Apple sign-in need a public HTTPS host to test against, or when more people deploy.

See [deployment](../architecture/deployment.md#the-server-on-flyio).

## Alternatives considered
- **Hosted staging (ADR 0019 as written):** catches host-specific problems (Fly config, Neon TLS, real OAuth callbacks) before production, but doubles the setup and secrets for one maintainer.
- **Neon branch as staging, no Fly staging app:** a cheap database copy, but it doesn't test the deployed server, and it still needs its own secrets.

## Consequences
- Half the setup: one Fly app, one Neon project, one GitHub environment (`server-production`).
- Problems that only appear on Fly or Neon (TLS, cold starts, release command) show up first in production. Deploy when you can watch the result, and check `/readyz` after each deploy.
- Real Google and Apple tokens can't be tested locally (local sign-in uses `POST /v1/auth/dev`). Test them on production before turning the web sign-in buttons on.
