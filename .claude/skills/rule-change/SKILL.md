---
name: rule-change
description: Applies a confirmed rule answer or correction to an existing Ouril/Oware variant. Updates the variant spec (value, confidence, evidence, sources), docs/game/rules.md, the test vectors and the catalog, and decides whether the change needs a new variant version. Use when a "To confirm" rule is settled, when players or sources correct a rule, when a rule or variant report issue is accepted, or when the user says a variant's rule is wrong.
argument-hint: "[variant-id] [parameter or question]"
allowed-tools: Bash(python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py *)
---

# Rule change

Turn a confirmed rule into consistent docs and tests. **The rule must come from the user, a player report, or a cited source.** Never from Abapa, general Oware knowledge or memory. If the user's answer is ambiguous, ask a precise follow-up question before editing anything.

## Input

`$ARGUMENTS`: the variant ID and the rule, for example `cv.standard feeding_overrides_single_seed_rule` or `cv.standard capture is optional`. The confirmed answer and **who confirmed it** (the user, a named player or community, an issue link, or a source) come from the conversation. Ask for them if they're missing.

## Read first

1. The variant spec `docs/game/variants/<country>/<variant>.md`: front matter, evidence table, Rule interactions, Open questions.
2. `docs/game/variants/README.md`: the parameters table (the only allowed keys and values).
3. `docs/architecture/versioning.md#variant-versions`: when a change needs a new version.
4. `docs/architecture/test-vectors.md`, and the variant's existing vectors in `core/test-vectors/<id>/` (if any).
5. For `cv.standard`, also read `docs/game/rules.md`.

## 1. Does a parameter express it?

Map the confirmed rule to exactly one parameter and value from the parameters table.

- If it fits, continue.
- If no parameter can express it, **stop.** A new parameter needs an ADR and a parameters-table change. Use the `adr` skill, and record the rule under `unsupported_rules` in the spec meanwhile.

## 2. Released or not?

- **Unreleased** (no shipped app has used this version; today that's every variant, including `cv.standard@1`): change the spec in place, keeping the same `version`.
- **Released:** don't change it in place. Bump `version: N+1` in the spec front matter and add a line at the top of the spec body: `**v<N+1> (YYYY-MM-DD):** <what changed and why>`. Old vectors stay untouched. New and adjusted vectors list `{id, version: N+1}`. The engine registry must keep both versions.

If you're not sure whether the version shipped, ask.

## 3. Update the spec

- Front matter: `overrides` (add, change or remove the key compared with the parent), `resolved`, and `confidence`. A rule confirmed by the user or players is `high` if it agrees with sources, `medium` if it's a single community's testimony, and `conflict` if sources disagree (keep both readings).
- Sources: add the confirmation as a source with `kind: player` (who, where, when, issue link) or the new document.
- Evidence table: update the row's value, confidence and evidence. Keep earlier evidence and add the new item.
- Rule interactions: update any interaction the change settles or creates.
- Open questions: remove the answered one. Add any new question the answer raises.

## 4. Update `rules.md` (for `cv.standard`)

- Rewrite the rule text in plain language, and remove its **To confirm** marker.
- Fix every worked example affected, and re-check each example's arithmetic by hand (seeds conserved, captures, whose turn it is).
- Update the "Open questions" list at the end.

## 5. Update the vectors

Use the `test-vectors` skill:

- Vectors tagged `to-confirm` for this rule: drop the tag if the confirmation matches them, otherwise rewrite them (unreleased) or copy them to the new version (released).
- Add vectors for any new behaviour or interaction.

## 6. Catalog and checks

- Catalog row in `docs/game/variants/README.md`: status unchanged unless the variant became `implemented`.
- Run the docs check and fix every error:

```bash
python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py "${CLAUDE_PROJECT_DIR}"
```

## Report

- the variant and version (and whether it was bumped)
- the parameter, its old and new values, and who confirmed it
- the files changed: spec, rules.md, vectors
- open questions added or removed
- a suggested commit title such as `docs(rules): cv.standard feeding overrides single-seed rule`

Don't commit.
