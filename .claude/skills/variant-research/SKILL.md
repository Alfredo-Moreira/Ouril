---
name: variant-research
description: Research the rules of an Oware/mancala variant, given a country (e.g. "Ghana", "Cape Verde") or a variant name (e.g. "Abapa", "Ouril of Brava"), and write a machine-readable variant spec into docs/game/variants/. Use when adding or verifying a variant, choosing a country's default variant, or resolving open rule questions.
---

# Variant research

Produce a **variant spec**: a Markdown file with YAML front matter that the rules engine and other agents can use directly. Every rule must be backed by cited sources, or explicitly marked unknown.

## Input

`$ARGUMENTS` is a country name or code, or a variant name. Examples: `Ghana`, `cv`, `Abapa`, `Ouril of Brava`.

- **Country:** identify every variant played there, pick (or confirm) the **country default**, and write a spec for the default. List the other variants as `lead` rows in the catalog. Write specs for them too if the user asks.
- **Variant:** work out which country it belongs to, then write that variant's spec.

## Read first

1. `docs/game/variants/README.md`: the ID scheme, the **rule parameters table** (the only allowed config keys and values), and the catalog.
2. Any existing spec for this country in `docs/game/variants/<country-name>/`. Update it rather than duplicating it.
3. `docs/game/variants/cape-verde/standard.md`: a complete reference example of the output.
4. [template.md](template.md): the output format.

## Research process

1. **Search widely, in several languages.** Use English plus the country's languages (for example Portuguese for Cape Verde, São Tomé and Portugal; French for Côte d'Ivoire and Senegal). Search the variant's local names too.
2. **Prefer sources in this order:**
   1. ethnographic and academic papers, federation or tournament rules
   2. Ludii (`ludii.games/details.php?keyword=<name>`), which cites its references
   3. Wikipedia (several language editions), and books
   4. well-documented game implementations, such as open-source apps with rule notes
   5. blogs and travel or culture sites

   Record the date you accessed each source.
3. **Gather evidence for each parameter.** For every parameter in the README table, record what each source says, quoting the original wording (with a translation if needed).
4. **Cross-check:**
   - Two or more independent sources agree → `high` confidence.
   - One reliable source, or sources that partly agree → `medium`.
   - A single informal source → `low`.
   - Sources disagree → `conflict`. Record both readings.
   - No source → `unknown`. Only the user can turn an `unknown` into a project **`decision`** (recorded as a `D<n>` source of kind `decision`). Never make that call yourself.

   Watch for copied text: several sites repeating the same paragraph count as **one** source.
5. **Map the evidence to parameters.**
   - Where a rule matches the parent variant's value, inherit it (leave it out of `overrides`), but still record the evidence.
   - A rule that **doesn't fit** any parameter goes under `unsupported_rules`. Never force it into an existing parameter.
   - Unknown values are `null`, with an open question. **Never invent a rule.** Never fill a gap with Abapa behaviour unless a source says so. The engine applies `base.oware` defaults, and the spec must say clearly that it's doing so.
6. **Choose the default** (country mode only): the variant most widely played or most officially recognized in that country. Explain why in the spec.
7. **Note interactions** between rules (for example the single-seed rule vs the feeding obligation) and whether the sources address them.

## Output

1. When updating a spec for a variant that has already shipped, **don't change it in place**: bump `version` and note what changed and why ([versioning](../../../docs/architecture/versioning.md#variant-versions)). Unreleased variants stay at version 1.
2. Write `docs/game/variants/<country-name>/<variant>.md` following [template.md](template.md) exactly. Country folder names are lowercase and hyphenated in English (`cape-verde`, `cote-divoire`).
3. Update the catalog table in `docs/game/variants/README.md`: status, default star, spec link.
4. If a new parameter is genuinely needed, **don't add it yourself**. List it under `unsupported_rules` and mention it in your final report, so a human can decide whether to extend the engine (and the parameters table).
5. Report back in a few lines:
   - the variant ID and the chosen default
   - how many parameters were found at each confidence level
   - any conflicts and unsupported rules
   - the top open questions to ask players from that community

## Pitfalls

- WebFetch can't read many PDFs or paywalled wikis (for example Fandom returns 402). Search for HTML copies, or note the source as "not accessible".
- "Oware" pages often describe Abapa. Don't attribute Abapa rules to a local variant without a source for that country.
- Portuguese school "Ouri" rules are derived from Cape Verdean Ouril, but they're a separate variant (`pt.ouri`).
- Pages for the same game under different names (Ouri, Uril, Urim, Oril) are still one variant unless they describe different rules.
