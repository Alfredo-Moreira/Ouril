# 0004. Pure, variant-configurable rules engine

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
The MVP ships Cape Verdean Ouril, but the long-term goal is to support **every Oware variant played worldwide**. These variants share most mechanics and differ in specific rules: lap behaviour, capture counts, the grand slam, feeding, and end-game handling. The same rules must run in every app, in the AI search, and on the server.

## Decision
- The engine is a **pure Rust crate** (`core/engine`, see [ADR 0007](0007-rust-core-with-generated-bindings.md)) with no platform dependencies, and it never mutates its inputs.
- Rule differences are expressed as a **`VariantConfig`** data value. Where flags aren't enough, an optional per-variant rule override can change part of the behaviour.
- `apply_move` returns the new state **and a list of events**, which every UI uses for animation.
- Games are stored as move lists.

See [engine.md](../architecture/engine.md).

## Alternatives considered
- **Hard-code Cape Verdean rules now and generalize later:** quicker at first, but it would mean a painful refactor and risk subtle bugs when we add variants.
- **A type per variant with duplicated logic:** harder to test all variants against the same scenarios.

## Consequences
- Adding a variant mostly means adding a config, with sources and test vectors.
- The engine is slightly more complex up front.
- Performance must be watched, because the AI calls the engine in tight loops. Use fixed-size arrays and avoid allocating memory per seed.
