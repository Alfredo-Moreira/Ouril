# 0026. Usage statistics with self-hosted Matomo

- **Status:** Proposed
- **Date:** 2026-10-05

## Context
[ADR 0014](0014-telemetry-consent.md) allows anonymous usage statistics only after the player opts in, but left the analytics tool open ("chosen with privacy in mind"). The MVP is a static web app with no server of its own ([ADR 0025](0025-web-first-static-mvp.md)), so the tool must accept hits straight from the browser, under a strict CSP, without cookies, advertising IDs or a third-party script.

## Decision
- Use the developer's **self-hosted Matomo** (`https://analytics.moreiralabs.org/`, site ID 1) for web usage statistics. The build reads `VITE_MATOMO_URL` and `VITE_MATOMO_SITE_ID`; with either empty, usage statistics are a no-op and the option isn't offered.
- The web app sends hits with its own small client for Matomo's HTTP tracking API (`matomo.php`), **not `matomo.js`**: no external script, no cookies, no referrer, no screen size. Only the CSP's `connect-src` gains the Matomo host.
- Only after usage-statistics consent: a screen view per route (IDs stripped) and events such as `game_finished` (level, outcome, plies). The visitor ID comes from the random install ID, so "Reset anonymous ID" starts a new visitor.
- Hits queue in `localStorage` (capped at 200), are sent in bulk when online, and are dropped after 23 hours (Matomo can't backdate further without a token). Withdrawing consent discards the queue.
- The Matomo instance anonymizes IP addresses and turns off location from IP, to keep ADR 0014's "no IP addresses stored, no location".

See [web design](../architecture/web-design.md) and [deployment](../architecture/deployment.md).

## Alternatives considered
- **Self-hosted PostHog:** richer product analytics, but heavier to run and its browser SDK is a large third-party script.
- **Our own endpoint:** full control, but needs the server the MVP doesn't have.
- **`matomo.js` tracker:** less code, but loads an external script (CSP `script-src` exception) and expects cookies or fingerprint-based visit recognition.

## Consequences
- Production players who opt in make requests to `analytics.moreiralabs.org`; those who don't still make none. The guest network test covers both.
- The Matomo server's privacy settings are part of the promise and must be kept (IP anonymization, no geolocation, data retention).
- The iOS and Android apps can use the same HTTP API when they're built.
- The privacy policy must name Matomo and what it receives.
