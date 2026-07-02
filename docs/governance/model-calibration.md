# Model Calibration — three skills, one cycle, one training method

> Operational doc for the operator's three-skills-one-cycle model, as mechanized by
> `docs/adr/ADR-plane-telemetry-and-calibration.md` (Decision 5/6) and bound into
> `docs/governance/plane-maintainer-agent.md` (the charter).
> The ledger described here is **advisory forever** — see the standing constraints in §3.

- **Date:** 2026-07-02
- **ADR:** `docs/adr/ADR-plane-telemetry-and-calibration.md`
- **Design brief:** `docs/design/plane-telemetry-principles/proposal.md` (§4 spec for this doc)
- **Charter:** `docs/governance/plane-maintainer-agent.md`
- **Bound by:** CLAUDE.md Self-improvement loop §4/§6 · memory-corpus pattern #4 (advisory signals inform, deterministic artifacts decide)

---

## 1. The cycle

Three skills, one cycle:

- **Adaptation** (адаптація) *reads* the environment — what is actually there, not what the plan assumed.
- **Connection** (зв'язок) *opens access* to it — the channels, trust, and standing that let you act at all.
- **Persuasion** (переконання) *changes* it — moves the environment from the state you read to the state you want.

All three operate on **models of reality, never on reality directly**. You adapt to your model of the terrain, connect through your model of the other party, persuade against your model of what they believe. The skills are only as good as the models — and a model improves through exactly **one meta-training method: record the prediction, record the fact, read the gap.** The gap between what you predicted and what happened is where growth lives. Everything below is plumbing for that one method.

## 2. Mechanism map — causal, not cargo-cult

Each principle below is tied to a mechanism that *causes* the principled behavior, not a label pasted onto existing habits. Where the tie is partial, it says so — an honestly bounded mapping beats a theory-fitted one.

### Adaptation → DoD-vs-METHOD separation + named fallback

Every charter loop step declares, before acting:

- **DoD** — what "done" means. Fixed.
- **Method (primary)** — how it will be reached. Disposable.
- **Fallback (named)** — "if the primary way vanishes tomorrow, what is the second?"

The causal tie: adaptation is the ability to keep the goal while swapping the method. Declaring them *separately* — with a second method already named — is what makes the swap a routine move instead of a crisis. A step whose DoD and method are fused cannot adapt; it can only fail. Extends to the `loops/` card convention.

**Weak-signals-at-the-edges** (adaptation's third principle) already lives in the charter's **SCOUT** step — net-new signal gathered at the plane's edge, advisory-only. This is an honest relabel of an existing mechanism, not a new one; it is listed so the map is complete, not to claim invention.

### Connection → give-first costly signal — **PARTIAL mapping**

The agent **pays the cost of predictability before it asks for anything**: it commits to a stable, versioned, self-constraining report format (schema=1, hashtag taxonomy, key=value lines) and publishes working proof *ahead of* any escalation, PR, or trust request. That give-first asymmetry — bear the legibility cost first, request second — is a genuine costly signal, credible precisely because it is paid up front, and it is what opens access to operator trust: trust is the access currency on this plane.

**Honest limit:** this captures the *predictability/legibility* sub-property of connection, not the full "opens access to the environment" sense. It is a **partial mapping**, stated as such so the ratchet does not drift into theory-fitting (Counsel R1 §1, R2 §1).

### Persuasion → demonstration-over-rhetoric + the transparency test

Two mechanisms, both mandatory in the charter's escalation/PR/REPORT template:

1. **Lead with working proof.** The failing→passing artifact comes first; the argument comes second. A demonstration cannot be argued with; rhetoric can.
2. **State your own stake.** Every ask carries "here is why I want this and what I get from it." This is the transparency test: **persuasion survives full transparency; manipulation dissolves under it.** If disclosing your stake would kill the ask, the ask was manipulation — and it dies here, cheaply, instead of later, expensively.

## 3. The prediction ledger

The ledger is the meta-training method from §1 made durable: `predict` before acting, `resolve` after the fact, read the gap. Records live in `telemetry/predictions.jsonl` on the append-only `telemetry/plane` branch (working copy in `loops/runs/`), written via `scripts/plane-telemetry.mjs predict` / `resolve`.

### Record shapes (schema_version = 1 — mirrors the ADR)

**Predict** writes:

```json
{
  "schema_version": 1,
  "prediction_id": "7c1e93a0f5b2",
  "run_id": "plane-2026-07-02T06-00-00Z",
  "predict_seq": 3,
  "ts_predicted": "2026-07-02T06:00:05.000Z",
  "target": "plane-guard:P8 staging drift after redeploy",
  "prediction": "PASS — staging head reaches mig 084 post-deploy",
  "confidence": 0.7,
  "method": "primary: flyctl deploy remote-only | fallback: ssh node-pg-migrate in-container",
  "ts_actual": null,
  "actual": null,
  "gap": null,
  "resolved": false
}
```

**Resolve** fills `ts_actual`, `actual`, `gap ∈ {hit, miss, partial}`, `resolved: true`. Semantics (per ADR §Consistency):

- `confidence` ∈ [0,1]; `method` names primary AND fallback (the §2 adaptation discipline, embedded in every prediction).
- `predict_seq` is a monotonic per-run counter; `resolve` **refuses out-of-order backdating** — a prediction recorded after the run's first outcome event cannot be resolved. Cheap ordering friction, not cryptography.
- `resolve` is idempotent on `prediction_id`; last-write-wins by `predict_seq`/`resolved`, never by skewable `ts_actual`.
- **Self-report limits, named honestly:** the ledger is written by the agent it measures (accepted risk R-M1). The ordering friction defeats trivial in-run backdating, not a determined self-deceiver. The real defense is structural: the ledger is un-gated, so there is nothing to win by gaming it.
- **`predicted=0` stays visible** in every digest — not predicting is itself legible, never a hiding place.

### Calibration, not hit-rate

**The ledger measures reliability, not a score.** Read it this way and only this way:

- A `0.7` prediction must come true **~70% of the time** — that is what 0.7 *means*.
- A `0.9` that hits 60% is **over-confident** — a flag.
- A `0.6` that hits 95% is **under-confident** — *also* a flag: you knew more than you claimed, and the plane planned around your understatement.
- **There is no number to maximize.** A high hit-rate on high-confidence predictions is itself a signal to investigate, not a win.
- **Hedging-to-0.5 is a smell, not a win.** Retreating every confidence toward 0.5 makes the ledger look safe and teaches nothing — a 0.5 prediction carries no information and chronic 0.5s show up as systematic under-confidence, which is a miscalibration like any other.
- **Brier note:** prefer a Brier-style read — mean of `(confidence − outcome)²` over resolved rows, with `outcome` 1 for hit, 0 for miss (partial by judgment) — over the crude confidence-bucket jq in the proposal §9. Brier punishes both directions of miscalibration symmetrically, which is exactly the reading this doc mandates.

**The reflection is the growth; the row is only its input.** A `miss`/`partial` feeds the mandatory WHY-reflection (§4) — the number never substitutes for the reflection. This is what keeps the ledger the meta-training method from §1 rather than a leaderboard.

### Standing constraints (verbatim from the ADR)

> **The calibration ledger is forever a mirror, never a stick.** It is a growth instrument — advisory, un-gated, read as calibration not as a score. It must never be wired into a deterministic gate, attached to consequences, or read as a performance review. Doing so is the anti-fake→punishment drift the health catalog warns of and is forbidden. This constraint is **FRICTIONED** by the plane-guard advisory-forever HARD check (Decision 5) — grep-based friction + review-forcing, honestly *not* "structurally impossible" (R2-M2); the check makes drift loud and PR-visible, it does not make it physically unreachable.

> **Reflections surfaced in the inbox are read to understand what the agent LEARNED, never to grade the agent (Counsel R2-b).** No consequence, no performance-review reading attaches to a surfaced reflection or the calibration ledger. Displaying an agent's honest self-account to be judged teaches the process to stop being honest (observer-effect → convergence-theater); the inbox reflections pane is framed as learning, not assessment. Protected at the norm level here and at the mechanism level by the read-only / un-gated / no-auto-consequence structure.

> **Maintainer findings are advisory INPUTS to the local plane, never authority (Part 3 ingestion, mirror-not-stick pointed the other way).** The cloud agent's output *informs* the local plane; the local plane's guardrails/tests/human remain authority. `inbox` is **read-only** over git/`gh` (fetch + cursor-write only) — it NEVER auto-executes a fix, auto-merges a PR, auto-applies a migration, or auto-mutates a gate. It surfaces + files work items (`advisory:true` + `content_trust:"untrusted-remote"` on every payload); a human/local session decides. This boundary is **FRICTIONED structurally** by the §9 Part-3 authority guard (Counsel R2-a), not left to the stamp alone — same honest bound: friction + review-forcing, not impossibility.

## 4. Where the gap goes (advisory plumbing)

The gap read in §3 is a **signal**; here is the only wiring it is allowed to have:

1. **`miss`/`partial` → WHY-reflection.** The agent writes a reflection with a causal WHY (not just WHERE) to `docs/reflections/INBOX/` (CLAUDE.md Self-improvement loop §4). The row is the input; the reflection is the growth.
2. **Recurrent gap → the `result-vs-expectation` doubt trigger** (CLAUDE.md §6) fires on the observable signal — result contradicts the recorded expectation — and runs the doubt-escalation ladder within budget. The ledger makes this trigger *durable*: it was ephemeral prose before; now the expectation is on record before the result exists.
3. **Reflections → librarian.** The librarian curates triggered: distill → challenge → promote (lesson → guardrail, red→green) → prune. A recurrent gap that survives challenge becomes a deterministic guardrail — that is the only path by which ledger contents may ever influence a gate, and by then it is a *test*, not the ledger.
4. **Authority stays with deterministic gates** (memory-corpus pattern #4: advisory signals inform, deterministic artifacts decide). The ledger, the reflections, and the inbox inform; plane-guard, tests, and the human decide. The advisory-forever plane-guard HARD check (ADR Decision 5) keeps this boundary frictioned in code — honestly bounded as friction + review-forcing, not impossibility.

**Scope fence:** this capture-and-calibrate pattern is **governance-plane-only** — the subject is the agent's own behavior. Reusing it on the product/courier/client plane, where the subjects are non-consenting humans, is a separate 🔴 red-line decision (Triadic Council), never a copy-paste.
