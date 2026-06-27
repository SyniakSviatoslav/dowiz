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
- **2026-06-27 — §8 step 3 (the machine oracle) built + §8 step 4 (Class-A auto-apply) enabled.**
  `oracle.ts` (`evaluate(hooks, thr)`) is the strict gate: KEEP iff **reversible AND green AND
  no-security-regression AND speedup ≥ 5%**; else **atomic rollback** (revert). 7 unit tests prove
  KEEP + every rollback path (not-reversible → refuse-to-apply, tests-RED, security-regression,
  no-speedup, slower-regression) and that `revert()` is called exactly when it should be.
  Auto-apply is wired behind `--apply` (opt-in trigger): each Class-A candidate runs through the
  oracle via per-candidate hooks; candidates with no reversible+benchmarkable adapter are skipped
  with an honest reason. **First live `--apply` run: 0 kept · 0 rolled-back · 3 skipped** — the
  account-managed MCP servers aren't loop-reversible (can't reconstruct the re-add) and the
  CLAUDE.md trim has no benchmark-replay speedup, so the oracle correctly declined all three; Class B
  never reached apply. This is the gate working as designed: keep only what's PROVEN, reject the rest.
- **2026-06-27 — the reversible+benchmarkable Class-A adapter built + proven end-to-end.**
  `benchmark.ts` (`runBenchmark` — deterministic parsed metric or median wall-clock, §2.3) +
  `repo-apply.ts` (`makeRepoHooks` — apply a patch, measure before/after, green+security, and
  **atomic revert via `git checkout -- <paths>`**; refuses if the tree is dirty so the revert
  restores exact bytes). `repo-apply.test.ts` (5) proves the FULL auto-apply path on a real throwaway
  git repo: a 20%-faster change is **KEPT on disk**; a tests-RED / security-regression / no-speedup
  change is **atomically ROLLED BACK** (git restores the exact original bytes); a dirty tree is
  refused. The oracle can now genuinely KEEP a real repo change. Wired into the loop's `buildHooks`
  for `repo-perf:` candidates.
  - **ponytail ceiling:** git-checkout isolation mutates the MAIN tree during verify → not
    concurrency-safe. Upgrade = git worktree + node_modules symlink when >1 loop runs (today
    teamConcurrency 1 — adequate).
- **2026-06-27 — the MAP source (the gap) closed: operator-declared config tuning.** `detectors.ts`
  — `configTuneDetector` reads operator-declared tunables (`loops/autoupgrade.tunables.json`: a knob
  `file`+`find` regex, a BOUNDED set of safe candidate `values`, a `benchmark`, optional green/security
  commands) and emits a `repo-perf:` Candidate per non-current value, each with a `RepoPerfSpec`
  (mechanical regex value-swap + benchmark + git-revert). The operator bounds the search space
  (safety); the loop mechanically tries each value, benchmarks it, and the oracle KEEPS only the one
  that is ≥5% faster (else atomic rollback). This is the ONLY autonomous repo-mutation path, by
  design: **mechanical + bounded + reversible + benchmarked — never an autonomous LLM patch (§0).**
  `detectors.test.ts` (5) proves the FULL pipeline end-to-end on a real git repo: a declared tunable
  → classify A → oracle applies → benchmark 50% faster → **KEPT on disk**; a slower value → atomic
  rollback; absent declaration → [] (no auto-tuning without an operator opt-in). Wired into
  `mapCandidates`. Example: `loops/autoupgrade.tunables.example.json`.
- **2026-06-27 — containment (§4) + Class-B GRADUATE queue (§8c) built.**
  - `containment.ts`: `assertCredentialIsolation(env)` refuses autonomous apply if secret-shaped env
    vars are present (§4 — a compromised step has nothing to exfiltrate); `evaluateClassA` checks it
    before any `--apply`. `isTrustedSource` allowlist — only candidates from a TRUSTED mechanical
    detector (config-tune) may auto-apply; web/LLM-derived candidates are forced propose-only (§0).
    Wired into `evaluateClassA` (credential gate) + `buildHooks` (trusted-source gate). 6 tests.
  - `proposals.ts`: the §8c queue — Class B is PROPOSED to `proposals.json` (durable, deduped by id,
    frequency-weighted count++, human-set status preserved, never auto-deleted), never auto-applied.
    Each Class-B candidate is queued each run. 5 tests. Live run queued the SECURITY-DEFINER migration
    (`queued ×1 security`).
  - **Deliberately NOT done — headless pg-boss scheduling (§6):** running autonomous repo-mutation
    from the product API server (which serves customers) is the wrong place. It belongs in a SEPARATE
    scheduled job (cron/CI), not `apps/api`. Left to ops, not wired into the product.
  - **Still deferred (correctly):** sandbox container + agent-vault key brokering (the runtime side of
    §4 — the env guard is the code side), web RESEARCH (contained), the worktree concurrency upgrade
    (only needed at >1 loop concurrency; teamConcurrency 1 today), widen Class A after several clean
    runs. The loop is end-to-end functional: MAP → CLASSIFY → ORACLE → KEEP|ROLLBACK → §5 report, with
    containment + a human-gated proposal queue.
