# MVP: Ouril on the web

> Scope and acceptance criteria for the first release. **Status:** Accepted. See [ADR 0025](../decisions/0025-web-first-static-mvp.md).

## Goal

Ship a polished, **offline-capable, single-player** Ouril game on the **web**, as a static site with no server, that faithfully implements the [Cape Verdean rules](../game/rules.md) and a few family variants.

## In scope

### 1. Play vs the computer
- New game against the computer: difficulty (Easy, Medium, Hard; see [AI](../architecture/ai.md)), who starts (me, the computer, random), and the **rules**:
  - **Standard** ([`cv.standard`](../game/variants/cape-verde/standard.md))
  - **Continuous sowing** ([`cv.continuous`](../game/variants/cape-verde/continuous.md))
  - **Capture across** ([`cv.across`](../game/variants/cape-verde/across.md))
  - **Capture across + continuous** ([`cv.across-continuous`](../game/variants/cape-verde/across-continuous.md))
- A **3D board** with a hand that sows, drops and carries captures to the store; keyboard and screen-reader play through hidden pit controls ([web design](../architecture/web-design.md), [ADR 0021](../decisions/0021-3d-web-board.md)).
- Legal moves highlighted; score pop and "+N" on captures; win celebration, calm loss screen.
- Undo, hints (optional), full screen, and the rules of the variant being played one tap away.
- The game is saved after every move and resumes from the home page. **Forfeit** from the game or the home page; starting a new game forfeits an unfinished one ([ADR 0022](../decisions/0022-forfeit-and-record-format-2.md)).

### 2. Learn
- An interactive tutorial: sowing, captures, chains, laps of 12+, the single-seed rule, feeding, grand slam, continuous sowing and capture across.
- A rules page with a tab per variant and collapsible sections.

### 3. The site
- A landing page with a live board, quick start, how to play, the islands' heritage, coming-soon features and recommended physical boards.
- An About page: the developer, social links, "Buy me a coffee" (PayPal, Venmo, Cash App, Zelle) and the board recommendations.
- Settings: sound, hints, language, game music (on/off, volume).
- Game music: a random playlist during games, with pause and skip.

### General
- **Web only**, as a static site. No sign-in, no server, no database; guests make **zero network requests** unless they opt into anonymous usage statistics, asked once on first launch ([ADR 0014](../decisions/0014-telemetry-consent.md), sent to self-hosted Matomo, [ADR 0026](../decisions/0026-usage-statistics-with-matomo.md)). Crash reports aren't offered yet.
- Works offline after the first visit (installable PWA), including the music.
- English UI with every string externalized and a test that forbids hard-coded text ([localization](features/localization.md)); the browser's language is used when the app supports it.
- Responsive for phones, tablets and desktops; reduced-motion support.

## Built but switched off (shipped after the MVP)

Sign-in (Google, Apple), sync, Stats and history, and replays: the code and the server exist and are tested, behind `VITE_FEATURE_ACCOUNTS` and `VITE_FEATURE_STATS` ([accounts](features/accounts.md), [backend](../architecture/backend.md)).

## Out of scope (see [roadmap](roadmap.md))

iOS and Android apps, accounts and sync, stats, online multiplayer, leaderboards, pass-and-play, other countries' variants, languages other than English, crash reports.

## Acceptance criteria

- [x] The Rust core passes its full test suite, including every worked example in [rules.md](../game/rules.md#worked-examples) and every variant's test vectors.
- [x] A full game against each AI level can be played to completion, with each of the four rule sets.
- [x] A guest session makes zero network requests without telemetry consent, and only usage-statistics hits with it (automated test).
- [x] An in-progress game survives a reload; a game whose rules changed is upgraded or set aside, never a blank page.
- [x] The game, including music, works with no network connection after the first visit.
- [x] The tutorial covers every rule and every variant.
- [x] No hard-coded text: every string is translatable (automated test).
- [ ] Deployed to the web on the production domain ([deployment](../architecture/deployment.md)).

## Open questions

- None blocking the launch.
