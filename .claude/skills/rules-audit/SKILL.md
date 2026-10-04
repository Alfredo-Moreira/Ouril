---
name: rules-audit
description: Audits consistency between Ouril's rule artifacts. Covers the variant parameters table, the engine design doc, variant specs and their resolved configs, rules.md worked examples, the glossary, test vectors and (once they exist) engine variant configs. Reports findings without fixing them. Use before implementing or releasing a variant, after any rule or parameter change, or when the user asks whether the rules docs, specs and vectors agree.
argument-hint: "[variant-id — default: all variants]"
allowed-tools:
  - Bash(python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/test-vectors/scripts/check_vectors.py *)
  - Bash(python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py *)
---

# Rules audit

A read-only review. **Report, don't fix.** Every finding goes to the user, and fixes go through `rule-change`, `adr` or a docs edit the user picks. Don't use Edit or Write during this skill.

## Input

`$ARGUMENTS`: a variant ID to focus on (for example `cv.standard`). If it's empty, audit every variant with a spec.

## Artifacts

| Artifact | Path |
|---|---|
| Parameters table (source of truth) | `docs/game/variants/README.md` |
| Engine design | `docs/architecture/engine.md` (`VariantConfig`, enums, `MoveEvent`, `MoveError`) |
| Vector format | `docs/architecture/test-vectors.md` |
| Spec template | `.claude/skills/variant-research/template.md` |
| Variant specs | `docs/game/variants/<country>/<variant>.md` |
| Plain-language rules | `docs/game/rules.md` (`cv.standard`) |
| Glossary | `docs/glossary.md` |
| Vectors | `core/test-vectors/` (once created) |
| Engine configs | `core/engine/variants/*.toml` (once created) |

## Checks

1. **Parameters:** the table, `VariantConfig` fields and enum values, and the template's `resolved` keys match one-to-one in name and allowed values.
2. **Each spec:**
   - `resolved` = the parent's resolved values + `overrides`, and every key is in the table
   - `confidence` has exactly the same keys as `resolved`
   - every evidence-table row agrees with `resolved` and `confidence`
   - each cited `S<n>` exists in `sources`
   - the catalog row matches the spec (status, default)
3. **rules.md vs spec (`cv.standard`):** every rule in the text matches the spec's `resolved` values, and every **To confirm** marker corresponds to a low, unknown or conflict confidence (or a Rule interactions note).
4. **Worked examples:** re-derive each example by hand from the spec: seeds conserved, sowing path, skip of the origin pit, captures and chains, single-seed legality, feeding, grand slam extra turn, and whose turn is next. Report any arithmetic or rule error.
5. **Events and errors:** the event types, event order and error codes in test-vectors.md match engine.md and `check_vectors.py`'s lists.
6. **Vectors** (if `core/test-vectors/` exists):
   - run `python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/test-vectors/scripts/check_vectors.py "${CLAUDE_PROJECT_DIR}"`
   - coverage: each worked example, evidence row and Rule interaction has a vector
   - vectors depending on low-confidence or To-confirm rules are tagged `to-confirm`
7. **Engine configs** (if they exist): each `core/engine/variants/<id>.toml` equals the spec's `resolved` for the same version.
8. **Glossary:** rule terms used in rules.md and specs (single-seed rule, grand slam, feeding, origin pit…) are defined in the glossary.

Also run the docs checker once and include any `ERROR` lines about these files:

```bash
python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py "${CLAUDE_PROJECT_DIR}"
```

## Report

A table: `severity | check | where (file:line) | finding | suggested fix (skill)`.

- **Severity:** `error` (contradiction or wrong arithmetic), `gap` (missing vector, evidence or definition), `note` (style or clarity).
- Quote both sides for every contradiction.
- End with a one-line verdict: is the variant ready to implement or release, and if not, what blocks it.
