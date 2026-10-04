# 0005. Server-authoritative multiplayer on a Rust backend

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
The online component is for **playing against other real people, both in real time and asynchronously**. Fair play needs one source of truth. If clients decided game outcomes, cheating would be trivial.

## Decision
- The **server is the source of truth** for online games. It validates each move with the shared Rust core, owns the clocks, and records results.
- **Real-time:** WebSocket connections, with each live game held as a lightweight task on the server (Tokio), plus matchmaking queues.
- **Async:** moves are sent over HTTP and stored. The opponent is notified by push (APNs, FCM, Web Push), and a background job enforces turn deadlines.
- Clients send *intents* (`move(game_id, ply, pit)`) and apply them optimistically. Client and server run identical logic, so they agree.
- Stack: **Rust (Axum + Tokio), Postgres, Redis**. See [backend.md](../architecture/backend.md).
- Online play requires a signed-in account ([ADR 0009](0009-oauth-accounts-google-apple.md)).

## Alternatives considered
- **Peer-to-peer or client-trusted:** cheap, but unfair and can't support ratings.
- **Node/TS server loading the core as WASM:** viable, but it adds a bridging layer and a second backend language.
- **Managed game backends** (Nakama, Firebase): their scripting runtimes can't easily run our Rust core. Supabase could, through WASM in edge functions, but splits the logic across platforms.

## Consequences
- We run and operate our own server, but it's one language with the core, and it's cheap to host.
- Online games need connectivity. Single-player stays fully offline.
