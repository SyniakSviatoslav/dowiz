# DeliveryOS / dowiz — Living Loop System: Hardening + Telemetry + Self-Training (v3-FINAL)

**Date:** 2026-06-27 · **Authoritative.** Consolidates and supersedes v1, v2, and the v3 amendment. One doc — read this; ignore the earlier three.

**Scope:** Every agent loop (convergence-to-green; 6-hr triage; any future loop) is a **trackable, comparable, self-improving** unit: hardened (breaker + reflection), fully instrumented (incl. fused free eco-telemetry), reporting a structured end-of-run report (stored compactly + permanently, printed in full to terminal), and **learning from its own complete history at iteration granularity** so iteration N is shaped by every prior iteration and run.

**Doctrine guardrails (binding — this design is ambitious; these keep it from becoming the work):**
- **Store the record, render prose on demand.** The report is a *view*. Persist the compact canonical run-record (it re-renders the exact prose); never duplicate fat text.
- **Never clean loop data.** No pruning, downsampling, or deletion — ever. Optimization is lossless-only (compression, dedup, compact canonical form). Total recall, light footprint.
- **No ML, no dashboard, no agent fleet.** "Self-training" = distilled memory + per-iteration recall + human-gated guard graduation, not gradient descent. codeburn already owns the cost dashboard.
- **Human approves graduation.** Lessons *propose* hard guards; they never silently mutate the codebase (respects no-fake-green + control).
- **Always-on.** The harness is the only way a loop runs. There is no bare loop.
- **On-phase.** This exists to make finishing Stages 30–35 cheaper, greener, and impossible to fake. The moment it grows a UI or a supervisory agent, it has drifted.

---

## 1. Universal loop contract (mandatory; covers all loops, current + future)

Every loop implements 5 hooks; the **harness** provides everything else (recall, telemetry, eco, breaker, reflection gate, report, learnings, storage). A new loop is hardened + tracked + self-training by construction — same dependency-inversion seam as `MessageBus`/`QueueProvider`. **There is no loop outside the harness.**

```ts
interface Loop {
  id: string;                         // "convergence" | "triage" | ...
  goal(ctx): string;                  // §5.1 — the run's intent
  iterate(ctx): IterationRecord;      // one pass; MUST return a progress metric
  progressMetric(state): number;      // breaker watches this (failing tests; unresolved issues; ↓=better)
  reflect(ctx): Reflection;           // §4 author self-critique
  isTerminal(state): boolean;         // success (e.g. green 3×) | natural stop
}
```

Harness responsibilities (write once, reuse for all loops): assign `run_index`; **per-iteration recall (§8)**; capture telemetry per iteration (§2); compute eco (§6); run the breaker (§3); run the fresh-context reviewer gate (§4); DISTILL + human-gated GRADUATE (§8); build the canonical run-record; **always render + print the full report to terminal (§5)**; persist permanently and losslessly (§7).

---

## 2. Telemetry record (per iteration) — eco fused in as first-class

One JSONL line per iteration (`runs/<loop>/<run_index>.iters.jsonl`, compressed at rest). Eco is a standard block computed every iteration from data you already emit.

```jsonc
{
  "loop": "convergence", "run_index": 42, "iteration": 7,
  "t_start": "2026-06-27T10:14:02Z", "t_end": "2026-06-27T10:18:40Z", "dur_s": 278,

  "code":   { "files": 4, "loc_add": 86, "loc_del": 31, "edits": 9,
              "tests_fail_before": 11, "tests_fail_after": 8, "delta": -3,
              "slop_score": 92, "lint": "pass", "typecheck": "pass", "fake_green_caught": 1 },

  "git_mem":{ "commits": 1, "branch": "fix/i18n-checkout-0627", "conflicts": 0, "prs": 0,
              "ctx_util_pct": 71, "compactions": 1, "rss_peak_mb": 5320, "oom": false },

  "agents": { "planner": 1, "generator": 2, "healer": 1, "reviewer": 1,
              "progress_by": "generator", "handoffs": 3 },

  "skills": { "used": {"playwright-mcp": 4, "sentry-mcp": 1}, "ghost": ["unused-skill-x"] },

  "tokens": { "in": 48210, "out": 9120, "cache_read": 31000,
              "by_model": {"claude-opus-4-8": 41000, "claude-haiku-4-5": 16330},
              "per_resolved": 19110, "read_edit_ratio": 3.2, "cost_usd": 0.41 },

  "eco":    { "kwh": 0.012, "gco2": 3.4, "water_ml": 14, "method": "ecologits", "estimate": true },

  "reflection": { "changed": ["CheckoutStepper validation"], "verified": ["e2e_checkout_cash green"],
                  "not_verified": ["429 retry-after"], "risks": ["menu_version drift"], "confidence": 0.6 },

  "breaker": { "state": "running", "stall_count": 0 }
}
```

**Sources (all already produced):** `code`/tests ← Playwright JSON reporter + aislop + tsc/eslint; `git_mem` ← git + `/proc/<pid>/status` (VmHWM); `agents`/`skills`/`tokens` ← Claude Code/Antigravity session file (same data codeburn reads; codeburn also flags ghost skills/bloat); `eco` ← §6.

---

## 3. No-progress circuit breaker (the real gap, universal)

opossum guards the *product*; nothing guards the *loop*. Harness-level, reads `progressMetric` deltas.

**Trip on ANY:** `stall_count ≥ K` (delta ≥ 0 for K iters; reset on delta<0; start K=3) · `iteration ≥ MAX_ITER` (e.g. 25) · `cumulative cost ≥ BUDGET` · `wall_clock ≥ TIME_CAP`.
**On trip:** stop (don't re-invoke) → write stall report (last 3 diffs, last error, matrix state, unresolved `not_verified`/`risks`) → Telegram-ops ping → exit non-zero → **still render + print a (partial) report (§5)**.
**Triage variant:** keep ≤1–2 PR/run + per-fingerprint attempt counter; same issue unfixed N runs → label `needs-human`, stop retrying (bounded execution, pattern #10).

This converts "burns tokens until I notice" into "stops and tells me."

---

## 4. Reflection with an independent evaluator (universal)

**4a. Author reflection** (cheap, in-loop): the `reflection` object — changed / verified-with-evidence / not-verified / risks / confidence.
**4b. Fresh-context reviewer (the gate):** a *separate* Claude Code invocation, **clean context**, reviews the diff against spec/inventory + the author's claims; it re-checks rather than trusting "verified". Its verdict gates green — not the author's self-assessment. (Evaluator-Optimizer #6 with a genuinely independent evaluator.)
**Doctrine:** every `not_verified` → a new assertion next pass or an explicit flag. Never silently a pass.

---

## 5. End-of-loop report — rendered from the canonical record, ALWAYS printed in full to terminal

On finish (success, stall, or abort), the harness builds the **canonical run-record (§7)**, then **renders this report from it and prints it to the terminal as plain text, in full, every time — no flag, no exception.** The prose is a *view* and is regenerable, so it is never the unit of storage.

```
════════════════════════════════════════════════════════════════
 LOOP REPORT · convergence · run #42 · iter 1–7 · GREEN ✓
 start 2026-06-27 10:02:11Z · end 10:41:55Z · wall 39m44s
════════════════════════════════════════════════════════════════

1. INITIAL GOAL
   Bring checkout + i18n flows to green for all 3 roles (al/en).

2. WHAT WAS DONE
   CheckoutStepper validation fixed; ws reconnect banner wired;
   i18n al/en switch verified on menu_version cache. 4 files, +86/−31.

3. ISSUES (unresolved / surfaced)
   • 429 retry-after path NOT verified (carried to run #43)
   • risk: menu_version drift if cart open during menu edit

4. PATTERNS · INSIGHTS · LEARNINGS
   • RECURRING (3rd run): stall on i18n contract → GRADUATING to guard
     [proposed] pre-load docs/frontend/contract-map.md#i18n before iterate
   • NEW: generator made 100% of progress this run; planner idle → candidate to skip
   • fake-green caught once (checkout) by reviewer

5. TELEMETRY
   CODE      tests 11→0 · 9 edits · +86/−31 · slop 92 · lint/type pass · fake-green ×1
   GIT/MEM   1 branch, 1 PR, 0 conflicts · ctx peak 78% · compactions 2 · RSS peak 5.6GB · OOM no
   AGENTS    planner ×1 · generator ×6 · healer ×2 · reviewer ×7 · handoffs 14
   SKILLS    playwright-mcp ×22, sentry-mcp ×1 · ghost: unused-skill-x (archive)
   TOKENS    in 312k · out 61k · cache-read 198k · cost $2.74 · per-resolved 19.1k ↓
   ECO       0.081 kWh · 23.6 gCO₂ · 96 ml water  (estimate, ecologits) ↓
   TIME      per-task: checkout 18m · i18n 12m · ws 6m · review 4m · total 39m44s

6. VS HISTORY (convergence, all prior runs + best)
   iters-to-green  7   (avg 6.2  ↑0.8 regression) (best 4)
   tokens/resolved 19.1k (avg 21.8k ↓12%)
   cost            $2.74 (avg $3.10 ↓)
   CO₂             23.6g (avg 27.1g ↓)  water 96ml ↓
   recurring: i18n-stall ×3 ↑ · fake-green-checkout ×3 ↑ → both graduating

7. CARRY FORWARD → run #43
   guards: [active] checkout-cash standing assertion · [proposed] i18n pre-load
   watch: 429 retry-after (unverified) · planner-skip experiment
════════════════════════════════════════════════════════════════
```

Sections map 1:1 to spec: initial goal · what was done · issues · patterns/insights · code · memory+git · agents/subagents · skills · tokens · CO2+water · time (per task/subtask, timestamps, total, run/iteration indexing) · historical comparison (over **all** prior runs — nothing is downsampled) · plus forward-carry (the self-training hook).

---

## 6. Eco-telemetry — fused, free, trend-honest

Standard `eco` block every iteration + aggregated per run. **Free path only:**
- **Product AI calls (Fastify menu-AI/OCR):** **EcoLogits** (`mlco2/ecologits`, free OSS) wraps the SDK call → energy (kWh), CO₂eq, and **water (liters)** + embodied. The clean source of the water signal.
- **Claude Code / Antigravity dev loop:** EcoLogits can't intercept Claude Code's internal calls, so compute from session token totals × EcoLogits per-model factors (a small offline function). (Carbonlog gives a live Claude Code status line, individual-free, partly commercial — convenience option.)
- **VPS compute:** **CodeCarbon** (free OSS) — actual Hetzner draw vs grid intensity (small slice).

**Honesty + why it's still worth it:** absolute eco numbers are *estimates* (closed providers disclose little; methodologies diverge widely). But your assumptions are **constant across runs**, so the **run-over-run trend is reliable** — exactly what the self-comparison/training uses. And footprint = token waste = the same lever as the breaker/aislop, so a falling CO₂ trend is free confirmation your loop is getting more efficient. Track the trend; don't quote the absolute as audited.

---

## 7. Storage — permanent, never cleaned, lossless-only optimization

**Loop data is never pruned, downsampled, gzip-then-dropped, or deleted. 100% of loop history is permanent and complete, forever. Reports are always stored.** There is no retention window; nothing ages out.

Optimization is achieved **only** by lossless means — never by removal:

| Technique | Effect | Lossless? |
|---|---|---|
| **Canonical record = the report** | Store the compact structured run-record (re-renders the *exact* prose report). Don't duplicate fat rendered prose. | ✓ (report fully regenerable) |
| **Compress at rest** | zstd/gzip every record + iteration log. | ✓ |
| **Dedup by hash/reference** | Repeated content (goal/spec text, recurring patterns) stored once by hash/ID, referenced elsewhere; run-to-run as deltas where natural. | ✓ |
| **Append-only logs** | metrics index + per-run records appended, never rewritten. | ✓ |

**"Always store the report" =** every report is durably persisted **as its canonical run-record**, which regenerates the identical prose on demand. The report is never lost, only stored efficiently (compact + compressed) instead of as fat duplicated text. (Literal prose archiving is optional and redundant — the record already *is* the report, losslessly.)

**Permanent store (complete, forever):**
- `runs/metrics.jsonl` — 1 compact line/run (trend + recall index): loop, run_index, ts, outcome, iters, wall_s, tokens, cost, kwh, gco2, water_ml, fail_start→end, per_resolved, slop, conflicts, recurring_flags. **Permanent.** ~1 KB/run.
- `runs/<loop>/<run_index>.json[.zst]` — full structured run-record. **Permanent, compressed, never dropped.**
- `runs/<loop>/<run_index>.iters.jsonl[.zst]` — full iteration trace. **Permanent, compressed, never dropped.**
- `runs/learnings.json` — bounded, deduplicated lessons (curated working memory for recall/graduation; the raw archive above keeps everything regardless).

Footprint stays modest because optimization is compression + non-duplication, not forgetting: compressed structured records are small, fat prose never hits disk — you get total recall *and* a light footprint.

---

## 8. Self-training cycle — DISTILL → GRADUATE → per-iteration RECALL

No model training. The system's **context and guards** improve from its own complete history, at **iteration granularity**.

### 8a. RECALL (per-iteration — the engine)
Before composing **each** iteration prompt, the harness injects two **compact** digests (headlines/deltas, not full records) read from the loop's own files:

1. **Within-run trace digest (always)** — distilled from this run's prior iterations (`*.iters.jsonl`): approaches tried, *what failed and why*, what's fixed, current failing set. Carries a hard instruction:
   > Do not re-attempt failed approaches: [A, B]. Current target: [failing tests]. Known-this-run fix: [Z].

   Highest-value recall — re-trying what already failed is the biggest intra-run token leak.

2. **Cross-run lessons digest (relevance-filtered)** — top learnings + known-good fix patterns from `learnings.json` whose tags match the *current* failing area (e.g. now failing on i18n → "i18n stall recurred ×3 → pre-load contract-map#i18n; fix pattern that worked: …").

Plus an **internal recall tool**, callable mid-iteration when the agent hits a wall:
```
loop_recall(query)  →  relevant prior-iteration + prior-run records,
                       filtered by current failing tests / task tags
```
This is what "accessible internally" means: the loop's complete history is a live, queryable resource during the run — not a one-shot prefix. Depth comes from this pull tool; the injected digests stay compact so the per-iteration token overhead is small and the tradeoff stays net-positive (repetition prevented ≫ digest cost). Retrieval = keyword/tag match on failing-test names + task tags at your scale; embeddings only later if recall gets noisy (likely never, by lean doctrine).

### 8b. DISTILL (run end)
Diff this run's metrics + reflection findings vs history; update `learnings.json` — new pattern → add; seen pattern → `count++`, refresh `last_seen`. Deduplicated, so lessons stay bounded and frequency-weighted.

### 8c. GRADUATE (threshold, human-gated)
A soft learning hits `count ≥ G` (e.g. 3) → **propose** a hard guard: a standing Playwright assertion, a CLAUDE.md rule, an opossum wrapper, or a pre-load step. **Human approves** → mark `graduated` and the guard becomes a real check. Proposal-not-mutation keeps control + no-fake-green intact.

**Effect:** recurring stalls get pre-empted *mid-run*, wasteful prompts get flagged, persistent fake-green spots harden into permanent guards — and run N is shaped by every prior iteration and run. The loop gets cheaper, greener, and harder to fake with each iteration; the proof is the §5 "VS HISTORY" trend.

---

## 9. Anti-gold-plating (read before building)

- **Render, don't duplicate** prose. The canonical record is the report; the metrics line is the trend/recall index.
- **Never clean** is intentional and permanent — the archive is the asset. Optimize only losslessly.
- **Graduation is human-gated.** No silent codebase mutation from learnings.
- **Recall digests stay compact**; depth via the pull tool only.
- **Token tradeoff, named:** reflection + reviewer + per-iteration recall ADD calls/tokens. Net positive — the breaker saves far more (runaway loops), graduated guards kill repeat rework, and within-run recall stops re-trying failed approaches (the priciest waste). Set BUDGET/MAX_ITER so the safety net can't become the leak; watch the §5 CO₂/cost trend to confirm net savings.
- **Stays:** lean, sequential, solo, always-on. No supervisor, no fleet, no UI, no dashboard.

---

## 10. Order of work

1. **Loop contract + harness skeleton (§1)** — the reuse spine; both existing loops adopt it. (Always-on from here.)
2. **Breaker (§3)** — biggest immediate win (stops runaway burn).
3. **Telemetry JSONL incl. eco block (§2, §6)** — wire codeburn token data + EcoLogits/CodeCarbon.
4. **Reflection + fresh-context reviewer gate (§4)** — locks no-fake-green.
5. **Canonical run-record + render-on-demand report (always printed) + permanent lossless storage (§5, §7).**
6. **Per-iteration RECALL + DISTILL (§8a, §8b)** — start learning from history immediately.
7. **GRADUATE (§8c)** — once a few patterns have recurred and you trust the signal.
8. **Opossum coverage audit (product, Stage 33)** — parallel track.

Then keep finishing 30–35. The system exists to make that finish cheaper, greener, and impossible to fake — not to replace it.

---

## Implementation status (appended by build — keep in sync)

- **2026-06-27 — Foundation (§10 steps 1–3, deterministic core) built + tested** in `tools/loop-harness/`:
  contract types (§1), no-progress breaker (§3), per-iteration telemetry types (§2), canonical
  run-record → report renderer (§5), permanent append-only lossless storage (§7), and a thin
  harness that composes them around any `Loop`. Unit-tested (breaker trip matrix, report render
  from record, storage round-trip, harness happy-path + stall-path).
- **2026-06-27 — Telemetry collectors + wiring (closes the "semi-empty telemetry" gap).** Built
  the data SOURCES the foundation deferred: `eco.ts` (§6 — token×per-model-factor; **eco uses
  COMPUTE tokens (in+out) only — cache-read tokens are not re-processed so must not inflate energy**),
  `collect.ts` (`collectGitMem` = git branch/commits + /proc RSS; `collectSessionTelemetry` = parse
  the Claude Code session JSONL over the run window → tokens by model + cost + skills + agents — the
  source codeburn reads), and `cli.ts finalize` — the **wiring seam**: an agent-run loop hands the
  harness a partial record (goal/what_done/issues/patterns + code deltas), finalize MEASURES the
  rest and emits the §5 report (always) + persists. Loops adopt it via the `harness:` node in their
  card (see loops/audit-gate.yaml + loops/registry.md). 26 tests (eco scaling/factors; session parse
  window+by-model+cost+skills+agents; gitmem). Proven on the audit-gate run (real tokens/skills/eco).
- **Still deferred (steps 4–7, integration-heavy):** per-iteration recall + distill + graduate (§8),
  fresh-context reviewer gate (§4 — a separate clean-context Claude Code invocation), CodeCarbon VPS
  draw (§6), and driving agent-loops *through* `runLoop` natively (today loops call `finalize` at finish).
