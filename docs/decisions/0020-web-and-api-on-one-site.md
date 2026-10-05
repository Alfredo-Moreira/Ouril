# 0020. Web on Vercel, API on a same-site subdomain on Fly.io

- **Status:** Proposed
- **Date:** 2026-10-04

## Context
The web app is a static Vite build (Vercel is the hosting candidate, [overview](../architecture/overview.md#open-questions)). The Rust server runs on Fly.io ([ADR 0019](0019-host-server-on-fly-io.md)). Web sign-in keeps the refresh token in an httpOnly `SameSite=Lax` cookie scoped to `/v1/auth` ([accounts](../product/features/accounts.md)). If the web app and the API are on **different sites**, browsers don't send that cookie, refresh fails, and every web session ends after the 15-minute access token. Rate limiting also needs each player's real IP address ([backend](../architecture/backend.md)).

## Decision
- Serve the web app from the main domain on **Vercel** (for example `ouril.example`), and the API from **`api.<same domain>`** on Fly.io (a custom domain on the Fly app). Both are the same *site*, so the `SameSite=Lax` refresh cookie is sent on API calls.
- The web build sets `VITE_API_BASE_URL=https://api.<domain>`. The client then calls the API cross-origin with `credentials: 'include'`. Without the variable it stays same-origin, as in local development behind the Vite proxy.
- The server's `ALLOWED_ORIGINS` lists the exact web origin(s) (`https://<domain>`, plus the staging origin), which CORS with credentials requires.
- Vercel serves `apps/web/vercel.json`: SPA routing, a strict Content Security Policy (`connect-src` includes the API origin), HSTS and other security headers.
- Each environment gets its own pair, for example `staging.<domain>` with `api-staging.<domain>`.

## Alternatives considered
- **Proxy `/v1` through Vercel (one origin):** simplest for cookies. But Fly would see Vercel's addresses instead of players' IPs, which breaks per-player rate limiting unless a trusted forwarding header is added. It's also an extra network hop on every API call.
- **API on a different domain (for example `ouril-server-prod.fly.dev`):** the cookie is cross-site and gets blocked, so sessions break.
- **Serve the web app from the Rust server:** one origin, but mixes static hosting into the API and loses Vercel's CDN and preview deployments.

## Consequences
- Needs a custom domain with DNS for both hosts, and TLS on each (Vercel and Fly both provide it).
- The CSP `connect-src` and `ALLOWED_ORIGINS` must name the real hosts. Both are placeholders until the domain is chosen ([human review](../review/human-review.md)).
- Players' real IPs reach Fly directly, so rate limiting keeps working.
