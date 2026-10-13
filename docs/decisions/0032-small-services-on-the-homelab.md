# 0032. Small, optional services run on the developer's homelab

- **Status:** Proposed
- **Date:** 2026-10-13

## Context

The web app and the server run on Vercel's free plan, with Postgres on Neon's free plan ([ADR 0030](0030-server-on-vercel-container-service.md), [ADR 0027](0027-postgres-on-neon-free-plan.md)). Two upcoming needs don't fit there. Live games need long-lived WebSockets, but a Vercel Function closes every connection after 300 s on Hobby, and instances share no memory ([multiplayer](../product/features/multiplayer.md)). Leaderboards and other housekeeping need scheduled jobs, but Hobby's cron runs once a day at most. The developer already runs a homelab that serves Ouril's usage statistics ([ADR 0026](0026-usage-statistics-with-matomo.md)): a UGREEN DH4300 Plus NAS (arm64, 8 GB RAM) running Docker behind an outbound-only Cloudflare Tunnel and the Zoraxy reverse proxy, deployed from its own repository (`home-lab`).

## Decision

- **Small services run on the homelab:** first the live-game realtime relay ([ADR 0033](0033-live-games-http-moves-and-relay.md)), a small Rust service in this repository (`apps/relay`, `ouril-relay`) at **`ouril-live.moreiralabs.org`**; then scheduled jobs (leaderboard snapshots, clean-ups) that call authenticated server endpoints.
- **Secure by default, because it's a home network:** each service exposes only what it needs through Cloudflare and Zoraxy, verifies a secret or a signed token on every request, has hard limits on connections and message sizes, and runs as a non-root, read-only container with no capabilities and capped memory. The relay accepts no messages from players except their token and heartbeats.
- **They're never the source of truth, and they fail safe.** Players, games, ratings and every write stay in the Vercel server and Neon. A homelab service holds no personal data beyond what's in flight. If the box is down, features slow down (live games fall back to polling, a job runs later) but nothing breaks or is lost.
- **Build and deploy the way the `home-lab` repository does:**
  - multi-arch images (`linux/amd64`, `linux/arm64`) built on the developer's machine by `deploy/homelab/build-and-push.sh`, tagged `vX.Y.Z` and `latest`, pushed to the private LAN registry;
  - deployed by pasting `deploy/homelab/docker-compose.homelab.yml` into the UGOS Docker UI. The NAS never builds, and no files are copied to it;
  - exposed only through Zoraxy and a Cloudflare Tunnel public hostname: no published ports;
  - every container read-only, with all capabilities dropped and `no-new-privileges`, on the `internal_public_apps` network (plus a private `internal` network when the stack needs one).
- CI checks the compose file and builds the images, but **never pushes**: the registry is only reachable on the home network.

See [deployment](../architecture/deployment.md#the-homelab).

## Alternatives considered

- **Everything on Vercel:** its WebSockets reconnect every 5 minutes and need Redis between instances; daily cron only.
- **Centrifugo instead of our own relay:** mature and feature-rich, but a much larger surface (admin UI, many transports, dynamic JWKS, with recent JWT and SSRF advisories) than a relay that only fans out messages.
- **A managed realtime service (Ably, Pusher):** free tiers cap simultaneous connections (Ably 200, Pusher 100) and add a sub-processor. It stays the fallback option if the homelab becomes a problem.
- **Moving the whole server home:** one box and a home connection would take down sign-in, sync and turn-by-turn games with every outage, and player data would leave Neon in the EU.
- **A paid always-on host (about $5–10 a month):** works, but costs money for what the homelab already provides.

## Consequences

- Live games and frequent jobs cost nothing extra, with no connection caps.
- The homelab becomes part of production. It depends on home power, the ISP and NAS updates, so every client path needs its fallback, and health checks belong on the homelab dashboard.
- Homelab deploys are manual, separate from the automatic Vercel deploy: server changes must stay compatible with the running homelab services.
- Images are arm64 as well as amd64, and carry their `.env` like `home-lab`'s, so they must never go to a public registry.
- The privacy policy must say the relay (operated by the developer) carries live-game messages.
