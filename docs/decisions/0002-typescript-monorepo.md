# 0002. TypeScript monorepo with pnpm and Turborepo

- **Status:** Rejected — superseded by [0007](0007-rust-core-with-generated-bindings.md)
- **Date:** 2026-10-03

## Context
Ouril targets web, iOS and Android, and later a backend. The rules engine has to behave **identically** on every client and on the server. Keeping everything in one repo makes it easy to share that code and change it in one place.

## Decision
- Use **TypeScript** across the whole project: app, engine, AI and server.
- Use **pnpm workspaces** for packages and **Turborepo** for task running and caching.
- Follow the layout in [monorepo.md](../architecture/monorepo.md): `apps/*` for deployables and `packages/*` for shared code.

## Alternatives considered
- **Polyrepo:** sharing the engine would mean publishing packages and keeping versions in sync, which is extra overhead for a small team.
- **Nx:** more powerful, but heavier than we need right now.
- **Different languages per platform** (Swift, Kotlin, JS): the rules would be written three times, with three sets of bugs.

## Consequences
- One language and one toolchain. The engine is shared directly.
- The tooling has to support React Native's bundler (Metro) in a pnpm monorepo, which needs some setup (`node-linker` or Metro config).
