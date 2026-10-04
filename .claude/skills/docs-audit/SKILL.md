---
name: docs-audit
description: Audits Ouril's documentation for broken relative links and anchors, missing purpose/Status lines or Open questions sections, ADR index drift, variant catalog drift and contradictions between documents. Use after editing anything in docs/, CLAUDE.md, CONTRIBUTING.md, README.md or a skill, before opening a docs pull request, or when the user asks to check, lint or tidy the docs.
argument-hint: "[path or area — default: all docs]"
allowed-tools: Bash(python3 ${CLAUDE_SKILL_DIR}/scripts/check_docs.py *)
---

# Docs audit

Two passes: a mechanical check by script, then a reading pass for contradictions. **Fix mechanical problems directly. Never resolve a contradiction by picking a side.** Quote both statements and ask the user (CLAUDE.md rule 1).

## Input

`$ARGUMENTS`: an optional path or area (`docs/architecture`, `variants`, `ADRs`). The script always checks everything. The reading pass focuses on the area given, or on the docs changed on this branch (`git diff --name-only main...HEAD -- '*.md'`) if none is given.

## Pass 1: mechanical

```bash
python3 ${CLAUDE_SKILL_DIR}/scripts/check_docs.py "$(git rev-parse --show-toplevel)"
```

It reports `ERROR`/`WARN` lines for:

- broken relative links and missing `#anchors`
- docs without a purpose line containing **Status:**
- docs without `## Open questions`
- ADR naming, title, Status, Date and sections, and index rows that disagree with the files
- variant specs vs catalog rows: ID, link, status, exactly one default per country
- docs missing from the `docs/README.md` index

Fix every `ERROR`. For each `WARN`, either fix it or ask whether the file should be exempt. Exemptions go in the script's `*_EXEMPT` sets with a comment. Re-run until no errors remain.

The same script runs automatically after every doc edit (the `.claude/hooks/docs-check.sh` hook), so Pass 1 usually confirms a clean state. Pass 2 is the part only you can do.

## Pass 2: contradictions

Read the docs in scope and check the invariants in [invariants.md](invariants.md). Typical problems:

- the same parameter, event, endpoint, phase or term named or valued differently in two places
- a doc treating a **Proposed** ADR as decided
- statements that drifted after an ADR changed (for example, the guest rule must always say "…unless they opted into telemetry", per ADR 0014)

For each finding, record both locations as `file:line`, quote both sentences, and propose which doc should change. Don't edit until the user chooses.

When you find a new kind of drift worth checking every time, propose adding it to [invariants.md](invariants.md).

## Report

A table: `severity | where | problem | proposed fix | fixed?`. Then a one-line summary of the checker output before and after your fixes. Don't commit.
