# 0011. Offline-first apps with outbox + cursor sync

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
Players must be able to use the apps without a network: playing, viewing stats, and editing settings. Their data must reach the shared online database whenever a connection is available, and stay consistent across devices.

## Decision
- Apps read and write their **local database** first. Writes also add an entry to a local **outbox**, in the same transaction.
- A **sync worker** pushes outbox entries to `POST /v1/sync/push`. Each entry has a unique ID, so repeats are ignored and retries are always safe.
- Devices **pull** server changes with `GET /v1/sync/pull?cursor=N`, based on a change sequence number that only goes up and includes deletion markers.
- **Conflicts are avoided by design:**
  - finished games are append-only (a union by device-generated UUIDv7)
  - stats are **derived** from game records by the Rust core, never synced as counters
  - profile and settings use field-level last-writer-wins
  - handle changes and online moves are server-validated requests
- Sync runs on app open, when the network returns, after a game, and periodically in the background, with exponential backoff.
- Pure sync logic (stats derivation, merge rules, mutation types) lives in the Rust core (`core/sync`). Networking and scheduling stay native.

See [data-and-sync.md](../architecture/data-and-sync.md).

## Alternatives considered
- **PowerSync** (Postgres ↔ SQLite sync engine, with Swift, Kotlin and Web SDKs): a strong fit, and worth revisiting if our data model grows complex. It adds a service to run or pay for, and our data is simple enough to sync with a small amount of our own code.
- **ElectricSQL:** mainly syncs from server to device. We'd still need our own upload path.
- **CRDTs** (Automerge, Yjs): powerful, but overkill when our data is mostly append-only.
- **Online-only with simple caching:** breaks offline stats and settings, and loses changes.

## Consequences
- No sync vendor. The sync protocol is small, documented, and tested with shared test vectors.
- The apps must handle rejected mutations (for example a handle that's taken) and show them clearly.
- Live multiplayer still needs a connection. Async moves can be queued offline but are validated when sent.
