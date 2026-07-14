# Spec — Finish the hydraulic-loop-v2 + living-knowledge remainder

## WHY
The operator authorized full autopilot to completion of ALL plans/roadmaps, including red-lines,
with SDD as a binding hook and max-lane parallelism. Deep research of the blueprints
(`docs/design/hydraulic-loop-v2/BLUEPRINTS.md`, 23 BPs / 5 waves) shows the bebop/bebop2 tree is
already built and committed (baseline green: 777 tests, 0 failed). The genuine remaining gaps are
narrow and must be closed with RED→GREEN proof — not by re-doing finished work.

## Acceptance (what "done" means)
- Every outstanding, verifiable blueprint item has a RED→GREEN gate proven with literal test output.
- Risky/red-line/history-mutating items are flagged for operator decision, NOT silently executed.
- Final state: `cargo test --workspace` (bebop) + `cargo test` (dowiz kernel) + living-knowledge
  reconciliation, with literal `0 failed` proof for everything touched.

## Out of scope (do not touch)
- Do NOT re-implement BPs already verified committed (01–21, 15, 17, 18–20 confirmed built).
- Do NOT modify rounding semantics in money.rs (red-line, half-up SCALE untouched).
- Do NOT force-push / rewrite git history (P10) without operator go-ahead + ref backup.
