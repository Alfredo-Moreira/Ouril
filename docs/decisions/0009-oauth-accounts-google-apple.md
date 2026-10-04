# 0009. Optional OAuth accounts: Google and Apple in the MVP

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
The MVP is single-player, but online play against real people (real-time and async) follows. Introducing accounts early makes that transition smooth: players already have an identity, their stats are backed up, and the auth part of the server is proven before multiplayer launches.

App Store guideline 4.8 requires apps that offer a third-party login (such as Google) to also offer an equivalent privacy-focused option. In practice, that means **Sign in with Apple**.

## Decision
- The MVP supports **Sign in with Google** (all platforms) and **Sign in with Apple** (iOS and web, optionally Android).
- **Signing in is optional.** Guests play offline vs AI with local stats. Signing in backs up stats and prepares for online play.
- **No passwords.** Only OAuth / OpenID Connect providers.
- The server verifies provider ID tokens and issues **its own sessions** (short-lived access token + rotating refresh token).
- Users are our own records. Providers are linked **identities** `(provider, subject)`, and email is optional.
- **Future providers:** **X (Twitter)** and **Meta (Facebook)**. They need no model changes, just one more identity type.
- In-app **account deletion** ships with the MVP (required by Apple and Google Play).

See [accounts.md](../product/features/accounts.md).

## Alternatives considered
- **Google only:** likely App Store rejection under guideline 4.8.
- **Required sign-in:** adds friction, and breaks offline first launch.
- **Email + password:** more to secure (password storage, resets), and a worse experience on mobile.
- **Managed auth** (Firebase Auth, Auth0, Supabase Auth): faster to set up, but adds a vendor dependency and cost for what is a small amount of code. We can revisit this if we need many providers.

## Consequences
- The MVP needs a small backend (Rust server + Postgres) and a privacy policy.
- Apple-specific work: storing the name on first sign-in, handling relay emails, revoking tokens on account deletion, and handling server-to-server notifications.
- Adding X and Meta later only touches auth endpoints and the apps' sign-in screens.
