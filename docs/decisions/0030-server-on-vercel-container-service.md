# 0030. Run the server on Vercel, as a container service next to the web app

- **Status:** Accepted
- **Date:** 2026-10-12
- **Supersedes:** [0019](0019-host-server-on-fly-io.md) (the server's host) and [0029](0029-api-through-vercel-proxy.md)

## Context
The server ran on Fly.io ([ADR 0019](0019-host-server-on-fly-io.md)), and the web app reached it through a `/v1` proxy with a shared secret ([ADR 0029](0029-api-through-vercel-proxy.md)). Fly.io has no free tier for new accounts: the trial ended and the API went down. The web app is already on Vercel's free Hobby plan, and the database on Neon's free plan in Frankfurt ([ADR 0027](0027-postgres-on-neon-free-plan.md)). Vercel's free plan now runs Docker images as Functions (Container runtime, beta) and several services in one project (Services, beta), both on all plans. Koyeb closed its free tier in 2026, Render's free services take about a minute to wake, and Cloud Run and Northflank need a card.

## Decision
- **One Vercel project, two services** (`apps/web/vercel.json`): `web`, the static site built in GitHub Actions as before, and `api`, the server, built by Vercel from `apps/server/Dockerfile.vercel`. Top-level rewrites send `/v1/*`, `/healthz` and `/readyz` to `api` and everything else to `web`.
- **Same origin, no proxy:** the browser calls `/v1` on the web app's own address, so the refresh cookie is first-party, and the server is only reachable through Vercel. The shared `ORIGIN_SECRET` isn't used (the server still supports it, off by default).
- **Visitor IP:** the server trusts the header named by `CLIENT_IP_HEADER`, set to `x-real-ip`, which Vercel sets on every request and overwrites if a client sends it. Rate limiting keys on it.
- **One automatic deploy:** `.github/workflows/deploy.yml` runs after CI passes on `main`: it builds the site, checks that `DATABASE_URL` accepts writes, runs the migrations (`sqlx migrate run`), and deploys both services with the server's settings as runtime environment variables from the `production` GitHub environment. A manual run can make a preview deployment instead (no migrations).
- **Region:** the project's Functions region is Frankfurt (`fra1`), next to Neon.
- Fly.io's configuration and deploy workflow are removed. **Revisit** when live games need long-lived WebSockets (Functions have a maximum duration), or when a free limit is reached.

See [deployment](../architecture/deployment.md#the-server-on-vercel).

## Alternatives considered
- **Google Cloud Run:** generous free tier and no beta parts, but needs a card and a separate provider, and keeps the proxy.
- **A separate Vercel project for the API, behind the existing proxy:** Vercel overwrites forwarded IP headers between projects, so rate limiting would see Vercel's addresses.
- **Northflank or Render:** Northflank needs a card and doesn't publish its free limits; Render's free services sleep and take about a minute to wake.

## Consequences
- No paid host: the web app, the server and the database are all on free plans. Hobby's limits apply: 1M invocations, 4 hours of active CPU and 360 GB-hours of memory a month, and runtime logs kept for 1 hour.
- The server scales to zero after 5 idle minutes; the next request waits for a cold start. The in-memory rate limiter is per instance.
- Every merge to `main` deploys the server too, with its migrations, so migrations must stay backward compatible (expand, then contract).
- Both the Container runtime and Services are beta; a breaking change there would block deploys until fixed.
- Hobby is for non-commercial use: donations are allowed; affiliate links are only a problem if they become the site's main purpose.
