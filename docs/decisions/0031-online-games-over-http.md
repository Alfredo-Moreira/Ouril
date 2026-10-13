# 0031. Games against other players over HTTP, turn by turn, with format-3 records

- **Status:** Accepted
- **Date:** 2026-10-13

## Context
[ADR 0005](0005-server-authoritative-multiplayer.md) made the server authoritative for online games, with live games over WebSockets held as server tasks, and async games over HTTP with push notifications and a background job for turn deadlines. The server now runs as a Vercel Function ([ADR 0030](0030-server-on-vercel-container-service.md)): a request lasts at most 300 s on the free plan, instances scale to zero, and they share no memory, so neither long-lived WebSockets nor an always-running job fits. Finished games must also reach both players' history and Stats ("Against other players"), which today only hold games against the computer (`mode = vs_ai`, written by the device).

## Decision
- **Turn by turn over HTTP first** ([multiplayer](../product/features/multiplayer.md)). The server stores each game's moves (`online_games`) and accepts a move only from the player to move, at the expected `ply`, when the engine says it's legal. Clients replay the moves with the same engine to draw the board. Timed live games come later, on the same model.
- **Near-live by polling:** while a game is open, the app asks for it every 2 s during the opponent's turn (15 s otherwise). No WebSockets.
- **Deadlines without a job:** 3 days per move. A game is settled whenever it's read or written: an active game past its deadline is lost on time by the player to move; an invitation nobody accepted within 7 days expires.
- **Starting a game:** invite a player by handle, challenge someone you've played, or create an invite link anyone signed in can accept once. The inviter plays South; who moves first is a coin toss.
- **Notifications:** in the app only (the games list shows invitations and your turn). Web push is a later decision.
- **Game record format 3**, written only by the server: when a game ends, each player gets their own record with `mode: "vs_player"`, `human_player` (their side), `opponent` (ID, name, handle) and, for a missed deadline, `reason: "timeout"`. It reaches their devices through sync pull. Clients never push such records; the server rejects them. The game also counts in `user_opponents`.
- **Account deletion** cancels the player's invitations and resigns their active games: opponents win, and keep the game against "Deleted player".

See [backend](../architecture/backend.md#games-against-other-players) and the [API](../../apps/server/API.md#games-against-other-players).

## Alternatives considered
- **WebSockets as in ADR 0005:** need a long-running server and shared state between instances (Redis), which the free hosting doesn't offer.
- **Server-sent events or long polling:** fewer requests, but each request would hold a Function for up to 300 s; plain polling is simpler and enough at this scale.
- **A scheduled job for deadlines:** Vercel's free cron runs once a day at most, and settling on read gives exact results without one.
- **Each device writing its own record of the game (format 1 or 2):** both players would have to agree on and push the same game; the server already has the authoritative moves.

## Consequences
- Online play works on the free hosting, with no new services.
- Polling costs requests: about 30 a minute per open game while waiting. Fine for Hobby's 1M invocations a month at today's scale; revisit with traffic.
- An app older than this release can't read format-3 records it pulls. The web app updates itself on the next visit.
- The privacy policy now covers games against other players: opponents see your display name and handle, and both keep the game.
- Timed live games, matchmaking, rated games, draw offers, chat and push notifications remain open ([multiplayer](../product/features/multiplayer.md)).
