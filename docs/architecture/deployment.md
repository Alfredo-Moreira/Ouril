# Deployment

> How the web app and the server are built and deployed to one Vercel project, with the database on Neon, and what to set when accounts are switched on. **Status:** Accepted. See [ADR 0025](../decisions/0025-web-first-static-mvp.md), [ADR 0030](../decisions/0030-server-on-vercel-container-service.md) and [ADR 0027](../decisions/0027-postgres-on-neon-free-plan.md).

## Overview

One Vercel project serves everything at `https://ouril.vercel.app`, as two services ([ADR 0030](../decisions/0030-server-on-vercel-container-service.md)):

- **`web`:** the static site (`apps/web/dist`): HTML, JavaScript, the WASM rules engine, the 3D board, fonts, images and music. Guests need nothing else.
- **`api`:** the Rust server (`apps/server`) as a container, built by Vercel from `apps/server/Dockerfile.vercel`. It serves `/v1`, `/healthz` and `/readyz` on the same origin, and talks to Postgres on Neon in Frankfurt.

```mermaid
flowchart LR
  CI[GitHub Actions: CI] -->|green on main| D[deploy-web.yml]
  D -->|build WASM + site<br/>toolbox image| W[out/web]
  D -->|sqlx migrate run| N[(Neon Postgres<br/>Frankfurt)]
  D -->|Rust sources +<br/>Dockerfile.vercel| A[out/api]
  W & A -->|vercel deploy --prod| V[Vercel project]
  V -->|"/v1, /healthz, /readyz"| S[api service<br/>container, fra1]
  V -->|everything else| C[web service<br/>CDN]
  S --- N
```

- **Workflow:** [`.github/workflows/deploy-web.yml`](../../.github/workflows/deploy-web.yml) runs after CI passes on `main`, or by hand. It builds the WASM engine and the site with the same toolbox image as CI ([ADR 0017](../decisions/0017-local-first-development.md)), because Vercel's build image has no Rust toolchain. It checks that `DATABASE_URL` accepts writes, runs the migrations, assembles `out/` (`vercel.json`, `web/`, and `api/` with the Rust workspace and the Dockerfile), and uploads it with `vercel deploy --prod`. Vercel then builds the server image. A failed CI run never deploys.
- **`apps/web/vercel.json`** defines the two services and the routing. It turns off Vercel's own Git deployments (`git.deploymentEnabled: false`), because a build from the repository on Vercel would fail on `ouril-wasm`. The `web` service carries the SPA routing (every path except assets, icons, audio and the service worker serves `index.html`) and the security headers (HSTS, CSP with `connect-src` limited to the site and the Matomo host, etc.).
- **Server settings** go to the deployment as runtime environment variables from the `production` GitHub environment (`vercel deploy --env`), so nothing needs setting in Vercel's dashboard: `DATABASE_URL`, `JWT_SECRET`, `GOOGLE_CLIENT_ID`, `ALLOWED_ORIGINS` (`https://ouril.vercel.app`, or the variable of that name), `CLIENT_IP_HEADER=x-real-ip`, `PORT=8080` and JSON logs.
- **Web build variables:** usage statistics ([ADR 0026](../decisions/0026-usage-statistics-with-matomo.md)), built with `VITE_MATOMO_URL=https://analytics.moreiralabs.org/` and `VITE_MATOMO_SITE_ID=1` (repository variables of the same name override them; the CSP's `connect-src` must allow the host), and the accounts flags below. Local builds set none of these, so development sends nothing.
- **Offline:** the service worker precaches the app (about 2 MB) and downloads the game music into its own cache while the browser is idle ([web design](web-design.md)).
- **Only production is deployed:** the local Docker stack is the staging environment ([ADR 0028](../decisions/0028-production-only-local-staging.md)). Before merging, check the change locally: `just dev`, `just test`, and `just build-server-image` for the production image. A manual run of *Deploy* with **preview** makes a deployment on its own URL without touching production (no migrations; sign-in doesn't work there, because its origin isn't allowed).

## Releasing

Merge to `main`. CI runs; when it's green, `deploy-web.yml` migrates the database and deploys that exact commit, web and server together. Players get the new site on their next visit (the service worker updates itself, `registerType: autoUpdate`). After a deploy that changes the server, open `https://ouril.vercel.app/readyz`: it should say `ok`.

## The server on Vercel

- **Image:** `apps/server/Dockerfile.vercel`, the same release build as `apps/server/Dockerfile` without BuildKit cache mounts. It never enables `dev-auth` (CI's release-image job checks the same build).
- **Scaling:** Vercel runs the container as a Function. It scales to zero after 5 idle minutes, and the next request waits for it to start. The in-memory rate limiter is per instance, and keys on `x-real-ip`, which Vercel sets on every request ([backend](backend.md)).
- **Region:** the project's Functions region must be Frankfurt (`fra1`), next to Neon (one-time setup below).
- **Migrations** are forward-only SQL in `apps/server/migrations` ([backend](backend.md#migrations)). `deploy-web.yml` runs `sqlx migrate run` before deploying; a failure stops the deploy and the running version keeps serving. Because every merge deploys the server, a migration must work with the code that's already running (expand, then contract).
- **Health:** `/healthz` (liveness) and `/readyz` (the database answers within 2 s, else 503) are routed to the server for manual checks. Vercel has no health checks of its own, so nothing keeps the database awake.
- **Logs:** the server's JSON logs are in the Vercel dashboard (project → Logs); the free plan keeps them for 1 hour.
- **Database connection:** `DATABASE_URL` is Neon's **direct** connection string for the **primary** compute (host without `-pooler`), with `sslmode=require`. The project has a read replica that the server doesn't use ([ADR 0027](../decisions/0027-postgres-on-neon-free-plan.md)): sessions are checked on every request and sync pull follows a push, so replica lag would show stale data. `deploy-web.yml` (and `ouril-server migrate`) refuse a read-only database, so a replica's connection string stops the deploy. Neon's pooled URL runs PgBouncer in transaction mode, which breaks sqlx's named prepared statements and the migrator's advisory lock. The server closes connections idle for 4 minutes, so Neon can suspend the compute after its 5 idle minutes; the first query after that waits a few hundred milliseconds while it wakes.
- **Free-plan limits:** Vercel Hobby includes 1M function invocations, 4 hours of active CPU and 360 GB-hours of memory a month, and is for non-commercial use (donations are fine). Neon's project gets 0.5 GB of storage and 100 compute-hours a month; when it runs out of compute-hours, Neon suspends it until the next month and the API fails. Check both dashboards now and then.

## One-time setup

Everything happens in the Vercel, Neon and GitHub websites.

1. **Vercel:** the project for the repository (Framework **Other**). Under **Settings → Functions**, set the **Function Region** to **Frankfurt, Germany (fra1)**. Add the repository secrets **`VERCEL_TOKEN`**, **`VERCEL_ORG_ID`** and **`VERCEL_PROJECT_ID`** (GitHub → Settings → Secrets and variables → Actions). Until `VERCEL_TOKEN` exists, the workflow skips the deploy.
2. **Neon** (free account at neon.com): a project in **AWS Europe Central 1 (Frankfurt)**. Open **Connect**, keep the default branch, choose the **primary** compute (not the read replica), turn **Connection pooling off** and copy the connection string (the host has no `-pooler`).
3. **GitHub:** Settings → Environments → `production` (used by `deploy-web.yml`), with the secrets:
   - `DATABASE_URL`: the Neon connection string (paste it straight into GitHub; it contains the database password);
   - `JWT_SECRET`: a long random value, for example from `openssl rand -base64 48`. Keep it stable: changing it signs every player out;
   - `GOOGLE_CLIENT_ID`: the Google OAuth client ID ([accounts](../product/features/accounts.md#setting-up-the-google-client)).

   **Required reviewers** on that environment would gate every deploy.
4. **First deploy:** Actions → *Deploy* → *Run workflow*. Try **preview** first, then **production**. Open `/readyz` on the deployment's URL: it should say `ok`, and `/v1/meta` should return JSON.
5. In Matomo (site 1), keep the privacy settings ADR 0026 relies on: **Anonymize IP addresses** (at least 2 bytes, also used for location), **no location from IP** (or the anonymized IP only), and a data-retention period. Add the production address to the site's URLs.

To change the database, update `DATABASE_URL` in the GitHub environment; the next deploy uses it.

### Browse the production database

`docker/pgadmin/compose.neon.yaml` runs pgAdmin for the Neon database as its own Compose project, independent of the local stack (which has its own pgAdmin for the local database, `just pgadmin` on port 5050).

1. Copy `docker/pgadmin/.env.example` to `docker/pgadmin/.env` (gitignored). Set a pgAdmin login (`PGADMIN_DEFAULT_EMAIL`, `PGADMIN_DEFAULT_PASSWORD`), and `NEON_HOST`, `NEON_USER` and `NEON_DATABASE` from Neon's **Connect** dialog with the **primary** compute selected.
2. Start it with `just pgadmin-neon` (or `docker compose -f docker/pgadmin/compose.neon.yaml up -d`), open <http://localhost:5051> and sign in. The server "Ouril production (Neon)" is already listed. Enter the Neon password on first connect; pgAdmin can save it in its volume.
3. **Stop it when you're done** (`just pgadmin-neon-down`). While pgAdmin is connected, its dashboards query the database every few seconds, which keeps the Neon compute awake and spends the free plan's compute-hours.

It listens on `127.0.0.1` only and asks for its own login, because it can change production data. The Neon password never goes in a file. The server list is loaded on the first start only: after changing `NEON_*`, reset with `docker compose -f docker/pgadmin/compose.neon.yaml down -v` (that also forgets the saved password). For a quick query, Neon's console **SQL Editor** needs no setup at all.

## Switching accounts on

Turning on accounts and sync ([ADR 0009](../decisions/0009-oauth-accounts-google-apple.md)) shows sign-in in the web app. The browser calls `/v1` on the site's own address, so the refresh cookie is first-party ([ADR 0030](../decisions/0030-server-on-vercel-container-service.md)). That release:

- sets the GitHub variables `VITE_FEATURE_ACCOUNTS=true` (and `VITE_FEATURE_STATS=true`). `deploy-web.yml` builds with them, plus `VITE_GOOGLE_CLIENT_ID` from the `GOOGLE_CLIENT_ID` secret. Vercel's own environment variables aren't used for the web build, which happens in GitHub Actions. `VITE_API_BASE_URL` stays empty, because the API is same-origin;
- adds `https://<site>/auth/google` to the Google client's redirect URIs ([accounts](../product/features/accounts.md#setting-up-the-google-client));
- keeps `ALLOWED_ORIGINS` (default `https://ouril.vercel.app`) in step with the site's address, and sets the Apple keys later;
- ✅ publishes the privacy policy (`/privacy`) and the account-deletion page (`/delete-account`) ([accounts](../product/features/accounts.md#account-deletion-and-privacy));
- moves the Google app from **Testing** to **In production** in Google Cloud (Google Auth Platform → Audience → Publish app), after adding the home page `https://ouril.vercel.app` and the privacy policy `https://ouril.vercel.app/privacy` under **Branding**. Until then only listed test users can sign in.

## Open questions

- A custom domain (the site works on `ouril.vercel.app`).
- Scheduled `pg_dump` backups to object storage: where they go, and when the first restore test happens.
