# Leaderboards

> Product design for ratings and rankings: an overall rating and one per variant and game mode, with world and country boards. **Status:** Draft. Decided in [ADR 0034](../../decisions/0034-ratings-and-leaderboards-in-postgres.md) (Proposed), which supersedes [ADR 0006](../../decisions/0006-glicko2-plus-points-leaderboards.md). Phase 7 of the [roadmap](../roadmap.md).

## Ratings

- **Glicko-2.** Each player has a rating, a rating deviation (RD, how unsure the rating is) and a volatility, in each pool. Updated right after every rated game, with fractional rating periods: RD grows with the real time since the player's last rated game, as on Lichess. The maths is in `core/rating` (`ouril-rating`).
- **Pools:**

  | Pool | Counts |
  |---|---|
  | `overall` | Every rated game, any variant and mode |
  | `<variant>:<mode>` | Rated games of that variant and mode, for example `cv.standard:async` or `cv.across:live` |

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
- **Later:** rank snapshots (for rank movement and seasons) written by a scheduled job on the homelab ([ADR 0032](../../decisions/0032-small-services-on-the-homelab.md), [backend](../../architecture/backend.md#scheduled-jobs)).

## Later

- A **points board** (activity and wins, seasonal) for casual players.
- **Region / island, city, friends** scopes. Cape Verde players may want islands (Santiago, São Vicente, Fogo, …) and diaspora communities (Boston, Lisbon, Rotterdam, …).
- Profile badges for top placements (for example "#1 in Cape Verde · Overall").
- Fair play: reports, and refunds to the opponents of a player caught cheating.

## Open questions

- Whether to limit how often a player can change their country, so nobody hops to an easier board.
- A location data source for cities and regions (for example GeoNames IDs), once those scopes come.
- Should diaspora communities be a separate scope ("club") instead of a city?
- Tuning τ, the starting RD and the eligibility thresholds once there are real games.
