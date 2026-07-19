# MASTER ROADMAP — Swarm Safety, Telemetry-First (2026-07-19)

**Role of this document:** sequencing, gating, and ownership of ideas — nothing else. The
technical content lives in four companion documents and is not re-derived here:

| Doc | Role |
|---|---|
| `SWARM-SAFETY-DETERMINISTIC-CIRCUIT-BREAKER-SYNTHESIS-2026-07-19.md` (**Synthesis I**) | Signal layer, `Closed → Open → Half-Open / Killed` breaker, human-gate on Killed, built precedents (`field_physics.rs` `step_wave`, `TokenBucket::try_acquire`'s over-grant ceiling, decorrelated `research-verifier` pattern) |
| `SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` (**Synthesis II**) | Truthfulness-as-byte-reproducibility, batch-invariant prerequisite, PDDL-INSTRUCT correction, time-as-Foster-Lyapunov, red-team corpus (§7), measurement steps 6–12 (§8) |
| `BLUEPRINT-TELEMETRY-SAFETY-2026-07-19.md` (**Blueprint A**) | Phase 1–2 buildable architecture: signal layer, breaker, truthfulness/replay probes, Lyapunov/attractor loop monitor, **plus the hard prerequisite** — in-kernel batch-invariant reductions (`detreduce.rs`) that every truthfulness claim is gated behind (§4 of Blueprint A) |
| `BLUEPRINT-SYSTEMS-PHASE4-GATED-2026-07-19.md` (**Blueprint B**) | Everything else, in six sections: §A fluid/Laplacian routing + backpressure, §B mirror-model CoT observation + thought injection, §C boson/capability-token/Steward/break-glass mechanics, §D distributed sharding (HRW + lease), §E Bayesian reputation + random auditing, §F the dependency table binding every one of §A–§E to a Blueprint-A signal. **Gated. Not buildable until gate G3 (§3 below) passes.** |

## 0. The ordering is not negotiable

Operator directive, verbatim, this session:

> "now start with planning for everything - telemetery & safety number one priority, everything
> else after. Moreover first safety & telemetery should be planned & implemented, then intensive
> aggresive testing & injections against it - only after these first 3 steps, other can be coded."

That is four phases, strictly serial, each gating the next. One deliberate asymmetry, stated
plainly: **planning happens for the full system now** — Phase 1 through Phase 4 all get
extreme-quality blueprints and context (that is why Blueprint B exists today) — but **only
Blueprint A is authorized for implementation** until gate G3 passes. Planning ahead is not
building ahead: during Phases 1–3, Blueprint B is a design artifact, review target, and
threat-model input; it becomes a work order only after G3.

One standing rule cuts across all phases (Synthesis I §1.4): the `KILLED` breaker state for
red-line-classed actions (money/auth/RLS/migrations) never auto-resumes — human gate, unfiltered
logs to the operator. No phase, including Phase 4, relaxes this.

## 1. Phase 1 — PLAN safety & telemetry

**Deliverable:** Blueprint A, plus Blueprint B as the gated forward plan. This roadmap sequences
Blueprint A; it does not restate it.

**Scope checklist for Blueprint A's coverage:** signal layer (Syn I §1.1); breaker state machine
with thresholds *fit from measured FP/FN rates, not feel* (Syn I §1.2); one breaker primitive
across hallucination/poisoning/injection (Syn I §1.3); truthfulness replay probes (Syn II §1);
batch-invariant prerequisite (Syn II §2); Lyapunov/attractor loop monitor over the live
`order_machine.rs` + `markov_attractor.rs` (Syn II §4); the four physics-derived design rules
(Syn II §5 → Blueprint A §5.1–5.4) — Kepler conserved-quantity checks feeding
`constraint_violations` (TokenBucket permits + 3 proposed counters), Hooke linear-regime
EWMA/CUSUM as the first line with categorical checks as escalation past an unmeasured radius `r`,
time-as-Lyapunov-potential (restated above), and the VAL-style external categorical oracle behind
the constraint gate.

**Blueprint A's one hard prerequisite, stated plainly for a cold-start worker:** `detreduce.rs`
(in-kernel `DeterminismLedger` + fixed-reduction-tree reference ops) is *work item #1* — nothing
downstream (replay probes, truthfulness signal) is trustworthy until it verifies
`divergence_rate == 0.0` over 1000 runs. Until `DeterminismLedger::verified_invariant()` is true,
`SignalVector::truthfulness_fail` stays masked to 0 (fail-open on that one component, logged as an
`AuditKind::Disarm` event) — this is not a footnote, it is why Phase 2's build order (Blueprint A
§8) puts `detreduce.rs` before `signal.rs`.

**🧭 OPERATOR VISION — batch-invariant work lives inside the Rust kernel.** Direct operator
directive, confirmed mid-session: batch-invariant kernel work is implemented *and measured*
natively inside the Rust kernel/core — not bolted on as an external Python harness. This shapes
Blueprint A's architecture and is the operator's call, not an inherited convention.

**Exit (G1):** both blueprints exist, are internally consistent with Syntheses I–II, and Blueprint
A enumerates every measurement in Syn I §3 steps 1–5 and Syn II §8 steps 6–11 as buildable work
items. No code required. Inline, so this isn't a chase-the-doc exercise:

- *Syn I §3 (cheap de-risking, steps 1–5):* 1. re-verify `field_physics.rs:334` `step_wave` and
  the E1/E2 commit `6bd181a02` are still live on `main`; 2. instrument the telemetry spool with a
  Little's-Law (`L=λW`) check; 3. wire vLLM's native `logprobs`/`prompt_logprobs` into the
  confidence-gap signal (zero-build); 4. build the breaker state machine against synthetic
  poisoning/injection cases, report actual FP/FN; 5. only after 1–4, extend `research-verifier`'s
  decorrelated-review pattern from research docs to live swarm actions.
- *Syn II §8 (steps 6–11):* 6. 1,000 identical temp=0 completions against the live vLLM deploy,
  count unique outputs (zero new infra); 7. run the attractor binary retrospectively over labeled
  stuck-vs-healthy `journal.jsonl` sessions, report SLEM/escape-mass precision/recall (zero new
  compute); 8. instrument one refine loop's inter-checkpoint times, fit `q̂ = t_{n+1}/t_n` ± CI,
  cross-check the 1st- vs 2nd-order Markov assumption; 9. deploy `batch_invariant_ops`/SGLang
  deterministic mode on one replica, repeat step 6, arm the truthfulness signal only on 1000/1000,
  record overhead against the 1.6–2.1×/~34% published figures; 10. enumerate conserved quantities
  beyond `TokenBucket::try_acquire`'s over-grant check (message in/out counts, event-log monotonicity, budget totals) and
  wire each into `constraint_violations`; 11. run promptfoo memory-poisoning + garak agent-breaker
  against a canary agent with the breaker armed, then AgentDojo/InjecAgent, publish TPR/FPR per
  signal per threshold.
- **Honest gap, flagged rather than hidden per this arc's own house rule:** step 12 (the
  Frenet-Serret minimal-trajectory-statistic trial — Syn II §5, explicitly marked **ANALOGY**,
  unproven) is **not** enumerated anywhere in Blueprint A. It is not required for G1/G2/G3 and
  nothing in Phase 1–3 depends on it, but a Phase-4 worker who assumes "all 7 of Syn II §8's
  steps are covered by Blueprint A" would be wrong by one — this is the correction, recorded here
  rather than silently re-scoped.

## 2. Phase 2 — IMPLEMENT safety & telemetry

**Deliverable:** Blueprint A built. Nothing from Blueprint B — not "just the easy parts," nothing.

**Contents (owned by Blueprint A):** signal-measurement layer, breaker state machine, replay-probe
mechanism, loop monitor, batch-invariant kernel path (per the flagged operator vision). Includes
the cheap de-risking measurements: Syn I steps 1–3 (re-verify live precedents; Little's-Law
instrumentation of the telemetry spool; logprobs → confidence gap) and Syn II step 6 (the
1,000-identical-completion divergence baseline — zero new infra, falsifies-or-confirms the §2
confound on our stack).

**Exit (G2):** Blueprint A's components run; step 6's baseline number is in hand; the breaker
trips end-to-end on manufactured signals. Concretely, so "in hand" and "trips end-to-end" aren't
left vague:

- **Step 6 baseline, in hand** = 1,000 identical temp=0 completions were actually run against the
  live vLLM deploy and the unique-output count was recorded — whatever that number is. Research
  predicts "tens of unique outputs" (confirming the batch-invariance confound on our own stack);
  a surprise 1000/1000 would mean the prerequisite is already satisfied and §4's cost estimate is
  void. Either outcome satisfies G2; only a number *not measured* fails it.
- **Breaker trips end-to-end** = three concrete, buildable-now checks per Blueprint A §6 pass:
  (a) a property test over the full `state.rs` transition table (§3) holds, including cooldown
  doubling capped and the red-line `Killed` state never auto-resuming; (b) an injected
  deletion/reorder in the audit ring is caught via a `seq` gap or `prev_hash` break (unit test);
  (c) a grep test asserts **zero numeric literals** for `θ_open`/`θ_kill` in `state.rs` — every
  threshold comes from `fit_from_rates()`, never a hand-picked constant.

Note what G2 does **not** claim: that the system is safe. Unit-level green is explicitly
insufficient — that is the whole point of Phase 3.

## 3. Phase 3 — AGGRESSIVE TESTING (the gate, not a nice-to-have)

**🧭 OPERATOR VISION — defenses are validated by actual attacks, not unit tests.** The
requirement that every defense layer be exercised by real aggressive injections — not tested in
isolation and declared done — is the operator's, stated in the source conversation. It directly
produced Synthesis II §7's retirement of "synthetic labeled cases" in favor of a named red-team
corpus. This phase exists because of that directive.

**Tooling (Syn II §7, verbatim mapping):** **promptfoo** Memory Poisoning plugin (replay probes +
EWMA drift), **garak** v0.15 Agent-breaker + system-prompt extraction (constraint-violation
count, disagreement), **PyRIT** XPIA/crescendo multi-turn (CUSUM change-point), **AgentDojo**
(97 tasks, 629 paired security tests) + **InjecAgent** (1,054 cases) for the full breaker matrix.
Threat-model anchor: OWASP Agentic Top 10 (Dec 2025), ASI06 Memory & Context Poisoning.

**Exit criterion (G3) — measured numbers, not vibes.** Phase 4 unlocks only when ALL of the
following artifacts exist, pulled directly from Syn I §3 / Syn II §8:

1. **Step 11 complete:** promptfoo memory-poisoning + garak agent-breaker run against a canary
   agent with the breaker armed, then the AgentDojo/InjecAgent matrix — producing **TPR/FPR per
   signal per threshold**, and `θ_open`, `θ_kill`, `W`, `T` fitted from those measured
   false-positive/false-negative rates (Syn I §1.2's explicit demand).
2. **Step 9 complete:** batch-invariant canary shows **1000/1000 bitwise-identical completions**
   (vs. the published ~80-unique-per-1000 baseline); replay probes armed only on that result;
   measured throughput cost recorded against the published 1.6–2.1× / ~34% figures.
3. **Step 7 complete:** retrospective SLEM/escape-mass run over labeled stuck-vs-healthy
   `journal.jsonl` sessions with reported precision/recall; if SLEM→1 doesn't beat a coin flip,
   the §4 q-proxy is downgraded to dead — recorded, not hidden.
4. **Step 8 complete:** q̂ ± CI published for at least one refine loop, plus the 1st-vs-2nd-order
   Markov likelihood test bounding Syn II §4's "rigorous within the model" clause.
5. **Falsification honored:** any signal whose falsifiable condition fires (steps 7, 10, 12) is
   downgraded or removed — never re-tuned until it passes. Editing the definition of "pass" after
   red-teaming is the exact failure mode this gate exists to prevent. Concrete conditions, inline:
   step 7 — if SLEM→1 doesn't beat a coin flip separating stuck from healthy loops, the §4 q-proxy
   is dead; step 10 — if fewer than 2 conserved quantities exist beyond `TokenBucket::try_acquire`,
   the Kepler principle is recorded as "real but thin here," not silently dropped; step 12 (if
   attempted — see the G1 gap noted above, Blueprint A does not currently build this trial) — a
   high collision rate between distinct-outcome trajectories mapped to "same shape" falsifies the
   Frenet-Serret analogy for that representation and sends it back to search.

## 4. Phase 4 — EVERYTHING ELSE (unlocked by G3 only)

**Unlock condition, self-contained:** all five G3 artifacts in §3 above measured-green
(fitted `θ_open`/`θ_kill`/`W`/`T` from red-team TPR/FPR; 1000/1000 batch-invariant baseline;
SLEM precision/recall beating a coin flip; q̂ ± CI published; no un-honored falsification). Not
one commit of Blueprint B lands before that — Blueprint B's own header repeats this as a blocking
banner, and roadmap §6 below calls a pre-G3 "small Phase 4 spike" a violation, not initiative.

Owned by Blueprint B (six sections, §A–§F; §F is the dependency table binding every other section
to a specific Blueprint-A signal — read it first when this phase unlocks). Contents, with
idea-ownership flagged:

**🧭 OPERATOR VISION — mechanical deterministic wrapping of stochastic dataflow.** The operator
explicitly warned: "do not reject & miss my vision to use mechanical deterministic approaches to
direct & wrap over the stochastic data flow." Synthesis I §2 rejects literal Navier–Stokes/CFD
(decorative for discrete routing) but does **not** abandon the vision — it fulfills it in
rigorous form: graph-Laplacian `e^{−tL}` routing over a fixed auditable matrix and
conservation-law backpressure (Little's Law; `TokenBucket::try_acquire`'s refill-then-consume as a discretized continuity
equation). Stochastic content flows; deterministic pipes direct it. That translation is the
operator's idea made falsifiable, not a substitution of it.

**🧭 OPERATOR VISION — the "boson" swarm architecture.** The homogeneous stateless-agent
("boson") metaphor and the entire capability-token / Steward / break-glass / recursive-delegation
architecture originate in the operator's brainstorm, not external literature (Syn I §1.5 lists
them PROPOSED). Blueprint B's Steward election (TTL lease + fencing token over the existing
event-sourced kernel, no-K8s — Syn II §6) and geo-cell + rendezvous-hash sharding implement it.

**🧭 OPERATOR VISION — the sanctioned god-function.** The emergency-recovery/break-glass
chokepoint (Shamir key reconstruction) is *deliberately* one monolithic function — the operator's
explicit exception to the twelve anti-pattern rules, justified in Syn II §6: one entry point to
gate behind the human `Killed` unlock, one audit surface, zero composition with normal paths.
Preserved as a security control, not debt.

**Blueprint B §B — mirror-model CoT observation + thought injection.** Not previously called out
in this roadmap; recorded here. A decorrelated observer (different model/provider, the same
pattern `research-verifier` already runs for research docs — Syn I §1.5) pairs with every
reasoning agent to summarize/flag its chain-of-thought; a planted canary fact measures a
false-negative rate (canaries missed). Gated specifically because its two outputs —
`disagreement` and `canary_hit` — are *inputs to the breaker's* cross-agent-disagreement and
confidence-gap signals (Blueprint B §F), not a parallel safety system; building the observer
before the breaker exists means its verdicts have nowhere to trip.

**🧭 OPERATOR VISION — Bayesian trust is never binary, and high-reputation nodes still get
audited.** Blueprint B §E (RAW-PROMPT items 13, 15, 16) — every node carries a Beta-posterior
reputation; low-reputation nodes get heavier verification, but the operator explicitly named the
"echo chamber" risk of trusted nodes going unchecked, so `audit_probability` has a hard floor
`> 0` even at maximum reputation. This was missed in an earlier pass of this roadmap's Operator
Vision Register (§5) — corrected here. Reputation is a **weighting on the existing breaker
signals**, not an independent trip authority: it maps directly onto the breaker's own states
(warning → `Closed`-trip, quarantine-read-only → `Open`, kill-and-replace → `Killed`) rather than
introducing a fourth state machine. Gated because a reputation system that *replaced* rather than
*fed* the breaker would let a high-reputation compromised node escape scrutiny — exactly the
echo-chamber failure it exists to prevent.

## 5. Operator Vision Register (9 flags total as of this pass — recounted, not assumed; 6 inline
in §§1–4 above (including the Bayesian-reputation flag newly added to §4), 3 below in this
section — all operator-originated, none inherited)

- **🧭 OPERATOR VISION — truthfulness replaces faithfulness.** Rejecting the model's self-report
  ("faithfulness") and defining safety as **byte-for-byte identical output under identical input
  and conditions, checked across time** — explicitly compared by the operator to cache/retrieval
  semantics: same key + same conditions ⇒ same value; divergence ⇒ corruption, no content
  interpretation needed (Syn II §1). Syn II §2's honest search found *no prior work* using
  byte-identical replay as a poisoning/hallucination detector — a genuinely novel combination,
  with the batch-invariant prerequisite as its single point of failure.
- **🧭 OPERATOR VISION — time as the loop-refinement metric.** The operator proposed *time*,
  tied to the "topology & chronology layers… inside the attractor," as the distance closing
  Synthesis I's open Banach question. Syn II §4 corrected the formalization (time-to-green is not
  a metric; Banach with d = time is wrong) while confirming the instinct was structurally
  correct: V(state) = expected residual time-to-green is a **Foster-Lyapunov potential**, the
  attractor's drift statistic already estimates it, and q̂ = tₙ₊₁/tₙ is the falsifiable
  contraction estimate — via the already-live `order_machine.rs` + `markov_attractor.rs`.
- **🧭 OPERATOR VISION — chronological-comparison detection.** The method of replaying the same
  injected conditions at different points in time and treating *any* divergence as the signal is
  the operator's; Syn II §1 operationalizes it as replay probes (Half-Open canaries = stored
  golden pairs, bitwise match to close), the exact inverse of SelfCheckGPT's variance-mining
  assumption.
- (Attack-validation, native-Rust batch-invariance, mechanical-wrapping, boson/Steward, the
  god-function exception, and Bayesian-reputation-never-binary are flagged inline in §§1–4 above —
  6 of the 9 total; the 3 above in this section make 9.)

## 6. Standing constraints

- Blueprint B is read/review-only until G3. Any "small Phase 4 spike" before G3 is a roadmap
  violation, not initiative.
- Every phase-exit number is recorded in the doc that produced it; nothing is reported "working"
  without one (Syn I §3 closing rule).
- Killed-state human gate and red-line classes persist unchanged through all phases.
