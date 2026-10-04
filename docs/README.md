# Ouril documentation

> Index of all project documentation. **Status:** Draft

## Conventions

- Each document begins with a one-line purpose and a **Status**:
  - `Draft`: being written, may change freely
  - `Proposed`: complete and waiting for a decision
  - `Accepted`: agreed; changing it needs a new discussion or ADR
- Anything still undecided goes in an **Open questions** section at the end of the document.
- Links are relative. Diagrams use [Mermaid](https://mermaid.js.org/), which renders on GitHub.
- Significant technical decisions are recorded as [ADRs](decisions/README.md).
- How to contribute: [CONTRIBUTING.md](../CONTRIBUTING.md) ([Code of Conduct](../CODE_OF_CONDUCT.md), [Security](../SECURITY.md), [Support](../SUPPORT.md)). Guidance for AI agents: [CLAUDE.md](../CLAUDE.md).

## Contents

### Overview
| Document | Purpose |
|---|---|
| [Vision](vision.md) | What Ouril is, who it's for, goals and non-goals |
| [Glossary](glossary.md) | Shared vocabulary for the game and the project |

### Game
| Document | Purpose |
|---|---|
| [Rules](game/rules.md) | Cape Verdean Ouril rules (`cv.standard`, the MVP default), with worked examples |
| [Variants](game/variants/README.md) | Variants by country (one default each), rule parameters, catalog |
| [Cape Verde · standard](game/variants/cape-verde/standard.md) | Researched spec for `cv.standard`, with sources and confidence for each rule |

To research a new country or variant, run the `variant-research` skill (`/variant-research Ghana`).

### Product
| Document | Purpose |
|---|---|
| [MVP](product/mvp.md) | Scope and acceptance criteria for the first release |
| [Roadmap](product/roadmap.md) | Phased plan from MVP to multiplayer and leaderboards |
| [Accounts](product/features/accounts.md) | Optional sign-in with Google and Apple (MVP); X and Meta later |
| [Multiplayer](product/features/multiplayer.md) | Live (sync) and turn-by-turn (async) online play |
| [Leaderboards](product/features/leaderboards.md) | Skill rating and points boards; global, country and city scopes |
| [Localization](product/features/localization.md) | Language support: English first, then Portuguese, Kriolu and more |

### Architecture
| Document | Purpose |
|---|---|
| [Overview](architecture/overview.md) | System diagram and the main building blocks |
| [Monorepo](architecture/monorepo.md) | Folder layout, tooling and boundaries (Rust core, native apps) |
| [Rules engine](architecture/engine.md) | Rust core: game state, variant configuration, move validation, bindings |
| [AI](architecture/ai.md) | Computer opponent and difficulty levels |
| [Data and sync](architecture/data-and-sync.md) | Postgres online, local databases on device, offline-first sync |
| [Test vectors](architecture/test-vectors.md) | JSON rule tests run in Rust and through every platform binding |
| [Versioning](architecture/versioning.md) | Versions for apps, API, variants, game records and schemas; old-app policy |
| [Backend](architecture/backend.md) | Rust server: accounts (MVP), real-time and async multiplayer |

### Tooling
| Document | Purpose |
|---|---|
| [Skills research](tooling/skills-research.md) | Research behind the project's Claude Code skills, hooks and third-party skills (2026-10-03 snapshot) |

### Decisions
| Document | Purpose |
|---|---|
| [ADR index](decisions/README.md) | Architecture Decision Records |
