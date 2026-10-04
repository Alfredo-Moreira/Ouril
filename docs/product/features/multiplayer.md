# Multiplayer

> Product design for online play: live (sync) and turn-by-turn (async). **Status:** Draft. Phases 3 and 4 of the [roadmap](../roadmap.md).

## Modes

| | **Async (turn-by-turn)** | **Sync (live)** |
|---|---|---|
| Pace | Move whenever you like | Both players online, real time |
| Clock | Per-move timeout (for example 1 or 3 days) | Game clock (for example 5 or 10 minutes per player) |
| Concurrent games | Many | One at a time |
| Notifications | Push notification when it's your turn | Not needed (in-game) |
| Best for | Friends and family across time zones | Competitive play |

Online play is against **other real people** and requires a signed-in [account](accounts.md). Guests can still play everything offline.

Both modes support **rated** games (which affect the [skill rating](leaderboards.md)) and **casual** games (which don't), and both work with any supported [variant](../../game/variants/README.md).

## Starting a game

- **Challenge a friend:** by username, from the friends list, or with a shareable invite link (which opens the app or the web).
- **Matchmaking (sync):** join a queue for a variant and time control. You're paired by rating, and the acceptable rating gap widens the longer you wait.
- **Open challenge (async):** post a challenge that anyone in a rating range can accept.

## During a game

- The server validates every move with the shared [rules engine](../../architecture/engine.md). Clients can't cheat on rules.
- Both players see the opponent's move animated.
- Actions: resign, offer a draw, and (async only) send a short preset message or emoji. Free-text chat may come later, with moderation.
- **Disconnect (sync):** a grace period (for example 60 seconds) to reconnect. After that, the player's clock keeps running and they may lose on time.
- **Timeout (async):** a missed turn deadline counts as a loss. Reminders are sent before the deadline.

## After a game

- Result screen with rating change, a rematch button, and a replay.
- The game is saved to both players' history.

## Fair play

- Server-authoritative moves and clocks ([ADR 0005](../../decisions/0005-server-authoritative-multiplayer.md)).
- Engine-assistance detection (comparing moves to strong AI play) is a later concern.
- Block and report players.

## Open questions

- Async timeout length, and whether players can choose it.
- Takebacks in casual games?
