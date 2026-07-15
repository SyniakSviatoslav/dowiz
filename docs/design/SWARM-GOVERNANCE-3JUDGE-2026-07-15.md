# Deliberative Governance Layer — 3-Independent-Judge + Escalation Protocol

Grounded in: `telemetry swarm`→`swarm_exec` (architect→cheap executors→verifier tier), the
DECART-square rule, and `player-roles.md` (author≠judge, different-model adversarial review).

## (A) Three independent judge models for HARD decisions

### How "hard" is detected
A decision is **hard** (routes to the panel) when ANY trigger fires:
- classification is `build`/`audit` (execution-speed, full discipline) with ≥2 candidates each carrying a falsifiable acceptance;
- a red line / irreversible flag (human-authority on `deliver`, server-schema/contract change, PII/money touched);
- blast radius spans >1 server contract;
- architect sets `hard:true` because no modern/Rust-native DECART dominant winner exists;
- token/risk budget exceeds threshold.

### Judge selection (anti self-confirmation)
- `author_id` is **excluded** from the judge pool. Judges are drawn from a disjoint model set
  `JUDGE_POOL={M_A, M_B, M_C}`, none equal to the drafter or integrator.
- Each judge runs in an isolated context (no draft handoff memory) — the fresh-eyes principle.

### Judge prompt template (per judge J_i)
Input: `{id, candidates[], criteria[], author_id, draft_DECART}`.
Instructions:
1. **Counterfactual reasoning (≥3 passes):** for the leading candidate, generate ≥3 counterfactuals
   ("if I chose X instead of Y, what breaks and why?").
2. **Evidence-based:** every cell cites a test name / number / link — never authority.
3. **DECART square:** fill candidate×criterion cells with falsifiable verdicts.
4. **DECISION line:** `DECISION: <chosen> — <falsifiable reason>.`
5. **Mandatory PROBE:** `PROBE: <strongest honest argument against chosen> + why it lost.` (no probe = reject).
6. **Return structured verdict:** `{judge_id, model, chosen, confidence[0..1], decart_cells, probe, evidence_refs}`.

### Aggregation rule (falsifiable)
- **≥2 of 3 agree on the same candidate → DECIDE.** Adopt; record the dissenting judge's `probe` as a
  logged challenge (`docs/decisions/`, §7).
- **Split (1-1-1, or 2-1 not on one candidate, or any abstain/unknown) → ESCALATE.** Emit
  `telemetry alert <id> decision "ESCALATE: split — no 2/3 majority"` → Telegram topic **267**
  (operator/Hermes) + **294** (Benchmarks). Operator resolves and writes a binding decision record.

## (B) Plug into the `swarm_exec` verifier gate

Current `swarm_exec`: fans blueprints to cheap executors, then a **cheap verifier self-check** of each
acceptance (RED+GREEN gate) closes. Extension:

1. `swarm_exec` reads each blueprint; if `hard`/no DECART dominant winner, it emits a **`judge_manifest`**
   (3 dispatch entries, `exclude=author`, model ids from `JUDGE_POOL`) alongside the executor manifest.
2. The verifier tier does **not** flip `verified=true` until the panel resolves:
   - majority DECIDE → run the winning candidate's falsifiable check;
   - split → verifier stays `BLOCKED: awaiting operator`, swarm_exec reports to 267+294, **never merges**.
3. **Falsifiability closure:** the verifier actually executes the chosen candidate's own DECART criterion.
   If that check fails, the decision is auto-overturned and re-escalated (DECART "verifier-actually-rejects").
   This prevents a judge from winning on a criterion the artifact cannot satisfy.

### Command flow
```
architect → blueprint + hard flag + draft DECART
  └─ telemetry swarm <blueprints>
       ├─ soft blueprints → cheap executor + verifier (unchanged)
       └─ hard blueprint  → judge_panel(blueprint, exclude=author)
                              ├─ 3 independent judges (disjoint models)
                              ├─ aggregate: 2/3 → DECIDE → verifier runs falsifiable check → merge
                              └─ split    → telemetry alert decision ESCALATE → topic 267 → operator binds
```
Outcome is logged as a `telemetry` decision event + DECART comparison report in the commit/PR, per the
standing Integration Decart Rule.
