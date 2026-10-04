# Vision

> What Ouril is, who it's for, and what guides our decisions. **Status:** Draft

## The idea

Ouril is a Cape Verdean seed-sowing game, part of the Oware family that's played across West Africa, the Caribbean and the Cape Verdean diaspora. You need a board with two rows of six pits and 48 seeds. A game takes only a few minutes to learn, but it offers a lifetime of strategy.

We want to make the best digital home for Ouril, and later for every Oware variant. Players should be able to:

1. **Play anywhere**: native apps on iPhone and Android, plus the browser, all using one shared rules engine so the game behaves the same everywhere.
2. **Learn easily**: an interactive tutorial teaches newcomers, and experienced players find the rules they grew up with.
3. **Play each other** (post-MVP): live matches and turn-by-turn ("correspondence") games with friends or strangers.
4. **Compete** (post-MVP): skill ratings and leaderboards at global, country and city level, so players can be the best in Praia, in Cape Verde or in the world.

## Audience

- **Cape Verdeans and the diaspora** who grew up playing Ouril and want to play with family and friends anywhere in the world.
- **Oware players** from Ghana, Côte d'Ivoire, Nigeria, the Caribbean and elsewhere who play their own variant.
- **Curious newcomers** who like abstract strategy games such as chess, checkers or Go.

## Goals

- A faithful, correct implementation of Cape Verdean Ouril first, then other variants.
- A polished, tactile feel: sowing seeds should feel satisfying.
- Fair, cheat-resistant online play.
- Respect for the game's cultural roots in its language, art and naming.
- Works offline for single-player games.

## Principles

- **One rules engine, many clients.** The game logic is written once, tested thoroughly, and shared by web, mobile and server.
- **Rules are data.** Each variant is a configuration of a common engine, not a fork of the code.
- **Standalone first.** Each phase ships something playable before the next adds online features.
- **Privacy by default.** Location for regional leaderboards is opt-in and coarse (country or city, never precise coordinates).
- **English first, multilingual by design.** UI text is externalized from day one, even before we add other languages.

## Non-goals (for now)

- Monetization: no ads, purchases or subscriptions. We'll revisit this after launch.
- Real-money play or betting.
- Mancala games outside the Oware family (for example Kalah, Bao or Omweso), unless we decide otherwise later.

## Open questions

- Final app name and branding (is "Ouril" the store name?).
- Art direction: realistic wood and seeds, or stylized?
- Minimum supported iOS and Android versions, and supported browsers.
