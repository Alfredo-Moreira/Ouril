# Architecture Decision Records

> A log of significant technical decisions and why we made them. **Status:** Accepted

## Process

1. Copy the template below into `NNNN-short-title.md`, using the next number.
2. Open it as **Proposed** in a PR.
3. Once it's agreed, set it to **Accepted**. A proposal that isn't adopted becomes **Rejected**. To reverse a decision, write a new ADR and mark the old one **Superseded by NNNN**. Never delete an ADR.

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-typescript-monorepo.md) | TypeScript monorepo (pnpm + Turborepo) | Rejected, superseded by 0007 |
| [0003](0003-expo-react-native-for-all-platforms.md) | Expo / React Native for iOS, Android and web | Rejected, superseded by 0008 |
| [0004](0004-variant-configurable-rules-engine.md) | Pure, variant-configurable rules engine | Accepted |
| [0005](0005-server-authoritative-multiplayer.md) | Server-authoritative multiplayer on a Rust backend | Accepted |
| [0006](0006-glicko2-plus-points-leaderboards.md) | Glicko-2 rating plus points leaderboards | Accepted |
| [0007](0007-rust-core-with-generated-bindings.md) | Shared Rust core with generated bindings | Accepted |
| [0008](0008-native-apps-per-platform.md) | Native apps: SwiftUI, Jetpack Compose, TypeScript web | Accepted |
| [0009](0009-oauth-accounts-google-apple.md) | Optional OAuth accounts: Google + Apple in MVP; X and Meta later | Accepted |
| [0010](0010-postgres-and-local-sqlite.md) | Postgres online; SQLite / IndexedDB on device | Accepted |
| [0011](0011-offline-first-sync.md) | Offline-first apps with outbox + cursor sync | Accepted |
| [0012](0012-variants-by-country.md) | Variants by country, with a default per country, and the `variant-research` skill | Accepted |
| [0013](0013-versioning-and-compatibility.md) | Versioning and compatibility (variants `id@version`, API, schemas, minimum app version) | Accepted |
| [0014](0014-telemetry-consent.md) | Telemetry only with consent, asked on first launch | Accepted |
| [0015](0015-generated-bindings-not-committed.md) | Generated bindings are build output, not committed | Accepted |
| [0016](0016-i18n-source-format.md) | Translation source format: ICU messages in JSON, generated per platform | Accepted |
| [0017](0017-local-first-development.md) | Local-first, Docker-first development; hosting decided before the first deployment | Accepted |
| [0018](0018-platform-build-order.md) | Build order: core, web, iOS, Android; launch all three together | Accepted |
| [0019](0019-host-server-on-fly-io.md) | Host the Rust server and Postgres on Fly.io | Accepted |

## Open questions

- None about the process itself. Open decisions are listed in each ADR and in the docs they affect.

## Template

```markdown
# NNNN. Title

- **Status:** Proposed | Accepted | Superseded by NNNN
- **Date:** YYYY-MM-DD

## Context
What problem are we solving, and what constraints apply?

## Decision
What we will do.

## Alternatives considered
Other options and why we didn't choose them.

## Consequences
What becomes easier or harder as a result.
```
