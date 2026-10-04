# 0010. Postgres online, SQLite / IndexedDB on device

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
We need one **shared online database** for identity (users, sign-in identities, sessions) and synced user data (game records, profiles, settings), and later for multiplayer games, friends and ratings. Each app also needs a **local database** so it works fully offline (see [ADR 0011](0011-offline-first-sync.md)).

## Decision
- **Server:** **PostgreSQL** as the single source of truth, accessed from Rust with **sqlx** (queries checked at compile time; migrations with `sqlx migrate`).
- **Server, from multiplayer onwards:** **Redis** (or its open-source fork **Valkey**) for short-lived data: presence, matchmaking queues, pub/sub, rate limits, and leaderboards later. It can always be rebuilt from Postgres.
- **Devices:** **SQLite** on iOS (GRDB) and Android (Room), **IndexedDB** on the web (Dexie).
- Data shapes (game records, settings, sync mutations) are defined once in the Rust core (`core/protocol`) and stored as matching tables on each platform.
- Session tokens live in platform secure storage, never in the local DB.

See [data-and-sync.md](../architecture/data-and-sync.md).

## Alternatives considered
- **MongoDB / document stores:** our data is relational (users ↔ identities ↔ games ↔ moves), and rating updates need multi-row transactions.
- **Firebase Firestore:** has built-in offline sync, but can't run our Rust core for move validation, locks us into Google, and makes complex queries hard.
- **SQLite inside the Rust core on every platform:** one schema everywhere, but it breaks the "core has no storage" rule, and SQLite in the browser (WASM + OPFS) is still awkward.
- **Realm / Atlas Device Sync:** discontinued by MongoDB.

## Consequences
- A familiar, well-supported, cheap stack. Managed Postgres is available from many hosts.
- Three local storage implementations (one per platform), kept consistent by the shared data shapes and the shared sync tests.
- Redis/Valkey isn't needed for the MVP.
