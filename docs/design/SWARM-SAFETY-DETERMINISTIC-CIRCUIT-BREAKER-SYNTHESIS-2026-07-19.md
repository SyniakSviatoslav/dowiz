# Swarm Safety: Deterministic Circuit-Breaker Synthesis

**Companion to:** `RAW-PROMPT-agentic-swarm-safety-fluid-architecture-2026-07-19.md` (verbatim
source). This file is the requested synthesis — priority-ordered per the operator's directive:
(1) deterministic safety/tracing first, (2) fluid/topological data processing + mirror-model
CoT observation second, honestly scoped. No claim in this doc is presented as "done" unless a
measurement backs it; anything unmeasured is marked **PROPOSED**, not **BUILT**.

---

## 0. Research findings — the 13 named projects (condensed)

Full detail in the three subagent transcripts this session; here is what's actionable.

**Course/curated-list repos (low code reuse, but two genuine hits):**
- `Shubhamsaboo/awesome-llm-apps` — real runnable code (Apache-2.0). Its "Trust-Gated
  Multi-Agent Research Team" app implements a **hash-chained audit trail per agent action** —
  directly reusable pattern for §2 below.
- `NirDiamant/agents-towards-production` — code-first tutorials, best fit of the seven:
  `agent-security-with-llamafirewall` (input/output/tool guardrails), `tracing-with-langsmith`
  (production observability), `agent-memory-with-mem0`/`-redis`.
- `mlabonne/llm-course`, `huggingface/agents-course`, `NirDiamant/GenAI_Agents`,
  `e2b-dev/awesome-ai-agents`, `ashishpatel26/500-AI-Agents-Projects` — course material or pure
  link-lists, incidental security mentions only. `GenAI_Agents` references **AgentContract**
  (open spec for tamper-evident behavioral-constraint enforcement across frameworks) — worth a
  direct look, not yet vetted.
- **Confirmed gap across all 7:** none have a dedicated memory-poisoning or hallucination-
  detection module. That has to come from elsewhere (Guardrails AI / NeMo Guardrails /
  LlamaFirewall itself) or be built.

**Training/serving tools:**
- **vLLM** — the one clear infrastructure win. Native OpenTelemetry tracing + Prometheus
  `/metrics` (queue depth, KV-cache occupancy) + **token-level `logprobs`/`prompt_logprobs` via
  its OpenAI-compatible API** — this is a ready-made, zero-build confidence-gap signal for
  hallucination detection (cluster II §2.3.1 in the compendium). Caveat: not bitwise-
  deterministic by default (dynamic batching); Thinking Machines Lab's batch-invariant-ops patch
  achieves 100% reproducible runs on top of it if that's a hard requirement.
- **Axolotl** (org now `axolotl-ai-cloud`) — declarative YAML training configs are themselves an
  audit artifact (diffable, versionable) — good fit for a system that wants training runs to be
  reconstructable, not just fine-tunes.
- **unsloth** — fast fine-tuning, no bespoke observability beyond what it inherits from HF
  Trainer; fused kernels can shift numerics vs. vanilla HF, worth flagging if reproducibility
  matters.
- **LM Studio** — closed-source binary. Direct conflict with a "not a black box" design
  requirement — fine for local prototyping, exclude from the audited/production path.
- **AutoTrain** — dead (official README says no longer maintained). Excluded.
- **Cross-cutting finding:** none of the five expose attention/hidden-state introspection as a
  first-class feature — that has to come from raw HF `transformers`
  (`output_hidden_states=True`) or an interpretability library (nnsight, TransformerLens),
  regardless of which serving/training stack is chosen. Relevant to §3.3 below.

**`shiaho777/web-to-app`** — resolves exactly as named, real and active (4,977 stars). Wraps a
website into an installable Android APK. No direct tie to the safety architecture; noted for
completeness since it was on the list.

---

## 1. The core reframe: replace "faithfulness" with a control-theory circuit breaker

The operator's instruction was explicit: don't trust "the model says it's fine" (faithfulness).
Use the same math safety-critical industries use — deterministic thresholds on measured
telemetry, not vibes. This is not a new idea for this repo: cluster II of the compendium already
describes a 5-layer guardrail stack and a 6-layer memory-poisoning defense; the brainstorm
already converges on a Closed→Quarantine→Kill state machine. What's missing is making the
**trip condition itself a closed-form, falsifiable function of measured signals** — which is
what a real circuit breaker is (Hystrix/resilience4j pattern: `Closed → Open → Half-Open`, one
numeric error-rate threshold, one cooldown timer).

### 1.1 Signal layer (what gets measured, on every action, no sampling)

| Signal | Formula | Source |
|---|---|---|
| Confidence gap | `1 − max(softmax(logits))` per token, or `mean(prompt_logprobs)` | vLLM logprobs — already available, zero build |
| Behavioral drift | EWMA of action-embedding distance from an agent's own rolling baseline: `d_t = ‖a_t − μ_{t-1}‖`, `μ_t = (1−α)μ_{t-1} + α·a_t` | needs an action-embedding function (reuse compendium §1.2 embeddings) |
| Change-point / regime shift | CUSUM: `S_t = max(0, S_{t-1} + (x_t − μ_0 − k))`, trip when `S_t > h` | classic SPC (statistical process control), h/k tuned from a measured false-positive budget, not intuition |
| Constraint violation count | count of writes rejected by the causal-graph predicate gate (already specified in cluster II §2.3.2 L4) | Tensor Arena / World Model write path |
| Cross-agent disagreement | fraction of shadow-execution pairs whose outputs diverge beyond a measured tolerance | requires shadow/dual execution (brainstorm idea #16) |

This table is the literal answer to "no ethics/faith, just pure deterministic approach" — every
row is a number with a formula, not a judgment call.

### 1.2 Breaker state machine (deterministic, not tunable per-vibe)

```
CLOSED (normal) --[trip_score > θ_open for W consecutive windows]--> OPEN (quarantine: read-only)
OPEN --[cooldown timer T elapses]--> HALF_OPEN (canary: N probe tasks only)
HALF_OPEN --[all N probes pass measured spec]--> CLOSED
HALF_OPEN --[any probe fails]--> OPEN (T doubles, capped) -- this is the hysteresis from
                                                               cluster "Мережа зв'язків, Вісь C"
OPEN --[trip_score stays > θ_kill for W_kill windows]--> KILLED (irreversible for this instance;
                                                                  Steward reissues a fresh
                                                                  capability token to a new
                                                                  instance, per brainstorm idea #9)
```

`θ_open`, `θ_kill`, `W`, `T` are fit from measured false-positive/false-negative rates on a
labeled test set (synthetic poisoning/injection cases), **not** picked by feel — this is the
concrete difference between this design and "human tolerance" thresholds.

### 1.3 One breaker primitive, reused across every anomaly class

The operator asked for this explicitly: extend the same math to poisoning, hallucination, and
prompt injection rather than building three bespoke systems. The breaker in §1.2 is
signal-agnostic — swap the input vector:

| Anomaly class | Signal vector fed to the breaker | Existing spec to build on |
|---|---|---|
| Hallucination | confidence gap + cross-agent disagreement | compendium cluster II §2.3.1 "Confidence Gap" row |
| Memory poisoning | anomalous-write rate + source-provenance mismatch + trust-tier violation | compendium cluster II §2.3.2 (L1–L6) — already fully specified, this synthesis just makes L on trip condition numeric |
| Prompt injection | input-classifier score (a small, fast, separately-trained classifier — NOT the agent judging its own input) + constraint-violation count on the FIRST action after the suspect input | `NirDiamant/agents-towards-production`'s `agent-security-with-llamafirewall` tutorial as a concrete starting implementation |

### 1.4 The one place the breaker does NOT self-close

Matches this repo's own existing red-line rule (money/auth/RLS/migrations preserved even under
full autonomy — see CLAUDE.md). `KILLED` state for an irreversible/red-line-classed action
never auto-resumes; it requires the human-in-the-loop gate the operator specified: "the model is
stopped immediately & automatically... while human in gate should manually check this out." All
logs (the audit ring-buffer from brainstorm idea #14) are shipped to the operator, unfiltered,
regardless of whether the breaker later determines it was a false trip.

### 1.5 What's already built vs. what's proposed

- **Already exists in this session's own tool roster** (not hypothetical — check before
  building): `research-verifier` is already a decorrelated adversarial verifier running on a
  *different model/provider* than the thing it checks — this is the "mirror model" from Part 3
  of the operator's ask, already implemented for research reports. `system-breaker` +
  `system-architect` + `counsel` is already an adversarial triad for design review.
  `invariant-guardian` and `security-sentinel` are already read-only semantic reviewers gating
  commits. The gap is not "build a mirror model" — it's "wire the existing decorrelated-review
  pattern to swarm *runtime* actions, not just design/research docs." **PROPOSED**, not built.
- **PROPOSED, not built:** the breaker state machine itself, the signal-measurement layer, the
  capability-token/Steward/break-glass machinery from the brainstorm.

---

## 2. Fluid/topological data processing + mirror-model CoT — honest scoping

The operator asked for literal mechanical/fluid equations for data routing, in n dimensions, not
a text metaphor, and explicitly said "instead of relying on established ways, think how to
actually achieve this." Honest answer: **the literal PDE (Navier–Stokes) is the wrong tool** —
continuous fluid simulation doesn't map onto discrete token/packet routing without becoming
decorative. But the operator's instinct is not new to this project and is NOT wrong at the level
that matters — it's already been validated, measured, and partially built under a different
name, three times over, in memory arcs this session inherited:

- **`physics-ui-capture-quantum-math-arc-2026-07-14`** already proved, with a citation
  (Chung, PNAS 2007), that recall/decay/UI-motion/blur/ripple are **five instances of one
  operator**: `f(L)` of a single graph Laplacian `L`. Gaussian blur ≡ heat equation `e^{−tL}` —
  proven equal, not analogous. The wave form `MÜ + ΓU̇ + c²LU = S` (mass/damping/stiffness/
  source) is **already implemented** at `crates/bebop/src/field_physics.rs:334` (`step_wave`) —
  verify this line still holds before citing it as fact, this memory is 5 days old.
- **`spectral-energy-flow-status-2026-07-16`** already landed E1 (Laplacian sign-parity proof)
  and E2 (a measured Wilson lower-bound of 0.7575 on a 12-query oracle, not an estimate) at
  commit `6bd181a02`.
- **`agentic-mesh-protocol-status-2026-07-17`** already has `TokenBucket::release` and
  `AgentBridge` landed (`f30189262`) — a token-bucket is, literally, a discretized
  conservation-law flow model (inflow − outflow = Δ stored, the same continuity equation
  `∂ρ/∂t + ∇·(ρv) = 0` underlying every fluid PDE, just applied to a scalar reservoir instead of
  a continuum).

So the honest translation of "data as fluid, not tokens" is: **you already have the operator.**
The legitimate mathematical core of the brainstorm's request is graph-Laplacian diffusion +
conservation-law flow accounting, not literal CFD. Concretely, buildable and measurable:

1. **Conservation-law backpressure** (PROPOSED): model every queue/spool in the swarm (e.g. the
   telemetry spool drainer already in `tools/telemetry/lib.sh`) as a node with a measured
   inflow/outflow rate, and check Little's Law (`L = λW`) holds empirically — a genuinely cheap,
   real, falsifiable first experiment, not a moonshot.
2. **Graph-Laplacian routing** (PROPOSED): position each data stream as a node in an embedding
   space (reuse the Hilbert-space machinery from compendium §4.1 — same inner-product,
   same-norm reasoning already used for embeddings), route/mix via `e^{−tL}` diffusion instead of
   ad-hoc branching logic — deterministic because `L` is a fixed, auditable matrix, not a learned
   black box. Stochastic *content* still flows through the pipe; the *pipe topology* is
   deterministic. This is the literal engineering meaning of the operator's "wrap stochastic
   signaling around deterministic streams" instruction.
3. **n-dimensional, not 2D**: nothing above is dimension-limited — `L` can be built over any
   embedding dimension; the "2D algebra" the operator objected to was never a constraint of the
   Laplacian approach, only of a naive adjacency-matrix-as-spreadsheet mental model.

### 2.1 Mirror-model chain-of-thought observation

Concrete and buildable, not speculative: pair every reasoning agent with a `research-verifier`-
style decorrelated observer (different model/provider) whose only job is to summarize/flag the
first model's CoT — exactly the pattern this session's agent roster already runs for research
reports (§1.5). "Thought injection" (a synthetic canary fact planted in context, checked for
correct handling in the CoT) is a real, standard technique — equivalent to a canary token, cheap
to build, cheap to measure (false-negative rate = canaries missed).

### 2.2 What is NOT proposed

No literal Navier–Stokes solver, no continuous n-dimensional gas simulation of token streams.
That would cost far more compute than the data-routing problem it's solving and has no measured
precedent of working better than graph-Laplacian/flow-network methods for discrete data — flagging
this explicitly per this project's own "honest audit" standard (the physics-UI arc already did
the same gut-check on quantum math and rejected the parts that were notation without payoff).

---

## 3. Suggested measurement-first order (not a full build plan — one falsifiable step at a time)

1. Re-verify `field_physics.rs:334` and the E1/E2 commit (`6bd181a02`) are still live on current
   `main` — cheap, prevents re-deriving proven work.
2. Instrument the existing telemetry spool with the Little's-Law check (§2.1) — hours, not days,
   and gives a real measured number instead of an estimate.
3. Wire vLLM's native `logprobs` into the confidence-gap signal (§1.1) — zero-build, it already
   exists in the serving layer if vLLM is adopted.
4. Build the breaker state machine (§1.2) against synthetic labeled poisoning/injection cases,
   report actual false-positive/false-negative rates.
5. Only after 1–4 are measured: extend `research-verifier`'s decorrelated-review pattern from
   research docs to live swarm actions (§1.5/§2.1).

Each step produces a number. Per the operator's own instruction, nothing above gets reported back
as "working" without one.
