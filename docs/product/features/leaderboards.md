# Leaderboards

> Product design for rankings: a skill rating and a points board, with global and regional scopes. **Status:** Draft. Phase 7 of the [roadmap](../roadmap.md).

## Two kinds of boards

Decided in [ADR 0006](../../decisions/0006-glicko2-plus-points-leaderboards.md).

### 1. Skill rating (competitive)
- Uses **Glicko-2**. Each player has a rating, a rating deviation (uncertainty) and a volatility.
- Only **rated online games** count.
- Kept separately for each variant and time category (for example *Ouril · Live* and *Ouril · Async*).
- Players appear on the board only once their rating is reliable. For example, their deviation is below a threshold and they've played at least 10 rated games.
- Inactive players' deviation grows over time, and they drop off the board until they play again.

### 2. Points (casual)
- Rewards activity and wins, so everyone can climb, including players who mostly play vs AI.
- Example scoring (to tune): win vs AI Easy/Medium/Hard = 1/3/5, online win = 10, online draw = 4, daily-play bonus.
- **Seasonal** (for example monthly) with an all-time board alongside.
- Points earned offline vs AI sync when the player is next online. Because the server can't verify offline games, AI points may be capped per day.

## Scopes

Every board can be filtered by:

| Scope | Source |
|---|---|
| **Global** | Everyone |
| **Country** | Chosen in the profile; suggested from device locale or IP |
| **Region / state / island** | Optional, chosen by the player |
| **City** | Optional, chosen by the player |
| **Friends** | The player's friends list |

**Privacy:** location is self-declared and coarse. We never store GPS coordinates. A player can hide their city at any time. Their entry then disappears from city boards but stays on wider scopes.

Cape Verde is an important case: players may want boards for islands (Santiago, São Vicente, Fogo, …) and for diaspora communities (Boston, Lisbon, Rotterdam, …). The region model has to handle both.

## UI

- Top 100 for the selected board and scope, plus "your position" with players near you.
- Profile badges for top placements (for example "#1 in Mindelo · Season 3").

## Open questions

- A location data source for cities and regions (for example GeoNames IDs) so names are consistent across languages.
- How often can a player change their country or city? Limits stop people from hopping to an easier board.
- Should diaspora communities be a separate scope ("club") instead of a city?
