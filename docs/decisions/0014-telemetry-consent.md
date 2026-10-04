# 0014. Telemetry only with consent, asked on first launch

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
Guests play fully offline and, by design, make no network requests ([ADR 0011](0011-offline-first-sync.md)). But guests will be most MVP players, and without crash reports we'd be blind to most crashes across three native apps. We want useful diagnostics without compromising privacy or the offline-first promise.

## Decision
- On **first launch**, the app shows a short, plain-language prompt with two separate choices:
  - **Crash reports:** anonymous crash and error reports.
  - **Usage statistics:** anonymous, aggregate events (for example "game finished, variant, AI level, duration"). No content, no location, no advertising IDs.
- **Both are off until the player accepts.** Declining is one tap and as prominent as accepting. Players can change their choices in Settings at any time.
- **Without consent, guests still make zero network requests.** With consent, the only traffic is telemetry. Gameplay never waits on it.
- Telemetry uses a **random install ID** that can be reset in Settings. It's **not linked to the account**, even for signed-in players. No IP addresses are stored.
- Events are queued locally and sent when online, using the same offline-first approach as sync. The queue is capped and old events are dropped.
- Tooling: **Sentry** (SDKs for Rust, Swift, Kotlin and JS; can be self-hosted) for crashes. The analytics tool is to be chosen with privacy in mind (for example self-hosted PostHog, or our own endpoint).
- The privacy policy and the store privacy questionnaires describe exactly this.

## Alternatives considered
- **Always-on anonymous crash reports:** more data, but breaks the "guests send nothing" promise without asking.
- **Strict (no telemetry for guests):** most private, but we'd be blind to most crashes in the MVP.
- **Third-party analytics with tracking** (ad SDKs): not acceptable. It would also require App Tracking Transparency prompts on iOS.

## Consequences
- Some players will decline, so we'll see a sample of crashes, not all of them.
- One more first-run screen, which needs careful UX and translation.
- The "guests make no network requests" rule becomes "**…unless they opted into telemetry**" everywhere it's stated.
