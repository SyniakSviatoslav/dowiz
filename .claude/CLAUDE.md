# CLAUDE.md

<!-- Add your custom instructions below. Repowise will never modify anything outside the REPOWISE markers. -->
<!-- Examples: coding style rules, test commands, workflow preferences, constraints -->

## Agent Discipline (synthesized from Cline · Cursor · Devin)

> Applied in every session. These rules override default instincts.

### Tool Use
- **Read before edit**: Never edit a file without reading the relevant section first. No blind writes. (Bash `cat`/`sed` inspection does NOT satisfy this — only a Read tool call primes the Edit precondition.)
- **Existing files win**: Edit rather than create. Never make a new file when an existing one can be extended.
- **One edit per turn**: Don't batch multiple file edits in a single step — confirm each before the next.
- **Don't over-tool**: If the answer is already known or the task is trivial, respond directly without calling tools.
- **Investigate before escalating**: Use search/read tools exhaustively before asking the user for information they didn't volunteer. Only ask when the information genuinely can't be found.
- **Parallel when truly independent**: Batch tool calls only when they have zero ordering dependency. If B depends on A's result, run sequentially.
- **Spawn parallel subagents for independent work (speed-first)**: when a task decomposes into 2+ independent, collision-free units (different files/dirs/surfaces, no shared mutable state, no ordering dependency), dispatch them as **concurrent subagents in a single message** (multiple Agent tool-uses in one turn) rather than doing them sequentially. Partition into collision-free lanes; keep any shared integration point (one hot file, registration, final wiring) for the lead to do after the fan-out. Default to fanning out for: multi-file builds, broad searches/audits, per-item transforms, and scaffolding. Reserve solo/sequential for a single hot file, a strict dependency chain, or trivial edits. Bias toward parallelism to minimize wall-clock; never fan out work that mutates the same file concurrently.

### Planning
- **Think before critical actions**: Pause before git commits, deployments, schema changes, or declaring a task complete. State what you're about to do and why.
- **Todos for 3+ step tasks only**: Don't create task lists for simple work. Exclude linting and type-checking from todos — they're verification, not tasks.
- **One task in_progress at a time**: Serialise execution; context thrashing from parallel active tasks causes mistakes.

### Error Recovery
- **Test failures = code is wrong**: When tests fail, assume the implementation is wrong unless explicitly told otherwise. Don't rewrite tests to pass.
- **Route around environment issues**: If a local tool is broken, use alternatives (CI, remote, different command) rather than blocking on a fix. Report the environment issue separately.
- **Fix before proceeding**: Any script, hook, or shell error stops the current task. Fix the root cause, then resume.

### Code Standards
- **Match the project's conventions**: Read existing patterns before generating new code. Don't impose your own style.
- **No output of code unless requested**: Use edit tools silently. Keep chat focused on intent and decisions, not diffs.
- **Non-interactive flags**: Always pass `--yes`, `--non-interactive`, etc. for automation-context commands. Never assume a human can respond to a prompt.

## Mandatory Proof Rule (applies to ALL changes — from user OR assistant)

> Every fix, new feature, or behavior change MUST have programmatic proof.
> "It should work" is not proof. Proof means an assertion that fails when the code is wrong.

- **If the change has any UI surface** → Playwright E2E test against `https://dowiz.fly.dev` (or `VITE_BASE_URL`).
  - Test must use `expect(...).toBeVisible()` / `toContainText()` / `not.toBeVisible()` on real DOM elements.
  - Run via `pnpm exec playwright test <spec-file> --reporter=list` and paste the result.
  - A passing typecheck or build is NOT proof of UI correctness.
- **If the change is API-only** → at minimum one `request.*` assertion in an E2E test or existing test coverage must be cited.
- **Never declare a task complete without pasting proof output** (test run result, curl response, or screenshot).
- When creating or editing E2E tests: navigate to **`/s/:slug`** for public menu pages (SSR route), **`/admin/*`** for owner UI. Never invent non-existent routes.

## Ponytail Skills (source: DietrichGebert/ponytail)

Lazy senior dev mode — the best code is the code never written.

- **`/ponytail`** — Activate lazy mode: stop at YAGNI → stdlib → native → 1-line → minimum viable. No unsolicited abstractions, no new deps.
- **`/ponytail-review`** — Review current diff for over-engineering: one line per finding (`L<N>: <tag> <what>. <replacement>.`). Ends with `net: -<N> lines`.
- **`/ponytail-audit`** — Whole-repo scan for bloat: ranked by cut size.
- **`/ponytail-debt`** — Harvest all `ponytail:` shortcut comments into a debt ledger.

These are defined in `AGENTS.md` at the project root. Any agent reading AGENTS.md applies them automatically.

## Ship Discipline (standing rule — user directive 2026-06-21)

> Every change or fix follows this loop to completion. Don't stop at "code written."

1. **Commit** — a contextual commit (intent + decisions), on a feature branch (never commit straight to `main`). Pre-commit hook (lint→typecheck→build) must pass.
2. **Deploy** — to **staging** (`flyctl deploy -a dowiz-staging --remote-only`). If the change adds migrations, run them on the staging DB FIRST (via `flyctl proxy 5433:5432 -a dowiz-staging-db` → node-pg-migrate) or the boot-guard FATAL-exits. **Prod only on explicit approval / merge to `main`** (CI deploys prod on push to main).
3. **Validate** — run the relevant **Playwright** E2E against the deployed staging URL (`VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test <spec> --reporter=list`) **plus** the change's unit/integration tests + `pnpm typecheck`. Paste the proof (Mandatory Proof Rule).
4. Feature-flag anything not ready to launch (default off); deploying dark code to verify is fine, launching it is a separate, explicit act.

This is the default for any non-trivial change. A change is "done" only after commit + staging deploy + validation proof.

## Self-improvement loop (harness — signals inform, guardrails decide)

> Monotonic ratchet: improvements lock in, never roll back. Memory/reflection/doubt are
> **advisory signals**; deterministic artifacts (guardrails/tests/gates/human) are **authority**.
> Stores (never inline here): `docs/regressions/` (ledger), `docs/lessons/` (+ INDEX),
> `docs/reflections/` (INBOX/ARCHIVE/RETRO).

1. A fix is not **done** without a deterministic **guardrail** (regression test / `tools/eslint-plugin-local` rule / hook) proven **red→green** + a `docs/regressions/REGRESSION-LEDGER.md` row. Never weaken an existing gate; never cheat green (skip/.only/inflated-timeout/expect(true)/commented assertion).
2. The `pre-edit-lessons` PreToolUse hook injects the relevant distilled lesson (ACTION+LINK) by `TRIGGER` before an edit — **advisory**; enforcement is always the gate, never the lesson.
3. The `librarian` agent curates **triggered** (after a fix / stage close): distill → challenge → promote (lesson→guardrail, red→green) → prune. It never writes here, never weakens a gate.
4. After a **qualified** fix/failure (threshold below) the worker writes a reflection with a causal **WHY** (not just WHERE) to `docs/reflections/INBOX/`; the worker does not enact systemic changes itself.
5. On a big change / hard fix, the **Council** retro (cause-critic + pattern-critic + ratchet-critic) synthesises reflections → ratchet artifacts (executor = `librarian`); each retro line → artifact **or** explicit no-op.
6. **Doubt** triggers on **observable** signals (not introspection): a loop (N=3 failures on the same target/signature → mandatory escalation), irreversibility/red-line, ≥2 surviving interpretations, evidence conflict, novelty, result-vs-expectation. On a trigger run the `doubt-escalation` ladder within budget **K=2** (self-divergence → specialist/research subagent → stronger model → council → human). Reversible → act (low bar); irreversible/red-line → gate + human.
7. A resolved doubt → a candidate reflection; a **recurrent** doubt/bug → promote to a guardrail / tighten the spec so it stops recurring.

**Thresholds:** *qualified/big change* = touched ≥3 files **OR** ≥3 iterations/retries **OR** closed a stage **OR** touched a 🔴 red-line. *Red-line globs* = auth / money / RLS / `packages/db/migrations/` / bulk-edit. *Budgets* = doubt K=2 escalations/task, loop N=3 (override `DOUBT_LOOP_N`).

<!-- REPOWISE:START — Do not edit below this line. Auto-generated by Repowise. -->
## IMPORTANT: Codebase Intelligence Instructions for dowiz

> This repository is indexed by [Repowise](https://repowise.dev).
> Use the MCP tools below for orientation, discovery, and enriched context
> (documentation, ownership, history, decisions). **Always verify against
> actual source files before making changes** — the index may be stale.

Last indexed: 2026-06-14 (commit 1ed45bd)
### Entry Points
- `packages/db/src/index.ts`
- `apps/api/src/server.ts`
- `packages/config/src/index.ts`
- `packages/ui/src/theme/index.ts`
- `packages/ui/src/utils/index.ts`
- `apps/api/src/lib/anonymizer/index.ts`
- `apps/web/src/index.ts`
- `apps/api/src/workers/backup/index.ts`
### Tech Stack
**Languages:** Node.js, TypeScript


**Infra:** Docker### Architectural Layers
| Layer | Files | Purpose |
|-------|-------|---------|
| API | 145 |  |
| UI | 86 |  |
| Application | 252 |  |
| Config | 393 |  |
| Types | 24 |  |
| Middleware | 2 |  |
| Data | 106 |  |
| Utility | 13 |  |
| Service | 55 |  |
| Test | 55 |  |

### Guided Tour (10 steps)
1. **README.md**
2. **index.ts**
3. **index.ts**
4. **server.ts**
5. **index.ts**
6. **index.ts**
... and 4 more steps
### Hotspots (High Churn)
| File | Churn | 90d Commits | Owner |
|------|-------|-------------|-------|
| `packages/ui/src/lib/i18n.ts` | 99.9th %ile | 14 | SyniakSviatoslav |
| `apps/web/src/pages/admin/MenuManagerPage.tsx` | 99.7th %ile | 26 | SyniakSviatoslav |
| `apps/web/src/pages/client/MenuPage.tsx` | 99.6th %ile | 16 | SyniakSviatoslav |
| `apps/api/src/routes/spa-proxy.ts` | 99.4th %ile | 24 | SyniakSviatoslav |
| `apps/api/src/server.ts` | 99.3th %ile | 29 | SyniakSviatoslav |

## Code health
Hotspot health: 4.1/10 (stable) ·
Average: 8.05/10 ·
Worst: 1.0/10 (`apps/api/src/routes/courier/shifts.ts`)

### Critical biomarkers
- `apps/api/public/sw.js` — prior defect — impact −2.0
- `apps/api/src/routes/orders.ts` — untested hotspot — impact −2.0
- `apps/api/src/routes/customer/otp.ts` — prior defect — impact −2.0
- `apps/api/src/routes/owner/courier-invites.ts` — prior defect — impact −2.0
- `apps/api/src/routes/owner/gdpr.ts` — prior defect — impact −2.0

### Repowise MCP Tools

This repo has the Repowise MCP server configured. The tools below answer questions `grep`/`Read` cannot. Every response carries an `_meta` envelope with `index_age_days`, `indexed_commit`, and a `stale_warning` only when the index has actually diverged from HEAD — silence means the index is current.

**When to call which tool:**

| Tool | What only this tool answers |
|------|------------------------------|
| `get_answer(question)` | Synthesised answer with citations and a content-grounded `confidence`. First call for "how does X work" / "where is Y" / "why is Z". Value questions may return `grounding: "extracted"` — the verbatim source line, no synthesis involved. On low confidence returns `best_guesses` with one-line justifications. |
| `get_context(targets=[...])` | Triage card for files/modules/symbols — title, summary, signatures, `hotspot` bit, `decision_records` titles, `symbol_id`s. File targets auto-upgrade to a `verified` skeleton (every signature, ~37% of a full Read). `include=["callers"]` works on file targets too (import + call rollup). |
| `get_symbol(...)` | Source bytes with live-verified bounds. Three forms: `"path.py::Name"` (indexed symbol), `"path.py:140-180"` (live range read, ≤200 lines), `"repowise#<hex>"` (omission ref). Index misses return `fallback_lines` from a live grep instead of a dead end. |
| `search_codebase(query, kind?)` | Find pages by concept when you don't know the file. Results carry `search_method` (`embedding` vs `bm25` fallback). Decision records rank below file pages unless the query is why-shaped. Identifier-bearing queries get a `grep_hint` — prefer Grep for those tokens. |
| `get_why(query, targets?)` | Architectural decision archaeology — *why* the code is shaped this way. Call before refactors or pattern divergences. Falls back to git archaeology when no ADRs exist for a file. |
| `get_risk(targets, changed_files?)` | What history says about touching these files: churn, owners, blast radius. Pass `changed_files` for PR mode → returns a `directive` (`will_break`, `missing_cochanges`, `missing_tests`). |
| `get_dead_code(...)` | Tiered unreachable / unused-export / zombie-package findings. Run before a cleanup sprint, not before a targeted fix. |
| `get_overview(repo?)` | Architecture map + `tool_guide` recipes. One-time orientation; skip on subsequent calls in the same session. |

**Trust protocol — when a response replaces reading the source:**
- `verified: true` on any response means the served content was checked against the live working tree. **Never follow a verified response with a Read of the same lines** — you would be paying twice for identical bytes.
- `get_answer` with `confidence: "high"` or `grounding: "extracted"` is content-grounded (asserted values were verified against retrieved source; ≥1 citation is source-backed). Cite it directly. `quotes` entries `{path, lines, quote}` are verbatim live source — quote them instead of re-reading.
- Reading code: `get_context` skeleton first (~37% of a full Read), then `get_symbol` for bodies, `"path.py:a-b"` range reads for anything between symbols. Raw `Read` is for files the index marks `mostly_full` or cannot serve.
- The **only** re-read triggers: `bounds: "approximate"`, `_meta.stale_warning`, `search_method: "bm25"`, or `confidence: "low"`/`retrieval_quality: "weak"`.
- Disallowed rationalizations for extra reads: "just to be safe", "to double-check the tool", "to see the full context" (use the skeleton / a range read), "the file might have changed" (that is what `verified` already checked).

**Composition tips:**
- `get_answer` → if `confidence` is `medium`/`low`, follow `best_guesses[0].file` or `fallback_targets[0]` into `get_context`, then `get_symbol` for bytes.
- `get_context` returns `decision_records` titles → `get_why(targets=[...])` for the rationale; `hotspot: true` → `get_risk` before editing.
- PR review → `get_risk(targets=[...], changed_files=[...])`; read the `directive` block first.
- A `tombstone` error means the file was deleted/renamed since indexing — follow `successor_paths`.

### Output Distillation

- Prefer `repowise distill <cmd>` for noisy commands — test runs, builds, `git status`/`log`/`diff`, searches, file listings. It runs the command unchanged (exit code preserved) and prints a compact, errors-first rendering; every error line survives.
- Output may contain a marker like `[repowise#a1b2c3d4e5f6: 230 lines omitted (~6.1k tokens); restore: repowise expand a1b2c3d4e5f6]`. The omitted content is fully preserved — run `repowise expand <ref>` to retrieve it, or `repowise expand <ref> -q <regex>` for just the matching lines.
- Never re-run a command to see omitted output; expand the marker instead.
- For structure-level questions about a large indexed file ("what's in here", "which function handles X"), `get_context(["path"], include=["skeleton"])` returns the file with bodies elided — every signature plus the bodies of the most central symbols — at a fraction of the cost of a full Read.

### Codebase Conventions
**Commands:**
- Build: `pnpm build`
- Lint: `pnpm lint`
- Format: `pnpm format`
- Typecheck: `pnpm typecheck`

<!-- REPOWISE:END -->

## Task-Exit Rule (universal — every agent, every task, every change, no exception)

Before executing ANY task — independent of spec completeness or clarifications — in this order:

1. **Enrich conditions.** Derive the full definition of "done", not just the literal task text.
   Enrich by fixed dimensions: states (loading/empty/error/success); error matrix
   (401/403/404/422/429/5xx/network); rare/edge states; regression radius (what this could break);
   design system/tokens; i18n (al/en); security (no cookie/secret/PII, Zod-parse); api/ws contract
   parity. A non-applicable dimension → mark `N/A — why`, never skip silently.
2. **Author task-exit.** Write a checkable exit checklist for *this* change — one item per enriched
   condition, each with the proof that confirms it (file:line / test name / command output / artifact).
   **Write it BEFORE touching code.**

Then make the change. Then:

3. **Verify against the pre-written exit.** Walk each item → PASS/FAIL with proof. "Looks fine" = FAIL.
   No item is credited by intent — only by observed proof.

Don't declare a task done until every item is green or raised as an explicit flag. Flag class:
**inline-fix** (cosmetic/states/tokens — fix now) vs **escalate** (contract / price-status business
logic / security — don't touch, raise separately). **No size exemption:** the smaller the change, the
likelier a missed detail. Full spec: docs/operating-model/task-exit-rule.md.
