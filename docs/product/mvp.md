# MVP: standalone Ouril

> Scope and acceptance criteria for the first release. **Status:** Draft

## Goal

Ship a polished, **offline-capable, single-player** Ouril game on **web, iOS and Android** that faithfully implements the [Cape Verdean rules](../game/rules.md).

## In scope

### 1. Play vs AI
- New game against the computer, with the player choosing who starts (or random).
- **Difficulty levels:** Easy, Medium, Hard (see [AI](../architecture/ai.md)).
- Animated sowing and captures, with each move visible step by step.
- Legal-move highlighting. Illegal pits can't be tapped.
- Score (seeds captured) shown for both sides.
- Undo the last move (vs AI only; may be disabled on Hard).
- Resume an unfinished game after closing the app.
- End-of-game screen: result, score, and "play again".

### 2. Tutorial and rules
- An interactive tutorial that walks through sowing, capturing, chain captures, laps of 12+ seeds, the grand slam, and feeding.
- A rules reference screen that is always available from the menu.
- Optional move hints for beginners.

### 3. Local stats and history
- Games played, wins, losses and draws, per difficulty.
- Current and best win streak.
- Recent games list showing date, opponent difficulty, result and score.
- *Nice to have:* replay a past game move by move. The engine stores games as move lists, so this is cheap to add.
- Stored on the device. Guests need no account.

### 4. Optional account (sign-in)
See [accounts](features/accounts.md).
- **Sign in with Google** (all platforms) and **Sign in with Apple** (iOS and web).
- Optional: **guests never need a network connection** and can use every MVP feature. Signing in backs up history and stats and prepares the player for online play.
- **Offline-first sync** for signed-in players: everything is saved locally and synced when the network is available ([data and sync](../architecture/data-and-sync.md)). Guest data is claimed and synced on sign-in.
- Profile: display name, unique handle, avatar.
- Sign out, and **delete account** in the app (required by both stores).
- Needs the first version of the server (`apps/server`: auth, profile, sync) plus Postgres.

### General
- **Build order:** core → web → iOS → Android. The MVP **launches on all three together**, and the web build is used for internal playtesting first ([ADR 0018](../decisions/0018-platform-build-order.md)).
- **Telemetry consent** on first launch: anonymous crash reports and usage statistics, each off until accepted and changeable in Settings ([ADR 0014](../decisions/0014-telemetry-consent.md)).
- English UI, with all strings externalized for later [localization](features/localization.md).
- Sound effects with a mute toggle.
- Responsive layout for phones, tablets and desktop browsers.

## Out of scope (see [roadmap](roadmap.md))

Pass-and-play, other variants, online multiplayer, leaderboards, X/Meta sign-in, linking several providers to one account, languages other than English, monetization.

## Acceptance criteria

- [ ] The Rust core passes the full test suite, including every example in [rules.md](../game/rules.md#worked-examples), and every open rule question is resolved.
- [ ] The shared test vectors pass through the Swift, Kotlin and WASM bindings in CI.
- [ ] A full game vs each AI level can be played to completion on iOS, Android and web.
- [ ] Sign in with Google and with Apple works on each supported platform. Guest stats are merged on sign-in, and account deletion removes the account and revokes the Apple token.
- [ ] The game stays fully playable when the server is unreachable. A guest session makes zero network requests unless the player opted into telemetry ([ADR 0014](../decisions/0014-telemetry-consent.md)).
- [ ] Games played offline while signed in sync automatically when the network returns, with no duplicates after retries. Two devices on the same account converge to the same history and stats.
- [ ] A privacy policy and a web page for account deletion requests are published.
- [ ] The AI never makes an illegal move. Hard responds in under 1 second on a mid-range phone.
- [ ] The tutorial can be completed by a first-time player without outside help.
- [ ] Stats and an in-progress game survive an app restart.
- [ ] The game works with no network connection after the first load or install.
- [ ] It's published to the App Store and Google Play, and deployed to the web.

## Open questions

- Should Hard allow undo?
- Is a game timer useful in single-player mode?
- Web: should the game be installable as a PWA in the MVP?
