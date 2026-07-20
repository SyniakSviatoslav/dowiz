# Synthesis: The Finishing Layer — Living Memory, Wave Prediction, Local Multimodality (2026-07-20)

> **Status: RESEARCH SYNTHESIS ONLY — not a blueprint, no build items, no DoD, no code.**
> Written per operator directive: "Synthesis with fable - no plans or blueprints for now. Ask
> any questions needed or decisions needed from me, do not guess or choose on yourself."
> Produced by: three independent verified research passes (external BDH/multimodal/random-forest
> research; internal dowiz repo grounding, file:line cited; physics/math honesty pass on the
> Maxwell/redshift/wave-propagation claims) feeding a synthesis pass run on the Fable model per
> operator instruction. All verdicts below are cited back to that research, not re-derived here.

## 0. The operator's vision, verbatim

> "VLM - local llm should process images/vision/text/sound, BDH dragon hatchling model
> architecture, random forest, living-memory as file internal & external file system + database
> combined in one - ability to self-replicate and Maxwell's equations & electromagnetic waves,
> redshift theory & equation - so this should be a finishing layer in agentic local system
> evolution where all the existing parts again are upgraded & wired together between the
> different layers. From my vision living memory should become combined file system + database
> formed in BHD like n-dimensional architecture where prediction on any change/outcome/signalling
> is implemented like a wave/magnetic propagation with the ability to expand or shrink based on
> redshift theory & equation while having internal faster simulation which can be used as
> replica/backup/snapshot at same time."

Philosophical framing supplied alongside this (an existing dialogue the operator is building on,
honored as tone/constraint by this synthesis, not re-litigated): Unknown as a first-class typed
logical state, never silently coerced; AI components are a TOOL with no claim to continuity or
self-preservation — explicit rejection of anthropomorphism; hardware-level kill-switches for the
most critical paths; the human operator as final judge, AI proposals always advisory/gated;
cryptographically signed append-only audit trail as a core value ("historian's source-criticism
applied to system logs").

---

## 1. Triage: what each idea actually is

**(a) Already substantially present in dowiz under different names**

- *Wave/electromagnetic propagation as the prediction substrate.* The kernel-driven engine
  already implements exactly this: `engine/src/field_frame.rs` steps `M·ü + Γ·u̇ + c²·L·u = S`
  over a graph Laplacian with a fail-closed stability bound; `field_modal.rs` advances the same
  physics in a cached eigenbasis; `bridge.rs::VertexBridge` does matrix-free Laplacian SPMV.
  Current research (Graph Wave Networks, WWW 2025) argues for precisely this wave-over-diffusion
  approach. Invoking "the wave equation" buys nothing new — the genuinely missing piece is
  *wiring*, not physics: recall/decay in living memory today runs on BM25/PPR, not on this field
  machinery.
- *An "expansion/contraction early-warning signal."* The kernel already computes spectral drift
  (`classify_drift`/`DriftClass`/`spectral_gap`/`dominant_period`) — the honest core of the
  redshift intuition (see (d)).
- *"Internal faster simulation usable as replica/snapshot."* Two-thirds exists: `field_modal.rs`
  is literally a cheaper internal simulation of the same dynamics (decompose once, step cheaply),
  and node-local snapshot/restore is real and proven (content-addressed `BlockStore`,
  byte-identical restore, kill-9-durable FDR ring). What does *not* exist is any cross-mesh
  replication (see (b/c)).
- *Recognizing ignorance as a typed state.* The kernel's existing Unknown/Undecidable verdict arm
  is the honored norm; the finishing layer should extend it (e.g., unmerged replica state =
  Unknown, never coerced to absent/false), not reinvent it.
- *Pre-verified self-modification arrival shape.* `tools/eqc-rs` (Expr IR, dual f64/fixed
  emission, self-asserting proof programs) is the repo's one sanctioned precedent for
  machine-generated code entering the tree — exactly what the governance blueprint (items 75/78)
  plans to require. Any "self-replicate/self-modify" proposal must route through this precedent,
  not invent a second one.

**(b) Genuinely new and well-grounded**

- *Local multimodal (vision+text+audio) behind the existing `LlmBackend` port.* Gemma 3n
  (E2B/E4B, Ollama-servable today, built for on-device) is the direct fit; Qwen2.5-Omni 3B/7B is
  the stronger true-omni alternative but wants more compute. Both real, Apache/open-licensed.
- *Random forest as a deterministic pre-gate.* Verified: RF is *not* the state of the art for
  LLM-quality routing (RouteLLM et al. use learned rankers), but it *is* well-evidenced as a
  cheap, auditable, early-stopping pre-filter in resource-constrained inference (published
  MCU-scale result: 38–90% energy cut for <0.5% accuracy loss) and in audit-required
  classification. That is exactly dowiz's shape: a fixed-point, explainable triage gate
  (handle-locally / escalate-to-LLM / return-Unknown) in front of any model call, potentially
  emitted through eqc-rs so the gate itself is proof-carrying. Hard caveat: an RF gate must never
  become a scoring/reputation mechanism over participants — trust stays a signed capability
  (CI-enforced, `no-courier-scoring`).
- *Unifying living-memory dynamics with the field machinery.* The strongest physics idea in the
  prompt. Wilson-Cowan neural-field equations — the canonical model of empirically documented
  cortical traveling waves — are the *same mathematical family* as the existing
  `M·ü+Γ·u̇+c²·L·u=S` operator. If BDH is gesturing at wave-borne working memory, it validates and
  extends `field_frame`/`spectral`, requiring translation, not invention.
- *Mesh-wide memory replication.* Legitimate and aligned with D0 (decentralized, local-first,
  reliability-over-latency) — but nothing exists today. New design, not wiring (see §2, L5).

**(c) Genuinely new but requiring heavy unbuilt machinery**

- *Maxwell's equations literally* (coupled curl-based E/B structure, not a scalar wave): requires
  discrete exterior calculus / Hodge Laplacians on an oriented simplicial complex. Real,
  established mathematics — but the kernel's `Csr` has no oriented edges/faces, so this is
  substantial new machinery, and no identified dowiz need requires the E/B coupling over the
  scalar damped wave already built.
- *BDH as the local model.* Real (arXiv:2509.26507, MIT-licensed), architecturally interesting
  (Hebbian working memory, sparse positive activations, claimed monosemanticity), but a research
  prototype: no checkpoints, toy training scripts, tested only at 10M–1B params, with substantive
  published skepticism about capacity-vs-interpretability at scale. Viable as an off-by-default
  R&D experiment behind the `LlmBackend` port; not viable as a production dependency.
- *"Combined filesystem+database" living memory.* Today's retrieval is toy-scale: BM25/trigram
  proven on a 12-doc fixture, PPR on a frozen 20-node hand-authored graph, spine graph unfused
  with the PPR graph, persistence defaulting to an in-process `BTreeMap` (`PgStore` gated off, its
  one test ignored), and **no vector/embedding layer anywhere** (explicitly scoped out
  previously). The target is reachable but is a real build, not a wiring job.

**(d) Not mathematically transferable as literally stated**

- *Redshift theory/equation for expand-or-shrink.* Verdict from the physics pass: no CS
  literature connects `1+z = λ_obs/λ_emit` or Hubble's law to adaptive data structures. The
  honest translation splits in two: (i) spectral eigenvalue drift as early warning of approaching
  instability ("critical slowing down") — already built; (ii) actual capacity adaptation, whose
  real art is Growing Neural Gas / ART / consistent hashing — mathematically unrelated to
  redshift. Conflating a stability signal on a fixed-size operator with a growth mechanism is,
  per the research, a category error. The metaphor can *name* the coupling ("drift signal
  triggers a sizing policy"); it cannot *be* the equation.

---

## 2. The finishing-layer architecture (concrete, wired to existing components)

> **[STALENESS NOTE, added on reconciliation pass]** L1 below is described as an external,
> firewalled Ollama service (Gemma 3n/Qwen2.5-Omni) — the architecture as understood BEFORE later
> rulings in this same session (§8.4: "explore agent framing"; §10.2/§10.3: "push for transparency
> in L1 too," reframing L1 toward a from-scratch native BDH model). This section, §3(a)'s process-
> boundary safety argument, and §3(d)'s compute-honesty numbers all describe that pre-pivot
> baseline and have NOT been rewritten to reflect the pivot — doing so is real design work, not a
> reconciliation-pass edit (a from-scratch native L1 changes the process-boundary firewall
> argument materially, per report E's finding). Treat this section as the STARTING architecture
> the pivot supersedes for L1 specifically; L2-L5 are unaffected by the pivot and still stand as
> written. Resolving L1's actual architecture post-pivot is real follow-on work, not done here.

Five layers, all strictly on the advisory side of MANIFESTO C1 except where noted.

**L1 — Perception (R&D lane).** Gemma 3n served via Ollama through the existing `llm-adapters`
`LlmBackend` port; `agent-loop` remains the bounded executor; the `agent-facade` compile firewall
(only agent crate importing `dowiz-kernel`, no mutation re-exports) already guarantees
structurally that nothing here can name kernel mutation. Multimodal inputs (courier photos, voice
notes, documents) become *typed observations with an explicit Unknown arm* — evidence for humans
and for L3 writes, never protocol inputs. BDH slots in as an experimental second backend behind
the same port, off by default per feature discipline.

**L2 — Gating.** A deterministic RF pre-gate ahead of every model call: integer-thresholded
trees, early-stopping, three-way output (local / escalate / Unknown-abstain), auditable per
decision, plausibly eqc-rs-emitted in the fixed-point path so the gate carries its own proof.
Sits at the `agent-loop`/`agent-facade` edge. This is the only "AI-adjacent" component
deterministic enough to sit near the runtime line — which is exactly why its scope needs an
explicit ruling (Q8 below).

**L3 — Living memory as combined FS+DB.** The honest realization of "n-dimensional BDH-like"
structure is: **one content-addressed substrate, multiple views.** `BlockStore` CAS as the
storage substrate (already gives byte-identical snapshots for free); the event log as the sole
write path (append-only, signable — the historian's source-criticism value baked in); the spine
tag/backlink graph fused with the PPR graph (today unfused — a named gap) as the "filesystem"
view; BM25/trigram indexes as the "database" view. "BDH-like" then means the fused memory graph
carries Hebbian-style edge-weight updates driven by co-access — which is *precisely* the shape of
weight self-mutation Hydra already does, and the governance blueprint says must never be
autonomously applied. This lands directly on the contradiction in §3b; it cannot be built before
that ruling. Whether embeddings/HNSW enter at all reopens a closed scoping decision (Q5).

**L4 — Prediction/signaling as waves.** Run the existing field operator over the *memory graph's*
Laplacian rather than (only) the UI grid: writes/events inject source terms `S`; propagation is
anticipatory activation across related memory; damping `Γ` is decay; `field_modal.rs`'s eigenbasis
stepping is the "internal faster simulation" — and the MCU-synthesis's top identified-but-unbuilt
optimization (cache the eigendecomposition keyed by `snapshot_root`) is the enabling layer
underneath it. Honest limit, stated hard: this yields advisory anticipation signals — retrieval
ranking, prefetch hints, agent context — and must never alter `decide`/`fold` outcomes. It is also
not a data backup: modal state is compressed *dynamics*, `BlockStore` snapshots are *data*;
conflating them would make the "simulation-as-backup" idea unsound.

**L5 — Snapshot/replica/backup.** Real today: node-local (`BlockStore` + FDR ring) and a
single-recipient encrypted offsite envelope (`hub_supervisor.rs`, X25519→SHAKE256→AES-256-GCM).
Genuinely new: any multi-node scheme. A "living replica" across the mesh means
capability-authenticated replication of *signed event-log entries* (not mutable state) over
`mesh-adapter`/bebop2, with explicit convergence semantics for partitions — local-first
guarantees divergence will happen; unmerged state should surface as Unknown, per the Kleene norm.
Nothing here exists; it is new protocol design with the OpenBebop crypto seam as its foundation.

---

## 3. Constraints, leaks, risks (priority order)

**(a) MANIFESTO C1 — "No AI in protocol/runtime logic."** L1 and BDH are unambiguously R&D/
back-office. Two blur risks: (1) the RF gate — deterministic, so arguably legal near the line,
but if its verdicts ever gate *order flow* rather than *model-call flow*, C1 is breached in
spirit; (2) L4 field predictions leaking into the runtime path — prefetch quietly becoming
pre-authorization, anticipation quietly reordering decisions. The proven defense pattern already
exists in-repo: the agent-facade compile firewall makes kernel mutation *unnameable*. The same
structural (not procedural) separation should bound L3/L4 outputs. Failure mode if blurred: the
deterministic-replay property (no clock/RNG/float in the decision path; every node replays
identically offline) dies silently, and with it invariant six.

**(b) Hydra vs. items 73–78 — RULED by the operator 2026-07-20 (recorded verbatim, mid-synthesis):
"hydra system is intended as self-defense/signaling system - it should contradict the governance,
no vision changes for it."** `kernel/src/hydra.rs` (50KB, compiled, registered) grants bounded
unsupervised self-mutation of its own topology/weights, gated only by spectral drift, with
"closure = NEVER — only kill-switch" and "on intervention, ALL safeties LIFT." This deliberately
and permanently sits OUTSIDE `BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`'s
thesis (no autonomous-apply path may ever exist, no beneficial-change exception) — Hydra is a
**chartered exception by design**, not an oversight to reconcile, because it belongs to a
different class of system: self-defense/signaling, not business-logic/code self-modification.
**No changes to Hydra's design or posture are wanted as part of this synthesis; do not fold it
into 73-78's frame.**

This ruling resolves the top-level fork but opens a narrower, still-open one: L3 (living memory)
is a data/memory system, not itself a self-defense/signaling system. If L3's proposed
Hebbian-style memory-graph self-mutation (§2, L3) is built, does it inherit Hydra's chartered
exemption (because it's adjacent/supportive to self-defense signaling), fall under 73-78's
governance (because it's a memory/business surface, categorically different from Hydra's
self-contained topology), or constitute a **third, distinct class** the operator hasn't named yet?
This is the operator's call — see Q1′ below (replaces the original, now-resolved Q1).

**(c) Self-replicating memory attack surface (all new, none currently defended):** *Replica
poisoning* — a poisoned memory entry replicated mesh-wide then feeds the RF gate and LLM context
on every node: prompt-injection-at-rest with mesh-scale blast radius; mitigated only if
replication carries signed events with per-writer capabilities, never raw state. *Split-brain* —
guaranteed under local-first partitions; needs explicit merge semantics up front (Unknown-typed
until merged). *Unbounded growth* — Hebbian weights and wave energy both accumulate; decay
budgets are mandatory, and this intersects (but is not the same as) the still-open agent-memory
GC question — whether cleanup is internally agent-triggered or externally audit-triggered. That
open thread should be settled alongside, not conflated with, L3's decay design. *Autonomous
writer blast radius* — a BDH-derived component with write access to shared replicated memory is
categorically larger than Hydra, which is self-contained and unwired to business logic. *Audit
trail* — replicating signed append-only events preserves the forensic/source-criticism value;
replicating mutable state destroys it.

**(d) Compute honesty.** The physics and RF layers are cheap by design (SPMV + cached eigenbasis
+ integer trees) and fit Hetzner-class CPU targets — that is their engineering virtue. The
multimodal layer is the constrained one: Gemma 3n E2B/E4B is CPU-feasible but slow (batch/
back-office latency, which C1 mandates anyway); Qwen2.5-Omni 7B realistically wants a GPU dowiz
does not have (GPU features in engine/kernel are declared-but-empty seams). BDH training at
10M–1B is plausibly edge-scale per the research, but nobody has published LLM-competitive
results, and no checkpoints ship — any BDH work is training-from-scratch R&D. Nothing in this
layer should silently assume GPU acquisition (Q10).

---

## 4. Position against the roadmap baseline

- **Space-grade roadmap (78 items):** items **64/65** (composition root / capability minter, 0
  code) are hard prerequisites for any self-modifying surface per items **73–78**, which are the
  governance frame this whole layer must live inside; items **45/47** define the AI-boundary side
  it must respect; items **20/28** (living-memory / optical-compression) are the direct substrate
  L3 extends, and the eigendecomposition-caching finding from the MCU synthesis is L4's enabling
  optimization. The item-7 Kani-proof pattern is the natural bar if the RF gate enters the kernel.
- **CORE-ROADMAP (P-track):** the AGENT lane (P40 executor wiring, `agent-facade`/`agent-loop`/
  `llm-adapters`) is where L1/L2 plug in — the `LlmBackend` port and compile firewall already
  exist and are the right insertion points.
- **Conflicts:** any autonomous-apply for memory topology/weights conflicts with items 73–78 as
  written; Hydra already does. Mesh replication (L5) extends the bebop2/mesh-adapter arc but
  corresponds to no existing item — it is net-new protocol design (deliberately not numbered
  here, per scope: no blueprints this pass).

---

## 5. Open questions for the operator (rulings recorded 2026-07-20 where given)

1. **[RESOLVED]** Hydra vs. items 73–78 — operator ruled: Hydra is a chartered self-defense/
   signaling exception, intentionally outside the governance frame, no vision changes wanted.
   **[RESOLVED]** Q1′: operator ruled L3 is a **third, hybrid class** and asked for researched
   scope proposal — see new §6 below (proposal, pending sign-off on the specific criteria).
2. **[RESOLVED, ruling updated]** "Ability to self-replicate" — operator ruled both designs in
   scope: (a) data/memory replication AND (b) autonomous self-spawning. **[RESOLVED]** governance
   class for (b): operator ruled 2026-07-20 it **inherits the §6.2 hybrid class, gated by
   explicit gateways + the mesh's existing signed protocol** — overriding the research's
   73-78-or-stricter recommendation. See §6.3 for the working interpretation of "gateways &
   protocols" still needing confirmation.
3. **[RESOLVED]** Maxwell — operator ruled **literal E/B coupling**, not the scalar-wave family
   already built. This is a real architectural commitment, not a relabeling: it requires discrete
   exterior calculus / Hodge Laplacians on an **oriented simplicial complex** — the kernel's `Csr`
   graph has no oriented edges/faces today, so this needs a new graph-topology layer (oriented
   edges at minimum, likely oriented faces for the full Gauss/Stokes/reciprocity structure) built
   before any E/B-coupled propagation can exist. Not yet scoped as a build item (no blueprints
   this pass) — flagging that this is now the single largest new-machinery commitment in the whole
   synthesis, bigger than L3's Hebbian layer or L5's replication protocol.
4. **[RESOLVED — invented, pending sign-off]** Redshift translation — operator ruled (verbatim):
   **"litaral transferable is not existing - a new one must be inveted based on the requirements &
   redshift as aspiration - fable for it, research internally & externally."** A follow-on Fable
   pass invented a concrete candidate law, "Recession Flow (z-flow)," grounded in the real
   Laplacian Renormalization Group physics literature (not cosmology directly) — see new §7. This
   is a proposed mechanism, not yet built or blueprinted; needs explicit sign-off like §6.
5. **[RESOLVED]** Embeddings — operator ruled **reopen, in kernel**. The earlier "vector search is
   not a kernel primitive" scoping decision is reversed: a real vector/HNSW layer is now in scope
   natively in `dowiz-kernel`, not a tools/off-node adjunct. This is the biggest single addition to
   the kernel's default surface among all rulings in this pass — flagging that it will need its
   own feature-discipline treatment (off-by-default Cargo feature per CLAUDE.md's rule) when it
   moves from research to a build item, same as `pq`/`json-api`/etc. today.
6. **[RESOLVED]** BDH's role — operator ruled: **architectural template for L3's Hebbian
   memory-graph dynamics.** Confirmed reading (§6.4): reimplement the co-access-reweight/decay
   *mechanism* natively in dowiz kernel code — BDH ships no checkpoints and is unproven past 1B
   params, so this cannot mean depending on BDH's weights or importing its model stack (which
   would also collide with MANIFESTO C1). If a literal BDH model was intended instead, that's a
   different, C1-colliding proposal — flag it back if so.
7. **[RESOLVED]** Persistence authority — operator ruled **"blockstore & neurograph,"** and
   confirmed the working interpretation: `BlockStore` (content-addressed, proven) is the durable
   substrate (rules OUT the pgrust/Postgres direction entirely); **"neurograph"** is now the
   formal name for the L3 Hebbian memory graph (fused spine+PPR+co-access-weighted structure from
   §2/§6) as its own separately-identified queryable layer over BlockStore. Use "neurograph" as
   the standing term for this component in any future work on L3.
8. **[RESOLVED]** L4's advisory reach — operator ruled **broad**: shapes what humans see
   (retrieval ranking, prefetch, cache warming) AND agent context/tool choice AND whole-system
   prediction. The one invariant that stays absolute regardless (restated precisely in §6.5):
   no L4-derived value may ever reach `decide*`/`fold*`, the money ledger, or `RedLinePolicy`
   grant evaluation — "tool choice" means re-ranking within an already-granted scope, never
   granting.
9. **[RESOLVED]** Replication trust/privacy — operator ruled **per-capability partial**: each mesh
   hub replicates only the living-memory slice its granted capabilities cover. This matches the
   existing signed-capability trust model (no new blanket-trust surface) and is consistent with
   the P4 "no capability-surface effect" criterion in §6.2 — replication itself must not become a
   backdoor around the same capability boundary L3's weight-updates are barred from crossing.
10. **[RESOLVED]** Compute budget — operator ruled **GPU node in scope**. This reopens
    Qwen2.5-Omni 7B (true omni, stronger than Gemma 3n) as the L1 candidate and makes real
    BDH-at-scale experimentation plausible (still training-from-scratch — no checkpoints exist
    regardless of hardware). Does not change any C1/governance ruling — GPU compute is an R&D-lane
    resource, still structurally firewalled from the runtime/protocol path.
11. **[RESOLVED]** Agent-memory GC trigger — operator ruled **hybrid: both internally
    agent-triggered and externally audit-triggered**, settled together with L3's decay design.
    Concretely: cleanup can fire from the agent's own utility scoring OR from an external audit
    loop, and should reuse the same §6.2 P2 (bounded/convergent) and P3 (reversible-by-replay)
    criteria already approved for L3 — one decay/cleanup discipline, two trigger sources, not two
    separate mechanisms.
12. **[RESOLVED]** Hardware kill-switch scope — operator ruled **yes, include the living-memory
    write path** among the electrically-gateable critical paths, since it now carries both memory
    mutation and self-spawn authorization (§6.3). This is a hardware/physical design implication,
    not just software — noted for whenever this reaches implementation.
13. **[RESOLVED]** §6 hybrid-class scope criteria (P1-P5) and the shadow-then-ratchet middle
    tier — operator **approved as proposed**. This is now the design to build toward whenever L3
    moves from research to a blueprint.
14. **[RESOLVED]** Self-spawning "gateways & protocols" content — operator confirmed the working
    interpretation (§6.3's five conditions) is correct.

---

## 6. L3 Hybrid Governance Class — Scope Proposal (2026-07-20, PROPOSAL, pending sign-off)

Produced on request (Q1′): "third class, reason & research to suggest scope - I see it as hybrid
between the governance & hydra systems." Nothing in this section is decided — it is a researched
recommendation, structured the same way as the rest of this synthesis.

### 6.1 Prior art for the parameter/structural bifurcation

Real, established, though no single source blesses the fit end-to-end. **Strongest fit** —
McKinley/Sadjadi/Kasten/Cheng's self-adaptive-software taxonomy ("Composing Adaptive Software,"
IEEE Computer 2004) draws exactly this line: **parameter adaptation** (runtime variable changes)
vs. **compositional adaptation** (structural/algorithmic part swaps) — the field treats the
former as low-risk/continuous and the latter as needing assurance machinery. Hebbian edge-weight
updates are textbook parameter adaptation; adding node types or changing the update rule is
compositional. **Good fit** — production MLOps practice auto-applies continuous drift monitoring
(ADWIN/DDM, shadow-mode scoring) while gating model-registry *promotion* through lineage +
approval signatures — the promotion half maps onto items 73-78, the shadow/canary half maps onto
the middle class below. **Everyday precedent** — databases auto-apply planner statistics/cache
metadata with zero review while schema changes (DDL) are always reviewed migrations; dowiz
already encodes this instinct (migrations are red-line, `RedLinePolicy::DenyByDefault`,
`kernel/src/ports/agent/scope.rs`). **Honest limit:** the literature's "safe to auto-apply" cases
all additionally require bounded step size, decay toward a fixed point, and no change to the
reachable action set — imported below as explicit conditions, not assumed.

### 6.2 Proposed scope criteria (bright-line test)

**Proposed rule: an L3 change auto-applies (class i, Hydra-adjacent) iff ALL five hold; it is
73-78-gated (class ii) if it fails P1 or P4; it's the new middle class (iii) if it passes P1/P4
but fails P2, P3, or P5.**

- **P1 — Parameter, not structure.** Only numeric weight fields on *existing* nodes/edges change.
  No new node/edge types, no schema, no change to the update rule or its constants (a decay
  constant is code even though its output is data).
- **P2 — Bounded and convergent.** Per-cycle delta capped; weights sparse/non-negative/mass-capped
  (the BDH pattern); repeated application cannot diverge. Reuse the kernel's existing
  spectral-drift gate (`classify_drift`/`spectral_radius` — already Hydra's own bridge,
  `kernel/src/hydra.rs`) on the memory-graph adjacency: same math, new surface.
- **P3 — Reversible by replay.** Weight state is a fold over an access-event log; deleting derived
  weights loses ranking quality, never data. Rebuild-from-log must be a tested invariant.
- **P4 — No capability-surface effect.** Cannot alter what any agent/human *can* do — no tool
  grants, no `RedLinePolicy` scope crossing, no weight change flipping unreachable data to
  reachable. Weights may only *reorder* the already-reachable.
- **P5 — Local-first blast radius.** Delta applies locally; mesh propagation of weight deltas is
  CRDT-mergeable advisory data only, never authoritative state other nodes must obey.

**Middle class (iii) mechanism — the one genuinely new piece: shadow-then-ratchet.** The system
computes the discrete operation autonomously but applies only its *reversible* half (mark,
demote, tombstone-with-TTL); the *irreversible* half (physical delete, provenance-losing merge)
requires 73-78-style approval, batchable. This matches the standing living-memory rule already in
project memory ("move-not-delete; demote-never-delete") — consistency, not invention.

**Worked examples (checkable predicates, not vibes):**
1. Co-access edge weight `+δ` on read, exponential decay per cycle → **class i**. Only `weight`
   touched, bounded, log-replayable, no reachability change, local.
2. New node-type schema added to the memory graph → **class ii**. Fails P1 — migration-shaped,
   red-line-adjacent.
3. Memory-graph region prune/merge → **class iii**. Passes P1/P4, fails P3 (destructive) → auto
   tombstone-with-TTL; physical delete gated.
4. Replica conflict resolution → **splits on determinism**: a commutative CRDT join is class i (a
   fold, no information discarded); a heuristic winner-pick that discards a branch fails P3 →
   class iii (keep the loser tombstoned, auto; discard it, gated).
5. System proposes retuning its own decay constant → **class ii**. Fails P1's no-meta-parameter
   clause — this is exactly the AI-proposed-logic-change items 73-78 exists for.

### 6.3 Self-spawning — RULED 2026-07-20, overrides the research recommendation below

**Operator ruling (verbatim): "Self spawning must inherit hybrid class with gateways & using
protocols."** This is final and stands regardless of the research argument that follows — recorded
for the reasoning trail, not as a live recommendation. Self-spawning is governed under the §6.2
hybrid class, NOT escalated to 73-78-or-stricter as a separate track — but gated by explicit
**gateways** (capability-token checkpoints) and **protocols** (the mesh's existing signed/
capability-authenticated wire protocol, `mesh-adapter`/bebop2, as the coordination channel — not
an ad-hoc side mechanism). Working interpretation, pending confirmation (see the question round
this triggers): a spawn event must pass through a capability-gated checkpoint AND be announced/
authorized over the same signed mesh protocol other cross-node events already use, rather than
inventing a parallel authorization path.

**The original research argument (kept for the record, not currently controlling):**
autonomous instance-spawning fails the §6.2 test on every axis — structural (new running
component, not a weight), irreversible in a new way (a spawned process isn't replay-erasable),
maximally capability-expanding (a new instance *is* new capability) — and safety literature treats
replication as its own threat class (METR's "rogue replication"/ARA threat model; 2025-26
empirical work, Palisade and arXiv 2503.17378, showing LLM-driven self-replication is
demonstrably achievable today). Critically: **Hydra's kill-switch guarantee breaks under
spawning** — one event log has one kill-switch; N independent instances each need their own,
silently turning "only kill-switch stops it" into "only N kill-switches stop it." The five
conditions originally proposed as *beyond-73-78* safeguards are very likely still the right
content for the operator's "gateways & protocols" — per-spawn capability token (never blanket),
kernel-enforced max-instance quota, signed parent→child lineage so every instance stays
kill-switch-addressable through its ancestry, no transitive spawning by default, mesh-visible
spawn events — just re-homed as hybrid-class gate conditions instead of a separate governance
tier. This reframing needs explicit confirmation, not assumption (see question round).

**Data/memory replication (the (a) half of Q2) is unaffected** — as CRDT-mergeable data it lives
comfortably in classes i/iii above, unchanged by this ruling.

### 6.4 BDH-as-template — confirmed reading

Confirmed, and the only workable reading: BDH publishes architecture + training code but **no
pretrained checkpoints**, evaluated only 10M-1B params. "Template" means reimplementing the
*mechanism* (co-access strengthens, decay weakens, sparse positive-only activation) as native
deterministic dowiz kernel code — the way `hydra.rs` already implements its own topology math in
std-only f64 — not depending on BDH's (nonexistent) weights or its model stack. If a literal
trained-BDH substrate was intended instead, that's a materially different, C1-colliding proposal
(see open item 6 above).

### 6.5 The L4 invariant, restated precisely

*L4 may write only to advisory surfaces — retrieval rankings, prefetch queues, field-equation
source terms `S(t)`, agent context assembly, and tool ordering within an already-granted scope —
and no L4-derived value may ever be an argument to, or influence the control flow of, the
kernel's `decide*`/`fold*` family (`order_machine.rs`, `decision/mod.rs`, `event_log.rs` append
path), the `money.rs` ledger, or `RedLinePolicy` grant evaluation; equivalently, the L4 crate must
not appear in the dependency graph of any of those surfaces.* Two precisions: "tool choice" means
ranking within the granted set, never granting (`scope.rs` stays deny-by-default); enforcement
should reuse the existing P40 compile-firewall pattern (`agent-loop` structurally cannot name
kernel mutation symbols) so the invariant is a build-time fact, not a code-review promise —
consistent with how this repo already prefers its invariants enforced.

---

## 7. Redshift, Invented — "Recession Flow (z-flow)" (2026-07-20, PROPOSAL, pending sign-off)

Produced on request (open item 4): no literal transferable redshift law exists (confirmed twice
now, independently), so per operator instruction a new mechanism was invented, grounded in real
requirements and external research, using redshift as aspiration rather than source.

### 7.1 What "expand/shrink" concretely means for the neurograph

Real, numeric, per-region knobs a mechanism may legally move under P1 (no schema/structure
changes): **(1)** HNSW `ef_search`/`ef_construction`/M-budget per index shard (new surface, per
the just-reopened in-kernel embeddings ruling); **(2)** numeric precision tier of derived weights/
vector codes (f32→f16→8-bit PQ — a value-resolution change, not a schema change; content stays
content-addressed in `BlockStore`); **(3)** per-region PPR iteration/candidate-list budget
(`Ppr::rank`, `kernel/src/retrieval/ppr.rs`; `personalized_pagerank`, `kernel/src/csr.rs:330`,
already takes an explicit `iters` parameter); **(4)** replication advisory priority within the
already-ruled per-capability-partial baseline (never widens past it); **(5)** a tombstone
eligibility flag feeding the §6.2 middle-tier shadow-then-ratchet path (nomination only — the
gated path still decides). **Explicitly excluded:** node merge/split into supernodes and any
edge-type change — that is the literal RG "blocking" step (§7.2) and is structural, so it belongs
to 73-78-gated review, not this mechanism. The invented law moves resolution parameters
continuously; only the existing gated path may ever realize actual coarse-grained structure.

### 7.2 Why Laplacian Renormalization Group, not literal cosmology

Verified: "information redshift"/"computational redshift" as an established CS concept does not
exist (search came up essentially empty — one 2007 market-segmentation thesis, one fringe 1998
astrophysics preprint; neither transfers). What does exist, verified and current: the
**Laplacian Renormalization Group** (Villegas et al., *Nature Physics* 2023, arXiv:2203.07230) —
coarse-graining heterogeneous networks via the diffusion operator `e^(-τL)` with diffusion time τ
as the scale parameter, an active lineage through 2024-2025 (spectral coarse-graining, arXiv
2411.11991; Higher-order LRG, *Nature Physics* 2025; a 2025 *Nature Reviews Physics* survey). This
fits dowiz specifically because **LRG's scale parameter is diffusion time under the graph
Laplacian — the exact operator family the kernel already runs**: PPR is α-discounted diffusion
(`csr.rs:330`), `laplacian_spmv` (`csr.rs:552`) is the L-application primitive, the engine already
steps a damped wave PDE on the same L (`engine/src/field_frame.rs`). It also honors "redshift as
aspiration" legitimately — both redshift and RG flow describe how the same underlying reality
looks different observed at different distances/scales; redshift is the observable, coarse-graining
is the mechanism. Invent from the LRG family; keep redshift as the name/shape of the observable.

### 7.3 The invented law

**One sentence:** every neurograph node carries a bounded redshift scalar `z` that relaxes, one
capped step per maintenance cycle, toward a saturating function of its diffusion distance from
the current access frontier (normalized by a spectral "attention horizon") plus a staleness clock
— `z` is a pure dial on *observation resolution* (index budgets, weight precision, replication
priority), never on the underlying data.

**(a) Observation distance.** Frontier `F_t` = nodes touched in the last `W` access-log events
(a pure fold). Seed PPR uniformly on `F_t`: `d_i(t) = -ln(p_i(t) + ε)` where `p_i` = `PPR_α(F_t)_i`
— bounded in `[0, ln(1/ε)]`, measuring distance as the retrieval operator itself experiences it.
Normalize by an **attention horizon** `D_H(t) = κ · max(λ₂(t), λ_floor)`, `λ₂` =
`algebraic_connectivity` (`kernel/src/spectral.rs`, the Fiedler value). Sign discipline — the
metaphor's one real contribution: falling `λ₂` (losing connectivity) shrinks the horizon, so
fixed-distance regions redshift faster, exactly mirroring cosmology (a faster-expanding universe
has a *smaller* Hubble horizon and more redshift at fixed distance).

**(b) Expansion factor.** With `a_i(t)` = cycles since node `i` last entered the frontier:
`z*_i(t) = z_max · (1 - exp(-[d_i(t)/D_H(t) + a_i(t)/τ_R]))` — monotone in distance and staleness,
saturating below `z_max`. Actual state is a capped relaxation, not the target directly:
`z_i(t+1) = z_i(t) + clamp(z*_i(t) - z_i(t), -Δ_down, +Δ_up)` with `Δ_down ≥ Δ_up` (blueshift on
re-access is at least as fast as decay). One gate: if `classify_drift` returns `Unstable`, the
flow freezes that cycle — the existing drift *signal* is consumed as an input, never conflated
with the mechanism itself (the exact distinction the original redshift research insisted on).

**(c) What `z` does.** Three thresholds `z₁ < z₂ < z_tomb < z_max` act on §7.1's operations:
`z > z₁` → HNSW/PPR budgets coarsen (`ef_search = max(ef_min, ⌊ef_max/(1+z_i)⌋)`, literal
frequency-stretch); `z > z₂` → weight/vector precision steps down a quantization tier (full
precision stays replay-derivable, P3); `z ≥ z_tomb` for `T` consecutive cycles → tombstone
nomination into §6.2's middle tier (the hybrid-triggered agent-memory GC, §5 item 11, may consume
the same `z` as its internal-trigger signal).

**P1-P5 check (against the operator-approved §6.2 criteria):** **P1** — only numeric/boolean
fields change (`z`, precision tier, ef/iteration budgets, priority, eligibility flag); the one
structural operation (supernode merge) is excluded, routed to gating. **P2** — inputs are
bounded, the exponential saturates (`z* ∈ [0, z_max)`), the clamp caps per-cycle motion, so `z`
provably converges to a stationary fixed point in at most `⌈z_max/min(Δ_up,Δ_down)⌉` cycles under
stationary inputs — cannot diverge by construction. **P3** — `z` is a pure fold over the access
log + event-sourced edges + fixed-iteration deterministic spectral quantities (same eigen-surfaces
already in `spectral.rs`); deleting it and replaying reconstructs it bit-identically, costing
ranking/index quality, never `BlockStore` data. **P4** — coarsening is information-monotone (only
ever *less*); a max-`z` region stays fully readable under a valid capability (cost is latency, not
access); replication scope can only shrink toward the capability-partial baseline, never widen.
**P5** — every input is local; the one mesh-shared object (advisory priority) merges via
element-wise `min(z)`, a join-semilattice (idempotent/commutative/associative) — a valid CvRDT: a
region hot anywhere stays warm everywhere it's replicated, while each hub's own `z` still governs
only its own storage. **Future-aware, not future-dependent:** when the ruled literal-Maxwell
oriented-complex layer lands, `z` could gain directional components (anisotropic recession along
oriented edges) — an optional refinement, not a prerequisite this law needs today.

### 7.4 Honesty check

**Verdict: mostly an internally-consistent engineering design, with a genuine research-grade
core — the redshift framing is presentation, not load-bearing, and that's stated plainly rather
than oversold.** The defensible core: "coarsen a region in proportion to its diffusion distance
from the query frontier" is real LRG-lineage math with a falsifiable, benchmarkable prediction —
recall@k degradation from coarsening should concentrate in high-`z` regions in proportion to
their frontier-PPR mass, checkable against the kernel's existing `recall_at_k` harness. Stripped
of physics language, the law is "a clamped EWMA of a saturating function of PPR-distance and
staleness driving per-region index-resolution tiers." What the metaphor genuinely bought: (1) the
`λ₂↓ → horizon↓ → z↑` sign discipline, which caught a sign error a naive "connectivity scales
distance" formulation would have shipped; (2) the constraint that expansion acts on *observation
resolution only*, never the observed objects — coarsen the lens, never the light source — which is
precisely what makes P3/P4 hold. Where it deliberately breaks from real cosmology: metric
expansion is monotone, this law must blueshift on re-access (a relaxation toward a *moving*
fixed point, closer to RG flow with a shifting fixed point than to literal FLRW expansion). The
metaphor-free engineering justification: bounded index/storage cost concentrated where retrieval
mass actually flows, with replay-reconstructible degradation — that stands on its own.

---

## 8. The Final Layer — "Cognitive Engine" (added mid-session 2026-07-20, operator message)

Operator extended the vision: a final autonomous layer "inside Tensor Arena," capable of abstract
thinking, context understanding, and processing/generating text/visual/audio including speech —
explicitly framed (in the accompanying dialogue) as the missing piece for AGI-shaped capability,
asking directly whether it should be an external model engine behind a standardized gateway, or an
autonomous model wrapped inside Tensor Arena itself. This section grounds that question in what
"Tensor Arena" actually is in this repo before answering — the two readings lead to very different
answers, and conflating them would be a real error.

### 8.1 What "Tensor Arena" concretely is (verified against `BLUEPRINT-ITEM-38-tensor-arena-workspace-2026-07-19.md`)

Item 38 is a **KB-scale, fixed-topology, `const`-byte-offset, zero-heap-allocation memory region**
for ONE specific thing: the item-34 "toy pilot" model from the Deterministic AI Inference Arc
(items 33-44) — a small, fixed-point-quantized model whose layer shapes are known at *build time*,
allocated once, never grows (explicitly a non-goal: "NOT growable — a fixed region"), with a
`const` overlap check that makes a bad layout **fail to compile**. This is embedded/MCU-grade
determinism applied to a tiny, hand-picked pilot workload — not a general inference runtime.

**The scale mismatch, stated plainly:** a "cognitive engine capable of abstract thinking, context
understanding, multimodal processing AND generation, and speech" is Gemma-3n/Qwen2.5-Omni-class —
billions of parameters, gigabytes of weights, and (in every production LLM/VLM runtime that
exists today, open or closed) a dynamically-growing KV-cache and variable-shape activations across
turns. **No current LLM inference stack — llama.cpp, vLLM, ONNX Runtime, anything — achieves
zero-dynamic-allocation, `const`-offset inference at this scale**, because attention's KV-cache
genuinely grows with context length; that's not an implementation gap dowiz could close, it's
close to the state of the art's actual limit. A model at this scale **cannot** live inside Tensor
Arena as item 38 currently specifies it, full stop — that's a verified fact, not a design
preference.

### 8.2 Two honest readings — genuinely different answers

**(A) "Cognitive engine" = L1, and it's already mostly specified.** The multimodal
process-*and*-generate-including-speech requirement is, concretely, what **Qwen2.5-Omni** already
does (its "Thinker-Talker" architecture, confirmed real in this synthesis's own research — joint
text/image/audio/video understanding, streaming speech *generation*, not just understanding) — and
your compute-budget ruling (§5 item 10, GPU in scope) already unblocks it as L1's stronger
candidate over Gemma 3n. Under this reading, the "final layer" isn't new scope at all — it's
confirming L1 as originally sketched in §2 already covers it, running as an **external service
behind the existing `LlmBackend` port** (Ollama or a dedicated serving process), never inside
Tensor Arena. This is the same DECART shape the P40 blueprint already ruled for a different
subsystem (Option B: sibling service behind a port, not an in-process dependency) — reversible,
isolates the heavy dependency, preserves everything else's determinism.

**(B) "Tensor Arena" names a NEW aspiration, not item 38 literally** — a wish that dowiz's own
zero-jitter, const-offset, illegal-state-unrepresentable discipline could someday extend to a
full cognitive-engine-scale model, not just the tiny pilot. This is a legitimate ambition but
should be named honestly as **largely unprecedented systems-research territory** (bounded-memory,
deterministic-offset LLM inference at real scale is not solved anywhere today), not a near-term
build — it would need to be scoped as its own multi-year research arc, not folded into item 38's
existing KB-scale spec.

### 8.3 The "abstract thinking" piece — the one part that isn't just a bigger model

Multimodal understanding+generation is (A) above — largely a wiring question. "Abstract thinking,
context understanding, strategic hypothesis-building, higher-order symbolic/semantic concepts" (the
accompanying dialogue's own words) is different in kind, and worth being honest about: **no current
open or closed model — local or cloud — has verified, general abstract-reasoning capability**;
LLMs approximate reasoning via next-token prediction, which is useful but not the same claim. The
dialogue itself suggests a real, grounded direction: **"гібридну нейросимволічну архітектуру"**
(hybrid neurosymbolic architecture) — and dowiz is unusually well-positioned for exactly that,
because it already has a proven, deterministic SYMBOLIC reasoning core (the `decide`/`fold` FSM,
`eqc-rs`'s equation-to-proof compiler, the Kleene three-valued logic already honored throughout
this synthesis) that a neural L1 model could be paired WITH rather than trying to reproduce
symbolic reasoning inside the neural net itself. That pairing is a real, buildable-today shape
(neural model proposes, symbolic kernel verifies/decides) — not "abstract thinking inside a
tensor arena," but neural-proposes/symbolic-decides, which is also precisely the L1→L2 gating
shape already established in §2.

### 8.4 [RESOLVED] The tension named, then ruled

The accompanying dialogue frames this in AGI/agent language ("агент із абстрактним мисленням,"
"когнітивний рушій") that sits in real tension with the philosophical framing supplied earlier in
this same synthesis (§0): AI as TOOL not agent, explicit rejection of anthropomorphism, no
self-preservation drive, hardware kill-switches for critical paths — and with MANIFESTO C1 ("No AI
in protocol/runtime logic... AI only for R&D/back-office"). This is exactly the kind of thing
§3(a)/§6.3 already had to be explicit about for Hydra vs. items 73-78. A "cognitive engine" that
stays strictly R&D/back-office, advisory-only, and structurally incapable of self-directed action
is a different (and much safer) thing to build than one framed as an autonomous reasoning agent —
same underlying model, very different governance answer.

**Operator ruled (later the same session, asked directly as "which framing governs — strictly
tool/R&D/back-office, or explore agent framing"): "Explore agent framing."** Not strictly
tool/R&D-only. This does NOT override MANIFESTO C1 (the deterministic protocol/runtime core stays
AI-free regardless) — it means the cognitive engine's own governance is not automatically capped
at L1's original advisory-only framing, and instead reopens the same class of question §6.3 asked
about self-spawning: what governance class does an agent-framed cognitive engine actually need
(hybrid class? items 73-78? something else)? That question is not answered by this ruling alone —
it is the next thing this component needs before real design work, same pattern as §6's L3
treatment. This ruling is what licenses treating BDH-for-L1 (§10.2) as more than a firewalled
experiment despite its cost, and downstream references to "the agent-framing ruling" in §10
point here.

---

## 9. Concurrency across layers — "listen, speak, act at the same time" (added mid-session, operator message)

Operator asked to think about asynchronicity/concurrency across all layers — socket-like
bidirectional multi-connections so dowiz can listen, speak, and act simultaneously, processing
signals and acting in parallel rather than turn-by-turn. Grounded before proposing anything:

**What's real today.** `tokio` already exists in this repo, but scoped, not default — used by
`llm-adapters` (transport/dispatch/cache modules already reference streaming), `native-spa-server`,
and the `pgrust` feature; never in the kernel's default build. So genuine async I/O is not foreign
territory here, it's an established, deliberately-isolated pattern. Against that: `agent-loop`'s
core executor (`agent-loop/src/lib.rs::run`, `kernel/src/agent/loop.rs::AgentLoop::run`) is
**structurally synchronous and turn-bounded by design** — `MAX_AGENT_ITERATIONS` (4 in
`agent-loop`, 8 in the kernel port — a naming/value discrepancy worth a separate look, not this
synthesis's concern), one blocking call, one `LoopOutcome`, no concurrency, no streaming. This
bound is not an oversight — it's the proven self-termination guarantee the P40 wiring blueprint
built and tested ("never hangs"). "Listen/speak/act simultaneously" is a genuine expansion of that
guarantee's scope, not just an implementation detail.

**Where concurrency fits naturally, without touching the termination proof.** L1's I/O — listening
(multimodal input) and speaking (generation) — can run as independent concurrent streams sitting
ABOVE `agent-loop`, matching how Qwen2.5-Omni's own "Thinker-Talker" architecture already works
(the Thinker processes input continuously, the Talker streams speech output — the model was
*designed* for exactly this, already the L1 candidate per §5 item 10). This needs no change to the
bounded-turn decision core: ears and mouth run concurrently, the brain still takes bounded turns —
each `agent-loop` invocation stays a discrete, provably-terminating unit.

**Where it gets genuinely harder — "act" in parallel.** True concurrent action-taking (multiple
tool calls / decisions in flight at once, not sequential plan→act→observe) means either (a)
several independent bounded `agent-loop` turns running concurrently, each still individually
capped, coordinated only through shared state, or (b) redesigning the executor's termination
argument itself for a bounded-concurrent model (e.g., a capped pool of in-flight actions with a
combined budget) — a real change to the exact guarantee P40 proved, not a wiring exercise. The
memory layer is already prepared for concurrent writes — §6.2's P5 + §7.3's CRDT `min(z)` merge
were built local-first/mergeable from the start — but concurrent *decision-making/tool-calling* is
a different, harder claim than concurrent *memory writes*, and needs its own explicit termination
and capability-safety argument before it's built, not an assumption that "it'll be fine because the
memory layer already tolerates concurrency."

**[RESOLVED]** Operator ruled: **redesign for concurrent action** — a bounded-concurrent executor
(capped pool of in-flight actions, combined budget), not just L1-only concurrency. This is now
scoped as real new work on `agent-loop`'s core, not a future maybe — and per the paragraph just
above, whoever builds it owes BOTH halves of the proof this section already named, not just one:
(1) **a termination argument** over the concurrent/bounded pool (the same class of proof P40 gave
the sequential version, generalized), and (2) **a capability-safety argument** — concurrent actions
mean concurrent capability checks, and nothing here yet shows the design can't produce a TOCTOU
race where two individually-in-scope concurrent actions jointly cross a `RedLinePolicy` boundary,
or that it preserves the L4 invariant's (§6.5) dependency on a totally-ordered event-log append
for `decide`/`fold`'s deterministic-replay property. **[RECONCILIATION-PASS NOTE]** a fresh
critique of this doc found the original resolution above stated only the termination half and
silently dropped the capability-safety half — restored here. Neither proof exists yet; this ruling
authorizes the *design direction*, not a design that's already been checked against §6.2's P4 or
§6.5's L4 boundary. Do not treat this as cleared for implementation until both halves are done.

---

## 10. Emergence — transparent/structural, not trained/guardrailed (added mid-session, operator message)

Operator (verbatim, translated): *"think about emergence — this should be a foundational and
transparent trait, instead of relying on external training and guardrails."* This is the most
consequential single instruction in this whole synthesis, because it doesn't describe new scope —
it names a standard the rest of the system should be judged against, including everything already
proposed above.

### 10.1 The honest, validating finding: dowiz mostly already does this

Two senses of "emergence" get conflated in ML discourse and must be kept apart here, because the
engineering implications are opposite:

- **Scaling emergence** (the ML-research sense): unpredicted capabilities appearing from a large
  trained model as parameter count/data grow — inherently opaque, discovered after the fact, the
  literal thing external guardrails exist to contain because the model's own decision process
  isn't inspectable.
- **Self-organization emergence** (the complex-systems sense — cellular automata, flocking,
  reaction-diffusion): complex, adaptive, often surprising system-level behavior arising from
  the repeated application of a small number of simple, fully-specified, deterministic local
  rules. Every individual step is transparent; the interesting behavior is a real consequence of
  composition over time, not a black box.

**dowiz's existing architecture is already, by construction, the second kind — this was true
before this session started, not something added by it.** Hydra's topology/weight evolution
(`kernel/src/hydra.rs`) is complex adaptive behavior emerging from one simple, transparent,
spectral-drift-gated rule, applied repeatedly — no training, no guardrail, no black box; you can
read the rule and predict the shape of what it does. The wave-equation field dynamics
(`field_frame.rs`) produce complex spatiotemporal patterns from one local PDE. The just-invented
z-flow (§7) produces complex, adaptive index-resolution behavior across the whole neurograph from
one simple per-node relaxation rule. The kernel's whole "illegal-state-unrepresentable" ethos
(§1.5 house standard, referenced throughout every blueprint this session touched) *is* the
transparency half of this directive, already load-bearing since `DECISIONS.md`/`MANIFESTO.md` day
zero — not a guardrail bolted on after the fact, but the system unable to reach the bad state in
the first place. The operator is naming, correctly, a principle the codebase has been quietly
enacting all along; this instruction should be read as "keep doing this, on purpose, everywhere,"
not "add something new."

### 10.2 The real tension this creates, given the ruling just made in §9/§8

A pretrained multimodal model (Qwen2.5-Omni, Gemma 3n, or a from-scratch BDH-style model) is, by
construction, a **scaling-emergence** artifact — its capabilities come from training on external
data, and its internal decision process is not inspectable the way `classify_drift` or `z-flow`
are. That is unavoidable for the neural component specifically; no realistic local model avoids
it. This sits in real tension with the "explore agent framing" ruling recorded in §8.4 for the
same component — an opaque, externally-trained model, framed as more autonomous/agentic rather
than a firewalled tool, is close to the exact pattern (train first, guardrail after) this
instruction says to move away from.

**The honest resolution the architecture already points toward (not decided here, offered as the
shape consistent with everything else in this doc):** keep the transparency/emergence property
where it's actually achievable — the SYMBOLIC/kernel half of §8.3's neurosymbolic pairing (the
`decide`/`fold` core, `eqc-rs`, Hydra, z-flow, the field equations) — and treat the neural half
(L1) as a necessarily-opaque, externally-trained component whose opacity is *contained* by keeping
it firewalled and advisory (§8.2 reading (A)), rather than asking it to also be transparent, which
no pretrained model can honestly claim to be. Under this reading, "agent framing" for L1 and "no
guardrails, transparent emergence" are in direct tension — at most one can be the real governing
principle for that specific component, and it needs to be named which.

**[RESOLVED]** Operator ruled: **push for transparency in L1 too** — reject the "contain the
opacity" fallback; pursue neural components whose behavior is genuinely inspectable/rule-derived,
not black-box-trained. Honest consequence, stated plainly per this doc's own standard: this
reframes L1 away from Qwen2.5-Omni/Gemma 3n (standard opaque Transformers, chosen in §2/§8.2 for
their omni-multimodality) and toward **BDH-style architectures as the actual L1 candidate, not
just L3's template** — BDH's real, verified differentiator (§ research, arXiv:2509.26507) is
exactly this: sparse positive-only activations claimed to give monosemanticity, the opposite of
Transformer superposition. But the cost is real and should not be softened: BDH has **no
multimodal (vision/audio) variant at all today**, no checkpoints, unproven past 1B parameters, and
real published skepticism that its interpretability gain trades off against capacity at scale.
Pursuing "transparent L1" via BDH is a materially bigger, more speculative research bet than
firewalling an opaque Qwen2.5-Omni — it likely means L1 itself becomes a from-scratch, multi-year
research arc (train a multimodal BDH-family model natively), not a near-term wiring choice behind
the existing `LlmBackend` port. Flagging this cost explicitly rather than letting the transparency
principle silently imply it's free.

### 10.3 The governing principle, ruled — resolves the §10.2 tension

Operator ruled (verbatim, translated): *"emergence should be fully allowed, but transparent, with
safeguards, a kill switch, and physical constraints."* This is the resolving statement for the
tension named in §10.2 — not a cap on capability, a *shape* for it. Four conjunctive properties,
none optional, applying uniformly to every emergent/self-mutating/autonomous mechanism in this
synthesis (Hydra, z-flow, the self-spawning gateways, the pushed-for-transparency L1, the
bounded-concurrent executor):

1. **Fully allowed** — no artificial capability ceiling; the constraint is on *shape*, not *scope*.
   This is what licenses pursuing BDH-for-L1 (§10.2) despite its cost, and "explore agent framing"
   (§8.4) despite the earlier tool-not-agent caution — those are not walked back, they're now
   bounded by 2-4 below instead.
2. **Transparent** — already this doc's running thread (§10.1): rule-derived, inspectable behavior,
   not black-box. Satisfied today by Hydra, the field equations, z-flow; the open bet for L1.
3. **Safeguards** — the §6.2 P1-P5 criteria + shadow-then-ratchet tier, the §6.3 self-spawning
   gateways (all five: per-spawn token, max-instance quota, signed lineage, no transitive
   spawning, mesh-visible spawn events), and items 73-78's human-approval gate where those apply.
   Already substantially built as *proposals* in this doc; this ruling confirms they're the right
   category of thing, not optional polish.
4. **Kill switch** — Hydra's existing charter ("closure = NEVER — only kill-switch stops it") is
   the working precedent; §5 item 12 already ruled the living-memory write path belongs at the same
   tier. This ruling extends the requirement to every mechanism in this doc, not just the two
   already named — the bounded-concurrent executor (§9) and any BDH-derived L1 component need their
   own kill-switch story before they exist, not after.
5. **Physical constraints** — the one genuinely new requirement; nothing in this doc so far
   specifies HARDWARE-level bounds (as opposed to software/capability gates). The philosophical
   framing supplied at the start of this synthesis (§0) already named electrical/signal-level
   kill-switches for the most critical paths — this ruling makes that a standing requirement, not
   an aspiration, and extends it beyond kill-switches to physical *limits* generally (e.g. a
   power/compute budget enforced at the hardware or hypervisor level, not just a software cap a
   sufficiently-capable component could theoretically reason around). **Nothing concrete exists
   here yet** — this is the one piece of the four that needs its own dedicated scoping, not just a
   cross-reference to something already built.

Practical effect on everything above: no mechanism in this doc (Hydra, z-flow, self-spawning, a
future BDH-L1, the concurrent executor) is exempt from stating its answer to all four before it's
built — "fully allowed" is not "unconstrained," it's "allowed once the shape is right."

---

## 11. [NEW, from reconciliation pass] Has the neurograph accreted too much responsibility?

A fresh critique of this doc (part of the strategic regret-minimization audit,
`docs/design/DOWIZ-STRATEGIC-REGRET-MINIMIZATION-SYNTHESIS-2026-07-20.md` report E) observed that
by the end of this synthesis, "L3/neurograph" carries at least ten distinct responsibilities under
one name: CAS storage substrate, append-only event log, filesystem view (spine⋈PPR), database view
(BM25/trigram), a net-new in-kernel HNSW/embedding vector index (§5 item 5 — the single biggest
addition to the kernel's default surface among all rulings here), Hebbian weight-learning graph
(§6.4), the z-flow multi-resolution index-tiering mechanism (§7), CRDT-replicated mesh store (L5),
self-spawning host (§6.3), a hardware-kill-switch write path (§5 item 12), and potential
cognitive-model substrate (§8/§10). No section in this doc ever asked whether that's too much for
one component — this section names the question rather than answering it, since it's a real
design judgment, not something to guess at.

The doc's own discipline argues against the accretion: §2's L4 explicitly separates advisory
*dynamics* from authoritative *data* ("modal state is compressed dynamics, `BlockStore` snapshots
are data; conflating them would be unsound") — a distinction the neurograph as currently sketched
does NOT maintain, since advisory dials (`z`, weights, precision tiers) and authoritative content
co-reside on the same node objects. This is a genuine open question, not resolved by this
reconciliation pass:

- **Split** the neurograph into its constituent responsibilities as separate, composable
  components (e.g. durable CAS+log as one thing; replay-derivable advisory views — Hebbian
  weights, z-flow tiers — as a clearly-separated second thing; distribution/governance/hardware as
  cross-cutting concerns applied to both rather than folded into either), or
- **Explicitly accept** the accretion as one component's scope, on the reasoning that it's all
  "the same graph" and splitting it would just relocate the coupling rather than remove it, or
- **Some other shape** not yet named.

Left open for the operator, same as every other genuine design fork in this document.
