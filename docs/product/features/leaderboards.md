# Leaderboards

> Product design for ratings and rankings: an overall rating and one per variant and game mode, with world and country boards. **Status:** Draft. Decided in [ADR 0034](../../decisions/0034-ratings-and-leaderboards-in-postgres.md) (Proposed), which supersedes [ADR 0006](../../decisions/0006-glicko2-plus-points-leaderboards.md). Phase 7 of the [roadmap](../roadmap.md).

## Ratings

- **Glicko-2.** Each player has a rating, a rating deviation (RD, how unsure the rating is) and a volatility, in each pool. Updated right after every rated game, with fractional rating periods: RD grows with the real time since the player's last rated game, as on Lichess. The maths is in `core/rating` (`ouril-rating`).
- **Pools:**

  | Pool | Counts |
  |---|---|
  | `overall` | Every rated game, any variant and mode |
  | `<variant>:<mode>` | Rated games of that variant and mode: turn by turn (`cv.standard:async`) or live (`cv.across:live`, [multiplayer](multiplayer.md#live-games)) |

  A rated game updates both players' `overall` rating and the rating of its own pool.
- **Starting values and constants** (to tune with real games):

  | Constant | Value | Notes |
  |---|---|---|
  | Starting rating | 1500 | Glickman, Lichess |
  | Starting RD | 350 | Glickman's default |
  | Starting volatility | 0.06 | Glickman's example |
  | τ (system constant) | 0.5 | Glickman: "reasonable choices are between 0.3 and 1.2" |
  | Rating period | 1 day | Fractional: RD grows with elapsed time |
  | RD range | 45 to 350 | Clamped after every update |
  | Provisional | RD > 110 | Shown as `1520?` |

- **What's rated:** games against other players marked **rated** (the default when inviting; casual is one tap away). A game counts only if both players made at least one move. Only the first 3 rated games between the same two players in 24 hours count, to stop rating farming. Games ended by an account deletion don't count.
- **Not rated:** games against the computer (played on the device, so they can't be verified) and casual games.

## Boards

| Board | Who's on it |
|---|---|
| **World** | Every eligible player in the pool |
| **Country** | Eligible players whose profile country matches (ISO 3166-1 alpha-2, `users.country`) |

- **Eligible:** at least **10 rated games** in the pool, RD at most **110** (after the time-based growth), and a rated game in the last **60 days**. Deleted accounts never appear.
- **What a board shows:** the top 100 (rank, display name, handle, country, rating, games), plus your own position when you're eligible.
- **Signed-in players only:** guests make no network requests ([ADR 0014](../../decisions/0014-telemetry-consent.md)), so the Leaderboard page asks them to sign in.
- **Privacy:** boards show display names, handles, countries and ratings to other signed-in players. The privacy policy says so. Deleting the account removes the player from every board.

## How it's computed

- The server updates ratings in the same transaction that finishes a rated game, and logs each change (`rating_changes`) for history.
- Boards are indexed queries on Postgres (`ratings`, by pool and rating). There's no cache layer.
- **Rank movement and best ranks:** a scheduled job on the homelab snapshots every board each hour, keeping one snapshot per day (UTC) ([ADR 0032](../../decisions/0032-small-services-on-the-homelab.md), [backend](../../architecture/backend.md#scheduled-jobs)). A board shows ▲/▼ places since the latest earlier day, and **new** for players who weren't on it then; your best rank on each board is kept even after old snapshots are pruned. Until the job has run once, boards show no movement.

## Points board (seasons)

A second board, **Points this season**, on the same page, so casual players can climb too (ADR 0034 listed it for later). Each calendar month (UTC) is a season; points start from zero on the 1st, and past seasons stay viewable. World and Country, like the rating boards.

| Earns | Points (to tune) |
|---|---|
| An online game that counted for ratings (both moved, the daily per-pair limit) | win 10, draw 4, loss 1 |
| A win against the computer, synced from the device | Easy 1, Medium 3, Hard 5, at most **20 a day** |

- **Why so little for the computer:** those games are played on the device. The server replays them for legal moves but can't know they were played fairly, so they're capped, and they count in the season they reach the server, never in a past one.
- Losses and forfeits against the computer, casual online games and games that didn't count for ratings earn nothing.
- The board ranks players with points, by points; ties go to who got there first.

## Player profiles

`/players/:id`, for signed-in players, opened from a leaderboard row or a name in the online lobby:

- display name, handle, country and when they started playing;
- **badges** for their best placements in the top 10 of any board (for example "#1 in Cape Verde · Overall", "#4 in the world · Standard · Live"), from best ranks;
- each pool's rating (provisional with a `?`), record and best rating, with a **chart of the rating over time** (the last 180 days; one line per pool, a tooltip on hover, and the same points as a table for screen readers);
- on someone else's profile, **your record against them**, and **Challenge** (a rated invitation).

## Later

- **Region / island, city, friends** scopes. Cape Verde players may want islands (Santiago, São Vicente, Fogo, …) and diaspora communities (Boston, Lisbon, Rotterdam, …).
- Fair play: reports, and refunds to the opponents of a player caught cheating.

## Open questions

- Whether to limit how often a player can change their country, so nobody hops to an easier board.
- A location data source for cities and regions (for example GeoNames IDs), once those scopes come.
- Should diaspora communities be a separate scope ("club") instead of a city?
- Tuning τ, the starting RD and the eligibility thresholds once there are real games.
