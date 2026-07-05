# loop-builder — run memory

## 2026-06-27 · §11 steps 1-2 + structural VALIDATE built (REPORT-ONLY)

The meta-loop on the v3-FINAL harness: goal G → designed harness-native loop (5 hooks + config).
`tools/loop-harness/src/loop-builder.ts`. INSTANTIATES the harness; never reinvents
telemetry/breaker/report (§0/§10).

**Two hard properties built:**
- **Oracle-admissibility FIRST (§4):** `assessAdmissibility` REFUSES subjective goals (prettier /
  feel-faster / improve-architecture / better-UX) and fail-safe-refuses unknown goals with no
  template/metric → escalate to human ("define measurable criteria"). It never emits a fuzzy loop.
- **Born hardened (§5):** `designLoop` fills the design from a template registry (be-polish · qa ·
  perf · i18n — the §7 examples): a DETERMINISTIC oracle, prefer-existing tools, iterate/isTerminal,
  scope class + **security carve-out** (auth/RLS/secrets/money/pii/migrations propose-only even in a
  Class-A loop), breaker config, and **reuse detection** (extend an existing loop, don't duplicate).

`validateDesign` = §2 structural checks (sound metric · terminates · reuses-harness · carve-out ·
not-duplicate). `runLoopBuilder` emits the §5 report (always printed); report-only — NO loop registered.

**Live demo:** "BE polishing" → DESIGNED (oracle: failing BE tests↓ + tsc/eslint clean + aislop↑;
carve-out auth/rls/secrets/money/pii/migrations; correctly flags EXTEND backend-contract-convergence).
"make the UI prettier" → ADMISSIBILITY-REFUSED + escalated. 7 tests; 74 harness tests total.

**Deferred (§11 steps 3-tail/4-6):** the SMOKE-test dry-run (§2.3: instantiate + run the generated
loop on a fixed seed; the metric must MOVE, TERMINATE, not churn out-of-scope) → then Class-A
auto-register → Class-B→proposals queue → headless pg-boss. NO loop is registered until the smoke
test proves it works. See [[loop-harness-2026-06-27]].

## 2026-06-27 · smoke + auto-register + Loop Selection Router (full hierarchy live)

- **Smoke (§2.3)** `smoke.ts`: dry-runs a design's contract (metric+terminal+breaker) through the REAL
  harness on a seed → asserts metric MOVES + TERMINATES(green) + scoped. Rejects stuck/tight-breaker/
  churn designs. **Auto-register (§11.4)** `registry.ts`: releasable (admissible+validate+smoke+ClassA+
  not-dup) design → `runs/registry.json` behind `--register`. Live: "i18n coverage" → smoke PASS →
  registered "i18n".
- **Loop Selection Router** (`router.ts`, docs/operating-model/loop-selection-router-v1.md): the
  intake gate. route(command,registry) → DIRECT(default) | RUN(specific-tag match; generic tags
  green/loop/flow/wiring don't match alone) | BUILD(admissible→builder) | BOUNCE(no metric). Announces
  1 line; logs runs/routing.jsonl. Routes only. Enforcement hook = tools/loop-harness/router-hook.sh
  (operator installs into .claude — protect-paths-gated). Deferred: LLM-classify ambiguity fallback.
- Full hierarchy live: ROUTER → RUN registered | BUILD→smoke→auto-register | BOUNCE | DIRECT. 89 tests.
