---
name: adr
description: Writes, accepts, rejects or supersedes an Architecture Decision Record in docs/decisions/ using Ouril's template, and updates the ADR index and every doc the decision affects. Use when a change adds or replaces a language, framework, database or external service; changes a public contract (API, realtime protocol, sync format, game record, test-vector format); changes what personal data is collected or how data is stored, synced or retained; adds a variant parameter; or when the user asks to record, propose, accept or supersede a decision.
argument-hint: "[decision title | accept NNNN | reject NNNN | supersede NNNN]"
allowed-tools: Bash(python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py *)
---

# Architecture Decision Records

One decision per ADR, in this repo's format, with the rest of the docs left consistent with it. A new ADR is **Proposed** until the user says it's agreed. Never delete an ADR, and never rewrite the decision of an Accepted one.

## Input

`$ARGUMENTS` is one of:

- a decision title or short description → write a new ADR
- `accept NNNN` or `reject NNNN` → change that ADR's status
- `supersede NNNN` plus the new decision → write a new ADR that replaces NNNN

If it's empty, infer the decision from the conversation and confirm the title with the user before writing.

## Read first

1. `docs/decisions/README.md`: the process, the index table and the **template**. That template is the only allowed format. Don't add fields from other ADR styles (no "Deciders", no MADR sections).
2. Related ADRs: scan the index titles and `grep -ril "<keyword>" docs/decisions`.
3. The docs the decision touches. CLAUDE.md's "Where things are decided" table maps topics to docs.

## Is an ADR needed?

Write one only if the change matches a trigger in CONTRIBUTING.md ("When to write an ADR", repeated in this skill's description). Otherwise, say so and update the relevant doc instead.

- Game-rule questions are **not** ADRs. They're settled in variant specs (`variant-research`, `rule-change`). The exception is adding a variant parameter, which is an ADR.
- If the decision conflicts with an Accepted ADR, the new ADR must **supersede** it.
- Don't record a choice the user hasn't made. Put undecided points in the relevant doc's **Open questions** instead.

## Write a new ADR

1. **Number:** one more than the highest existing file (`ls docs/decisions`), 4 digits. Never reuse a number, even a rejected one. If `gh` is available, check open PRs for a number already taken: `gh pr list --search "docs/decisions in:files"`.
2. **File:** `docs/decisions/NNNN-short-kebab-title.md`, around 3–6 words.
3. **Header**, exactly:
   ```markdown
   # NNNN. Title in sentence case

   - **Status:** Proposed
   - **Date:** YYYY-MM-DD
   ```
   When superseding, add `- **Supersedes:** [MMMM](MMMM-….md)` as a third line (see 0007).
4. **Sections** `## Context`, `## Decision`, `## Alternatives considered`, `## Consequences`:
   - **Context:** the problem and constraints, linking the docs that state them. No decision here.
   - **Decision:** concrete bullets ("Use X for Y"). End with `See [doc](../architecture/doc.md).` when details live elsewhere.
   - **Alternatives considered:** at least two real options, each with why it lost. Include "do nothing" if it's plausible.
   - **Consequences:** what gets easier, what gets harder, and follow-up work.
   - Keep it short. Existing ADRs are 17–32 lines. Details belong in architecture and product docs, not in the ADR.
5. **Index:** add a row to the table in `docs/decisions/README.md`: `| [NNNN](NNNN-….md) | Short title | Proposed |`.
6. **Propagate in the same change:**
   - Put the details in the architecture or product doc and link the ADR from its Status line or text, as existing docs do.
   - Remove any **Open questions** this decision answers.
   - Update CLAUDE.md's table if a source of truth moved, CONTRIBUTING.md if the workflow changed, and `docs/glossary.md` for new terms.

## Change a status

- **Accept:** set `Accepted` in the file and in the index. Leave the date and text alone.
- **Reject:** set `Rejected` in both places and keep the file.
- **Supersede:** in the old ADR, change only the Status line. Leave its body unchanged.
  - Use `Superseded by [NNNN](NNNN-….md)` if it was Accepted.
  - Use `Rejected — superseded by [NNNN](…)` if it was never accepted, as with 0002 and 0003.
  - Update both index rows.

## Check

Run the docs check and fix every error it reports:

```bash
python3 ${CLAUDE_PROJECT_DIR}/.claude/skills/docs-audit/scripts/check_docs.py "${CLAUDE_PROJECT_DIR}"
```

## Report

In a few lines:

- the ADR path, number and status
- whether it supersedes anything
- the docs you updated, and any you think need a follow-up
- the open questions left for the user
- a suggested commit title such as `docs(adr): NNNN short title`

Don't commit.
