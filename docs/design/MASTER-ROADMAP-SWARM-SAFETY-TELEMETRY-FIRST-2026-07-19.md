# MASTER ROADMAP — Swarm Safety, Telemetry-First (2026-07-19)

**Role of this document:** sequencing, gating, and ownership of ideas — nothing else. The
technical content lives in four companion documents and is not re-derived here:

| Doc | Role |
|---|---|
| `SWARM-SAFETY-DETERMINISTIC-CIRCUIT-BREAKER-SYNTHESIS-2026-07-19.md` (**Synthesis I**) | Signal layer, `Closed → Open → Half-Open / Killed` breaker, human-gate on Killed, built precedents (`field_physics.rs` `step_wave`, `TokenBucket::release`, decorrelated `research-verifier` pattern) |
| `SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` (**Synthesis II**) | Truthfulness-as-byte-reproducibility, batch-invariant prerequisite, PDDL-INSTRUCT correction, time-as-Foster-Lyapunov, red-team corpus (§7), measurement steps 6–12 (§8) |
| `BLUEPRINT-TELEMETRY-SAFETY-2026-07-19.md` (**Blueprint A**) | Phase 1–2 buildable architecture: signal layer, breaker, truthfulness/replay probes, Lyapunov/attractor loop monitor |
| `BLUEPRINT-SYSTEMS-PHASE4-GATED-2026-07-19.md` (**Blueprint B**) | Everything else — fluid/topology routing, capability-token/Steward/break-glass mechanics, distributed sharding. **Gated. Not buildable until the Phase 3 gate passes.** |

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
`order_machine.rs` + `markov_attractor.rs` (Syn II §4).

**🧭 OPERATOR VISION — batch-invariant work lives inside the Rust kernel.** Direct operator
directive, confirmed mid-session: batch-invariant kernel work is implemented *and measured*
natively inside the Rust kernel/core — not bolted on as an external Python harness. This shapes
Blueprint A's architecture and is the operator's call, not an inherited convention.

**Exit (G1):** both blueprints exist, are internally consistent with Syntheses I–II, and Blueprint
A enumerates every measurement in Syn I §3 steps 1–5 and Syn II §8 steps 6–10 as buildable work
items. No code required.

## 2. Phase 2 — IMPLEMENT safety & telemetry

**Deliverable:** Blueprint A built. Nothing from Blueprint B — not "just the easy parts," nothing.

**Contents (owned by Blueprint A):** signal-measurement layer, breaker state machine, replay-probe
mechanism, loop monitor, batch-invariant kernel path (per the flagged operator vision). Includes
the cheap de-risking measurements: Syn I steps 1–3 (re-verify live precedents; Little's-Law
instrumentation of the telemetry spool; logprobs → confidence gap) and Syn II step 6 (the
1,000-identical-completion divergence baseline — zero new infra, falsifies-or-confirms the §2
confound on our stack).

**Exit (G2):** Blueprint A's components run; step 6's baseline number is in hand; the breaker
trips end-to-end on manufactured signals. Note what G2 does **not** claim: that the system is
safe. Unit-level green is explicitly insufficient — that is the whole point of Phase 3.

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
   red-teaming is the exact failure mode this gate exists to prevent.

## 4. Phase 4 — EVERYTHING ELSE (unlocked by G3 only)

Owned by Blueprint B. Contents, with idea-ownership flagged:

**🧭 OPERATOR VISION — mechanical deterministic wrapping of stochastic dataflow.** The operator
explicitly warned: "do not reject & miss my vision to use mechanical deterministic approaches to
direct & wrap over the stochastic data flow." Synthesis I §2 rejects literal Navier–Stokes/CFD
(decorative for discrete routing) but does **not** abandon the vision — it fulfills it in
rigorous form: graph-Laplacian `e^{−tL}` routing over a fixed auditable matrix and
conservation-law backpressure (Little's Law; `TokenBucket::release` as a discretized continuity
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

## 5. Operator Vision Register (remaining items — all operator-originated, none inherited)

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
- (Attack-validation, native-Rust batch-invariance, mechanical-wrapping, boson/Steward, and the
  god-function exception are flagged inline in §§1–4 above.)

## 6. Standing constraints

- Blueprint B is read/review-only until G3. Any "small Phase 4 spike" before G3 is a roadmap
  violation, not initiative.
- Every phase-exit number is recorded in the doc that produced it; nothing is reported "working"
  without one (Syn I §3 closing rule).
- Killed-state human gate and red-line classes persist unchanged through all phases.
