# 0012. Variants organized by country, with a default per country

- **Status:** Accepted
- **Date:** 2026-10-03

## Context
Oware is played in many countries, and often differently within a country. Cape Verde alone has island differences, such as Brava's 2×7 board. Players identify with "the way we play it here". We need a structure that scales to every variant, is easy to browse in the app, and stays honest about how reliable each rule is.

## Decision
- Variants are grouped by **country**. Each has an ID `<iso2>.<slug>` (for example `cv.standard`, `cv.brava`), and the ID is never changed once used.
- **Exactly one default variant per country.** It's preselected when a player picks that country. The MVP default is `cv.standard`.
- Variants **inherit**: `base.oware` → country default → regional variants. Each one declares only its overrides, and the build resolves complete configs.
- Each variant has a **spec file** (`docs/game/variants/<country>/<variant>.md`) with YAML front matter (overrides, resolved config, confidence per parameter, sources), followed by the evidence. These specs drive implementation and test vectors.
- Specs are produced and updated with the project skill **`variant-research`** (`.claude/skills/variant-research/`). It takes a country or variant name, researches sources in several languages, and writes the spec in a fixed format.
- The parameter list in [variants/README.md](../game/variants/README.md) is the single source of truth. Research flags rules it can't express instead of forcing them into existing parameters.

## Alternatives considered
- **A flat list of variants:** hard to browse, and loses the "my country's way" framing.
- **Group by game family name** (Oware, Wari, Ouri…): names overlap and vary by spelling, while countries are unambiguous.
- **Free-form research notes:** agents and engineers can't use them reliably to implement rules.

## Consequences
- Adding a variant becomes a repeatable workflow: research → review → implement.
- Some variants cross borders (for example Warri in Antigua and Barbados). Each country gets its own entry, which may extend another country's variant.
- Rating pools per variant may be thin. Whether rated play is offered for every variant is decided separately ([leaderboards](../product/features/leaderboards.md)).
