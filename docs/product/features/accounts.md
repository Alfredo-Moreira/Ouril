# Accounts and sign-in

> How players create accounts and sign in. **Status:** Accepted. See [ADR 0009](../../decisions/0009-oauth-accounts-google-apple.md).

## Why accounts before multiplayer

> **Timing:** accounts were planned for the MVP; they're built but switched off, and ship after the web MVP ([ADR 0025](../../decisions/0025-web-first-static-mvp.md)). The design below is unchanged.

The MVP is a single-player game, but online play (real-time and async against other people) is next. Shipping accounts before multiplayer means:

- players already have an identity when multiplayer launches, so there's no migration and no "create an account now" wall
- stats and history are backed up and follow the player across devices
- the auth part of the server is built and battle-tested early

## Principles

- **Signing in is optional, and guests never need the network.** Anyone can play vs AI right away, offline, as a **guest**. A guest makes no server calls at all, and no network requests unless they opted into telemetry ([ADR 0014](../../decisions/0014-telemetry-consent.md)). Signing in is offered (never forced) from the menu, the stats screen, and later when trying to play online.
- **No passwords.** We only use OAuth / OpenID Connect providers.
- **We own the account.** A provider login is an *identity linked* to our own user ID, so players can add or change providers without losing their account.
- **Collect the minimum:** provider subject ID, display name, and email if given. Never contacts or friends lists.

## Providers

| Provider | When | Platforms | Notes |
|---|---|---|---|
| **Google** | MVP | iOS, Android, Web | Android uses Credential Manager ("Sign in with Google"). iOS uses the Google Sign-In SDK. Web uses an OpenID Connect redirect ([below](#google-on-the-web)). |
| **Apple** | MVP | iOS, Web (Android optional) | **Required on iOS** by App Store guideline 4.8 when a third-party login is offered. iOS uses AuthenticationServices; web uses Sign in with Apple JS. |
| **X (Twitter)** | Future | All | OAuth 2.0 with PKCE. May not return an email, so accounts must not depend on email. |
| **Meta (Facebook)** | Future | All | Facebook Login. On iOS, Apple sign-in already satisfies guideline 4.8. |

## Sign-in flow

```mermaid
sequenceDiagram
  actor P as Player
  participant App as App (iOS / Android / Web)
  participant IdP as Google / Apple
  participant S as Ouril server
  participant DB as Postgres

  P->>App: Tap "Sign in with Google/Apple"
  App->>IdP: Native sign-in (with nonce)
  IdP-->>App: ID token (signed JWT)
  App->>S: POST /v1/auth/{provider} {id_token, nonce}
  S->>IdP: Fetch public keys (JWKS, cached)
  S->>S: Verify signature, issuer, audience, expiry, nonce
  S->>DB: Find identity (provider, subject) or create user + identity
  S-->>App: access token (short-lived) + refresh token
  App->>S: POST /v1/sync/push (guest games + settings, claimed by the account)
  App->>S: GET /v1/sync/pull (data from the account's other devices)
  S-->>App: changes + cursor
```

- The **server** verifies every ID token. The app never decides who the user is.
- Our session = a short-lived **access token** (JWT, ~15 minutes) + a **refresh token** that is rotated on each use and stored hashed on the server.
- Tokens are stored in the **Keychain** (iOS), **Android Keystore**-backed storage, and an **httpOnly secure cookie** on the web.
- Signed-in players still play offline. Stats sync the next time the device is online.
- **Local development:** debug builds use a development identity provider (`POST /v1/auth/dev`) to sign in as seeded test users without Google or Apple. It's never compiled into release builds ([ADR 0017](../../decisions/0017-local-first-development.md)).

## Google on the web

The web app signs in with Google through a plain **OpenID Connect redirect**, not the Google Identity Services script (`apps/web/src/auth/google.ts`):

1. Tapping **Sign in with Google** stores a random `state` and `nonce` in `sessionStorage` and navigates to `accounts.google.com` with `response_type=id_token`.
2. Google sends the player back to `/auth/google` with the ID token in the URL fragment, which never reaches a server or the usage statistics. The app checks `state` (and that the sign-in started less than 10 minutes ago), removes the fragment from the address bar and history, and posts the token and the nonce to `POST /v1/auth/google`. The server verifies the token as for every platform.

Why not the Google script: it would load on the sign-in screen, before the player chooses to sign in, and guests make no network requests ([ADR 0014](../../decisions/0014-telemetry-consent.md)). It would also need Google's hosts in the CSP. With the redirect, nothing leaves the device until the player taps the button, and the CSP is unchanged.

The button is enabled when the build has `VITE_GOOGLE_CLIENT_ID`; the server needs `GOOGLE_CLIENT_ID` (the same value). Without them the button stays a disabled placeholder and the server answers `501 not_configured`.

### Setting up the Google client

In the [Google Cloud console](https://console.cloud.google.com/), in a project for Ouril, open **Google Auth Platform**:

1. **Branding:** app name (Ouril), a support email, and a developer contact email. Before publishing, also the home page and privacy policy links.
2. **Audience:** user type **External**. While the app is in **Testing**, only the test users listed there can sign in; add yourself. Publish it to **In production** before launch. The app asks only for `openid`, `email` and `profile`, which are non-sensitive scopes.
3. **Clients → Create client → Web application.** Under **Authorized redirect URIs** add `http://localhost:5173/auth/google` for local development, and later `https://<domain>/auth/google`. The URI must match exactly, so open the local app on `localhost`, not `127.0.0.1`. **Authorized JavaScript origins** aren't needed for the redirect.
4. Copy the **client ID**. The client secret isn't used.

To try it locally, set `GOOGLE_CLIENT_ID` and `VITE_GOOGLE_CLIENT_ID` to the client ID and `VITE_FEATURE_ACCOUNTS=true` in `.env`, then run `just dev` and open <http://localhost:5173/sign-in>. For production, see [deployment](../../architecture/deployment.md#switching-accounts-on).

## Guest → account

- On first launch the app creates a local **guest profile**: a random ID. A guest's finished games are stored on the device, but **Stats, history and replays are for signed-in players**: guests don't see them, and `/stats` asks them to sign in. The paused game, settings and the telemetry choice also stay on the device.
- When the guest signs in, their local game records are claimed by the account and synced. If the account already has data from another device, the game histories are **combined** (by game ID), and stats are recomputed from the combined history. No counters are merged, so nothing is double-counted.
- Signing out keeps the local data on the device but stops syncing.
- Full details: [data and sync](../../architecture/data-and-sync.md).

## Profile

- **Display name:** pre-filled from the provider, editable.
- **Handle** (unique, for example `@mindelo_master`): chosen at sign-up, needed later for friend invites and challenges.
- **Avatar:** from the provider or a default, changeable later.
- **Locale and country:** suggested from the device. Country is optional and will be used later for leaderboards.

## Account deletion and privacy

- **In-app account deletion** is required by Apple (guideline 5.1.1(v)) and Google Play: Settings → Delete account. Google Play also requires a **web page** where users can request deletion: [`/delete-account`](https://ouril.vercel.app/delete-account) (`apps/web/src/routes/DeleteAccount.tsx`) explains the in-app steps, the email route for players who can't sign in, and what's deleted or kept.
- Deletion removes personal data, revokes all sessions, and **revokes the Sign in with Apple token** through Apple's REST API. Past online games keep an anonymized "Deleted player" entry so opponents' history stays intact.
- How revocation works: at Apple sign-in the app also sends the `authorization_code`; the server exchanges it for Apple's refresh token (client secret signed with the `.p8` key), keeps it only for this, and revokes it on account deletion. Apple sign-in stays off if the key can't be loaded, so no Apple account is ever created that couldn't be revoked.
- The **privacy policy** is at [`/privacy`](https://ouril.vercel.app/privacy), part of the app so it works offline (`apps/web/src/routes/Privacy.tsx`, text in `privacy.*` strings, facts in `apps/web/src/content/privacy.ts`). The controller is the developer, Alfredo Ramos Moreira; requests go to `devops.moreira@gmail.com`. It must match what the app and server do: update it, and its date, in the same change as anything that alters what data is collected, where it goes, or how long it's kept. It's a draft written from the code, not legal advice; have it reviewed when possible. Settings, About and the sign-in screen link to it.
- Apple only sends the user's name on the **first** sign-in, so it must be saved then.
- Apple users may hide their email behind a relay address. We treat email as optional everywhere.

## Edge cases

- **Same person, two providers** (signed up with Google, later taps Apple): this creates a separate account unless they link providers from settings. Linking is post-MVP; automatic merging by email is **not** done because it isn't safe.
- **Provider account removed or revoked:** sessions stay valid until expiry. Apple sends server-to-server notifications for revocations, which we handle.
- **Server down:** sign-in fails gracefully. The game stays fully playable as a guest or a signed-in offline player.

## Open questions

- Handle rules: length, allowed characters, reserved words, and how often it can be changed.
- Offer Apple sign-in on Android too, or only on iOS and web?
- Minimum age and parental consent (COPPA/GDPR-K): the privacy policy says signing in is for players aged 13 or older (16 where local law requires) and younger players play as guests. Confirm this, and whether the sign-in screen should ask.
