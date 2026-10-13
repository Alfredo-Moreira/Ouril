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
- **Disconnect (sync):** the player's clock keeps running while they're away, so they can come back and carry on, or lose on time. A player who never makes their first move has the game aborted, with no result.
- **Timeout (async):** a missed turn deadline counts as a loss. Reminders are sent before the deadline.

## After a game

- Result screen with rating change, a rematch button, and a replay.
- The game is saved to both players' history.

## Fair play

- Server-authoritative moves and clocks ([ADR 0005](../../decisions/0005-server-authoritative-multiplayer.md)).
- Engine-assistance detection (comparing moves to strong AI play) is a later concern.
- Block and report players.

## Live games (planned)

Proposed in [ADR 0033](../../decisions/0033-live-games-http-moves-and-relay.md), on the hosting from [ADR 0032](../../decisions/0032-small-services-on-the-homelab.md):

- **Same game screen** as turn-by-turn and computer games, plus a clock for each player in the scoreboard, with a low-time warning.
- **Time controls** (to playtest): 3 + 2, 5 + 3 (default) and 10 + 5, in minutes per player plus seconds added per move. PlayOK's usual oware setting is 5 minutes each.
- **Moves arrive in well under a second** through a realtime relay on the developer's homelab. If it can't be reached, the app polls the game every second instead, so a game is never stuck.
- **Starting:** **Play now** (pick a time control; you're paired with someone near your rating, and the range widens the longer you wait), open challenges in a lobby, or a live challenge to a handle or past opponent. With few players online, the lobby and direct challenges matter more than automatic pairing; the app shows how many are looking and offers a game against the computer while you wait.
- **During the game:** resign, offer a draw, and abort before your first move. The server owns the clocks and every rule.

## As built (async, ADR 0031)

Turn-by-turn games are built ([ADR 0031](../../decisions/0031-online-games-over-http.md)), on the web under **Game** → **Online** (`/game?tab=online`; the Game page's other tab is the computer). Signed-in players see the lobby; guests are asked to sign in:

- **Start:** invite by @handle, challenge someone from "People you've played", or create an invite link (`/join/<code>`) to share; anyone signed in can accept it once. A guest who opens the link is asked to sign in and comes back to it.
- **Play:** the inviter plays South, and who moves first is a coin toss. The board is drawn from your side. You have **3 days per move**; missing it loses the game. An invitation nobody accepts expires after 7 days.
- **Same game screen:** a started game looks and sounds like a game against the computer: full screen, music, animated moves, scoreboard, rules and the result dialog ([web design](../../architecture/web-design.md#the-game-screen)).
- **Near-live:** while both players have the game open, the opponent's move shows up within about 2 seconds (the app polls; there's no live connection).
- **Notifications:** in the app only. The games list shows invitations for you and games waiting for your move. Push notifications come later.
- **After the game:** **Rematch** invites the same player to a new game. Both players get it in their history, and in Stats under "Against other players", and the opponent appears under "People you've played".
- **Rated or casual:** invitations are rated by default; a rated game updates both players' ratings and shows the change in the result ([leaderboards](leaderboards.md), [ADR 0034](../../decisions/0034-ratings-and-leaderboards-in-postgres.md)).
- **Not yet:** timed live games and matchmaking ([planned](#live-games-planned)), draw offers, preset messages, block and report, and the friends list.

## Open questions

- Whether players can choose the turn deadline (3 days for now).
- Final live time controls, once typical game length is measured with AI self-play.
- Rated or casual by default for friend invitations?
- Takebacks in casual games?
