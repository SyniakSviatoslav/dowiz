# DeliveryOS / dowiz — Meta-System: Safety Surface + Map (FINAL)

**Date:** 2026-06-27 · The capstone. Adds the three components that complete the safety surface (global governor, human-review queue, oracle-integrity check) and maps the whole meta-system. With this, the design is *complete* — further components are premature (§5).

---

## 0. System map

```
you type a command → LOOP SELECTION ROUTER (mandatory pre-hook)
   DIRECT (no loop) ◄─┘   RUN │   BUILD → LOOP BUILDER → registers in registry.json
                             ▼
                    HARNESS (v3-FINAL): contract · telemetry · breaker · reflection · report · storage · recall
       convergence · triage · QA · BE-polish · perf · AUTOUPGRADE · loop-builder
   ═══ cross-cutting safety (this doc) ═══
   GOVERNOR (§1)         — ABOVE all loops: aggregate ceilings + master halt + reaper
   ORACLE-INTEGRITY (§3) — protects the ground truth (tests/reviewer/benchmark)
   REVIEW QUEUE (§2)     — the sink for every "→ human" branch (Class B + graduations)
```

Corpus: Loop-System-v3-FINAL · Loop-Builder-v1 · Autoupgrade-v1 · Loop-Selection-Router-v1 · **this doc**.

---

## 1. Global Governor — a DETERMINISTIC watchdog (not the rejected supervisor)

Zero intelligence, no routing, no coordination. Watches aggregate numbers and pulls a brake.
- **Aggregate ceilings (all loops):** cost/day, cost/hour, concurrent RAM, churn/day, max concurrent loops.
- **Master halt:** one switch (a `HALT` flag file) — manual OR auto (any ceiling breach auto-halts). **Halted is explicit; resume is MANUAL** — a runaway can never auto-resume itself (the safety asymmetry).
- **Liveness reaper:** a loop silent (no heartbeat for T) → reaped (catches OOM/hung loops the per-loop breaker can't).
- **Sees everything:** reads `runs/metrics.jsonl` for live aggregate spend/churn.

---

## 2. Human-Review Queue — the Class B sink + feedback

Every "→ human" branch (autoupgrade Class B, builder Class B, graduations, carve-out edits) lands here.
- **Structure:** `proposals.json` (queue, deduped) + `decisions.jsonl` (permanent decisions log, never cleaned).
- **Decide:** accept → `approved` (apply via the standard oracle + graduate) · reject → `rejected` · defer → `queued`.
- **Feedback (the part that matters):** reject = a NEGATIVE LEARNING — `isRejected()` lets the source loops STOP re-proposing it (rejection must teach, or you get the same proposal forever).

---

## 3. Oracle-Integrity Meta-Check — protect the ground truth

The deepest self-modification risk: a loop that "fixes" a test by weakening it corrupts the oracle (fake-green at the infrastructure level). Runs on any change touching tests/reviewer/benchmark, BEFORE accept. Trips → block + route to review (ground-truth changes are inherently Class B). Trips on: test/assertion count silently drops · assertion weakened (`.skip`/`.only`/`expect(true)`/inflated timeout/commented assertion) · benchmark scenario mutated (immutable to loops) · reviewer not provably fresh-context. **Independent by design** — it must not be checkable by the loops it polices.

---

## 4. Defense in depth — a change clears every layer that applies

per-loop breaker · **governor** · **oracle-integrity** · Class A/B + carve-out · containment · **review queue** · the oracle (green+security+speed+reversible). No single point of trust.

## 5. Anti-gold-plating — and you may be done

These three close the surface (aggregate cap, the dangling Class-B branches, the corruptible oracle). Beyond is premature. **Avoid inter-loop messaging** — loops coordinate passively (shared registry, review queue, learnings); the moment they message, you've rebuilt the supervisor. The honest line: point the apparatus at **Stages 30–35** and let real pain name the next gap.

## 6. Order of work

1. Governor · 2. Oracle-integrity · 3. Review queue · 4. confirm defense-in-depth on one real change · 5. **finish 30–35.**

---

## Implementation status (appended by build)

- **2026-06-27 — all three safety components built + wired + tested.**
  - `governor.ts` (§1): `checkGovernor(baseDir, {nowMs,freeRamMb,concurrentLoops})` — aggregate cost/day,
    cost/hour, churn/day, RAM, max-concurrent over `runs/metrics.jsonl`. `masterHalt`/`isHalted`/`clearHalt`
    (HALT flag); **breach → AUTO-HALT, resume is MANUAL only**. `staleLoops` reaper. Wired into autoupgrade
    `evaluateClassA` — `--apply` REFUSES when halted/breached. 6 tests. **Live: master-halt → autoupgrade
    refuses every candidate; clearHalt → resumes.** (Added `edits?` to MetricsLine for churn.)
  - `oracle-integrity.ts` (§3): `checkOracleIntegrity(files, {benchmarkPaths, reviewerFresh})` — trips on
    test/assertion count drop · weakening (skip/only/expect(true)/inflated-timeout/commented) · benchmark
    mutation · reviewer-not-fresh. The no-fake-green rule against the loops' OWN test edits. 6 tests.
  - `review-queue.ts` (§2): built on `proposals.ts` — `decide(accept|reject|defer)` → status + permanent
    `decisions.jsonl`; `isRejected` = negative learning (autoupgrade + builder skip re-proposing rejected).
    `listReview`/`decisionsLog`. 5 tests. Builder Class-B/extend designs now queue here (§11 step 5).
  - **NOT wired into the product** (governor/queue are dev-plane, runs/ local). **Deferred (correctly, §5):**
    the always-on watchdog PROCESS (the functions are the gate; a daemon/cron runs them), oracle-integrity
    wiring into the repo-apply path (function ready; gates test-touching changes), Telegram batch-ping.
    NO scheduler/merge-coordinator/dashboard/inter-loop-messaging — premature by §5. **Next: finish 30–35.**
