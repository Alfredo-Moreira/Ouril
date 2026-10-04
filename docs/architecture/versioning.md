# Versioning and compatibility

> How each part of the system is versioned, and how old apps, old games and old data keep working. **Status:** Accepted. See [ADR 0013](../decisions/0013-versioning-and-compatibility.md).

## Why this matters

Players don't update apps promptly: versions from months ago stay installed, and offline devices may sync after weeks. Finished games must replay correctly years later, even after rules are clarified. We need to decide this before writing the first line of protocol or storage code.

## What gets versioned

| Thing | Scheme | Example | Notes |
|---|---|---|---|
| Apps (iOS, Android, Web) | SemVer + build number | `1.4.0 (52)` | Shown in settings. Sent on every request. |
| Rust core (all `core/*` crates) | One SemVer for the workspace | `core 0.7.2` | Embedded in each app build |
| HTTP API | Major in URL, additive changes within it | `/v1/…` | Breaking change → `/v2`, with `/v1` kept during deprecation |
| Realtime protocol | Integer, negotiated on connect | `proto: 3` | Server supports the current version and the one before |
| **Variant rules** | `id` + integer `version` | `cv.standard@1` | **Immutable once released** |
| Game record format | Integer `format` field | `format: 1` | Readers support every older format |
| Local DB schema (each app) | Integer, forward-only migrations | `schema: 4` | Migrated on app start |
| Server DB schema | sqlx migrations (timestamped) | `20261003_…` | Expand → migrate → contract, never breaking in one step |
| Sync payloads | `schema` field per mutation type | `game_finished@1` | Server accepts older versions and upgrades them |

## Variant versions

- A variant's behaviour is identified by **`id@version`**. Once a version has shipped, any rule change **creates a new version**, even a one-line one. For example, if players on Fogo confirm that capture is optional after `cv.standard@1` shipped, that becomes `cv.standard@2`.
- **Before a variant first ships**, version 1 may still change freely, including settling its To-confirm rules before the MVP launch.
- The engine registry keeps **every released version**, so `variant("cv.standard", 1)` always means the same rules.
- New offline games use the **latest** version of the selected variant.
- **Online games** pin `id@version` when created. Both players' apps must support it. An app that doesn't know that version is asked to update before joining.
- Test vectors list the versions they apply to ([test vectors](test-vectors.md)). Vectors for released versions are never edited.

## Game records

Every stored game, local or on the server, contains everything needed to replay it:

```json
{
  "format": 1,
  "id": "0192f1c2-…",
  "variant": { "id": "cv.standard", "version": 1 },
  "first_player": "south",
  "moves": [2, 7, 4, 9],
  "core_version": "0.7.2",
  "result": { "outcome": "south_wins", "stores": [26, 18], "reason": "threshold" },
  "started_at": "2026-10-03T18:20:00Z",
  "ended_at": "2026-10-03T18:31:00Z"
}
```

`core_version` is for diagnostics only. Replay depends only on `variant@version` and `moves`.

## Old apps

The server publishes app requirements at `GET /v1/meta`:

```json
{
  "min_supported": { "ios": "1.2.0", "android": "1.2.0", "web": "1.2.0" },
  "recommended":   { "ios": "1.4.0", "android": "1.4.0", "web": "1.4.0" },
  "api": { "current": "v1", "deprecated": [] },
  "realtime_proto": { "current": 3, "min": 2 }
}
```

- **Below `recommended`:** a gentle "update available" notice.
- **Below `min_supported`:** online features (sign-in, sync, multiplayer) are disabled, with an "update required" message. **Offline play is never blocked.** Guests never even make this check unless they've opted into telemetry.
- Every request sends `X-Ouril-Client: ios/1.4.0 (52); core/0.7.2`, so the server can apply compatibility rules and we can see which versions are in use.
- The web app is always current after a reload. The service worker updates in the background and asks the player to reload.

## API and protocol changes

- **Within a major API version, only additive changes:** new endpoints, new optional fields. Clients ignore unknown fields, and the server tolerates missing optional fields.
- **Breaking changes** need a new major version, an ADR, and a deprecation window. Default: the old version stays supported until `min_supported` passes the last app version that uses it (target at least 6 months).
- Enums (event types, error codes) may get new values. Clients must handle unknown values gracefully.

## Data migrations

- **Local DBs:** numbered, forward-only migrations run in a transaction on app start. They're tested from **every** previous schema version, because an app might skip several versions.
- **Server DB:** expand/contract. Add the new structure, then deploy code that writes both, backfill, switch reads, and finally drop the old structure in a later release.
- **Sync:** the outbox stores each mutation with its payload schema version, so a device offline for weeks can still push after updating the app.

## Open questions

- Should the minimum supported version be different per platform? (The format allows it.)
- How long should old variant versions be offered for **new** online games, as opposed to replay only?
