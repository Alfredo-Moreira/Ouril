# 0033. Live games: moves over HTTP, clocks from timestamps, a realtime relay

- **Status:** Proposed
- **Date:** 2026-10-13

## Context

[ADR 0005](0005-server-authoritative-multiplayer.md) planned live games as WebSocket sessions held by server tasks, with Redis for queues and pub/sub ([ADR 0010](0010-postgres-and-local-sqlite.md)). The server now runs as Vercel Functions that scale to zero, last at most 300 s and share no memory ([ADR 0030](0030-server-on-vercel-container-service.md)), so there's no long-running task to own a clock or a queue. Turn-by-turn games already work over HTTP, with deadlines settled whenever a game is read or written ([ADR 0031](0031-online-games-over-http.md)). Live games add game clocks (for example 5 minutes each), matchmaking, and moves that must reach the opponent in well under a second.

## Decision

- **Moves stay on HTTP**, as in ADR 0031: `POST /v1/games/{id}/moves` with the ply. The server validates the move with the engine and saves it, and Postgres stays the source of truth.
- **Clocks are computed from timestamps; no timer runs.** Each live game stores each side's remaining time, the increment and when the current turn started (server time). A move deducts the elapsed time and adds the increment. A side whose time is up loses, settled on the next read or write of the game. Responses carry the server's time so clients count down accurately.
- **Moves reach players through a realtime relay** on the homelab (`apps/relay`, [ADR 0032](0032-small-services-on-the-homelab.md)). After saving a move, the server publishes the new state to the relay's HTTP API. Clients subscribe to their game's channel over WebSocket with a short-lived token the server signs. The relay only fans out messages: it validates nothing and stores nothing durable.
- **Polling is the fallback.** If the relay can't be reached, clients poll the game as turn-by-turn games do now, so a homelab outage slows live games down but doesn't stop them.
- **Matchmaking is a Postgres table** of open seeks, paired in a transaction (`FOR UPDATE SKIP LOCKED`) within a rating window that widens with waiting time. The relay tells waiting players when they're matched.
- No Redis for live games. Server-side WebSockets and per-game tasks (ADR 0005) aren't used on this hosting.

See [multiplayer](../product/features/multiplayer.md#live-games) and [backend](../architecture/backend.md#realtime).

## Alternatives considered

- **Polling only:** no new service, but about 1,200 requests per 10-minute game, each a Function invocation and a database query, which uses up the free plans quickly. Up to a second of delay.
- **WebSockets on Vercel with Redis between instances:** in beta, every connection drops after 5 minutes on Hobby, and support for container (Rust) Functions is unclear.
- **A managed pub/sub service (Ably or Pusher)** in the relay's place: the same design with a vendor instead of the homelab. Free tiers cap simultaneous connections. It remains the swap-in if the homelab can't serve.
- **ADR 0005 as written** on an always-on host: a timer and a WebSocket per game, but it needs a paid server and Redis to grow past one machine.

## Consequences

- Live games work on the free hosting. Function invocations scale with moves, not with time spent watching.
- The rules and clock stay in one place (the server), so a relay outage can't corrupt a game.
- A client must handle two delivery paths (relay and polling), plus reconnects and heartbeats (Cloudflare closes WebSockets idle for 100 s).
- New contract work: time controls, clocks and `server_now` in `OnlineGame`, seek endpoints, relay tokens ([API](../../apps/server/API.md)).
- Once accepted, ADR 0005's and ADR 0010's status lines should note this refinement, as 0005's already notes 0031.
