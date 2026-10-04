# Glossary

> Shared vocabulary for the game and the codebase. Use these terms in docs, code and UI. **Status:** Draft

## Game terms

| Term | Meaning |
|---|---|
| **Board** | Two rows of six pits. Each player owns the row nearest to them. |
| **Pit** (house) | One of the 12 holes that hold seeds. In code they are indexed `0–11`, see [engine](architecture/engine.md). |
| **Seed** | A playing piece. A standard game uses 48 (4 per pit). |
| **Store** | A player's captured seeds. This may be a physical pit at the end of the board, or simply a pile beside it. Store seeds are out of play. |
| **Sowing** | A move. You pick up every seed from one of your pits and drop them one at a time into the following pits, counter-clockwise. |
| **Lap** | A sowing that goes all the way around the board. This happens when a pit holds 12 or more seeds. |
| **Origin pit** | The pit a sowing started from. In most variants it is skipped on a lap. |
| **Capture** | Taking seeds from the opponent's pits into your store, usually when your last seed makes a count of 2 or 3. |
| **Chain capture** | After a capture, continuing backwards through the opponent's adjacent pits and capturing each one that also holds 2 or 3. |
| **Grand slam** | A move that would capture every seed on the opponent's side. Variants handle it differently. |
| **Starvation / feeding** | When a player has no seeds, the opponent must make a move that gives them seeds, if such a move exists. |
| **Single-seed rule** | In Cape Verdean Ouril, you may not play a pit holding one seed while any of your pits holds two or more. |
| **Endless cycle** | A position that repeats (same pits, same player to move), or 100 moves in a row without a capture. The game ends and each player collects the seeds on their own side (in `cv.standard`). |
| **Variant** | A specific rule set (for example `cv.standard` or `gh.abapa`), defined as a configuration of the engine. |
| **Country default** | The one variant preselected for a country (for example `cv.standard` for Cape Verde). |
| **Regional variant** | A local rule set within a country (for example `cv.brava`). It extends the country default. |
| **App decision** | A rule no source covers, chosen by the project and recorded in the variant spec with confidence `decision` and a `D<n>` source. Players can still correct it later. |
| **Variant spec** | The researched, machine-readable description of a variant, in `docs/game/variants/<country>/<variant>.md`. |

## Project terms

| Term | Meaning |
|---|---|
| **Guest** | A player who hasn't signed in. They can play offline, with stats stored only on the device. |
| **Account** | A signed-in player (Google or Apple; later X and Meta). Needed for online play. |
| **Core** | The shared Rust code (rules engine, AI, protocol types) used by every app and the server. |
| **Sync game** | A live, real-time online match where both players are connected, usually with a clock. |
| **Async game** | A turn-by-turn online game. Players move whenever they like and are notified when it's their turn. |
| **Rating** | A skill estimate (Glicko-2) based on rated online games. |
| **Points** | A casual score that grows with activity and wins and is shown on the points leaderboard. |
| **Scope** | A leaderboard's geographic filter: global, country, region or city. |
| **ADR** | Architecture Decision Record, see [decisions](decisions/README.md). |

## Kriolu and Portuguese terms

To collect and confirm with native speakers. These will also feed into [localization](product/features/localization.md).

| English | Kriolu (Cape Verdean) | Portuguese | Notes |
|---|---|---|---|
| The game | *Ouril* | Ouril / Ouri | Also *Uril, Urim, Oril, Ori, Urinca*, depending on the island |
| Pit | *buraco* | casa / buraco | |
| Seed | *ouri* | semente | Nickernut seeds from the *ourinzeira* shrub |
| Store | *To confirm* | depósito | The larger pits at each end |
| Capture | *To confirm* | captura / capturar | |
| Large pile of seeds in one pit | *kroo* | | Informal |
| Proverb | *Doze boc', quarenta e oit' dent'* | | "Twelve mouths, forty-eight teeth" (the board) |

Sources: [Sal Cabo Verde](https://salcaboverde.com/how-to-play-ouril-cape-verdes-traditional-mancala-game/), [Wikipédia: Ouri](https://pt.wikipedia.org/wiki/Ouri). Native speakers still need to review these.

## Open questions

- Kriolu words for store and capture, and island spelling differences, to be reviewed by native speakers.
