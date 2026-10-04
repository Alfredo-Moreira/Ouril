# Claude Code skills for Ouril: research and recommendations

> Which Claude Code skills, hooks, subagents and settings to add to the Ouril repo, why, and in what order. **Status:** Accepted (research snapshot from 2026-10-03, kept for reference and not updated). Researched against the official Claude Code docs (Claude Code v2.1.285).

## Outcome (what we adopted)

- **Added (Phase 0):** the project skills `adr`, `docs-audit`, `test-vectors`, `rule-change` and `rules-audit` in `.claude/skills/`, and the docs-check hook (`.claude/hooks/docs-check.sh`, registered in `.claude/settings.json`). The prototype scripts below now live in those skill folders.
- **Fixed:** the doc drift listed in §1 (parameter names, missing move-limit parameter, move events, `VariantConfig.version`, missing Open questions sections).
- **Installed third-party skills** for Rust, SwiftUI and Swift concurrency, Kotlin/Compose, Postgres, security and architecture. They're listed in `skills-lock.json`, not committed (no licence files), and restored with `npx skills experimental_install`. `getsentry/skills@security-review` was installed, then removed: it replaced the built-in `/security-review` and pre-approved Bash.
- **Not adopted yet:** later-phase skills (§4.6 onwards), the reviewer subagent, permission rules, and CI jobs.
- **Unverified claims, treat as leads:** the `verify` skill's run-before-commit behaviour (v2.1.286+), Apple's `xcrun agent skills export`, and the unit of the skill-listing budget.

**How to read this:** §1 is the ranked shortlist. §2 lists the platform facts the design depends on, with citations. §3 says what to reuse from public skills and plugins. §4 has one proposal per skill. §5 covers hooks, subagents and settings. §6 contains the three `SKILL.md` drafts. Their two bundled scripts were tested against the repo and are in the appendix. §7 lists rejected ideas, and §8 the sources.

---

## 1. Executive summary

Ouril is a docs-first, rules-critical, polyglot project. Its expensive mistakes are a rule implemented from memory instead of from a spec, a vector edited for a released variant version, docs that drift from each other or from the ADRs, a guest making a network request, a sync or migration change that breaks old offline devices, and a feature that lands on one platform but not the others. Skills pay off most where a procedure is long, knowledge-heavy and repeated, and where a mistake is subtle. Hard rules (for example "never hand-edit generated bindings") belong in **hooks and CI**, not in skills. The official docs say: "If a rule must hold every time, make it a hook rather than a prompt instruction" ([features overview](https://code.claude.com/docs/en/features-overview)).

**Key findings**

1. **Three skills are worth adding now (Phase 0)**, before any code exists: `adr`, `docs-audit` and `test-vectors`. Two more, `rule-change` and `rules-audit`, belong in Phase 0 because the MVP acceptance criteria require every open rule question to be resolved. Full drafts of the first three are in §6, and their scripts are tested (appendix).
2. **The docs have already drifted** in ways a `rules-audit` would catch (verified by grep). `engine.md` uses `endless_cycle` but the parameters table uses `endless_cycle_outcome`. No parameter exists for the no-capture move limit that `rules.md` says is "To confirm". The engine's `MoveEvent` lists `Sow | SkipOrigin | Capture | GameOver`, but the vector format also has `grand_slam`, `extra_turn` and `collect_remaining`. `VariantConfig` has no `version` field, although everything else is keyed by `id@version`. `glossary.md` and `docs/decisions/README.md` have no **Open questions** section. Relative links and anchors are currently all valid.
3. **A project skill named `verify` is special.** When one exists, Claude Code tells Claude to run it before each commit (v2.1.286+, excluding docs-only and tests-only changes). You have v2.1.285, so update first. Generate it with the bundled `/run-skill-generator` and `/verify` once the monorepo is scaffolded ([skills docs](https://code.claude.com/docs/en/skills#run-your-checks-before-each-commit)).
4. **Reuse rather than build** for generic language and platform guidance:
   - Official LSP plugins (`rust-analyzer-lsp`, `swift-lsp`, `kotlin-lsp`, `typescript-lsp`)
   - Apple's own Xcode 27 skills (`xcrun agent skills export`)
   - Chris Banes' Compose/Kotlin skills, Addy Osmani's web-quality skills, and Google Chrome's `modern-web-guidance`
   - Trail of Bits' `property-based-testing`, `rust-review` and `spec-to-code-compliance`
   - The bundled `/security-review` and `/code-review`, and the `security-guidance` plugin

   Build custom skills only for Ouril's own workflows (specs, vectors, versioning, sync, i18n pipeline, store compliance), where no public skill has the knowledge.
5. **Phase the skills in.** Add a skill only once the files and `just` recipes it references exist. A skill that points at a nonexistent `shared/i18n` generator misleads Claude. Every model-invocable skill also adds its description to every turn's context.

### Ranked shortlist

| # | Skill | Priority | Effort | Why |
|---|---|---|---|---|
| 1 | `adr` | **Now** | S | Every significant change needs an ADR. The skill enforces the repo's own template, numbering, index, supersede/reject semantics and same-change doc updates. |
| 2 | `docs-audit` (+ PostToolUse hook) | **Now** | S | Docs are the only artifact today. A tested script catches broken links/anchors, ADR-index and catalog drift, and missing Status/Open-questions sections. A judgment pass finds contradictions. |
| 3 | `test-vectors` | **Now** | M | Vectors are the executable form of the specs and the cross-platform contract. Writing them by hand is error-prone (seed conservation, event order, laps). A tested checker catches bookkeeping errors without becoming a second rules engine. |
| 4 | `rule-change` | **Now** | M | Settles "To confirm" rules (an MVP acceptance criterion) and later clarifications. It decides in-place edit vs **new variant version**, and updates spec, rules.md, vectors and catalog together. |
| 5 | `rules-audit` | **Now** | M | Semantic cross-check of rules.md ↔ spec ↔ parameters table ↔ engine.md ↔ vectors ↔ glossary. Drift already exists (finding 2). |
| 6 | `pr-ready` | Now | S | Maps the diff to the repo's Definition of Done and PR template, runs what's runnable, and drafts the PR body. Not a commit or PR creator. |
| 7 | `verify` (generated) | MVP start | S | Claude Code runs it before each commit automatically. Records how to build and test each area. |
| 8 | `variant-implement` | MVP | M | Spec → `core/engine/variants/<id>.toml` → vectors → registry checks → catalog `implemented`. |
| 9 | `core-api-change` | MVP | M | Changes to the public core API ripple through the UniFFI and wasm-bindgen surfaces, three platform call sites and four vector runners. |
| 10 | `i18n-string` | MVP | S | Add or rename a UI string once in `shared/i18n` (ICU), regenerate, and use it on all three platforms. No hard-coded strings. |
| 11 | `sync-mutation` | MVP | L | The most cross-cutting change in the system: protocol type with schema version, core merge rules, server idempotency, three outboxes, sync vectors, docs. |
| 12 | `db-migration` | MVP | M | Expand/contract on the server (sqlx), forward-only local migrations tested from every older version (GRDB, Room, Dexie). |
| 13 | `privacy-review` | MVP | M | Guards ADR 0014 (guests make zero requests without consent), data minimization, store privacy answers. Runs as a read-only forked reviewer. |
| 14 | `a11y-review` | MVP | M | Board-game-specific accessibility on SwiftUI, Compose and web (announcing sowing and captures, keyboard play, reduced motion). |
| 15 | `protocol-change` | MVP | M | HTTP endpoint or realtime message under the versioning rules (additive within `/v1`, unknown enum values, `/v1/meta`). |
| 16 | `store-release` | MVP (launch) | M | App Store and Play submission checklist, privacy questionnaires consistent with ADR 0014, `min_supported` and release notes. Manual-only. |
| 17 | `tutorial-lesson` | MVP | M | Authors data-driven tutorial lessons validated like vectors. Needs a lesson-format decision first. |
| 18 | `ai-tuning` | MVP | M | Runs the seeded AI-vs-AI harness, reads win rates and time budgets, and records tuning in `ai.md`. |
| 19 | `platform-parity` | MVP/Phase 2 | S | Read-only matrix: does feature X exist on iOS, Android and web with strings, accessibility and tests? |
| 20 | `translation-review` | Phase 2 | S | Prepares PT and Kriolu (`kea`, ALUPEC) review packets for native speakers and checks ICU validity. Never ships machine-translated Kriolu as final. |
| 21 | `variant-report-triage` | Phase 2 | S | Turns "rule or variant report" issues into spec evidence and open questions (feeds `variant-research` or `rule-change`). |

**Start with:** `adr`, `docs-audit` (+ hook), `test-vectors`, then `rule-change` and `rules-audit`. Small upgrades to the existing `variant-research` skill are in §4.0.

---

## 2. How Claude Code skills work (verified facts)

All facts below come from the official docs as fetched on 2026-10-03. Section anchors are from [code.claude.com/docs/en/skills](https://code.claude.com/docs/en/skills) unless noted.

### 2.1 What a skill is and where it lives

- A skill is a directory with a `SKILL.md`: YAML frontmatter plus Markdown instructions. **Custom slash commands have been merged into skills**: `.claude/commands/x.md` and `.claude/skills/x/SKILL.md` both create `/x`, but only skills support supporting files and the `name` and `paths` fields. Prefer skills for new work. (`/en/skills`, Note at top; `/en/slash-commands` now serves the same page.)
- Skills follow the open **Agent Skills** standard ([agentskills.io](https://agentskills.io)). Claude Code adds invocation control, subagent execution and dynamic context injection on top.
- **Locations** ([where skills live](https://code.claude.com/docs/en/skills#where-skills-live)): enterprise (managed), personal `~/.claude/skills/`, **project `.claude/skills/` (commit it, the right place for Ouril)**, nested `<subdir>/.claude/skills/` (loaded once Claude touches files there), `--add-dir` directories, plugins (`/plugin:skill`), and skills synced from claude.ai.
  - Precedence on name clashes: enterprise > personal > project. A project skill replaces a bundled skill of the same name, but not its aliases.
  - Don't name a folder `synced` or `anthropic-skills*`.
- **Monorepo behaviour:** skills load from `.claude/skills/` in the start directory and every parent up to the repo root. A nested skill with a clashing name becomes `/apps/web:deploy`. This matters later if per-app skills live in `apps/*/.claude/skills/`.
- **Cloud sessions and routines** load project skills committed in the repo but **not** `~/.claude/skills/`. Another reason Ouril's workflows should be project skills.
- Skill files are **live-reloaded** in the session (except a top-level skills dir created mid-session, which needs `/reload-skills`).

### 2.2 Frontmatter fields (complete list from the docs)

All fields are optional. Unknown fields are **silently ignored**, so a typo such as `allowed_tools` does nothing. Frontmatter is only read when `---` is the first line. If the YAML doesn't parse, the skill loads with **no** fields, so auto-invocation silently stops working ([frontmatter reference](https://code.claude.com/docs/en/skills#frontmatter-reference)). `claude plugin validate .claude/skills` finds parse errors (v2.1.233+).

| Field | Meaning / constraint |
|---|---|
| `name` | Command name. Defaults to the directory name. Spec limits: ≤64 chars, lowercase letters, digits and hyphens; no "anthropic"/"claude" ([best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)). |
| `description` | **What it does and when to use it.** Drives auto-invocation. `description` + `when_to_use` are truncated at **1,536 chars** in the listing. The Agent Skills spec and API cap `description` at **1,024 chars**: stay under that for portability. Write in **third person**. |
| `when_to_use` | Extra trigger phrases. Appended to the description and counts toward the cap. |
| `argument-hint` | Autocomplete hint, e.g. `[variant-id] [area]`. |
| `arguments` | Named positional args → `$name` substitutions. |
| `disable-model-invocation` | `true` = only the user can invoke it. The description is **removed from context**. Use for side-effecting or long workflows (`/store-release`). Also blocks preloading into subagents. |
| `user-invocable` | `false` = hidden from the `/` menu. Background knowledge only Claude invokes. |
| `allowed-tools` | Pre-approves tools **for the invoking turn only**. Does **not** restrict tools. Applies even in untrusted folders and `-p` runs, so keep it narrow ([pre-approve tools](https://code.claude.com/docs/en/skills#pre-approve-tools-for-a-skill)). |
| `disallowed-tools` | Removes tools while the skill is active (until the next user message). |
| `model`, `effort` | Per-skill model or effort override for the current turn (`effort`: low…max). |
| `context: fork` + `agent` + `background` | Run the skill as a subagent (`Explore`, `Plan`, `general-purpose` or a custom agent). It runs in the **background by default**, and `background: false` waits for the result. The fork does **not** see the conversation. A backgrounded fork has a narrower tool set, and its edits bypass `/rewind` checkpoints ([run skills in a subagent](https://code.claude.com/docs/en/skills#run-skills-in-a-subagent)). |
| `hooks` | Hooks registered when the skill is invoked, **for the rest of the session**. `once: true` removes a hook after its first success ([hooks in skills](https://code.claude.com/docs/en/hooks#hooks-in-skills-and-agents)). |
| `paths` | Globs: Claude auto-loads the skill only when working on matching files. |
| `shell` | `bash` (default) or `powershell` for injected commands. |
| `metadata`, `license`, `compatibility` | Spec fields. Claude Code accepts them but doesn't act on them. |

**Portability constraint:** claude.ai uploads, the Skills API and `package_skill.py` accept only `name`, `description`, `license`, `compatibility`, `metadata` and `allowed-tools`, and **hard-fail** on other keys. That doesn't matter for project skills used in Claude Code, but it matters if Ouril skills are ever uploaded to claude.ai.

### 2.3 Body features

- **Arguments:** `$ARGUMENTS` (full string), `$ARGUMENTS[N]` / `$N` (0-based, shell-style quoting), `$name`. With no placeholder, Claude Code appends `ARGUMENTS: …`. Escape `\$1` for a literal.
- **Path variables:** `${CLAUDE_SKILL_DIR}` and `${CLAUDE_PROJECT_DIR}` (v2.1.196+) are substituted in the body **and** in `allowed-tools` Bash rules. That is the documented way to let a skill run its own bundled script without a prompt: `allowed-tools: Bash(python3 ${CLAUDE_SKILL_DIR}/scripts/x.py *)`.
- **Dynamic context:** a line starting with `` !`cmd` `` (or a ` ```! ` block) runs **before** Claude sees the skill, and its output is inlined. A non-zero exit **aborts the whole invocation**, except exit 1 for grep/diff-like commands, so append `|| true` to check scripts. Injected commands go through permission checks and never prompt: an unapproved command aborts outside auto mode. Not run for skills synced from claude.ai.
- **Supporting files:** keep `SKILL.md` **under 500 lines**. Put reference material in sibling files linked **one level deep** (Claude may only `head` nested references). Scripts are **executed, not loaded**, so they cost no context ([best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)).
- **Lifecycle:** an invoked skill's rendered content enters the conversation once and **stays**. The file isn't re-read. After compaction, only the first **5,000 tokens** of each recent skill are re-attached, within a 25,000-token total budget. So **put the most important rules at the top** and phrase them as standing instructions ([skill content lifecycle](https://code.claude.com/docs/en/skills#skill-content-lifecycle)).
- `ultrathink` anywhere in the body requests deeper reasoning.

### 2.4 How auto-invocation works

- Descriptions of all model-invocable skills are always in context, and Claude matches requests against them. The listing has a budget that "scales at 1% of the model's context window". When it overflows, descriptions of the **least-used** skills are dropped, so those skills stop triggering. `/doctor` estimates the cost and `/skill-doctor` (v2.1.252+) reports unused skills ([descriptions cut short](https://code.claude.com/docs/en/skills#skill-descriptions-are-cut-short)). *Uncertain: the docs don't say whether the 1% is measured in characters or tokens.*
- Best practice: put the key use case first, include the words users actually say, and be specific to avoid false triggers. Test with should-trigger and should-not-trigger prompts, for example with the **`skill-creator`** plugin's description-tuning loop ([evaluate a skill](https://code.claude.com/docs/en/skills#evaluate-and-iterate-on-a-skill)).
- `skillOverrides` in settings can set a skill to `on` / `name-only` / `user-invocable-only` / `off` without editing it. Useful to quiet a project skill locally.
- Permissions can gate skills: `Skill(name)`, `Skill(name *)`.

### 2.5 Skill vs hook vs subagent vs rule vs command

Condensed from the [features overview](https://code.claude.com/docs/en/features-overview) and the [subagents docs](https://code.claude.com/docs/en/sub-agents):

| Use | When | Ouril example |
|---|---|---|
| **CLAUDE.md** | Facts every session needs. Keep it under ~200 lines. | Already good (45 lines). |
| **`.claude/rules/*.md` with `paths:`** | Area-specific conventions that load when Claude reads or edits matching files | "core/** is pure", "apps/** never hard-codes strings" |
| **Skill** | A repeatable procedure or reference that needs judgment | Writing an ADR, authoring vectors, a sync mutation |
| **Skill with `disable-model-invocation`** | Workflows with side effects that you trigger yourself | `/store-release` |
| **Subagent** | Verbose, self-contained work that returns a summary, or a different tool or permission set | Read-only privacy, accessibility or rules reviewers |
| **Hook** | Must happen **every time**, deterministically. Zero context cost unless it outputs. | Run the docs check after doc edits, block edits to generated bindings |
| **CI / git hooks** | Enforcement independent of Claude, for humans too | Vectors through all bindings, binding-drift check |

Hook mechanics used in §5 ([hooks reference](https://code.claude.com/docs/en/hooks)):
- Hooks live in `.claude/settings.json` (committable), skill or subagent frontmatter, or plugins.
- Matchers are exact names or `|` lists (`Edit|Write`). The `if` field takes one permission-rule pattern (`Edit(docs/**)`).
- Exit 2 blocks a `PreToolUse` call. On `PostToolUse` the tool has already run, and exit 2 feeds stderr back to Claude.
- `async: true` runs a hook in the background, and its output arrives on the next turn.
- `type: "agent"` hooks exist but are **experimental**.
- `PostToolUse` on `Edit|Write` does **not** fire when a Bash command rewrites the file, so use CI for real enforcement.

### 2.6 Other constraints found

- **Workspace trust:** `extraKnownMarketplaces`, `permissions.allow` and most `env` keys in a committed `.claude/settings.json` take effect only after each teammate trusts the folder. `deny`/`ask` rules apply at once ([settings](https://code.claude.com/docs/en/settings)). Project-skill frontmatter hooks register on invocation even in untrusted folders. Project-subagent frontmatter hooks need trust (v2.1.218+).
- **Subagent frontmatter uses camelCase** (`disallowedTools`, `maxTurns`, `permissionMode`), unlike skills (kebab-case). `skills:` in a subagent preloads full skill bodies, which works only for model-invocable skills. `memory: project` gives a subagent a persistent, committable `.claude/agent-memory/<name>/` ([subagents](https://code.claude.com/docs/en/sub-agents#supported-frontmatter-fields)).
- **Bundled run/verify skills:** `/run`, `/verify` and `/run-skill-generator` record per-project build and launch recipes as project skills (`.claude/skills/run-<name>/`, `.claude/skills/verify/`). In a monorepo, `/verify` writes its recipe in the touched package directory ([run and verify](https://code.claude.com/docs/en/skills#run-and-verify-your-app)).

---

## 3. Survey: existing skills and plugins worth reusing

Stars and last push from the GitHub API, 2026-10-03. "Install" means through `/plugin` or `npx skills`, personally or via `enabledPlugins` in project settings. **Don't vendor third-party skills into this MIT repo without checking their license.** Trail of Bits is CC-BY-SA-4.0, and several repos have no license.

### 3.1 Official (Anthropic)

| Skill / plugin | What it gives Ouril | Verdict |
|---|---|---|
| Bundled `/security-review`, `/code-review`, `/simplify` | Diff security review, correctness review (with `--comment`/`--fix`), cleanup | **Use as-is.** Don't write custom equivalents. |
| Bundled `/run`, `/verify`, `/run-skill-generator` | Recorded build, run and verify recipes. A project `verify` is auto-run before commits. | **Adopt at MVP start** (§4 #7) |
| [`skill-creator`](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/skill-creator) plugin | Evals for our skills, with-vs-without baselines, **description tuning** for trigger accuracy | **Adopt** (personal) to tune `adr`, `test-vectors` and the others |
| `rust-analyzer-lsp`, `swift-lsp`, `kotlin-lsp`, `typescript-lsp` | Code intelligence: diagnostics after edits, go-to-definition | **Adopt** when code exists. Declare in `enabledPlugins`. |
| [`security-guidance`](https://github.com/anthropics/claude-plugins-official/tree/main/plugins/security-guidance) | Pattern warnings on edits, LLM diff review on Stop, agentic review on `git commit` | **Adopt for server/auth work** (MVP). Costs tokens per turn, so enable when `apps/server` exists. |
| `claude-security` | Multi-agent threat-model scan with verified findings and patch files | **Run before launch** and before multiplayer |
| `claude-md-management` | Audits CLAUDE.md against the codebase; `/revise-claude-md` | Optional. Useful once code exists. |
| `hookify` | Writes hook rules from plain-language requests | Optional. §5 gives hand-written hooks instead. |
| `pr-review-toolkit`, `feature-dev`, `code-review` (plugin) | Specialist review agents; a 7-phase feature workflow | Optional. Overlaps bundled `/code-review`. |
| `commit-commands` | `/commit`, push, PR | **Skip.** CLAUDE.md says "don't commit unless asked". Use only on request. |
| [anthropics/skills](https://github.com/anthropics/skills) `webapp-testing`, `mcp-builder`, `frontend-design` | Playwright-based web-app testing; MCP authoring; UI design | `webapp-testing` is useful for the web app later. The others don't apply. |
| `playwright` (official marketplace, Microsoft MCP) | Drive the PWA, **read its network log** | **Adopt for web:** directly checks "guest makes zero requests" |
| `context7` | Up-to-date library docs (UniFFI 0.32, wasm-bindgen 0.2.129, Axum, sqlx 0.9 move fast) | **Adopt** (personal) |
| `sentry` | Query crash reports | **Later** (ops, after ADR 0014 telemetry ships) |

### 3.2 Platform-specific (community and first-party vendors)

| Source | Content | Verdict |
|---|---|---|
| **Apple Xcode 27 agent skills** (`xcrun agent skills export`, per [Blake Crosley](https://blakecrosley.com/blog/xcode-27-agent-skills-export) and [DEV](https://dev.to/arshtechpro/wwdc-2026-xcode-27-ships-with-apples-own-agent-skills-what-they-are-and-how-to-use-them-3g2)) | SwiftUI specialist, what's new in SwiftUI 27, Swift Testing modernizer, Xcode security settings audit | **Adopt from your own Xcode** (first-party). *Not verified locally: run `xcrun agent skills export --help`.* Avoid the third-party re-exports ([superagents-lab/xcode27-skills](https://github.com/superagents-lab/xcode27-skills), no license). |
| [twostraws/SwiftUI-Agent-Skill](https://github.com/twostraws/SwiftUI-Agent-Skill) `swiftui-pro` (MIT, ~5k★) | SwiftUI review for modern APIs and performance | **Adopt** for `apps/ios` (preload into an `ios-dev` subagent) |
| [AvdLee/SwiftUI-Agent-Skill](https://github.com/AvdLee/SwiftUI-Agent-Skill) (MIT, ~3.6k★) | SwiftUI best-practices references | Alternative to the above. Pick one to save context. |
| [chrisbanes/skills](https://github.com/chrisbanes/skills) (Apache-2.0, active) | `compose-state-and-effects`, `compose-performance`, `compose-animations`, `compose-ui-testing-patterns`, `kotlin-concurrency-and-flow`, `gradle-run` | **Adopt** for `apps/android`. The animation and performance skills suit seed-sowing animation. |
| [aldefy/compose-skill](https://github.com/aldefy/compose-skill) | Compose guidance with androidx source receipts | Alternative. Unclear license. |
| [addyosmani/web-quality-skills](https://github.com/addyosmani/web-quality-skills) (MIT) | `accessibility` (WCAG 2.2), `performance`, `core-web-vitals`, `best-practices` | **Adopt** for `apps/web`. Our `a11y-review` adds the board-game specifics. |
| [GoogleChrome/modern-web-guidance](https://github.com/GoogleChrome/modern-web-guidance) (Apache-2.0, in official marketplace) | Current web platform guidance (PWA, service workers) | **Adopt** for web |
| [leonardomso/rust-skills](https://github.com/leonardomso/rust-skills) (MIT) | 265 Rust rules, indexed for progressive disclosure | **Trial:** measure with `skill-creator`. Claude already writes good Rust, so value is unproven. |
| [actionbook/rust-skills](https://github.com/actionbook/rust-skills) | Rust "meta-problem" knowledge system | Skip. No license, heavy. |
| [trailofbits/skills](https://github.com/trailofbits/skills) (CC-BY-SA-4.0) | `property-based-testing` (covers **proptest**), `rust-review` (unsafe/**FFI** boundaries: relevant to UniFFI and wasm surfaces), `spec-to-code-compliance` (doc ↔ code gaps), `mutation-testing`, `differential-review` | **Adopt as plugins** (don't copy, share-alike license). Their `supply-chain-risk-auditor` covers npm, PyPI and Go only, **not Cargo**: use `cargo-deny`/`cargo-audit` in CI instead. |
| [obra/superpowers](https://github.com/obra/superpowers) (MIT, very popular, in official marketplace) | TDD, systematic debugging, verification-before-completion, plans | **Optional, personal.** Broad triggers ("use when implementing any feature") can override Ouril's docs-first/ADR flow, so don't enable it project-wide. |
| [greenstevester/fastlane-skill](https://github.com/greenstevester/fastlane-skill) (MIT, ~40★) | fastlane setup, beta and release for iOS/Android | Reference only (immature). Mine it when writing `store-release`. |
| [warunacds/apple-asc-mcp](https://github.com/warunacds/apple-asc-mcp) | App Store Connect MCP | Note: privacy nutrition labels are **not** in the public ASC API, so privacy answers stay a manual, documented step. |
| ADR skills ([affaan-m/ECC](https://github.com/affaan-m/ECC/blob/main/skills/architecture-decision-records/SKILL.md), [khalilbenaz adr-writer](https://github.com/khalilbenaz/claude-skills-collection/blob/main/docs/adr-writer/SKILL.md)) | Generic ADR authoring (different templates, "deciders") | **Don't reuse.** Ouril has its own template, statuses and index. Custom `adr` is ~80 lines. |
| i18n skills (mcpmarket, aitmpl, [classmethod xcstrings article](https://dev.classmethod.jp/en/articles/xcstrings-ai-localization-with-claude-code/)) | Generic i18n, `.xcstrings` translation | **Don't reuse.** Ouril generates platform files from `shared/i18n`, so editing `.xcstrings` directly would be wrong. |
| Accessibility collections ([mrKanoh](https://github.com/mrKanoh/claude-wcag-accessibility-skill) 4★, [wshobson/agents](https://github.com/wshobson/agents) `accessibility-compliance`) | Generic WCAG | Skip. Covered by Addy Osmani's skill plus a custom board-specific skill. |
| UniFFI / wasm-bindgen skills | Only an unverified listing on skills.lc | **None credible.** Custom `core-api-change` needed. |
| Expo skills (official marketplace) | Expo/React Native | **Not applicable** (ADR 0003 rejected). |
| Curated lists ([VoltAgent](https://github.com/VoltAgent/awesome-agent-skills), [ComposioHQ](https://github.com/ComposioHQ/awesome-claude-skills), [BehiSecc](https://github.com/BehiSecc/awesome-claude-skills), [hesreallyhim/awesome-claude-code](https://github.com/hesreallyhim/awesome-claude-code)) | Discovery | Use for discovery. Vet each skill before installing: a skill's `allowed-tools` and hooks run with your permissions. |

**Toolchain note found while researching:** `wasm-bindgen` and `wasm-pack` moved from the archived `rustwasm` org to the [`wasm-bindgen` org](https://github.com/wasm-bindgen/wasm-bindgen) (crates.io: wasm-bindgen 0.2.129, wasm-pack 0.15.0). UniFFI is at 0.32.2 and sqlx-cli at 0.9.0. Skills should say "check the installed version" rather than hard-code flags.

---

## 4. Detailed proposals

Conventions for every proposal:
- Skills live in `.claude/skills/<name>/`, in the noun-phrase style of `variant-research`.
- Descriptions are third person, ≤1,024 chars, with the key use first.
- Each skill ends with a short report and **never commits** (CLAUDE.md rule 9).
- "Reads/updates" lists the source-of-truth docs from CLAUDE.md's table.

### 4.0 Upgrades to the existing `variant-research` skill (S, now)

It is already high quality. Small improvements:
- Add `argument-hint: "[country | ISO code | variant name]"`.
- Consider `effort: high`. Research quality beats speed here.
- Add a source kind `player` (testimony from the community, with consent) to `template.md` and the source ranking, because the rule-report issue form collects exactly that.
- Step "Output 4" already says not to add parameters. Cross-reference CONTRIBUTING ("adds a variant parameter → ADR") and the `adr` skill, so the human follow-up is explicit.
- Optional: `context: fork` would keep dozens of WebFetch results out of the main context. The trade-off is that the fork can't ask clarifying questions mid-run and runs in the background. Keep it inline unless context pressure becomes a problem.

### 4.1 `adr` (Now · S · no dependencies)

- **Purpose:** Create, accept, reject or supersede ADRs exactly per `docs/decisions/README.md`: next number, template, index row, supersede semantics, never delete. Update every affected doc in the same change and decide whether an ADR is needed at all (CONTRIBUTING's triggers).
- **Description:** see the full draft in §6.1.
- **Args:** `[title | accept NNNN | reject NNNN | supersede NNNN]`.
- **Procedure:**
  1. Read the process, template and related ADRs.
  2. Decide whether an ADR is needed.
  3. Pick the number, write the file and add the index row.
  4. Propagate the decision to the architecture/product docs, CLAUDE.md and glossary.
  5. Run the docs check and report.
- **Files:** `SKILL.md` only. It reuses `docs-audit/scripts/check_docs.py`.
- **Tools:** Read, Grep, Glob, Edit/Write. Pre-approve only the docs-check command.
- **Reads/updates:** `docs/decisions/*`, linked architecture/product docs, CLAUDE.md, CONTRIBUTING, glossary.
- **Guardrails:**
  - Status stays **Proposed** unless the user says it's agreed.
  - Never rewrite an Accepted ADR's decision text; supersede it instead.
  - Game-rule questions aren't ADRs, with one exception: adding a variant parameter.
  - Parallel branches can collide on numbers, so check open PRs.
- **Alternative form?** No. It needs judgment. Pair it with the docs hook.

### 4.2 `docs-audit` (Now · S · no dependencies)

- **Purpose:** Two passes.
  1. A deterministic script (tested; see appendix A): relative links and anchors, purpose/Status lines, Open-questions sections, ADR file/index/status consistency, variant catalog ↔ spec front matter (one default per country, status agreement), and `docs/README.md` index completeness.
  2. A judgment pass over a **list of known cross-doc invariants**: telemetry wording on guest no-network, phase names, API paths, toolchain names, glossary terms, ADR statuses cited as decided.
- **Description:** see §6.3.
- **Args:** optional path or area.
- **Files:** `scripts/check_docs.py` (stdlib only, ~200 lines, 73 ms on the repo) and `invariants.md`, the list of cross-doc invariants (one level deep).
- **Tools:** Read, Grep, Glob, Bash(the script).
- **Output:** a findings table. Mechanical fixes are applied. Contradictions are listed with both quotes and **not resolved without the user** (CLAUDE.md rule 1).
- **Complement:** a **PostToolUse hook** runs the same script after each doc edit (§5.1, tested), and CI runs it on every PR.
- **Today's result:** 0 errors, 2 warnings (`glossary.md` and `docs/decisions/README.md` lack Open questions; decide whether to exempt or add them).

### 4.3 `test-vectors` (Now · M · creates `core/test-vectors/` on first use)

- **Purpose:** Author and maintain JSON vectors from worked examples and spec evidence rows, with traceable `source` anchors, `to-confirm` tags and immutability for released versions. A tested static checker (appendix B) validates format, id == path, known events and errors, board shape, seed conservation, and bookkeeping between consecutive states:
  - sow-event count equals the seeds in the moved pit
  - captured seeds equal the mover's store gain
  - the origin pit is empty unless re-sown
  - legal moves are on the mover's side

  It is deliberately **not a rules engine**, so the "one engine" principle holds. Once `ouril-engine` exists, `cargo test` (and later `just test-vectors` through every binding) is the authority.
- **Description:** see §6.2.
- **Args:** `[variant-id[@version]] [area | example | parameter]`.
- **Files:** `scripts/check_vectors.py` (stdlib).
- **Guardrails:**
  - Never derive expectations from engine output ("golden master") for rule vectors.
  - If the engine disagrees, re-derive by hand. Either the engine has a bug or the spec is ambiguous, and in that case ask.
  - Stop at `null` or To-confirm rules unless the case is tagged `to-confirm` and uses the documented `base.oware` default.
- **Doc gaps the skill must surface rather than guess:**
  - The order of `grand_slam`/`extra_turn` events relative to `capture` events is not specified.
  - No JSON Schema file exists yet.
  - There is no endless-cycle move-limit parameter.

### 4.4 `rule-change` (Now · M · uses `test-vectors`)

- **Purpose:** Apply a confirmed rule answer or correction to an existing variant. Typical cases: "players on Fogo confirm feeding overrides the single-seed rule", or "capture is optional".
- **Description:** "Applies a confirmed rule answer or correction to an existing Oware variant: updates the variant spec (value, confidence, evidence, sources), docs/game/rules.md, test vectors and the catalog, and decides whether the change needs a new variant version. Use when a 'To confirm' rule is settled, when players or sources correct a rule, when a rule-report issue is accepted, or when the user says a variant's rule is wrong."
- **Args:** `[variant-id] [parameter or question]`.
- **Procedure:**
  1. Read the spec, versioning.md and test-vectors.md.
  2. **Released?** Unreleased v1 (true for all variants today) may change in place. Released means a new `version: N+1` with a changelog line in the spec, copied and adjusted vectors listing `{id, version: N+1}`, old vectors untouched, and the registry keeping both versions.
  3. Update the front matter (`overrides`, `resolved`, `confidence`) and add the evidence row with a source, including `player` testimony.
  4. Update `rules.md` text and worked examples, and remove the "To confirm" marker.
  5. Re-tag or replace `to-confirm` vectors via `test-vectors`.
  6. Remove the answered Open question from the spec and rules.md.
  7. If no existing parameter fits, stop: an ADR plus a parameters-table change is needed (`adr`).
- **Guardrails:** never infer the rule from Abapa or from memory. Record who confirmed it and when.
- **Why a skill:** the version-bump decision tree is the error-prone part, and it's needed at least four times before the MVP (the open questions in `rules.md`).

### 4.5 `rules-audit` (Now · M · forked read-only reviewer)

- **Purpose:** Semantic consistency across every rule artifact:
  - `variants/README.md` parameters table vs `engine.md` `VariantConfig`, enums and events
  - `template.md`, spec front matter (`resolved` equals parent + `overrides`), the spec evidence table vs `confidence`
  - `rules.md` text and worked examples vs the spec (the arithmetic of each example re-checked by hand)
  - vectors vs spec, glossary terms, and later `core/engine/variants/*.toml` vs spec `resolved`

  Pairs well with Trail of Bits' `spec-to-code-compliance` once code exists.
- **Description:** "Audits consistency between Ouril's rule artifacts: variant parameters table, engine design doc, variant specs and their resolved configs, rules.md worked examples, glossary, test vectors and engine variant configs. Use before implementing or releasing a variant, after any rule or parameter change, or when the user asks whether the rules docs, specs and vectors agree."
- **Frontmatter:** `context: fork`, `agent: ouril-reviewer` (§5.3), `effort: high`. It returns a findings table.
- **Known findings to seed it:**
  - `endless_cycle` vs `endless_cycle_outcome`
  - a missing move-limit parameter
  - `MoveEvent` vs vector event list
  - `VariantConfig` without `version`
  - engine.md's `GrandSlam` comment matches the table (OK)
- **Guardrail:** report only. Fixes go through `rule-change`, `adr` or a docs edit chosen by the user.

### 4.6 `pr-ready` (Now · S)

- **Purpose:** Before a PR, map changed paths to the **Definition of Done** and PR template:
  - `core/**` → vectors through all bindings, generated bindings regenerated
  - `apps/**` → i18n, accessibility, offline, screenshots table
  - `docs/**` → docs-audit
  - any decision → ADR

  It runs what's runnable (`docs-audit`, `test-vectors` check, later `just` recipes), then drafts the PR title (Conventional Commit + scope) and body with boxes ticked or explained.
- **Description:** "Checks the current branch against Ouril's Definition of Done and pull request template, runs the applicable checks, and drafts the PR title and body. Use when the user is about to open a pull request, asks whether a change is ready or done, or asks for a PR description."
- **Frontmatter:** `disable-model-invocation: false`, but **never** runs `git commit`, `git push` or `gh pr create` itself. It prints the text.
- **Overlap:** the bundled `/code-review` checks correctness; `pr-ready` checks process completeness.

### 4.7 `verify` (MVP start · S · needs scaffolding and the `justfile`)

- **Purpose:** The project's "how to build and test each area" recipe (`just test-core`, `just test-ios`, …), selected by changed paths. It is recorded by `/run-skill-generator` or `/verify` rather than hand-written, then reviewed.
- **Special behaviour:** with a project skill named `verify` that Claude can invoke, Claude Code's commit instructions tell Claude to run it before each commit (code changes only; **v2.1.286+**, local is 2.1.285).
- **Guardrail:** keep it fast (<2–3 min for the common path). Put the full cross-binding matrix in CI.

### 4.8 `variant-implement` (MVP/Phase 2 · M · needs `core/engine` and its registry)

- **Purpose:** Turn a reviewed spec into an implemented variant:
  1. Write `core/engine/variants/<id>.toml` from `resolved` (keys only from the parameters table).
  2. Check that `extends` resolves and the country has exactly one default.
  3. Write the full vector set (via `test-vectors`) and run the engine plus bindings.
  4. Localize the variant name (`i18n-string`).
  5. Set the catalog and spec status to `implemented`.
- **Description:** "Implements a researched Oware variant in the Ouril engine from its spec: engine variant config, registry checks, full test-vector coverage, localized name and catalog status. Use when the user asks to implement, add or enable a variant (e.g. cv.brava, gh.abapa) that already has a spec in docs/game/variants/."
- **Guardrails:**
  - Refuse while the spec has `null` values in `resolved`, unless the user accepts a `base.oware` default (record that in the spec).
  - `unsupported_rules` non-empty means stop: a per-variant rule hook needs an ADR (engine.md open question).

### 4.9 `core-api-change` (MVP · M · needs `core/ffi`, `core/wasm`, apps)

- **Purpose:** Change or add a public function or type in `core/*` and carry it through:
  - UniFFI exports (proc-macro `#[uniffi::export]`, records, enums, error enums; check the installed UniFFI version)
  - wasm-bindgen exports (`u64` → `BigInt`, enums and `serde-wasm-bindgen` choices)
  - regenerated bindings (`just bindings`) — **never hand-edited**
  - the iOS, Android and web call sites
  - the four vector runners
  - `engine.md` and the `core_version` bump in the workspace SemVer
- **Description:** "Changes the public API of Ouril's Rust core (engine, ai, sync, protocol) and propagates it through UniFFI (Swift, Kotlin) and wasm-bindgen (TypeScript) bindings, platform call sites, and the shared test-vector runners. Use when adding or changing a core function, type, enum or error that apps or the server call, or when a binding fails to build."
- **Files:** `type-mapping.md`, a reference table of Rust ↔ Swift/Kotlin/TS types and their pitfalls (Kotlin unsigned types, `u64` in JS, enum exhaustiveness and unknown values).
- **Guardrails:**
  - Keep `core/*` pure: no I/O, clocks or unseeded randomness.
  - Adding an enum variant is a client-visible change, so clients must handle unknown values (versioning.md).
  - Breaking protocol or record shapes need an ADR.

### 4.10 `i18n-string` (MVP · S · needs the `shared/i18n` format and generator — undecided today)

- **Purpose:**
  1. Add or rename a key in the `shared/i18n` source (ICU MessageFormat, plurals: "1 seed / 3 seeds"), English only.
  2. Run the generator (→ `.xcstrings`, `strings.xml`/plurals, i18next JSON, server).
  3. Replace the literal at each call site on the three platforms.
  4. Add a translator comment and use glossary terms.
  5. Mark PT and `kea` as missing (never machine-fill them).
- **Description:** "Adds, renames or removes a user-facing string in Ouril's shared translation source (shared/i18n, ICU MessageFormat), regenerates the iOS, Android and web string files, and wires the key into each app. Use whenever UI text, accessibility labels, notification text or store-visible strings are added or changed, or when a hard-coded string is found."
- **Guardrails:**
  - Never edit generated `.xcstrings`/`strings.xml`/JSON directly.
  - Accessibility labels are strings too.
  - Key naming convention: to be defined in the format decision (ADR).
- **Dependency note:** write this skill **after** the i18n source-format ADR. `localization.md` names ICU but not the file format, key naming or generator.

### 4.11 `sync-mutation` (MVP · L · needs `core/sync`, server and the three local DBs)

- **Purpose:** Add a new synced mutation type, or a new schema version of one, end-to-end:
  1. Payload type in `core/protocol` with `type@N` (e.g. `game_finished@1`) and the upgrade path for older versions on the server.
  2. Merge rule in `core/sync` (append-only union / field LWW / server-validated request).
  3. Sync vectors (`core/test-vectors/sync/`, format to be defined).
  4. Server handler that is idempotent by mutation ID, with `change_seq` and `deleted_at`, plus a migration (`db-migration`).
  5. Outbox write **in the same local transaction** as the data change, on iOS (GRDB), Android (Room) and web (Dexie).
  6. Rejection handling and UI.
  7. Guest path: the outbox fills but is never uploaded.
  8. Update the `data-and-sync.md` tables.
- **Description:** "Adds or versions an offline-first sync mutation type end-to-end in Ouril: protocol payload with schema version, merge rule in core/sync, sync vectors, idempotent server handler and migration, outbox writes on iOS, Android and web, and docs. Use when new user data must sync between devices, when a synced payload's shape changes, or when the user mentions the outbox, push/pull, change_seq or a mutation type."
- **Files:** `checklist.md` (per-layer), `platform-notes.md` (GRDB/Room/Dexie transaction patterns).
- **Guardrails:**
  - Never sync counters (derive stats instead).
  - Never break older payload versions: devices push weeks later.
  - Retries must be safe.
  - Guests make no requests.
  - A new data category means an ADR if it changes what personal data is collected.

### 4.12 `db-migration` (MVP · M · needs the server and apps)

- **Purpose:**
  - **Server:** `sqlx migrate add` (timestamped) under `apps/server/migrations`, following **expand → dual-write → backfill → switch reads → contract in a later release**. Never a breaking change in one step. Keep `cargo sqlx prepare` offline query data current.
  - **Local:** a numbered, forward-only migration per platform, in a transaction on app start, **tested from every previous schema version**.
  - Keep data shapes aligned with `core/protocol`.
- **Description:** "Plans and writes database schema migrations for Ouril: Postgres via sqlx using expand/contract, and forward-only local migrations for iOS (GRDB), Android (Room) and web (IndexedDB/Dexie) tested from every older schema version. Use when adding or changing a table, column, index or local store, or when a sync or protocol change needs storage changes."
- **Files:** `server.md`, `ios-grdb.md`, `android-room.md`, `web-dexie.md` (progressive disclosure, one level deep).
- **Guardrails:**
  - Never edit an applied migration.
  - No destructive step in the same release as its replacement.
  - Personal-data changes need privacy review and possibly an ADR.

### 4.13 `privacy-review` (MVP · M · forked read-only)

- **Purpose:** Review a diff or a feature against:
  - ADR 0014: two separate consents, both off by default; with no consent, **zero** guest requests; random resettable install ID not linked to the account; no IP stored; capped queue
  - the guest no-network rule (the `/v1/meta` check is skipped for guests)
  - data minimization (accounts.md)
  - token storage (Keychain, Keystore, httpOnly cookie)
  - account deletion and Apple token revocation
  - logs and Sentry (no PII; `sendDefaultPii` off)
  - location coarseness (leaderboards)
  - consistency with the published privacy policy and store questionnaires
- **Description:** "Reviews Ouril code or designs for privacy compliance: guests make no network requests without telemetry consent (ADR 0014), consent defaults, telemetry identifiers, data minimization, token storage, account deletion, logging of personal data, and consistency with store privacy answers. Use when code touches networking, telemetry, analytics, crash reporting, sign-in, storage of personal data or location, or before a release."
- **Frontmatter:** `context: fork`, `agent: ouril-reviewer`.
- **Guardrail:** a review cannot prove "zero requests". Pair it with an **automated runtime test** per platform: a guest-mode UI test under a network-blocking or recording proxy (URLProtocol stub on iOS, OkHttp interceptor or `NetworkSecurityConfig` on Android, Playwright request log on web). That test enforces the MVP acceptance criterion.

### 4.14 `a11y-review` (MVP · M · forked; reuses third-party web/platform skills)

- **Purpose:** Accessibility checklist per platform, specialized for a board game:
  - each pit is an element with label "Your pit 3, 4 seeds", legal or illegal state, and a hint
  - announcements for the opponent's or AI's move and for captures from the engine's `events` (VoiceOver `AccessibilityNotification`, Compose `LiveRegion`, ARIA live region)
  - full keyboard and switch play on web (roving tabindex), and ≥44 pt / 48 dp targets
  - Dynamic Type / font scale, contrast in light and dark mode, colour not the only cue for legal pits
  - reduced motion (skip seed animation)
  - the tutorial must be completable with a screen reader
- **Description:** "Reviews Ouril UI for accessibility on iOS (SwiftUI), Android (Jetpack Compose) and web (React): screen-reader labels and announcements for pits, moves and captures, keyboard and switch control, tap target size, contrast, Dynamic Type and reduced motion. Use when building or changing any screen, the board, animations or the tutorial, or before a release."
- **Files:** `ios.md`, `android.md`, `web.md`.
- **Guardrails:** report and suggest fixes. Manual screen-reader testing is still required and goes in the PR's testing notes.

### 4.15 `protocol-change` (MVP · M)

- **Purpose:** Add an HTTP endpoint or realtime message following versioning.md:
  - additive only within `/v1`; clients ignore unknown fields; the server tolerates missing optional fields
  - new enum values allowed but handled as unknown
  - breaking change → `/v2` + ADR + deprecation window
  - realtime `proto` integer negotiation (current + previous)
  - `X-Ouril-Client` header; `ply` idempotency for moves
  - guests never call it
  - types live in `core/protocol` (shared by all clients); update the `backend.md` tables
- **Description:** "Adds or changes an Ouril HTTP API endpoint or realtime WebSocket message following the project's versioning rules (additive changes within /v1, enum evolution, protocol negotiation, deprecation via /v1/meta). Use when the user adds an endpoint, request or response field, realtime message, error code, or changes API behaviour."
- **Guardrail:** any breaking change stops and goes to `adr`.

### 4.16 `store-release` (MVP launch · M · `disable-model-invocation: true`)

- **Purpose:** A manual release checklist:
  - version and build numbers (SemVer + build); core version embedded
  - store **privacy answers derived from the code and ADR 0014** (App Privacy labels, Play Data safety form; kept in a repo file such as `docs/release/privacy-answers.md` so they're reviewable, since the ASC API doesn't expose them)
  - Apple 4.8 (Sign in with Apple present), 5.1.1(v) in-app deletion, Play deletion web page, privacy policy URL
  - release notes from Conventional Commits
  - update `/v1/meta` `recommended`/`min_supported` only when needed
  - web deploy with service-worker update prompt
  - fastlane / Gradle Play Publisher lanes
- **Description:** "Walks through releasing Ouril to the App Store, Google Play and the web: versioning, store privacy questionnaires consistent with the telemetry and data docs, account-deletion and sign-in compliance, release notes, minimum supported versions and deployment steps. Use only when the user starts a release."
- **Guardrail:** never submits or publishes without explicit confirmation of each irreversible step.

### 4.17 `tutorial-lesson` (MVP · M · needs a lesson-format decision)

- **Purpose:** Author tutorial lessons as **data**: board setup, scripted moves, the expected engine events, string keys, and the allowed taps. Store them in a shared location so all three apps run the same lessons, validate them through the engine like vectors, and keep them sourced to `rules.md` sections. The MVP requires the tutorial to cover sowing, capture, chains, laps, grand slam and feeding, and to be completable without help.
- **Precondition:** an ADR on where lessons live (`shared/tutorial/` JSON?) and their schema. Without it, the tutorial gets built three times differently (ADR 0008's main cost).

### 4.18 `ai-tuning` (MVP · M · needs `core/ai` and a bench binary)

- **Purpose:** Run a seeded tournament harness, e.g. `just ai-bench --levels easy,medium,hard --games N --seed S`, plus a time-budget probe on reference devices. Interpret win rates (Easy beatable by beginners; Hard <1 s on a mid-range phone per the MVP criteria) and record parameter changes and results in `docs/architecture/ai.md`. Bug-report reproduction relies on the seeded RNG.
- **Guardrail:** tuning never touches rules. The AI only calls `legal_moves`/`apply_move`.

### 4.19 `platform-parity` (MVP/Phase 2 · S · forked read-only)

- **Purpose:** For a feature name, find its implementation on iOS, Android and web. Build a matrix of present, strings, accessibility, tests and offline behaviour, and list gaps. Run it before each release, since the UI is built three times (ADR 0008).

### 4.20 `translation-review` (Phase 2 · S)

- **Purpose:** Produce a review packet per language (PT, `kea`) for native speakers: source text, context, screenshot reference, glossary terms, ICU placeholders and plural categories, and ALUPEC spelling notes. Validate the returned files (ICU syntax, placeholder parity, length). Kriolu translations are **never** finalized by the model. The glossary's "To confirm" Kriolu terms feed in.

### 4.21 `variant-report-triage` (Phase 2 · S)

- **Purpose:** Read issues from the "rule or variant report" form (`gh issue view`). Classify each as a new variant lead, a rule difference or a source. Add `lead` catalog rows or spec evidence/open questions, and hand off to `variant-research` or `rule-change`.
- **Guardrail:** don't copy reporters' personal details into specs (the form itself warns about this).

---

## 5. Complementary hooks, subagents and settings

### 5.1 Hooks (in `.claude/settings.json`, committed)

| # | Event / matcher | What | When | Notes |
|---|---|---|---|---|
| H1 | `PostToolUse` `Edit\|Write` | `.claude/hooks/docs-check.sh`: after a `.md` edit in `docs/`, the root or skills, run `check_docs.py` and **exit 2 with only the errors concerning that file**, so Claude fixes them immediately | **Now** | Prototyped and tested (appendix C). ~0.1 s. Scoped so pre-existing errors elsewhere don't nag on every edit. Removing a heading that other docs link to is only caught by the full audit or CI. |
| H2 | `PreToolUse` `Edit\|Write` | Block edits under generated-binding output paths (exit 2: "regenerate with `just bindings`") | MVP | Paths depend on the open question "commit generated bindings or CI-only?" (monorepo.md). Bash rewrites bypass it, so CI does `just bindings && git diff --exit-code`. |
| H3 | `PreToolUse` `Edit\|Write` | Block edits to vectors and `core/engine/variants/*.toml` for **released** variant versions | After first release | Needs a machine-readable released-versions manifest (e.g. `core/engine/variants/RELEASED.toml`). Propose it via `adr`. |
| H4 | `PostToolUse` `Edit\|Write`, `async: true` | Format the edited file: `rustfmt`, `swiftformat`, `ktlint -F`, `prettier --write` by extension | MVP | Deterministic, cheap. Async so it doesn't block. Lint failures surface next turn. |
| H5 | (optional) `SessionStart` `startup` | Print a one-line "Phase N; open To-confirm rules: K" | Optional | Low value. CLAUDE.md already states the phase. |

Rejected hook ideas: an agent-type `Stop` hook that checks the Definition of Done on every turn (experimental, slow and costly; use `pr-ready` on demand), and hooks that auto-commit.

Sketch for `.claude/settings.json` (H1 now; H2 and H4 later):

```json
{
  "hooks": {
    "PostToolUse": [
      { "matcher": "Edit|Write",
        "hooks": [ { "type": "command", "command": "${CLAUDE_PROJECT_DIR}/.claude/hooks/docs-check.sh", "timeout": 30 } ] }
    ]
  },
  "permissions": {
    "ask":  [ "Bash(git commit *)", "Bash(git push *)", "Bash(gh pr create *)", "Bash(gh release *)" ],
    "deny": [ "Read(./**/*.p8)", "Read(./**/.env*)" ]
  }
}
```

- The `ask` rules back up CLAUDE.md rule 9 ("don't commit, push or release unless asked").
- The `deny` rules keep Apple private keys and env files out of context (backend.md: secrets never in the repo).
- Add `"enabledPlugins": { "rust-analyzer-lsp@claude-plugins-official": true, … }` once code exists. *The exact plugin-ID format follows `name@marketplace`. Check with `/plugin` before committing.*
- Remember that `allow` rules and marketplaces need folder trust.

### 5.2 Path-scoped rules (`.claude/rules/*.md` with `paths:`)

These are better than skills for "always true while editing X" ([memory docs](https://code.claude.com/docs/en/memory#path-specific-rules)).
- `core.md` (`core/**`): purity, determinism, injected seed, no allocation per seed in hot paths, `ouril-*` naming.
- `variants.md` (`docs/game/variants/**`, `core/test-vectors/**`, `core/engine/variants/**`): rules only from specs, keys only from the parameters table, version-bump rule. Point to `rule-change` and `test-vectors`.
- `apps.md` (`apps/{ios,android,web}/**`): no rule logic in apps, call the core only via bindings, no hard-coded strings, accessibility basics, offline, guest no-network.
- `server.md` (`apps/server/**`): validate with `ouril-engine`, additive `/v1`, idempotency, secrets.

These move detail out of CLAUDE.md as it grows. Add them when the directories exist.

### 5.3 Subagents (`.claude/agents/*.md`)

- **`ouril-reviewer`** (Now):
  - read-only reviewer used by the forked skills `rules-audit`, `privacy-review`, `a11y-review` and `platform-parity` (`context: fork`, `agent: ouril-reviewer`)
  - `tools: Read, Grep, Glob, Bash`; `disallowedTools: Edit, Write`
  - `model: inherit`
  - system prompt: CLAUDE.md's rules, "report, don't fix", findings table format with file:line evidence, "quote both sides of a contradiction"
  - optional `memory: project` to accumulate recurring findings in `.claude/agent-memory/ouril-reviewer/` (committed; review it like code)
- **`ios-dev`, `android-dev`, `web-dev`** (MVP, once the apps exist):
  - implementers with `skills:` preloading the platform skills from §3.2 (e.g. `swiftui-pro`, Chris Banes' Compose skills, `accessibility` + `modern-web-guidance`) plus `i18n-string` and `a11y-review`
  - `isolation: worktree` for building one feature on three platforms in parallel without conflicts

  This answers ADR 0008's "UI built three times" cost directly.
- Don't add a subagent for variant research. The skill is fine inline (§4.0).

### 5.4 CI and git hooks (outside Claude, but the real enforcement)

- CI jobs:
  - `python3 .claude/skills/docs-audit/scripts/check_docs.py .`
  - `check_vectors.py`
  - later, `just test-vectors` on all bindings
  - the bindings drift check
  - `cargo deny`/`cargo audit`
  - the guest zero-network UI tests (§4.13)
- Long-term, move both scripts to `tools/` and call them from `just docs-check` / `just vectors-check`. The skills then call the `just` recipes, so humans, CI, hooks and Claude all share one implementation.

---

## 6. Full drafts of the top three skills

They match the style of `variant-research`. Each is ready to drop into `.claude/skills/<name>/SKILL.md`. `docs-audit` and `test-vectors` reference the tested scripts in the appendix (copy them to `scripts/`).

### 6.1 `.claude/skills/adr/SKILL.md`

````markdown
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
````

### 6.2 `.claude/skills/test-vectors/SKILL.md`

````markdown
---
name: test-vectors
description: Writes or updates Ouril's language-neutral JSON rule test vectors in core/test-vectors/ from a variant spec or a worked example in docs/game/rules.md, then checks them for seed conservation and bookkeeping errors. Use when adding or changing a game rule, implementing a variant, settling a "To confirm" rule, turning a bug report or game record into a regression test, or when the user asks for test vectors, rule tests or golden games.
argument-hint: "[variant-id[@version]] [area | rules.md example | spec parameter]"
allowed-tools: Bash(python3 ${CLAUDE_SKILL_DIR}/scripts/check_vectors.py *)
---

# Rule test vectors

Vectors are the executable form of the variant specs and the contract every platform must pass. **Every expected value must trace to a doc** (a worked example or a spec evidence row), never to general Oware knowledge and never to what the engine currently outputs. If the spec says `null`, unknown or **To confirm**, stop and ask, unless the user agrees to a `to-confirm` vector using the documented `base.oware` default.

## Input

`$ARGUMENTS`: a variant ID (with an optional `@version`) and what to cover, for example `cv.standard capture`, `cv.standard rules.md example 5`, `cv.standard feeding_overrides_single_seed_rule`. If it's empty, cover whatever rule the conversation is changing.

## Read first

1. `docs/architecture/test-vectors.md`: format, event types, error codes and the rules for writing vectors. It's authoritative. If anything here disagrees with it, follow that doc.
2. The variant spec `docs/game/variants/<country>/<variant>.md`: `resolved`, `confidence`, the evidence table and **Rule interactions**. For `cv.standard`, also read `docs/game/rules.md` and its worked examples.
3. `docs/architecture/versioning.md#variant-versions`.
4. The existing vectors for that variant: `ls -R core/test-vectors/<id>/`. Extend rather than duplicate.

## Released or not?

Vectors for a **released** variant version are never edited. A version is released once an app build using it has shipped. Ask the user if you're unsure. Before the MVP ships, `cv.standard@1` is unreleased and may change.

For a released version, the rule change needs a new version first (`rule-change` skill). Then copy the affected vectors, set `variants` to the new version, and change only the copies.

## Plan coverage first

Before writing, list `source → vector path → tags`, and show the list to the user if it has more than about 5 entries. Coverage rules:

- every worked example in `rules.md`: at least one vector
- every evidence-table row in the spec: at least one vector
- every case in the spec's **Rule interactions**: at least one vector
- every error code the variant can produce: at least one `expect_error` vector
- confidence `low`, `unknown` or `conflict`, or text marked **To confirm**: tag `to-confirm`

## Write each vector

1. **Path:** `core/test-vectors/<variant-id>/<area>/<kebab-scenario>.json`. Areas: `sowing`, `capture`, `single-seed`, `feeding`, `grand-slam`, `end`, and `games/` for recorded full games only. Use `_common/` only when the behaviour is identical for every variant listed in `variants`.
2. **`id`** is the path under `core/test-vectors/` without `.json`. **`description`** is one sentence a human can check against the source. **`source`** is the repo-relative doc path plus heading anchor, for example `docs/game/rules.md#5-grand-slam-with-extra-turn`.
3. **Setup:** the smallest explicit position that isolates the rule. Pits are indexed South `0..n-1`, then North `n..2n-1`. Sowing goes to the next index, wrapping around. `stores` is always `[south, north]`. Fill unused pits and stores so `sum(pits) + sum(stores) = 2 × pits_per_side × seeds_per_pit`.
4. **Derive the result by hand**, step by step, in your reasoning (not in the file):
   - one `sow` per seed, in order
   - on a lap, a `skip_origin` event instead of sowing into the origin pit
   - capture check on the last pit, then backwards while on the opponent's side and the count is in `capture_counts`
   - then grand slam, feeding and end-of-game rules from the spec
   - `to_move` comes from the rules: an extra turn keeps the same player
5. **Assert** what the vector is about: `expect_legal_moves` (sorted, before the move) when legality is the point; the complete, ordered `events`; and `pits`, `stores`, `to_move`, `status` after the move. Use `expect_error` instead of `expect` for illegal moves.
6. Keep each vector to one rule and 1–3 steps.

## Check

Run the checker on the files you wrote and fix every problem until it's clean:

```bash
python3 ${CLAUDE_SKILL_DIR}/scripts/check_vectors.py "$(git rev-parse --show-toplevel)" <files or dirs>
```

The checker isn't a rules engine. It catches the following, but not wrong rule logic:

- format and id/path mismatches
- unknown events or error codes
- wrong board size and seed-conservation errors
- sow counts that don't match the moved pit
- store gains that don't match captures
- legal moves on the wrong side
- bad `source` anchors

When the engine exists, also run `cargo test -p ouril-engine`, and run the platform runners when they exist. **If the engine disagrees with a vector, don't edit the vector to match.** Re-derive it from the spec: either the engine has a bug, or the spec is ambiguous and you ask the user.

## Report

- vectors added or changed, each with its source and tags
- how many are tagged `to-confirm`
- spec rows or examples still without vectors
- any ambiguity you found, phrased as an open question to add to the spec

## Pitfalls

- With 12+ seeds the origin is skipped, so the number of `sow` events still equals the seeds picked up, plus one `skip_origin` per lap. See rules.md example 3.
- Chain captures stop at the mover's own side or at the first pit not in `capture_counts`. List captures in capture order: the last pit first.
- Legal moves must apply the single-seed rule **and** the feeding obligation together (rules.md examples 5–6).
- Grand slam (`captures_all_extra_turn_must_feed`) emits `grand_slam` and `extra_turn`, and `to_move` stays with the mover. If the store reaches `win_threshold`, `game_over` replaces the extra turn. test-vectors.md doesn't fix the order of `grand_slam` and `extra_turn` relative to the `capture` events: ask, then record the answer in that doc.
- Never fill in expectations from engine output ("golden master") for rule vectors. Only `games/` replays of real recorded games may use known results.
````

### 6.3 `.claude/skills/docs-audit/SKILL.md`

````markdown
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

## Pass 2: contradictions

Read the docs in scope and check the invariants in [invariants.md](invariants.md). Typical problems:

- the same parameter, event, endpoint, phase or term named or valued differently in two places
- a doc treating a **Proposed** ADR as decided
- statements that drifted after an ADR changed (for example, the guest rule must always say "…unless they opted into telemetry", per ADR 0014)

For each finding, record both locations as `file:line`, quote both sentences, and propose which doc should change. Don't edit until the user chooses.

## Report

A table: `severity | where | problem | proposed fix | fixed?`. Then a one-line summary of the checker output before and after your fixes. Don't commit.
````

The bundled `invariants.md` (draft, first version):

```markdown
# Cross-doc invariants

- Guest network rule: wherever it appears, it says guests make no network requests **unless they opted into telemetry** (ADR 0014).
- Variant parameters: names and values match exactly in docs/game/variants/README.md (source of truth), docs/architecture/engine.md (VariantConfig), .claude/skills/variant-research/template.md, and every spec's front matter. Known drift: `endless_cycle` (engine.md) vs `endless_cycle_outcome`; no parameter for the no-capture move limit.
- Events and error codes: docs/architecture/test-vectors.md lists the same set as engine.md's MoveEvent and MoveError. Known drift: grand_slam, extra_turn and collect_remaining are missing from engine.md.
- Variant identity: anything stored or exchanged uses `id@version` / `{id, version}` (versioning.md). engine.md's VariantConfig has no version field.
- Phases: names and numbers match in roadmap.md, mvp.md, the feature docs and backend.md's scope table.
- API: endpoints in backend.md, data-and-sync.md and versioning.md agree (/v1/meta, /v1/sync/push, /v1/sync/pull, auth).
- Toolchain: CONTRIBUTING.md and architecture/monorepo.md list the same tools.
- ADR status: a doc may call something "decided" only if its ADR is Accepted (0006 is Proposed).
- Glossary: docs use glossary terms (pit, store, sowing, lap, origin pit, grand slam, feeding).
- CLAUDE.md "Where things are decided" rows point at existing files that really are the source of truth.
```

---

## 7. Rejected ideas (and why)

| Idea | Why not |
|---|---|
| `scaffold-monorepo` skill | One-time job. Do it in plan mode with ADRs for the open questions (commit generated bindings? `apps/site`?), then record recipes with `/run-skill-generator` and `/verify`. A skill would rot after first use. |
| `bindings-regen` skill | Deterministic, so it belongs in a `just bindings` recipe, the H2 hook and the CI drift check. The knowledge part (UniFFI and wasm type mapping) lives in `core-api-change`. |
| Custom security-review skill | Bundled `/security-review`, `security-guidance`, `claude-security` and Trail of Bits' `rust-review` cover it. Project-specific privacy concerns are in `privacy-review`. |
| Commit or changelog skill | CLAUDE.md forbids committing unless asked. Changelogs belong in CI tooling (e.g. git-cliff or release-please from Conventional Commits). Store release notes are part of `store-release`. |
| Dependency-update skill | Dependabot is already configured. Add `cargo deny` in CI. |
| Glicko-2 / leaderboard / matchmaking skills | One-time implementations, years away (Phase 4–5), designed through ADR 0006. Revisit only if seasonal resets or rating recalcs become a recurring ops task. |
| Generic Rust, Swift, Kotlin or TS style skills (custom) | Better third-party and first-party skills exist (§3.2), and linters enforce style. |
| `glossary-sync` skill | Folded into `docs-audit` (invariants) and `translation-review`. |
| Separate `bump-variant-version` skill | Folded into `rule-change`. The bump is a branch of the same decision tree. |
| Agent-type `Stop` hook for the Definition of Done | Experimental, adds latency and cost to every turn. `pr-ready` is on demand. |
| Ops/incident runbook skill | Nothing to operate yet. Revisit at launch with the `sentry` plugin. |
| Apple privacy-label automation | Not in the public App Store Connect API, so keep answers in a reviewed repo file (`store-release`). |
| Vendoring superpowers project-wide | Its broad triggers compete with the docs-first/ADR workflow. It's fine as a personal choice. |

---

## 8. Sources

Official Claude Code and Anthropic documentation (fetched 2026-10-03):
- Skills: https://code.claude.com/docs/en/skills (also served at /en/slash-commands)
- Subagents: https://code.claude.com/docs/en/sub-agents
- Hooks reference: https://code.claude.com/docs/en/hooks
- Features overview (skill vs hook vs subagent vs rules): https://code.claude.com/docs/en/features-overview
- Memory and `.claude/rules`: https://code.claude.com/docs/en/memory
- Settings: https://code.claude.com/docs/en/settings · Settings reference: https://code.claude.com/docs/en/settings-reference
- Plugins reference: https://code.claude.com/docs/en/plugins-reference · Plugin marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Skill authoring best practices: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices
- Agent Skills standard: https://agentskills.io
- Official plugins: https://github.com/anthropics/claude-plugins-official (marketplace.json; READMEs of security-guidance, claude-security, claude-md-management, hookify, pr-review-toolkit, feature-dev, commit-commands, skill-creator, plugin-dev, claude-code-setup, code-review, the *-lsp plugins)
- Anthropic skills: https://github.com/anthropics/skills

Community and vendor skills (GitHub API metadata 2026-10-03):
- https://github.com/obra/superpowers · https://github.com/trailofbits/skills · https://github.com/chrisbanes/skills · https://github.com/twostraws/SwiftUI-Agent-Skill · https://github.com/AvdLee/SwiftUI-Agent-Skill · https://github.com/dpearson2699/swift-ios-skills · https://github.com/superagents-lab/xcode27-skills · https://github.com/artemnovichkov/xcode-skills · https://github.com/aldefy/compose-skill · https://github.com/dpconde/claude-android-skill · https://github.com/addyosmani/web-quality-skills · https://github.com/GoogleChrome/modern-web-guidance · https://github.com/leonardomso/rust-skills · https://github.com/actionbook/rust-skills · https://github.com/greenstevester/fastlane-skill · https://github.com/warunacds/apple-asc-mcp · https://github.com/mrKanoh/claude-wcag-accessibility-skill · https://github.com/wshobson/agents · https://github.com/affaan-m/ECC · https://github.com/khalilbenaz/claude-skills-collection · https://github.com/expo/skills
- Curated lists: https://github.com/VoltAgent/awesome-agent-skills · https://github.com/ComposioHQ/awesome-claude-skills · https://github.com/BehiSecc/awesome-claude-skills · https://github.com/hesreallyhim/awesome-claude-code
- Xcode 27 agent skills: https://blakecrosley.com/blog/xcode-27-agent-skills-export · https://dev.to/arshtechpro/wwdc-2026-xcode-27-ships-with-apples-own-agent-skills-what-they-are-and-how-to-use-them-3g2
- xcstrings + Claude Code: https://dev.classmethod.jp/en/articles/xcstrings-ai-localization-with-claude-code/
- Toolchain versions: crates.io API (uniffi, wasm-bindgen, wasm-pack, sqlx-cli, cargo-ndk, proptest); https://github.com/wasm-bindgen/wasm-bindgen · https://github.com/mozilla/uniffi-rs

**Uncertain or unverified:**
- the unit of the skill-listing budget ("1% of the context window")
- the exact `xcrun agent skills export` output and its license terms (not run locally)
- the quality of third-party skills (assessed from READMEs and frontmatter, not from eval runs; use `skill-creator` before adopting)
- the plugin-ID format in `enabledPlugins` (confirm via `/plugin`)

---

## Appendix A: `docs-audit/scripts/check_docs.py` (tested)

Now at `.claude/skills/docs-audit/scripts/check_docs.py` (later extended to skip third-party skills listed in `skills-lock.json`).
- **Run on the real repo:** 43 files, 0 errors, 2 warnings, 73 ms.
- **Mutation test on a scratch copy:** a broken link, a misspelt anchor, an ADR index status mismatch, a second `cv` default and spec/catalog status drift were all reported, with exit 1.

Copy it verbatim into `.claude/skills/docs-audit/scripts/`. It's stdlib-only Python 3 and read-only.

## Appendix B: `test-vectors/scripts/check_vectors.py` (tested)

Now at `.claude/skills/test-vectors/scripts/check_vectors.py`. It reads board shape and version from the spec front matter.
- **Run on the example vector from `docs/architecture/test-vectors.md`:** passes.
- **Run on a corrupted copy:** reported all six injected faults (id ≠ path, bad anchor, north pit in South's legal moves, seed conservation, sow count ≠ seeds, store gain ≠ captures).

Copy it into `.claude/skills/test-vectors/scripts/`.

## Appendix C: `.claude/hooks/docs-check.sh` (tested)

Now at `.claude/hooks/docs-check.sh` (rewritten to match exact file paths with `awk` instead of GNU `grep`).
- It needs `jq` and `python3`, both present.
- **Tested with simulated `PostToolUse` input:**
  - an edit to a doc with a bad anchor → exit 2 with just that file's error
  - an ADR edit → the index mismatch for that ADR
  - an unrelated clean doc → exit 0
  - a non-Markdown file → exit 0

Register it with the `settings.json` snippet in §5.1.

## Open questions

- When to add the later-phase skills (§4.6 onwards). Add each one once the code, `just` recipes and decisions it depends on exist.
- Verify the unconfirmed claims above before relying on them.
