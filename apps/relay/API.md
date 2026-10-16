# ouril-relay API (live-game relay)

> The contract between the server, the relay and players' devices for live-game updates. **Status:** Proposed, with [ADR 0033](../../docs/decisions/0033-live-games-http-moves-and-relay.md). The relay runs on the homelab ([ADR 0032](../../docs/decisions/0032-small-services-on-the-homelab.md), [deployment](../../docs/architecture/deployment.md#the-homelab)).

The relay only carries the server's notices to players. It validates nothing, stores nothing, and its events hold only IDs and counts: players fetch the game itself from the server (`GET /v1/games/{id}`). Types live in `core/protocol` (`RelayClaims`, `RelayPublish`, `RelayEvent`).

## Paths

| Method | Path | Who | Notes |
|---|---|---|---|
| `GET` | `/ws` | Players | WebSocket upgrade. Needs an allowed `Origin`; per-IP connection and upgrade limits (`429`) |
| `POST` | `/publish` | The server | `Authorization: Bearer <RELAY_API_KEY>` (constant-time check), JSON `RelayPublish`, at most 4 KiB. `204`, or `401` / `400` / `413` |
| `GET` | `/healthz` | Docker, Zoraxy | `200 ok` |

Every other path is `404`.

## Channels

- `game:<uuid>`: a game's changes.
- `user:<uuid>`: a player's own notices (matched, games list changed).

## Tokens

Players get a token from the server, `POST /v1/live/token` ([server API](../server/API.md#live-games)): a JWT, **HS256** with `RELAY_TOKEN_SECRET` (never the server's `JWT_SECRET`), claims:

| Claim | Value |
|---|---|
| `iss` | `ouril` |
| `aud` | `ouril-relay` |
| `sub` | the player's user ID |
| `iat`, `exp` | issued and expiry times; valid for `RELAY_TOKEN_TTL_SECS` (600) |
| `ch` | the channels this token may read: the player's own `user:` channel, plus `game:` for a game they play. 1 to `RELAY_MAX_CHANNELS` (4) |

The relay refuses any other algorithm (including `none`), a bad signature, issuer or audience, an expired token (no leeway), or an invalid channel.

## The WebSocket conversation

1. The client opens `/ws` and, within 5 seconds, sends `{"token": "…"}` as its **first message** (never in the URL, so it can't reach proxy logs).
2. The relay answers `{"type":"ready"}` and starts forwarding the token's channels.
3. Events arrive as JSON `RelayEvent`s:
   - `{"type":"game_changed","game_id":"…","moves":N}`
   - `{"type":"matched","game_id":"…"}`
   - `{"type":"games_changed"}`
4. The client sends `{"type":"ping"}` every 25 seconds (the relay answers `{"type":"pong"}`); the relay also sends WebSocket pings every 30 seconds. Cloudflare closes connections idle for 100 seconds.

Clients can send nothing else: any other message closes the connection.

| Close code | Meaning | Client action |
|---|---|---|
| `4001` | The token expired | Fetch a new token and reconnect at once |
| `4003` | Missing, late or invalid token | Don't retry with the same token |
| `1008` | A message that isn't allowed, or the client fell too far behind | Reconnect with backoff |
| `1001` | Idle (no ping or pong for 75 s) | Reconnect with backoff |

## Settings

| Variable | Default | Notes |
|---|---|---|
| `RELAY_API_KEY` | required | At least 32 bytes; shared with the server |
| `RELAY_TOKEN_SECRET` | required | At least 32 bytes, different from the key; shared with the server |
| `ALLOWED_ORIGINS` | required | Exact web origins, comma-separated |
| `TRUST_CF_CONNECTING_IP` | `false` | `true` on the homelab, where the relay is reachable only through Cloudflare |
| `MAX_CONNECTIONS` / `MAX_CONNECTIONS_PER_IP` | `2000` / `20` | |
| `UPGRADES_PER_MINUTE_PER_IP` | `30` | |
| `PORT` | `8090` | |

Values can also come from `/app/.env` in the image (or `$RELAY_ENV_FILE`); the environment wins.

## Open questions

- Native apps send no `Origin`: they'll need their own rule (for example a separate path or a token-only check) when they exist.
