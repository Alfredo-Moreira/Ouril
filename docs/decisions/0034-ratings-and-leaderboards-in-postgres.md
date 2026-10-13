# 0034. Ratings and leaderboards in Postgres: overall, per variant and mode, world and country

- **Status:** Proposed
- **Date:** 2026-10-13
- **Supersedes:** [0006](0006-glicko2-plus-points-leaderboards.md)

## Context

[ADR 0006](0006-glicko2-plus-points-leaderboards.md) chose Glicko-2 with pools per variant and mode, a points board, five scopes, and Redis sorted sets to serve the boards. The current hosting has no Redis ([ADR 0030](0030-server-on-vercel-container-service.md), [ADR 0033](0033-live-games-http-moves-and-relay.md)), and games against other players already finish on the server, in one transaction ([ADR 0031](0031-online-games-over-http.md)). Players want an overall ranking as well as one per variant and game mode, worldwide and for their country (`users.country` already exists).

## Decision

- **Glicko-2, updated after every rated game** with fractional rating periods: a player's deviation grows with the real time since their last rated game, as on Lichess. The maths lives in a new pure crate, `core/rating` (`ouril-rating`), tested against Glickman's worked example.
- **Pools:** `overall`, plus one per variant and mode (`<variant>:<mode>`, for example `cv.standard:async`; `live` once live games exist). A rated game updates the players' `overall` rating and the rating of its own pool. Every playable variant can be rated.
- **Rated or casual is chosen per game**, rated by default. A game counts only if both players made at least one move, and only the first 3 rated games between the same two players in 24 hours count. Games ended by an account deletion don't count.
- **Leaderboards come from Postgres** (indexed queries, no cache layer), for each pool and two scopes: **World** and **Country** (from the player's profile). A player appears once they have 10 rated games in that pool, a deviation of at most 110, and a rated game in the last 60 days. Deleted accounts never appear.
- **Signed-in players only** see the boards, so guests still make no requests ([ADR 0014](0014-telemetry-consent.md)).
- **Later:** the points board, region, city and friends scopes, and rank snapshots by a homelab job ([ADR 0032](0032-small-services-on-the-homelab.md)).

See [leaderboards](../product/features/leaderboards.md).

## Alternatives considered

- **ADR 0006 as written:** Redis sorted sets need a Redis instance the hosting doesn't have, and at this scale an index on `(pool, rating)` answers the same query.
- **Per-pool ratings only, with no overall rating:** splits a small community into many short boards. The overall pool gives everyone one number to climb.
- **Elo:** no measure of uncertainty, so new and returning players are ranked badly (ADR 0006's reason still holds).

## Consequences

- Ratings and leaderboards need no new service, and update in the same transaction as the game's result.
- Each rated game writes two rating rows per player (overall and its pool), plus a change log for history.
- The eligibility thresholds and the system constant τ need tuning with real games.
- Players must set their country in the profile to appear on a country board.
