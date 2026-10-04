# 0013. Versioning and compatibility

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
Ouril runs on three app platforms plus a server. It's offline-first, so devices may sync weeks late, and its game rules may be clarified over time. Old app versions stay installed for months. Without explicit versioning, old games could replay differently, old apps could break silently, and offline data could fail to sync.

## Decision
- **Variant rules are versioned** as `id@version` and are immutable once released. The engine keeps every released version. Games store `variant@version` + moves, and online games pin a version when created.
- **HTTP API:** a major version in the URL (`/v1`), with only additive changes inside a major version. The **realtime protocol** is an integer negotiated on connect, and the server supports the current and previous versions.
- **Game records, sync payloads and local DB schemas** carry explicit format or schema versions. Readers and migrations support every older version.
- **`GET /v1/meta`** publishes the minimum supported and recommended app versions. Apps below the minimum lose online features but **never offline play**.
- Server DB changes follow **expand/contract**.

Details: [versioning.md](../architecture/versioning.md).

## Alternatives considered
- **Versioning only the app:** can't replay old games after rule clarifications, and makes protocol negotiation unclear.
- **Forcing updates for everything:** blocks offline players and goes against the offline-first principle.
- **Header-based API versioning:** less visible than the URL. The URL major version plus additive changes is simpler to operate.

## Consequences
- A small amount of extra work on every rule change (a new version plus vectors), in exchange for games that always replay correctly.
- The engine registry grows with each version, but configs are tiny.
- Migrations must be tested from every older schema version.
