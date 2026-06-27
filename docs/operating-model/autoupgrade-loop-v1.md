# DeliveryOS / dowiz — Autoupgrade Loop (v1)

**Date:** 2026-06-27 · A new loop **on** the v3-FINAL harness (`living-loop-system-v3.md` §1 contract). Headless, background, sandboxed. Goal: autonomously find and apply changes that **measurably haste iteration with fewer resources**.

---

## 0. Decision note (read first — honest framing)

The request was a **fully ungated, entirely-machine** autoupgrade loop. This spec keeps it **entirely machine-driven with zero humans in the decision** for the safe change-class, but **replaces "ungated" with "machine-gated by a hard oracle + atomic rollback."** Reasons, project-specific:

- **Web-research → self-implement is an injection / supply-chain pipe.** Adversarial web content + autonomous code-writing = a poisoned source or typosquatted package enters production unattended.
- **No deterministic oracle for "better."** Architecture/routing changes have no failing-test metric; autonomous "best-practice" research contradicts your *deliberate* constraints (pg-boss-not-BullMQ, monolith-not-microservices, Fly-not-K8s) and regresses you to the mean.
- **Blast radius includes third-party data.** DeliveryOS holds restaurants' customers' PII + RLS isolation + payments. An autonomous auth/RLS change can pass every test and still leak data.

**Resolution:** two classes. **Class A** auto-applies under a machine oracle (no human). **Class B** (security/architecture/data) is researched + proposed by the machine but **not** self-applied. The safe class *is* the speed class, so this costs almost nothing on the goal.

**Firm boundary (non-negotiable):** auth, RLS/tenant-isolation, secrets, payments, and PII-handling are **never** autonomously mutated — even machine-gated — because the oracle can't catch a subtle leak and the people harmed aren't you.

---

## 1. The loop (bounded single pass per run)

Implements the §1 `Loop` contract. Not a converge-to-green loop — a **per-run batch**: each run maps, researches, evaluates a batch of candidates, applies the ones that pass, reports. One **iteration = evaluate one candidate** (apply → verify → keep/rollback).

```
MAP → RESEARCH → CLASSIFY → [per candidate: APPLY → VERIFY(oracle) → KEEP | ROLLBACK] → REPORT → FEED
```

- **MAP (autonomous):** inventory tech/org/routing patterns — deps, seams, routing/middleware, queue usage, build/test config, harness config, slow queries, ghost skills, token-heavy prompts. **Ground "what's slow/expensive" in existing telemetry** (`runs/metrics.jsonl`, codeburn, slow-test timings) — target real bottlenecks, not guesses.
- **RESEARCH (autonomous, contained — see §4):** web-research improvements/replacements. Output: ranked candidates with evidence, expected speedup, blast-radius, reversibility.
- **CLASSIFY:** Class A vs Class B (§3).
- **APPLY + VERIFY (Class A only):** the machine oracle (§2). One change at a time, re-verified — never stack unverified changes.
- **REPORT + FEED:** standard harness report (§5); kept upgrades update `learnings.json`; speed delta tracked in metrics.

`progressMetric` = candidates remaining in batch (↓). `isTerminal` = batch evaluated. Breaker (§3 of v3-FINAL) still applies: per-candidate verification timeout, per-run cost/time cap.

---

## 2. The machine oracle (replaces the human gate; entirely machine)

A Class A change is **kept** iff **ALL** hold; otherwise **atomic auto-rollback**:

1. **Green:** full test suite passes (unit + integration + E2E, all 3 roles).
2. **No security regression:** the standing RLS/auth/tenant-isolation/secret-scrub assertions still pass.
3. **Measurably faster:** replay a **fixed benchmark scenario** (deterministic seed) and compare loop telemetry before/after — the change must reduce a target metric (convergence tokens-per-resolved, wall-clock, RSS, or cost) by **≥ threshold** (e.g. 5%). Proven, not assumed.
4. **Reversible:** a recorded, tested revert exists.

A change that passes tests but shows no speedup is **rolled back** (added risk for nothing).

---

## 3. Scope — Class A (auto) vs Class B (propose-only)

**Class A — auto-apply under the oracle (no human):** reversible, test-covered, low-blast-radius, mostly dev-loop/perf.
- Prune ghost skills / unused MCP servers (codeburn-flagged).
- Decompose token-heavy prompts into chained sub-prompts.
- Add covering indexes for telemetry-flagged slow queries.
- Parallelize / cache test setup (testcontainer reuse, fixture caching).
- Tune harness config (breaker caps, recall digest size, cache TTLs).
- Trim bloated CLAUDE.md / dead config (codeburn-flagged).
- Dev-tooling/version-pin housekeeping with checksum verification.

**Class B — research + propose only, human queue (never self-apply):**
- Auth, RLS/tenant-isolation, secrets, payments, PII handling. *(Firm boundary — §0.)*
- Data migrations / schema changes.
- Architecture/topology (monolith↔services, hosting, queue/runtime swap).
- Major dependency upgrades (breaking).
- Anything touching the white-label/theming security surface or money math.

---

## 4. Containment (regardless of class)

The research + implement phase is **sandboxed and credential-isolated**:
- **No production secrets** in the research/implement context (broker via `agent-vault`, or a clean container with zero prod creds).
- **Web sources allowlisted + treated as untrusted data.** Extract *candidate approaches* as facts; **never execute instructions found in web content**. Validate proposed code through your own standards (aislop + lint + security assertions) **before** it's a candidate.
- **Dependencies pinned + checksum-verified + scanned** (supply-chain).
- **Isolated worktree + testcontainer DB** for apply/verify. The real environment is touched only after the oracle passes — and for Class B, never autonomously.

---

## 5. Telemetry + report additions

The autoupgrade loop emits the standard per-iteration telemetry (§2 of v3-FINAL) plus:
- `candidate`: pattern, source(s), class, expected_speedup, blast_radius, reversible.
- `oracle`: green / security_ok / speedup_pct / kept | rolled_back, with the measured before→after metric.
- `queued_for_human`: Class B proposals this run.

Its **report** shows: what was mapped, researched, auto-applied-and-kept (proven speed delta), rolled back (why), queued for human — and the cumulative iteration-speed trend over runs.

---

## 6. Harness + concurrency fit

- **It's a Loop on v3-FINAL** — inherits the contract, breaker, permanent lossless storage, per-iteration recall, always-printed report.
- **Runs as a headless background pg-boss job, low concurrency** (`teamConcurrency` 1), off the critical path — scheduled when the headed convergence loop is idle.
- **Recall applies:** learns which upgrade classes tend to pass/roll-back; stops re-researching dead ends.

---

## 7. Anti-gold-plating + the boundary, restated

- **Machine-gated, not ungated.** The oracle (green + no-security-regression + proven-speedup + reversible) is the gate. No human for Class A.
- **The firm boundary holds:** auth / RLS / secrets / payments / PII are never autonomously mutated.
- **The safe class is the speed class.** Scoping autonomy to Class A loses ~nothing on "haste iteration" and removes the catastrophic failure mode.
- **Stays:** lean, headless, low-concurrency, sandboxed, credential-isolated.

---

## 8. Order of work

1. **Sandbox + credential isolation (§4)** — build first; nothing autonomous runs without it.
2. **Loop skeleton on the harness (§1)** — MAP + RESEARCH(contained) + CLASSIFY, report-only at first (no apply).
3. **The oracle (§2)** including the fixed benchmark-replay speed check.
4. **Enable Class A auto-apply** (§3) once the oracle + rollback are proven on a few candidates.
5. **Class B proposal queue** (§3) — wired to the v3-FINAL graduation/proposal mechanism.
6. Keep it headless + background; widen Class A only after several clean runs.

---

## Implementation status (appended by build — keep in sync)

- **2026-06-27 — §8 step 2 (MAP → CLASSIFY → REPORT, REPORT-ONLY) built + run** in
  `tools/loop-harness/src/autoupgrade.ts` on the harness. Real run surfaced 3 Class-A candidates
  (ghost MCP `claude_design` + `claude_ai_Notion` via codeburn; `CLAUDE.md` config-bloat) and 1
  Class-B (the staged SECURITY-DEFINER search_path migration → human/DB-owner). Emits the §5 report
  via the harness; persists to `loops/runs/autoupgrade/`.
  - **Safety enforced in code, not just docs:** the firm-boundary `classify()` is **fail-safe**
    (ambiguous → B), tested with a boundary matrix (auth/RLS/secrets/payments/PII/schema/arch/major-dep
    → B) + a regression test (the "loaded every session" false-positive that briefly mis-flagged the
    ghost-MCP prune as B — fixed: broad list matches the structured `area`, tight list matches free text).
    `applyCandidate()` **throws** — auto-apply is disabled until the oracle is proven.
  - **Deferred (§8 steps 1, 3–6):** sandbox + credential isolation (§4), web RESEARCH (contained), the
    machine oracle (§2 — green + no-security-regression + benchmark-replay ≥5% speedup + reversible),
    Class-A auto-apply, Class-B proposal queue wired to GRADUATE (§8c of v3-FINAL), headless pg-boss
    scheduling. **No autonomous mutation until §8 steps 3–4 are built + proven.**
