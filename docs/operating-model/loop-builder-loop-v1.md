# DeliveryOS / dowiz — Loop Builder Loop (v1)

**Date:** 2026-06-27 · A **meta-loop** on the v3-FINAL harness (`living-loop-system-v3.md`). Given a goal (e.g. "BE polishing", "QA loop"), it researches the project's reality and **synthesizes the best loop for that goal under existing resources** — then validates and releases it. Headless, background. It is itself a Loop, and it produces Loops; the harness is the fixed point.

---

## 0. Core principle

A loop = the §1 contract's **5 hooks** (`goal`, `iterate`, `progressMetric`, `reflect`, `isTerminal`) + config, plugged into the existing harness. The builder **instantiates** that; it **never reinvents** telemetry/breaker/report/storage/recall. Output is a small loop-definition, not new infra.

Two hard properties:
- **Oracle-admissibility first (§4).** No deterministic success metric for the goal → no loop. It escalates, it doesn't guess.
- **Born hardened (§5).** Generated loops inherit the accumulated learnings/guards at birth.

Inherits from the autoupgrade spec: Class A auto / Class B human, per-file security carve-out, containment, reuse-don't-duplicate.

---

## 1. The loop (bounded per run, given goal G)

```
GATHER → DESIGN → VALIDATE(oracle) → RELEASE | QUEUE → REPORT → FEED
```

- **GATHER (the substrate — §5):** goal G + project docs + memories/learnings + full loop run history + prior research + the **resource inventory** (tools/agents/skills/MCPs, test infra, telemetry, the harness contract).
- **DESIGN (synthesize the hooks — §3):** pick the oracle, tools, iteration shape, termination, scope class + carve-out, breaker config for G — preferring patterns/tools already used.
- **VALIDATE (the builder's oracle — §2):** sound metric? terminates? smoke-passes? doctrine-clean? not a duplicate?
- **RELEASE | QUEUE:** Class A + passes → auto-register. Class B / uncertain → human queue.
- **REPORT + FEED:** standard harness report; learnings accumulate *which loop designs work for which goal types*.

`progressMetric` = candidate loops remaining to design (usually 1). `isTerminal` = designed+validated. Breaker applies.

---

## 2. The builder's oracle (what makes a generated loop releasable)

Released iff ALL hold; else rejected/escalated:
1. **Sound, non-empty progress metric** — a deterministic signal that strictly improves toward G (§4). Empty/fuzzy → reject (escalate).
2. **Provable termination** — a real `isTerminal` + breaker bound.
3. **Smoke test (the hard gate):** dry-run the generated loop a few iterations on a **fixed seeded scenario** — the metric must **actually move**, **terminate**, and **not churn** out-of-scope files. A plausible loop that doesn't demonstrably work is **rejected**.
4. **Doctrine-clean:** reuses the harness, lean, no architecture rewrite, security carve-out present.
5. **Not a duplicate:** loop for G exists → **refine/extend**, don't emit a second.

---

## 3. What it designs

`progressMetric` (the deterministic oracle — most important) · tools/agents/skills (prefer already-used) · `iterate` · `isTerminal` · `reflect` (reuse harness reviewer) · scope class + per-file carve-out · breaker config.

---

## 4. Oracle-admissibility (the first gate)

- **Admissible → build:** "BE polishing" (failing BE tests ↓ + tsc/eslint + slop ↑), "QA loop" (failing matrix flows ↓), "performance" (telemetry-flagged slow paths improved, measured), "i18n" (untranslated ↓ + al/en green).
- **Inadmissible → escalate:** "make the UI prettier", "improve the architecture", "make it feel faster" — no falsifiable metric.

---

## 5–10. (Grounding, scope+carve-out, examples, telemetry, harness fit, anti-gold-plating)

See the authoritative request; the build encodes: born-hardened reuse, per-file security carve-out (a Class-A loop is still propose-only for auth/RLS/secrets/payment files within G), containment inherited from autoupgrade, instantiate-don't-reinvent, smoke-test non-negotiable, lean output.

---

## 11. Order of work

1. **GATHER + DESIGN** — report-only first.
2. **Oracle-admissibility gate** — refuse-and-escalate.
3. **VALIDATE incl. smoke test** — the gate that makes auto-release safe.
4. **Enable Class A auto-register** once smoke is proven on a few goals.
5. **Class B + carve-out → human queue.**
6. Headless + background.

---

## Implementation status (appended by build — keep in sync)

- **2026-06-27 — §11 steps 1–2 + structural VALIDATE built (REPORT-ONLY)** in
  `tools/loop-harness/src/loop-builder.ts` on the harness.
  - `assessAdmissibility(goal)` — the §4 gate. Subjective goals (prettier/feel/architecture/UX) +
    unknown goals with no template/metric → REFUSE + escalate (fail-safe: never guess). Admissible
    goals map to a template with a DETERMINISTIC oracle.
  - `designLoop(goal)` — synthesizes the 5-hook design from a template registry (be-polish · qa ·
    perf · i18n, the §7 examples): oracle, prefer-existing tools, iterate/isTerminal, scope class +
    **security carve-out** (auth/RLS/secrets/money/pii/migrations propose-only even in a Class-A loop),
    breaker config, and **reuse detection** (extend an existing loop instead of duplicating).
  - `validateDesign` — §2 STRUCTURAL checks (sound non-empty metric · terminates · reuses-harness ·
    security-carve-out · not-duplicate). Emits the §5 report (always printed).
  - Proven: 7 tests (admissibility matrix incl. refuse + fail-safe; design oracle/carve-out; reuse
    detection; structural validation). Live: "BE polishing" → designed (extend backend-contract-
    convergence); "make the UI prettier" → refused+escalated.
  - **Deferred (§11 steps 3-tail/4–6):** the SMOKE-test dry-run (§2.3 — instantiate + run the generated
    loop on a seed; metric must move/terminate/no-churn) + Class-A AUTO-REGISTER + Class-B→proposals
    queue + headless pg-boss. No loop is registered until the smoke test proves it works.
