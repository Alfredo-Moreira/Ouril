# Multiplayer

> Product design for online play: live (sync) and turn-by-turn (async). **Status:** Draft. Phases 4 (async) and 6 (live) of the [roadmap](../roadmap.md).

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

## As built (async, ADR 0031)

Turn-by-turn games are built ([ADR 0031](../../decisions/0031-online-games-over-http.md)), on the web at **Online** (`/online`, signed-in players):

- **Start:** invite by @handle, challenge someone from "People you've played", or create an invite link (`/join/<code>`) to share; anyone signed in can accept it once. A guest who opens the link is asked to sign in and comes back to it.
- **Play:** the inviter plays South, and who moves first is a coin toss. The board is drawn from your side. You have **3 days per move**; missing it loses the game. An invitation nobody accepts expires after 7 days.
- **Near-live:** while both players have the game open, the opponent's move shows up within about 2 seconds (the app polls; there's no live connection).
- **Notifications:** in the app only. The games list shows invitations for you and games waiting for your move. Push notifications come later.
- **After the game:** both players get it in their history, and in Stats under "Against other players", and the opponent appears under "People you've played".
- **Not yet:** timed live games and matchmaking, rated games and the rating, draw offers, preset messages, rematch, block and report, and the friends list.

## Open questions

- Whether players can choose the turn deadline (3 days for now).
- Takebacks in casual games?
