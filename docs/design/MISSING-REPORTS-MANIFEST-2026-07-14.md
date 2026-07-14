# MANIFEST — Missing 2026-07-11 Research/Design Reports

> **Status: UNVERIFIED, not fabricated.** Per AGENTS.md (no fabrication; ground-truth
> outranks plans) this document does NOT invent the ~20 reports the brief cited. It
> records the *gap* and what genuinely exists, so future work never silently trusts
> a missing brief claim.

## 1. The gap (verified 2026-07-14)
- The 2026-07-11 brief cited ~20 research/design reports as the basis of the plan.
- `ROADMAP-GROUND-TRUTH-2026-07-11.md` §0.1 confirms: **none of them exist on local
  disk**. Searched `/root/dowiz/docs` + `/root/dowiz` — only these survived:
  - `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`
  - `PARALLEL-EXECUTION-PLAN-2026-07-11.md`
  - `DRIFT-ANALYSIS-*.md` (drift analysis)
- The stale roadmap's own rule (§0.1 line 33): *"the brief's findings must be treated
  as UNVERIFIED until then. Do NOT build code against claims in missing reports."*

## 2. What IS real and verified on disk (2026-07-14)
These are the *actual* artifacts. Trust these, not the brief:

**Canonical product (`/root/dowiz`, branch `feat/kernel-fsm-graph-analysis` → `main` a9041ad6):**
- `kernel/` — Rust→WASM deterministic core. FSM graph-analysis (has_cycle, cyclomatic,
  topological_order, reachable, spectral_radius, fsm_graph_report, FSM_GOLDEN_SIGNATURE
  drift-gate). 109 tests green.
- `engine/` — 17 tests green.
- `docs/design/spectral-graph-fsm.md` — spectral graph theory × FSM (ρ drift alarm).
- `docs/design/field-ui-engine.md` — field-sim UI under kernel plan.
- `docs/design/ROADMAP-GROUND-TRUTH-2026-07-14.md` — re-verified ground truth.

**Protocol research (`/root/bebop-repo`):**
- `docs/design/bebop2-deep-research-2026-07-11.md`
- `docs/design/bebop-math-physics-fable-research-2026-07-11.md`
- `docs/design/BEBOP-UNIFIED-MASTER-PLAN-2026-07-12.md`
- `docs/design/BEBOP-AGENT-MODES-AND-CINEMATIC-TELEMETRY-2026-07-12.md`
- `docs/design/cycle-consistency-theorem.md`

## 3. Why NOT reconstructing the 20 now
- The brief's specific titles/content are unknown (only "the brief cites ~20" survives).
- Fabricating 20 reports would violate AGENTS.md (no fake-green / no fabrication) and
  produce precisely the "stale claims masquerading as done" failure mode the red-team
  docs warn about.
- If the operator wants them, the source brief must be supplied. Until then: **UNVERIFIED.**

## 4. Decision (operator, 2026-07-14)
Operator authorized "go" on re-authoring OR accepting UNVERIFIED. Chosen: **accept as
UNVERIFIED** + keep this manifest as the durable record. No silent trust of brief claims.
