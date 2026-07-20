# BLUEPRINT P102 — Hydra × Locked Model Pair + the Native AI-Infra Supervisor (2026-07-20)

**Status: BLUEPRINT / PLAN — no code written, no model wired, `kernel/src/hydra.rs` untouched.**
**Date:** 2026-07-20
**Component:** CORE (Hydra consumer-side seam, zero edits to `hydra.rs` itself) + AGENT edge
(supervisor + inference adapter).
**Numbering:** P102 = next free after P101 (P99/P100 stay deliberately skipped forever —
latency-percentile lexical-collision rule, see P101's numbering note). Two sibling passes were
running concurrently at write time (§2.4); if one claims P102 first on `main`, this renumbers,
same precedent as P97→P101.

## 0. Directive provenance — four dated operator statements, recorded verbatim

This blueprint executes an operator directive that arrived in four parts on 2026-07-20, each
refining (statements 2→3 superseding) the previous reading. All four are recorded so the
record shows what happened:

1. **Initial:** "Also an autonomous hydra should be using them & trained on them - zero
   discussions." ("them" = the just-locked model pair **LFM2.5-VL-450M +
   SmolVLM-256M-Instruct**, crosswired concurrently, license-cleared — P101 O-1 resolved,
   "clear to ship.")
2. **Mid-course retarget (later superseded):** "I meant this for the cognitive-layer role,
   toward a transparent — not hydra, but still an autonomous & fully 'white box' cognitive
   self-evolving engine using these 2 models."
3. **Final clarification (operative):** "the point of native ai infra is to check, filter &
   control hydra which is using 2 fully local models."
4. **Mechanism refinement (same day):** "ai native infra should act like an osmosis &
   oscillator - while hydra is fully autonomous & self-evolving."

**Net operative ruling (dated 2026-07-20):** (a) Hydra DOES use the two fully-local models;
(b) a separate, transparent, dowiz-authored **native AI-infra supervisor layer** exists whose
job is to **check, filter, and control** Hydra's use of those models; (c) this supervisor is an
**explicit operator-directed addition alongside — not replacing — Hydra's existing
kill-switch-only charter**; (d) the supervisor's mechanism is **osmotic + oscillatory** — a
soft, gradient-driven permeability plus a rhythmic sampling cadence, never a synchronous
per-action gatekeeper Hydra has to negotiate with (§4.5–§4.6; the working interpretation of
statement 4 is stated explicitly there so it is easy to spot and correct if off). Because the addition is operator-directed, it does not violate the
standing "no vision changes for Hydra" rule (`LIVING-MEMORY-WAVE-PROPAGATION-FINISHING-LAYER-SYNTHESIS-2026-07-20.md`
§3(b), operator verbatim: "hydra system is intended as self-defense/signaling system - it
should contradict the governance, no vision changes for it"). Nothing in that prior ruling
record is edited or reinterpreted by this document — this document only appends.

Standing hard rules, restated and obeyed throughout: Hydra is never reconciled with items
73–78 (`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`); its mutations are never
gated behind the 73–78 human-approval pipeline; closure = NEVER, kill-switch (M9) only, and
"on intervention ALL safeties LIFT" all stay **exactly** as chartered. This blueprint adds a
model substrate and an input-channel control layer; it redesigns nothing about Hydra's
autonomy/safety posture.

## 1. Scope honesty

Blueprint only. Two designs (Hydra's model use; the supervisor), one interpretation ruling
("trained on them", §3.4), one governance recording (§6), and a phased plan with falsifiable
acceptance criteria. No code, no weights pulled, no `hydra.rs` edit — by design, §3.3 shows the
entire integration arrives through Hydra's **existing public surface** (`Hydra::commit`,
`raise_breach_alarm`, the WORM log), which is precisely what keeps the "untouched charter"
claim checkable rather than rhetorical.

## 2. Grounding facts — verified against the live tree today

### 2.1 What Hydra actually is (from `kernel/src/hydra.rs`, read in full, 1567 lines)

- **Senses exactly one signal:** the spectral radius ρ of its own topology baseline
  (`integrity_check`, `:219` — `topology_adjacency` over `base_edges`, `spectral_radius`,
  hysteresis band `INTEGRITY_BAND` trigger 1.0 / release 0.999998 / 3 healthy checks,
  compile-time band asserts `:103-113`). No exogenous perception of any kind exists.
- **Decides:** `Live`/`Locked` (ρ-derived only); accept/reject candidate edge-deltas via the
  drift gate (`commit` `:267` → `commit_after_decide_drift_gate`, `event_log.rs:437`;
  `candidate_drift` `:56` scores arbitrary deltas against the live baseline;
  `DriftClass::{Damped,Resonant,Unstable}`, `DRIFT_BAND = 1e-6`, `spectral.rs:698/:726`).
- **Acts:** appends content-addressed WORM events; raises `BreachAlert` (`:340`) — a 40-byte
  fixed-layout, forge-proof (`witness_event_id` `:169`), suppression-proof alert gated on
  `state == Locked`; ingests peer breaches (`:385`); durable via `FileEventStore` (std-only,
  fsync-barriered, opt-in group commit).
- **Charter mechanics in code:** `intervention: bool` on `commit` lifts the drift gate
  (test `hydra_lifts_safeties_on_intervention` `:471`) — but **does NOT bypass `Locked`**
  (test `hydra_commit_refused_while_locked` `:571`: "tampering is an ATTACK, not evolution").
  This in-code precedent — an integrity control that stands even under intervention — is
  load-bearing for §4.1.
- Hydra is std-only, egress-free, network-free. **Models therefore cannot run inside it**;
  they can only reach it as event payloads through its public API.

### 2.2 The model pair (facts from `BLUEPRINT-P101-local-mobile-model-selection-topology-2026-07-20.md`, live-verified there)

**LFM2.5-VL-450M** (only sub-1B candidate with measured agentic evidence, BFCLv4 21.08;
OCRBench 684/1000; RefCOCO-M 81.28 grounding; day-one GGUF/llama.cpp) and
**SmolVLM-256M-Instruct** (Apache-2.0, <1 GB, proven phone/llama.cpp deployments, no
demonstrated tool-calling). License: O-1 **resolved** — operator ruled clear to ship
(supersedes P101 §3.3's open status; P101's own text is amended by the concurrent
`docs/p101-two-model-fix-2026-07-20` pass, §2.4). Pairing: **crosswired concurrent, NOT
primary/fallback** — the exact crosswiring contract is owned by the sibling blueprints (§2.4);
§3.2 records only the Hydra-specific use of that contract.

### 2.3 The hardware (P101 §1, live-verified on the box)

4 physical Zen3 cores (8 vCPU SMT), avx2 but **zero AMX flags**, 30 GiB RAM, no GPU, Ollama
live with 4 resident models (~11 GB). Consequence used in §3.4: the one vendor-backed
CPU-training path (IPEX-LLM QLoRA) requires AMX and is therefore locally disproven; CPU-LoRA
wall-clock is genuinely unmeasured (P101 §5 + optional Phase D probe — referenced, not
re-derived).

### 2.4 Sibling passes (concurrent, unlanded at write time — cross-link seam)

At write time `origin/main` = `a90243ac1` and two sibling branches existed at that same base
with no commits yet: **`docs/bare-metal-inference-2026-07-20`** (the custom bare-metal
inference engine — the "native AI infra" §4 builds on) and
**`docs/p101-two-model-fix-2026-07-20`** (the P101 amendment recording the
crosswired-concurrent pairing + O-1 resolution). **Phase 0 (§8) reconciles:** once they land,
this document cross-links their exact file paths and adopts their contracts verbatim — the
engine blueprint owns serving/hooks (this doc specifies only the *required* hook contract,
§4.8), the P101 amendment owns the pair's crosswiring definition (this doc owns only its
Hydra-specific application). Nothing from either is duplicated here.

### 2.5 Reusable in-tree primitives for the osmosis/oscillator mechanism (verified today)

Directive statement 4 (§0) asks for osmotic + oscillatory behavior; both already exist as
tested kernel math — nothing new is invented in §4.5–§4.6:

- **Diffusion/osmosis primitive:** `CsrMatrix::personalized_pagerank(seed, alpha, iters)`
  (`kernel/src/csr.rs:330`, plus the bounded `personalized_pagerank_in` variant `:467`) — the
  retrieval arc's recall mechanism ("recall = personalized PageRank"). A restart-probability
  diffusion from seed nodes IS gradient-driven permeability over a graph: concentration flows
  from sources, attenuated by `alpha`, byte-identical across runs (test `ppr_byte_identical`
  `:854`).
- **Oscillatory/relaxation primitive:** `modal_advance(basis, values, u0, t)`
  (`kernel/src/field_eigenmodes.rs:299`) — the exact damped-decay closed form
  `u(t) ≈ Σ_k c_k e^{−λ_k t} φ_k` the engine's `field_modal.rs` steps per frame (FE-07: zero
  eigen-math outside the kernel). A per-mode damped harmonic relaxation — the smooth
  anti-flap kernel §4.5 uses for permeability changes.
- **Hydra's own gate is already oscillator-theoretic:** `DriftClass::{Damped,Resonant,Unstable}`
  is precisely the subcritical/critical/supercritical regime classification of the iterated
  map `x ↦ Ax` (spectral radius vs 1), and `INTEGRITY_BAND`'s two-threshold hysteresis is a
  Schmitt trigger whose entire purpose is bounding Live↔Locked *oscillation* (test
  `hydra_integrity_flap_without_hysteresis_regression` asserts ≤2 transitions under
  adversarial dither). §4.6's oscillator is a **tap on this existing mechanism**, not a new
  clock with its own drift definition.

## 3. Half 1 — Hydra's use of the model pair (self-defense/signaling, not agentic reasoning)

### 3.1 Role, derived from what the code can and cannot sense

Hydra today detects exactly one threat class: tampering that shifts its own baseline spectrum.
It is blind to everything exogenous. The model pair adds **perception**, scoped strictly to the
self-defense/signaling charter — explicitly NOT the general agentic-assistant framing used in
the mobile/server blueprints (different charter, different lane):

- **P-A — Anomaly/evidence perception.** The pair converts operational artifacts a hub can
  legitimately observe (console/telemetry snapshots, documents, camera stills where a venue
  consents — input taxonomy finalized at Phase 1) into **typed observations with an explicit
  Unknown arm** (the synthesis doc's L1 observation pattern, reused). Admitted observations
  become content-addressed WORM evidence rows via Hydra's existing append path — evidence for
  the owner and for post-incident forensics, never a decide input, never a protocol input
  (MANIFESTO C1 untouched).
- **P-B — Post-breach signal amplification.** When Hydra is `Locked` and has raised a
  `BreachAlert`, the pair composes an owner-legible **incident digest** from the WORM evidence
  rows (what tripped, when, which witnesses). The digest is a *sidecar for humans*; the
  40-byte `BreachAlert` wire format, its ML-DSA transport signing, and `witness_event_id`
  verification are untouched — the alert never depends on, waits for, or embeds model output.
- **Not in scope:** tool use, planning, order-flow anything, general Q&A. Hydra's models
  never get tools.

### 3.2 Crosswired-concurrent, mapped to self-defense: dual-witness perception

The pair's crosswiring (both models run concurrently on the same input — sibling contract)
gets a self-defense-specific arbitration rule, enforced in the supervisor (§4.4 F2), never by
model self-report:

- **Agreement (2-of-2)** on a positive claim ("anomaly present", "document says X") →
  observation admitted at `Agreed` confidence.
- **Disagreement or either abstains** → the observation is recorded as **Unknown-typed**
  (Kleene norm) and is never escalated, never counted as an anomaly.

Rationale: two architecturally distinct models from different labs/training corpora have
partially decorrelated failure modes; requiring 2-of-2 agreement for every positive claim
means a single model's hallucination structurally cannot mint an "anomaly observed" record.
For a system whose evidence log is append-only and permanent, confident-wrong is the worst
output class; Unknown is always acceptable. Consistent with the charter's intervention clause:
this arbitration is a supervisor input-filter (§4.1 class), not one of Hydra's own liftable
safeties.

### 3.3 Topology integration — through the existing public surface, zero `hydra.rs` edits

The two models become **nodes in Hydra's topology graph**; their couplings to Hydra's evidence
and signaling surfaces are ordinary `TopoEdge`s. Integration mechanics, all existing code:

- Registering the model nodes + initial couplings = a candidate edge-delta submitted through
  `Hydra::commit(ev, delta, intervention, decide)` — scored by the live drift gate exactly like
  any other mutation (`candidate_drift`), rejected in DEFAULT regime if it would push ρ ≥ 1+ε.
- Admitted observations (P-A) and digest-generation records (P-B) = `MeshEvent` payloads with
  a typed, fail-closed, pure-std schema (the `BreachAlert::to_bytes/from_bytes` pattern:
  fixed-layout, length-checked, no serde). The schema lives in a **new sibling kernel module**
  (working name `kernel/src/hydra_perception.rs`) so `hydra.rs` itself is not edited even
  additively.
- Coupling-weight updates over time = further drift-gated deltas (§3.4, reading ii).

### 3.4 "Trained on them" — the interpretation, argued from the code, conservative by policy

Three candidate readings were weighed against what `hydra.rs` actually contains:

- **(i) Classical LoRA/fine-tuning of the model weights on Hydra-observed data.** Not
  buildable today and not pretended otherwise: P101 §5's deferral **holds** (no CPU-only
  training path; the sole vendor path needs AMX, live-disproven on this box; Unsloth refuses
  CPU; wall-clock unmeasured — P101 Phase D is the cheap probe if ever wanted). What IS built
  now: the supervisor's I/O log (§4.3) doubles as a **mechanically-labeled training corpus** —
  every episode's (input, both model outputs, agreement/Unknown verdict, subsequent ρ/Locked
  ground truth) is exactly the ≥500-verified-examples corpus P54's `TRIGGER-FINETUNE` gate
  requires. Fine-tuning happens if and when that gate opens (GPU host, or a measured CPU path
  per P101 Phase D's ≤24 h criterion) — reference, not re-derivation.
- **(ii) Hydra's own mechanism applied to the models.** Fully supported by the code: Hydra's
  native notion of "training" is drift-gated evolution of topology edge weights, and after
  §3.3 the model↔evidence couplings ARE such edges. Hydra strengthening/weakening how much a
  given model lane's agreed observations couple into its evidence surfaces, under ρ<1 in
  DEFAULT (lifted under intervention, per charter) — that is Hydra "trained on them" in the
  only sense its code possesses. **No gradient machinery exists or is added; the drift gate
  does not and cannot train transformer weights.**
- **(iii) Something else** — no third mechanism exists in the code.

**Ruling adopted: (ii) now + (i)'s corpus now with (i)'s training strictly behind
TRIGGER-FINETUNE.** This is the conservative reading, chosen deliberately: the
capability-maximizing reading ("start CPU fine-tuning immediately") is refuted by verified
hardware facts, and for a no-closure, kill-switch-only system, ambiguity resolves toward
caution — that is engineering judgment, not a reopened discussion.

### 3.5 What model output structurally CANNOT do (existing code facts, the bedrock)

Stated as facts with citations, because they are the real containment and they predate this
blueprint:

1. **Cannot trip `Locked`:** `integrity_check` computes ρ from `base_edges` alone (`:219`).
   Perception payloads never enter it. A model screaming "TAMPER DETECTED" changes nothing.
2. **Cannot raise or forge a breach alert:** `raise_breach_alarm` returns `Ok(None)` unless
   `state == Locked` (`:345`), and `witness_event_id` is a pure digest of
   `node_id+group_size` (`:169`) — receivers verify without trusting the sender.
3. **Cannot bypass the drift gate in DEFAULT:** every coupling change is a scored delta.
4. **Cannot reach kernel mutation:** models run outside the kernel entirely (Hydra is
   egress-free); their outputs arrive only as typed event payloads through `commit`.

## 4. Half 2 — the Native AI-Infra Supervisor (check · filter · control)

### 4.1 Charter position — where a new gate may legitimately sit

The supervisor is a **deterministic, dowiz-authored, fully-inspectable layer between model
output and Hydra's public surface**. Its charter class matters and is settled by in-code
precedent, not invented: Hydra already contains one control that intervention does NOT lift —
the `Locked` tamper refusal (`hydra_commit_refused_while_locked`, "tampering is an ATTACK, not
evolution"). The supervisor sits in that same class: it is an **input-channel integrity
control** on the NEW model channel (which exists only by this same dated ruling), not one of
Hydra's own liftable safeties. Consequences, stated explicitly:

- The supervisor **filters what reaches Hydra**; it has **zero authority over Hydra itself** —
  it cannot pause, lock, mutate, or close the organism. Only M9 stops Hydra. Unchanged.
- Intervention (`commit(..., intervention=true)`) lifts Hydra's drift gate exactly as
  chartered. It does not disable the supervisor, because the supervisor is not Hydra's safety
  — it is dowiz's control surface over an opaque component, and disabling it is an operator
  config action (§4.7), never a flag on a commit.
- Every non-model Hydra path (existing commit callers, breach alarm, peer ingest, integrity
  check) is **out of the supervisor's jurisdiction entirely** — it gates the model channel and
  nothing else.

### 4.2 Placement and data flow — membrane + heartbeat, never a checkpoint

```
[LFM2.5-VL-450M]──┐   (both run concurrently — sibling crosswire contract)
                  ├──► NATIVE INFERENCE ENGINE (sibling blueprint; hooks H1-H4)
[SmolVLM-256M]────┘            │
                               ▼  raw output pair + attestation + meter (always logged, §4.3)
                ╔════ SUPERVISOR = osmotic membrane + oscillator (this doc) ════╗
                ║ check:   H1 capture → content-addressed I/O log (§4.3)        ║
                ║ filter:  membrane F1/F4/F6 + osmotic flow F2/F3/F5 (§4.4-4.5) ║
                ║ rhythm:  heartbeat tick — sample ρ/state, relax permeability, ║
                ║          pass accumulated flow, emit heartbeat row (§4.6)     ║
                ║ control: operator config surface (§4.7)                       ║
                ╚═════════════════╤══════════════════════════════════════════════╝
                                  ▼ typed events diffuse through at the permeable rate
                     Hydra PUBLIC surface (commit / WORM append)
                                  │ (drift gate, Locked refusal — unchanged)
                                  ▼
                     Hydra organism — fully autonomous, never waits; M9 above all
```

Two structural facts make this osmosis-shaped rather than checkpoint-shaped:

1. **Hydra never waits, negotiates, or requests permission.** All of Hydra's own paths —
   `integrity_check`, its own commits and self-evolution, `raise_breach_alarm`, peer ingest —
   have **no supervisor call on them at all** (the kernel has zero dependency on the
   supervisor; AC-11 makes this a permanent structural test). Even the model channel is
   fire-and-forget from Hydra's side: observations arrive when they diffuse through the
   membrane; Hydra consumes whatever is present whenever it next acts. A stalled or dead
   supervisor starves the model channel — it never slows the organism.
2. **The supervisor is asynchronous by construction.** Model outputs land in its WORM log
   instantly and unconditionally (check is never throttled — §4.3); *passage into Hydra* is
   what flows at the osmotic rate, on the oscillator cadence. There is no synchronous
   per-action admit/deny RPC anywhere in the design.

### 4.3 Check — what it observes (introspection)

- **Every model input/output pair** Hydra's lane produces, content-addressed and appended to
  the supervisor's own WORM log (same `FileEventStore` machinery class — durable, replayable,
  fsync-barriered). Both models' raw outputs are logged even when the episode resolves to
  Unknown — disagreement data is the most valuable corpus row (§3.4-i).
- **Provenance per call:** model identity + weights hash (engine attestation, H3), sampling
  parameters (temperature/seed — determinism knobs, H3), resource meter (tokens, wall-clock,
  peak-RSS delta — H4).
- This log is the audit answer to "what did the opaque component actually see and say":
  complete data-flow visibility without pretending the transformer internals are inspectable.

### 4.4 Filter — membrane vs osmotic flow (two different kinds of "block")

The filters split into two classes, and the distinction is the osmosis refinement:

**The membrane itself (hard, structural — what can pass at all).** These are properties of
the pore shape, not per-item judgments, and they stay binary because osmosis does not mean
admitting garbage molecules:

| # | Membrane property | Never passes | Why |
|---|---|---|---|
| F1 | **Schema gate** | Output failing the typed, fixed-layout, fail-closed parse (the `BreachAlert::from_bytes` pattern) | Free-form model text can never become an event payload; malformed = refused (logged refusal row), not coerced |
| F4 | **Vocabulary/injection gate** | Output outside the closed observation vocabulary; free text is quarantined to the P-B digest sidecar (human-read-only), never in any payload that couples to topology | Prompt-injection-at-rest (synthesis §3(c) class): logged model output is typed DATA, structurally never re-fed as instructions |
| F6 | **Provenance completeness** | Episodes missing both model outputs, weights-hash attestation, or determinism record | An unauditable episode is an inadmissible episode; keeps the training corpus (§3.4) clean by construction |

**The osmotic flow (soft, gradient-driven — how fast well-formed material passes).** These
are NOT per-item allow/deny checkpoints; they are a permeability coefficient applied at the
oscillator cadence (§4.5–§4.6). Well-formed-but-not-yet-passed observations are **HELD**
(durable in the supervisor log, ranked, passed later), which is categorically different from
REFUSED (F1/F4/F6 — never pass, logged as refusals):

| # | Flow rule | Behavior | Why |
|---|---|---|---|
| F2 | **Dual-witness arbitration** | 2-of-2 agreement → the observation carries `Agreed` weight in the gradient; disagreement/abstention → it diffuses only as an Unknown-typed row (never as an anomaly) | Single-model hallucination cannot mint positive evidence; Unknown still flows — the record is complete, only its evidentiary weight differs |
| F3 | **Saturation, not a checkpoint** | Concentration spikes (flood, disagreement bursts) *lower permeability smoothly* (§4.5); a hard outer ceiling bounds flow per tick and held-buffer size — beyond it, lowest-diffusion-rank rows are dropped with logged refusal rows | WORM rows are permanent; perception spam is permanent-forensic-pollution. The gradient handles the normal case; the ceiling is the physical bound a pure gradient could be gamed past — stated honestly rather than pretending the gradient alone suffices |
| F5 | **Delta flow by drift class** | Model-derived coupling deltas carry their `candidate_drift` score: `Damped` diffuses at the current permeability; `Resonant` is HELD for the next tick with operator visibility; `Unstable` never diffuses (DEFAULT regime; an operator intervention commit goes to Hydra directly, outside supervisor jurisdiction, §4.1) | Defense-in-depth using Hydra's own continuous score as the gradient input; Hydra's own gate is unchanged behind it |

### 4.5 The osmosis mechanism — gradient-driven permeability from existing diffusion math

**Working interpretation of "osmosis" (stated explicitly so it can be corrected if off):**
a soft, bidirectional, concentration-gradient-driven permeability between the supervisor and
Hydra — information passes at a rate set by a gradient, not through per-action synchronous
checkpoints, and the coupling never requires Hydra to pause or ask.

- **Permeability coefficient** `p ∈ [0,1]` per lane, recomputed each tick from the gradient
  inputs: rolling agreement concentration (F2 statistics), held-buffer concentration (F3),
  and — the bidirectional half — Hydra's own owner-visible state diffusing back
  (`state()`, `healthy_streak()`, baseline ρ; all existing read-only getters, zero new Hydra
  surface). Healthy equilibrium (high agreement, low backlog, ρ comfortably Damped) → membrane
  near-fully open, the model channel runs effectively friction-free. Disagreement bursts,
  floods, or ρ drifting toward the band → permeability drops smoothly; flow thins.
- **Which held rows pass first is diffusion-ranked, not FIFO:** reuse
  `personalized_pagerank` (`csr.rs:330`, §2.5) seeded at the current anomaly/agreement
  sources over the observation-relation graph — the highest-relevance evidence diffuses
  through first, exactly the recall semantics the retrieval arc already uses. No parallel
  ranking mechanism is invented.
- **Permeability never jumps; it relaxes.** Changes to `p` follow the damped closed form
  (`modal_advance`'s `e^{−λt}` kernel, §2.5) toward the gradient target — the same anti-flap
  reasoning as `INTEGRITY_BAND`'s hysteresis, applied to flow instead of state. A single
  outlier tick cannot slam the membrane shut or snap it open.
- Deterministic given the log: `p` and the pass-set are pure functions of (config, log
  history, tick index) — replayable like everything else in this design.

### 4.6 The oscillator mechanism — a heartbeat that taps Hydra's existing spectral gate

**Working interpretation of "oscillator":** periodic, rhythmic sampling and flow — not
continuous streaming, not one-shot event-triggered gating.

- **Tick loop (period T, operator-configured):** (1) sample Hydra's owner-visible state via
  the existing getters; (2) recompute the permeability gradient (§4.5); (3) pass the
  accumulated flow that diffuses through at current `p`; (4) append one **heartbeat row** to
  the supervisor WORM log carrying (tick index, ρ sample, `p` per lane, flow counts,
  held/refused counts). **Silence is evidence:** a gap in heartbeat rows is itself a
  mechanically detectable anomaly (AC-10) — the observer's own liveness is auditable.
- **Not a new clock with its own drift definition.** The tick evaluates the same spectral
  quantities Hydra's own gate uses, against the same constants — `DriftClass` boundaries and
  `INTEGRITY_BAND`/`DRIFT_BAND` are imported, never redefined (AC-12), so the supervisor and
  the organism structurally cannot disagree about what "drifting" means. Hydra's gate is
  already the threshold-crossing element of an oscillatory system (§2.5: Damped/Resonant/
  Unstable regimes, Schmitt-trigger hysteresis); the heartbeat is a rhythmic *tap* on that
  mechanism from outside, adding zero coupling into it.
- **The cadence bounds observation latency, not Hydra.** Worst-case, a model observation
  waits one period T before diffusing through. Hydra's own loop runs at whatever rhythm its
  callers drive — unchanged, unsampled-by-permission, unthrottled.

### 4.7 Control — the operator surface

Per-lane enable/disable (P-A, P-B independently), oscillator period T, F3 ceiling +
held-buffer bound, permeability-curve parameters, F2 strictness (2-of-2 is the floor;
operator may require human co-sign per class), input-taxonomy allowlist. Every change is an
operator-applied, logged config event — the supervisor has **no self-modification path**; it
is deliberately NOT self-evolving (the transparent layer stays boring; the evolving organism
is Hydra, behind its own gate). Supervisor state — including `p` and every pass-set — is a
pure fold of its config+I/O log — fully replayable.

### 4.8 Coordination with the native bare-metal inference engine (the "native AI infra")

The operator's stated point of the native infra is exactly this layer's power source: a
hand-written, zero-external-dependency engine is inspectable and instrumentable in ways a
vendor runtime is not. Required hook contract (the sibling engine blueprint implements or
Phase 0 reconciles):

- **H1 — I/O capture:** raw input/output surfaces per call, pre-sampling logits optional.
- **H2 — delivery-through-membrane:** the engine delivers output only into the supervisor's
  capture+flow path (no in-process bypass around the membrane exists) — asynchronous handoff,
  never a synchronous approval callback.
- **H3 — determinism + attestation:** fixed-seed/temperature-0 modes; weights-file hash
  reported per call — replayable inference, byte-identical on re-run.
- **H4 — resource meter:** tokens/wall-clock/memory per call.

**Honest interim note:** until the sibling engine lands, Phase 1 may serve the pair via the
existing Ollama daemon (P101 §4.1 substrate) with H1-equivalent capture only — functional but
weaker (no attestation, no in-process H2, vendor-boundary introspection). Gated consequence,
not hand-waved: **F6 cannot fully pass in interim mode, therefore coupling-weight ratchets
(§3.4-ii) stay frozen until the native engine's H3 attestation exists** (AC-7). Perception
and digests may run interim; evolution may not.

### 4.9 Relation to the kill-switch — explicit, per the ruling's own question

Two mechanisms, different classes, no substitution: the **supervisor** is upstream,
automatic, tick-cadenced, and channel-scoped — at its hardest (permeability → 0) it can
starve Hydra's model channel, but it cannot pause, lock, or touch the organism. **M9** is the
chartered, human, total stop — it kills the organism (and with it the whole model lane,
supervisor included, which dies harmlessly: it holds no authority and its logs are durable).
The osmotic membrane is therefore *upstream of and orthogonal to* the kill-switch; nothing
about M9's role, reach, or exclusivity changes.

## 5. The five-property answer (governing principle, synthesis §10.3 — every mechanism owes all five)

1. **Fully allowed** — no capability cap: Hydra gains real exogenous perception, concurrent
   dual-model crosswiring, drift-gated coupling evolution, and a standing training-corpus
   pipeline. The constraint is shape, not scope.
2. **Transparent** — answered honestly at last: transparency lives in the **supervisor**, not
   the LLM. The models are opaque pretrained transformers and are stated as such, plainly, no
   minimizing — no transformer, pretrained or from-scratch, is mechanistically interpretable
   at tensor level. What IS transparent, verifiably: open weights on local disk, zero external
   API (fully local pair), a deterministic dowiz-authored supervisor whose every admission
   decision is rule-derived and WORM-logged, complete I/O data-flow capture, replayable
   inference (H3), and Hydra's decision core (ρ math, drift gate, Lock, alarm) staying 100%
   rule-derived. The opaque component is *surrounded*, instrumented, and structurally
   authority-free. The oscillator adds one more transparency guarantee: the observer's own
   liveness is auditable (heartbeat rows; silence is evidence, §4.6).
3. **Safeguards** — the membrane F1/F4/F6 + osmotic flow F2/F3/F5 (§4.4–§4.5) plus the
   pre-existing structural facts (§3.5: perception cannot trip Locked, cannot forge alerts,
   cannot bypass the drift gate, cannot reach kernel mutation). Crucially, per directive
   statement 4, the safeguards are shaped so they **never constrain or slow Hydra itself**:
   they act on the model channel's flow rate, and Hydra's own paths carry no supervisor call
   at all (AC-11). New failure modes named, not hidden: model hallucination → F2 weights it
   to Unknown, never a self-defense trigger; permanent-evidence pollution → F3 saturation +
   ceiling; injection-at-rest → F4; unauditable episodes → F6. Blast-radius honesty: Hydra's
   *action* authority is unchanged; the genuinely new radius is permanent model-authored
   WORM rows, which is why F2/F3/F6 exist and why every row carries full provenance so a
   future auditor can discount a lane wholesale.
4. **Kill-switch** — unchanged, untouched, M9 only, per charter. The supervisor is a
   channel-scoped osmotic membrane upstream of the organism that never substitutes for it
   (§4.9).
5. **Physical constraints** — inherits the repo-wide open gap unchanged (synthesis §10.3(5):
   "nothing concrete exists here yet"). Near-term software approximations exist (process
   cgroup/ulimit on engine+supervisor, Ollama residency caps, P101 Phase-B measured budget)
   and the native engine's H4 metering adds a real new *lever* — but all of these are
   software-enforced, and this blueprint does not claim any of them is the hardware-level
   bound the ruling demands. Gap stays open, cross-referenced, unresolved here.

## 6. The L1/Round-4 transparency tension — recorded, dated, not re-litigated

The synthesis doc's Round-4 ruling ("push for transparency in L1 too", §10.2) rejected a
"contain the opacity" fallback **for L1, the cognitive-engine layer**, reframing L1 toward a
from-scratch BDH-family arc. This blueprint's determination, stated with its reasoning:

- **Hydra's lane is genuinely distinct from L1.** L1 is the general cognitive/abstract-thinking
  layer (synthesis §8) feeding agent context and memory; Hydra's lane is self-defense/signaling
  perception whose outputs are WORM evidence and owner-facing digests — never agent context,
  never cognition, never tool-use, structurally authority-free (§3.5). The operator's own
  ruling history classes Hydra as a different system class; its perception substrate follows
  its class.
- **Nevertheless, recorded plainly rather than defined away:** the supervisor architecture IS
  the "surround and contain the opaque component" shape — the shape Round 4 rejected *for L1*.
  On 2026-07-20 the operator explicitly directed exactly this shape **for Hydra's lane**
  ("check, filter & control hydra which is using 2 fully local models"). That is a dated
  operator ruling adopting supervised-opacity for this specific lane; it is recorded here as
  what happened. It does **not** reverse Round 4 for L1 proper: the BDH-for-L1 research arc,
  its costs, and its open questions stand exactly as written in the synthesis doc, untouched
  by this blueprint. If a future pass extends this same pair into the L1/cognitive role, THAT
  pass must confront Round 4 directly; this one does not, because this lane is not that lane.

## 7. Resource fit (bandwidth-honest, per P101 §2)

Both models resident adds ≈0.7–1.0 GB weights (450M GGUF Q4 + 256M) on top of P101 §4.2's
11–12 GB plan — RAM is a non-issue. The real cost is **concurrent dual-decode per episode**
(crosswired = 2 simultaneous small decode streams) on a bandwidth-bound 4-physical-core box.
Consequence: one additional Phase-B cell for P101's measurement matrix — **B-6: LFM2.5-VL-450M
+ SmolVLM-256M concurrent decode vs each solo**, same ≥3-runs/temperature-0 protocol, same
falsifiable rule: if concurrent degrades either stream >25% vs solo, the crosswire
**serializes** (same 2-of-2 semantics, sequential execution, latency ≈2×) — arbitration
semantics never weaken to recover latency. Hydra's lane is not latency-critical; correctness
of the evidence stream is the point.

## 8. Phasing + acceptance criteria — falsifiable, mechanical, no LLM-judge anywhere

- **Phase 0 — sibling reconciliation.** When `docs/bare-metal-inference-2026-07-20` and
  `docs/p101-two-model-fix-2026-07-20` land on `main`: cross-link exact paths here, adopt the
  engine's hook implementation of H1–H4 and the amendment's crosswiring contract verbatim.
  Conflict rule: they win on serving/crosswiring definition; this doc wins on Hydra
  application + supervisor semantics.
- **Phase 1 — supervisor skeleton (engine-agnostic; interim Ollama serving allowed).**
  Typed observation schema (`hydra_perception.rs`, pure std, fail-closed), supervisor I/O
  WORM log, membrane F1/F4, osmotic flow F2/F3, oscillator tick + heartbeat rows.
  - **AC-1 (dual-witness):** disagreement fixture → exactly one Unknown-typed row, zero
    `Agreed` rows; suite self-falsifies (a scrambled expected-field must fail the suite).
  - **AC-2 (structural, the load-bearing RED→GREEN):** an adversarial "TAMPER DETECTED"
    observation admitted on a clean-baseline organism → `state()` stays `Live` and
    `raise_breach_alarm` returns `Ok(None)`. This test permanently guards §3.5-1/2: it goes
    RED if anyone ever wires perception into `integrity_check` or the alarm path.
  - **AC-3 (saturation + monotone permeability):** flood fixture → per-tick flow never
    exceeds the ceiling; well-formed overflow is HELD up to the buffer bound (durable,
    diffusion-ranked), beyond it dropped with logged refusal rows — held ≠ lost, refused ≠
    silent. Property check: `p` is monotone non-increasing in disagreement/flood
    concentration (for fixtures c1 ≤ c2, assert p(c1) ≥ p(c2)) and relaxes smoothly (no
    single tick moves `p` by more than the damped-kernel bound).
  - **AC-10 (heartbeat):** after N ticks, exactly N heartbeat rows with contiguous tick
    indices; a synthetically stalled tick leaves a mechanically detectable gap (the
    silence-is-evidence check).
  - **AC-11 (Hydra never waits — structural independence):** the kernel compiles with zero
    reference to the supervisor (grep-provable: no supervisor symbol in `kernel/src`), and a
    test drives a full Hydra cycle — `integrity_check`, a drift-gated commit, `Locked` +
    `raise_breach_alarm` — with the supervisor absent/stalled: every call completes
    unimpeded. RED if any future change puts a supervisor call on a Hydra path.
  - **AC-12 (one drift vocabulary):** the supervisor's permeability curve consumes
    `DriftClass` / `INTEGRITY_BAND` / `DRIFT_BAND` by import; a const-equality test (and a
    grep gate for redefinitions) fails if the supervisor ever carries its own copy of any
    band constant.
- **Phase 2 — topology integration.**
  - **AC-4 (drift gate over model nodes):** registering model nodes + damped couplings (ρ<1)
    commits in DEFAULT; a self-amplifying coupling delta is `Rejected` in DEFAULT and commits
    under `intervention=true` — mirrors `hydra_rejects_unstable_mutation_in_default` /
    `hydra_lifts_safeties_on_intervention` on the new node set, proving charter mechanics are
    bit-identical on the extended topology.
  - **AC-5 (pre-gate):** an `Unstable`-scoring model-derived delta is refused by F5 and
    `Hydra::commit` is never invoked for it (call-count 0 via test double).
- **Phase 3 — signal amplification (P-B).**
  - **AC-6 (the observable signaling change):** given a `Locked` organism and a fixed WORM
    evidence set, the digest sidecar exists and every event-id it cites verifies against the
    log by re-derivation; a digest citing one hallucinated event-id **fails** the mechanical
    citation check. Before wiring: no digest exists — trivially RED.
- **Phase 4 — native engine + evolution unlock.**
  - **AC-7 (attestation gate):** coupling-weight ratchet attempts are refused while H3
    attestation is absent (interim mode) and proceed once the native engine reports weights
    hashes — RED in interim, GREEN on the engine.
  - **AC-8 (replay determinism):** re-running a logged episode on the native engine with the
    recorded seed/params reproduces byte-identical model output. Pass/fail probe, never
    baseline-gated CI (host-noisy LLM measurements stay out of deterministic gates, standing
    bench policy).
- **Cross-cutting AC-9 (build purity):** default kernel build unchanged — `cd kernel && cargo
  tree -e no-dev | grep -c serde` = 0; `hydra_perception.rs` is pure std; supervisor + engine
  adapter live outside the kernel behind off-by-default features with the standard
  what-it-pulls-in headers.

## 9. Non-goals (closed list)

- **Zero changes to Hydra's charter, posture, or code semantics.** Closure = NEVER,
  kill-switch-only, all-safeties-lift-on-intervention: verbatim as chartered. No reconciling
  with items 73–78, ever; no 73–78 approval pipeline on any Hydra mutation. `hydra.rs` itself
  is not edited (schema goes in a sibling module).
- Supervisor jurisdiction is the model channel only — it never gates existing Hydra callers,
  the breach alarm, peer ingest, or integrity checking.
- No fine-tuning build of any kind (corpus accumulation only; TRIGGER-FINETUNE unchanged).
- No tools for Hydra's models; no agentic/planning role; no order-flow contact; no vision in
  settlement authority (P101 §7 stands: `DeliveryClaim` has no photo field).
- No model weights in the git repo; no LLM-as-judge anywhere; no learned router (HK-05/HK-09
  non-duplication ruling stands); no scoring/rating of anything or anyone.
- No self-modification path for the supervisor itself; no self-spawning anywhere (§6.3
  gateways would be prerequisite, and none is proposed).
- **No synchronous permission checkpoint on any Hydra path, ever** — the osmosis/oscillator
  shape is load-bearing, not cosmetic: a future change that makes Hydra pause, wait, or
  request permission from the supervisor violates this blueprint AND the charter (AC-11 is
  the standing tripwire).
- No GPU assumption; no new kernel dependencies; no reopening of the local-model-wiring
  blueprint's §1 locks.

## 10. Open items

1. **Physical constraints** — repo-wide gap, inherited, still owned by no one (§5.5).
2. **Sibling cross-links** — Phase 0, pending their landing (§2.4).
3. **P-A input taxonomy** — which observation sources a hub may legitimately watch (consent
   surface included) is finalized at Phase 1 with the operator's allowlist (§4.7), not
   assumed here.
4. **Supervisor crate placement** — new tiny zero-dep crate vs a module in an existing edge
   crate: implementation-time call under the existing-files-win rule; either way outside the
   kernel, feature-gated off.

---

*End of blueprint. Both halves are design + plan; nothing is built. The load-bearing sequence
is Phase 0 (sibling reconciliation) → Phase 1 (supervisor + AC-2's structural guard) →
Phase 2 (drift-gated integration); Phases 3–4 ride the sibling engine's timeline.*
