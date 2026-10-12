# 0029. The web app reaches the API through a /v1 proxy on Vercel

- **Status:** Accepted
- **Date:** 2026-10-12
- **Supersedes:** [0020](0020-web-and-api-on-one-site.md)

## Context
The web app is at `https://ouril.vercel.app` and the API at `https://ouril-server-prod.fly.dev` ([ADR 0019](0019-host-server-on-fly-io.md)); there is no custom domain. Both hosts are on the Public Suffix List, so they are different *sites*: browsers don't send the API's `SameSite=Lax` refresh cookie (`ouril_refresh`, [accounts](../product/features/accounts.md)) on the web app's requests, and Safari blocks it as a third-party cookie. Players would be signed out on every reload. [ADR 0020](0020-web-and-api-on-one-site.md) solved this with `api.<domain>` on a custom domain; it rejected a Vercel proxy because Fly would see Vercel's addresses instead of players', and the rate limiter keys on the client IP ([backend](../architecture/backend.md)).

## Decision
- **Proxy `/v1` through Vercel:** `vercel.json` forwards `https://ouril.vercel.app/v1/*` to `https://ouril-server-prod.fly.dev/v1/*`. The browser only talks to the web app's origin, so the refresh cookie is first-party and the CSP needs nothing new. The web build leaves `VITE_API_BASE_URL` empty (same origin).
- **Only the proxy may call `/v1`:** Vercel sets `x-ouril-origin-secret` to `ORIGIN_SECRET` (a Vercel environment variable) on every forwarded request. With `ORIGIN_SECRET` set, the server answers `/v1` requests without the matching header with 403 `forbidden` (constant-time comparison). `/healthz` and `/readyz` stay open for Fly's checks.
- **Rate limiting keys on the visitor:** for requests that passed the secret check, the server trusts `x-real-ip`, which Vercel sets to the visitor's IP. Otherwise it keeps using `Fly-Client-IP`, so nobody can choose their own key.
- `ALLOWED_ORIGINS` is `https://ouril.vercel.app`. Vercel doesn't cache API responses: the server sends `Cache-Control: no-store`, and `vercel.json` also turns rewrite caching off for `/v1`.
- **Revisit** with a custom domain (`api.<domain>`, ADR 0020's design), or when native apps ship. They can call the same `/v1` proxy.

See [deployment](../architecture/deployment.md#switching-accounts-on).

## Alternatives considered
- **A custom domain with `api.<domain>` (ADR 0020):** no proxy hop and no shared secret, but it needs a domain the project doesn't have yet.
- **Call `fly.dev` directly with `SameSite=None` or partitioned cookies:** blocked or unreliable across browsers (Safari blocks third-party cookies), so sessions wouldn't last.
- **Proxy without a secret, trusting `X-Forwarded-For`:** anyone could call `fly.dev` directly with a forged header and pick their own rate-limit key.

## Consequences
- Players stay signed in on `ouril.vercel.app` with no domain to buy.
- Every API call takes an extra hop through Vercel's network, and Vercel's limits for proxied requests apply.
- One more shared secret to keep in step in two places: Vercel's environment variables and Fly (copied from the GitHub secret by the server deploy). If they differ, every `/v1` request fails with 403. Rotating it: set the new value in both, then redeploy both.
- Vercel preview deployments have other origins and aren't in `ALLOWED_ORIGINS`, so sign-in only works on production.
