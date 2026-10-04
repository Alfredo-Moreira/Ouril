# 0006. Glicko-2 rating plus points leaderboards

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
We want competitive leaderboards that reflect real skill, and also boards where casual players (including those who mostly play vs AI) can see progress. Both need global and regional (country, region, city) scopes.

## Decision
- **Skill rating:** use **Glicko-2**, with separate rating pools per variant and mode (live and async). Only rated online games count, and a player appears on the board once their rating deviation is low enough.
- **Points:** an activity- and win-based score, with **seasonal** boards plus an all-time board.
- **Scopes:** global, country, region, city and friends, based on self-declared, coarse location.
- Store the data in Postgres and serve it from Redis sorted sets.

See [leaderboards.md](../product/features/leaderboards.md).

## Alternatives considered
- **Elo:** simpler, but it has no uncertainty measure, so new and inactive players are ranked poorly.
- **TrueSkill / OpenSkill:** designed for team and multiplayer games, which we don't need for 1v1.
- **Points only:** rewards grinding rather than skill.

## Consequences
- We maintain two board types, which need clear UI labels ("Rating" vs "Points").
- Points from offline AI games can't be verified, so they may need daily caps.
