# Swarm Deliberative Governance — Authoritative Design (HK-09)

Consolidated spec from 3 research subagents (EV-tiering / 3-judge / precedent registry)
+ the implemented `tools/telemetry/governance.sh` + kernel `control.rs`. The code is the
source of truth; this doc is the design rationale.

## A. EV-driven multi-layer tiering (not "cheapest")
Measured track-record keyed by (model × task_type) in `track_record.jsonl`. After every
task, `gov_record` appends `{ts,model,task,success,value,cost}`; the rollup yields
`(p_hat, v_hat, c_hat)` per route. Selection per task: for each route compute
`EV = p·v − (1−p)·c` and `ruin = (q/p)^budget` (kernel `ev`/`ruin_prob`); **reject** any
route with `ruin > cap`; **pick max EV survivor** (`ev_route_select`). This auto-escalates
off "cheapest" whenever cheap's ruin breaches cap or EV ≤ 0.
- **Lane width** (½-Kelly): `b=(v−s)/s`, `f*=kelly_fraction(p,b)`, `L=floor(½·f*·(B/s))`
  (`gov_lane_width`). ½-Kelly margin; Chernoff spend-guard can shrink L if
  `chernoff_token_tail > ε_spend`.

## B. Research: generate-then-argue
`gov_research <q> <rounds>` dispatches to `swarm_exec`: GEN → fan-out N executors at the
tier from (A) → ARGUE critic tier (capable model, states strongest *falsifiable* objection
= mechanized DECART probe, estimates downside via EV) → RED/GREEN verifier gate; RED →
re-dispatch GEN with objection (≤K rounds); GREEN → adopt + emit DECART report. The winning
argument is recorded as precedent (§C). Each round logged to `research_argue.jsonl`.

## C. 3 independent judges + escalation (hard calls)
**Hardness detection** (`gov_hard`): route to the panel if ANY trigger fires — class is
`build`/`audit`; red-line/irreversible flag; blast-radius > 1 contract; architect sets
`no-decart-winner`; budget exceeded. Soft blueprints → cheap executor + verifier only.
**Anti self-confirmation**: judges drawn from `JUDGE_POOL` disjoint from author; each in
isolated context; ≥3 counterfactual passes + evidence-based DECART square + `DECISION` +
mandatory `PROBE`. **Aggregation** (`jury_aggregate`): ≥2/3 on one candidate → `Decide`;
split/abstain → `Escalate` to topic 267 + 294 (operator binds, never auto-merges).
**Falsifiability closure**: verifier runs the winner's *own* DECART check; if it fails →
auto-overturn + re-escalate. **Citation gate** (`gov_judge_gate`): a verdict lacking the
token `CITES:`/`DISTINGUISHES:`/`NO-BINDING-PRECEDENT` is RED-rejected by the verifier.

## D. Anglo-Saxon precedent registry (stare decisis)
Store `precedents.jsonl`, one row:
`{id, question, winner, evidence[], date, overturned, argued_rounds, jury[], binding}`.
**Bind gate** (`gov_precedent_bind`): bind a prior only if similarity ≥ τ=0.82 (Jaccard
proxy here; embed+cosine in prod) **and** `overturned==null`; else `NO-BINDING-PRECEDENT`
(greenfield decart). **Favor** the prior winner as presumption; **re-run decart**; outcomes:
`AFFIRM` / `DISTINGUISH` (material new criterion) / `OVERTURN` (prior fails a falsifiable
test it passed, or new evidence beats by ≥2× margin — link back-edit `overturned`). Mandatory
probe clause before affirming. **Judge feeding**: inject a PRECEDENT BRIEF; judges must open
with `CITES P-<id>` / `DISTINGUISHES P-<id> on <crit>` / `NO-BINDING-PRECEDENT`. Thus a
hard-won research victory compounds as binding authority — stare decisis over the
deliberative corpus, not popularity.

## E. Verification
Implemented in `governance.sh` + kernel `control.rs`; exercised by 73 kernel tests +
live functional checks (route picks max-EV under cap; precedent favors; judge escalates on
split; lane width ½-Kelly; bind gate; citation gate). See `HK-09-GOVERNANCE.md`.
