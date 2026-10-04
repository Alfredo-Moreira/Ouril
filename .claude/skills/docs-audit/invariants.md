# Cross-doc invariants

Facts that must read the same everywhere they appear. Check each one during Pass 2 of the docs audit.

- **Guest network rule:** wherever it appears, it says guests make no network requests **unless they opted into telemetry** (ADR 0014).
- **Variant parameters:** names and values match exactly in `docs/game/variants/README.md` (source of truth), `docs/architecture/engine.md` (`VariantConfig`), `.claude/skills/variant-research/template.md`, and every spec's front matter (`resolved` and `confidence` list the same keys).
- **Events and error codes:** `docs/architecture/test-vectors.md` lists the same set as engine.md's `MoveEvent`/`MoveError` and the checker in `.claude/skills/test-vectors/scripts/check_vectors.py`.
- **Variant identity:** anything stored or exchanged uses `id@version` / `{id, version}` (versioning.md), and `VariantConfig` carries `version`.
- **Phases:** names and numbers match in roadmap.md, mvp.md, the feature docs and backend.md's scope table.
- **API:** endpoints in backend.md, data-and-sync.md, accounts.md and versioning.md agree (`/v1/meta`, `/v1/sync/push`, `/v1/sync/pull`, auth endpoints).
- **Toolchain:** CONTRIBUTING.md and architecture/monorepo.md list the same tools.
- **ADR status:** a doc may call something "decided" only if its ADR is Accepted. Check the index for any ADR still marked Proposed.
- **Glossary:** docs use glossary terms (pit, store, sowing, lap, origin pit, grand slam, feeding, single-seed rule).
- **CLAUDE.md:** the "Where things are decided" rows point at existing files that really are the source of truth.
