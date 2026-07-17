# CORE ROADMAP STANDARD (2026-07-17) — the planning ideas, saved before execution

> **Status: this document is the operator's standing quality bar for ALL future planning in this
> repo, not a one-off deliverable.** Per operator directive (2026-07-17, verbatim intent preserved
> in `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/00-SOURCE-PROMPT.md` and this session's
> transcript): "This should be not a guidance - but level of quality constant invoked for any
> planning... zero divergencies from it." Every future blueprint in this repo is measured against
> §2 below until the operator says otherwise.
>
> **Sequencing, per operator instruction:** this document is Step 1 — "save the planning ideas
> first." Step 2 — "then save the plans after finishing them" — is the phase-by-phase execution
> this document orchestrates, starting immediately after this lands (§5).

---

## 0. Ground-truth inventory (verified this session, not assumed)

Before designing anything new, the existing planning corpus was enumerated live (not from memory
or an older doc's claim):

**Pre-existing master roadmaps (dowiz, duplicated verbatim into the `dowiz-agentic-mesh` and
`dowiz-spectral-evolution` worktrees by shared history):**
- `MASTER-ROADMAP-MVP-2026-07-12.md` (repo root) — earliest, MVP-scoped.
- `docs/design/MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`
- `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md`
- `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md`
- `docs/design/MASTER-EXECUTION-PLAN-2026-07-13.md` — **added 2026-07-17**: omitted from this
  inventory as originally written; the P-I Wave-1 audit (§0/§2.5) found it (the SOVEREIGN doc's own
  header already named it) — so the superseded set is **5 older docs, not 4**, and the banner pass
  covered all 5.
- `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` — **the newest, most-actively-
  referenced one**, with 19 phase blueprints already written (`docs/design/sovereign-roadmap-
  2026-07-16/BLUEPRINT-P01..P19-*.md`) and a live P06-blocks-three-arcs finding already tracked in
  memory (`sovereign-architecture-19-phase-roadmap-2026-07-17.md`).

**Consolidated arc summaries (each a self-contained sub-roadmap for one initiative):**
- `dowiz-agentic-mesh/docs/design/agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md`
- `dowiz-spectral-evolution/docs/design/spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md`
- `docs/design/living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`

**Today's mesh-masterwork corpus (this session):**
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/` — source prompt, 10 batch/correction docs,
  the v1 synthesis, and (in flight) the v2 synthesis.
- 13 additional standalone research blueprints landed yesterday/today (latency, eigenvector,
  cache-tensor-arena, event-driven orchestrator, fault isolation, Linux-engineering-adoption,
  memory-optimization, native-telemetry, wave-scheduling, delivery-flows audits, web3 synthesis).

**Decision on consolidation (per "not revisit twice"):** `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-
2026-07-16.md` becomes the **single canonical entry point**. It already has the newest phase
structure and is already the one this session has been updating (§8.12/Phase 30 added earlier
today). The four older MASTER-* docs are **not deleted** (they're historical record of earlier
planning rounds, real audit trail) but get an explicit **SUPERSEDED-BY banner** pointing here, so
a future reader never re-derives from a stale one. This document (`CORE-ROADMAP-STANDARD`) is the
**quality contract** the canonical roadmap and every phase blueprint under it must satisfy — it is
deliberately a *different document* from the roadmap itself, because a standard shouldn't be
rewritten every time a phase changes.

---

## 1. Scope boundary (operator-stated, binding)

- **All local repos**: `dowiz` (this repo), `bebop-repo` (`bebop2/` crates — active), `openbebop`
  (live push remote for bebop-repo), `dowiz-agentic-mesh`, `dowiz-spectral-evolution`,
  `hermes-agent-kernel-rewrite` where cited.
- **Excluded**: the original `bebop` repo (git `origin` remote of `bebop-repo`,
  `git@github.com:SyniakSviatoslav/bebop.git`) — legacy, archived, ideas may be raided from it but
  it is not maintained or planned against.
- **Execution substrate**: kernel/Rust/WASM only, per the standing execution-model rule
  (`bebop2-mesh-masterwork-2026-07-17.md`) — Node/TS/JS/Python are adapters/bridges at most, being
  actively eliminated (`tools/eqc-rs` port, `apps/web`+`packages/*` decommission already landed
  this session).
- **Deployment target**: decentralized local nodes (courier devices, owner-operated hub servers,
  client devices) — Fly/Supabase decommissioned this session, not the planning target going
  forward.

---

## 2. THE STANDARD — what every phase blueprint under the canonical roadmap must contain

This is the reusable contract (operator: "not a guidance — a quality constant"). A blueprint that
skips any of these is incomplete, not merely light:

1. **Ground truth section** — every claim about existing code carries a `file:line` cite verified
   *this pass*, not inherited from an older doc's claim. "Ground truth is non-discussible."
2. **DoD (Definition of Done)** — falsifiable, machine-checkable where possible (a test that goes
   RED before the change and GREEN after, not a prose checkbox).
3. **Spec-driven + event-driven TDD plan** — the spec (types/schemas/invariants) precedes the test,
   the test precedes the code; state transitions are modeled as events (matches the kernel's own
   `decide`/fold law), tests assert on event sequences, not just end-state.
4. **Predefined types & constants** — every new domain concept gets a named Rust type/const before
   implementation starts (no stringly-typed or magic-number placeholders in a blueprint).
5. **Adversarial/chaos test cases, including intentionally-failing ones** — at least one test per
   blueprint designed to break the invariant under test (operator: "test cases... designed to
   literally break everything"), not only happy-path coverage.
6. **AI/system-hazard safety section grounded in math/engineering** — reachability of an unsafe
   state must be argued from type-system/invariant structure (per the Monocoque doctrine already
   established: `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`
   + `bebop2-mesh-tensor-hermetic-2026-07-17/19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md`'s
   "finite anchored authority, not zero" finding) — never a policy/prose assurance.
7. **Links to docs & memory** — every blueprint cross-references the memory files and design docs
   it depends on or supersedes, by name, so the index (§6) stays navigable.
8. **Schemas designed for scaling** — data shapes must state their scaling axis (nodes/tiles/
   events/sec) and the point at which they'd need to change, not be presented as timeless.
9. **Linux-OS-development-style engineering discipline** — per
   `docs/design/BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s verdict framework
   (ALREADY-EQUIVALENT / REINFORCES / EXTENDS / GAP / DOES-NOT-TRANSFER) — reused, not re-derived.
10. **Benchmarks + telemetry** — every hot-path change ships with a measured before/after number
    (this session's own build-test-first passes are the model: real `cargo bench`/microbenchmark
    output, not an estimate) and a telemetry hook so regressions show up automatically, not only at
    review time.
11. **Microservice-style isolation / bulkhead** — a blueprint touching a shared resource must name
    the isolation boundary that keeps its failure from propagating (per idea #141 in the mesh
    dialogue's 185-item ledger, and the already-built `bounded_drainer.rs`/`budget.rs`
    degrade-closed patterns).
12. **Mesh-networking awareness** — where relevant, state whether the feature is node-local,
    gossip-propagated, or requires the transport layer (`iroh_transport.rs`/`discovery.rs`), and
    cite the real payload-size/frequency budget it needs.
13. **Rollback/fallback + self-healing/self-terminating, stated as math, not metaphor** — per the
    operator's own three-way synthesis (idea #185): Self-Healing = redundant/error-correcting math
    property; Self-Termination = a hard invariant boundary (unrepresentable-state, not a
    supervisor's decision); Snapshot Re-entry = cheap regenerative recovery from the last valid
    epoch. A blueprint claiming any of these three must show which one and why, not use the words
    loosely.
14. **Error-propagation isolation + "smart index" for catching mistakes** — cite the specific gate
    (type system, drift-gate, CI check) that would turn the bug class this blueprint introduces
    into a compile-time or CI-time failure, not a runtime surprise.
15. **Living-memory awareness (time/topology/data-flow)** — cross-reference
    `internal-retrieval-living-memory-arc-2026-07-14` where the blueprint's data has a temporal or
    topological access pattern, rather than treating storage as flat.
16. **Tensor/spectral representation where applicable** — reuse the hybrid/spectral tensor-graph
    machinery already built (`kernel/src/spectral.rs`, `spectral_cache.rs`, the Phase-28 arena) and
    the `tools/eqc-rs` equation-compiler for any closed-form math, storing generated equations as
    data (RGB-seed/procedural-encoding pattern, idea #130, ADOPTED per
    `20-BUILD-TEST-FIRST-REEXAMINATION.md`'s CORDIC proof) where a deterministic portable
    implementation exists — never a per-platform-libm form.
17. **Regression tracking** — every blueprint that fixes or changes behavior gets a named regression
    test that stays in the suite permanently, referenced in `docs/regressions/REGRESSION-LEDGER.md`.
18. **Clear instructions for other agentic workers** — a blueprint must be executable by an agent
    with zero prior session context: explicit file targets, explicit acceptance criteria, no
    "you'll know it when you see it" language.
19. **Reuse-first, upgrade-if-needed, unbounded token/time budget** — a blueprint proposing new
    machinery must first show the existing pattern it could extend and why extension doesn't work;
    "it would take too long" or "it's simpler to skip" are not valid reasons to avoid a needed
    refactor (operator: "Refactoring or major changes must not be avoided to avoid responsibility").
20. **Hermetic principles honored explicitly** — cite which of the seven Hermetic principles
    (`HERMETIC-ARCHITECTURE-PRINCIPLES.md`) the blueprint's design choice reflects or tests against.

---

## 3. Phase structure — lowest (core) to highest, absorbing existing work rather than re-deriving

> **NAMING RULING (2026-07-17, Wave 3 — P-I audit §4):** the "P-A..P-I" letters below are ratified
> as **`Layer A..I`** — an orthogonal **altitude axis** grouping clusters of numeric phases, never a
> renumbering of the canonical execution numbering **P01–P30** (P01–P19 as numbered blueprint files
> in `sovereign-roadmap-2026-07-16/`; P20–P30 as standalone blueprints indexed from SOVEREIGN
> §8.1–§8.12 — this section's original "P01-P19" reconcile scope is stale by 11 phases). The "P-"
> prefix is retired from prose to kill the P-D/P04 lexical collision; on-disk filenames keep their
> provenance names. Crosswalk table: `CORE-ROADMAP-INDEX.md`.

Ordering rule (operator, restated across this whole session): smallest kernel-level abstraction
first, highest-level product/UI last. Each phase below states what it absorbs from the existing
252-document corpus rather than starting blank.

| Phase | Scope | Absorbs / supersedes | New this pass |
|---|---|---|---|
| **P-A. Core kernel primitives** | Equations-not-primitives (`eqc-rs`), tensor/sparse/branchless memory layout, HugePages/tiling | Mesh-masterwork Batch 1, Batch 8, `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN`, `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA` | Wire `eqc-rs` into `geo.rs:39`/`domain.rs:95` (already identified, unstarted) |
| **P-B. State/consistency + living memory** | Event log, content-hashing, snapshots/epochs, CRDT boundary | Batch 2, System-Coherence doc 19 (tile→normalize→hash→snapshot chain + the 2 real bugs found) | The normalize-before-hash fix; drift-gated arena snapshot |
| **P-C. Safety / self-healing / self-terminating** | Circuit breakers, invariants, the watchdog/authority boundary | Batch 3, doc 19 Part 2 (finite-anchored-authority finding) | Hysteresis fix (`hydra.rs`), restart-intensity as a launch-path predicate (T-6) |
| **P-D. Consensus / trust / capability** | Sybil-resistance, DecisionUnit gossip, PoQ | Batch 4, 6, 7, `BLUEPRINT-LATENCY-ELIMINATION` §2 (Decision Compiler) | `RootDelegationPolicy` closure (`node_id.rs:156-184`) — open operator decision |
| **P-E. Network / hardware / crypto-in-core** | Mesh transport, hardware attestation, crypto-verification speedup | Batch 5→14v2 (target-corrected), the SIMD-batched-verify + core/cache-domain-NUMA redirect (in flight) | Pending the current Opus redo |
| **P-F. Local AI / MoE mesh** | DecisionUnit compilation, MoE-as-mesh-mirror, STARK-in-core | Batch 21 (distributed-inference rejection), pending MoE-specific redo (in flight) | Pending |
| **P-G. Product/UI on kernel** | WASM bridge wiring, physics-UI, RLS-safe migration | Batch 9 (bridge already exists, wiring gap), `BLUEPRINT-P16-product-ui-rebuild`, `LIVING-INTERFACE-ROADMAP` | Money dual-authority flip (explicitly gated, not Wave 1) |
| **P-H. Ops / telemetry / benchmarks / regression** | Native telemetry, chaos testing, regression ledger | `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS`, `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION`, `REGRESSION-LEDGER.md` | Chaos-injection test harness (idea #143), unconditional-fail test suite |
| **P-I. Cross-repo consolidation** | Update the 4 older MASTER-* docs with SUPERSEDED banners, reconcile P01-P19 against this structure | All 5 pre-existing master roadmaps | The consolidation pass itself |

P06 (ML-DSA `key_V` split-identity verifier) remains the cross-cutting blocker already identified
(memory: `sovereign-architecture-19-phase-roadmap-2026-07-17.md`) — it gates P-C's independent-
verification leg, P-D's capability issuance, and P-G's product-safety story. Highest-leverage
single build item across every phase above, unchanged finding.

---

## 4. Orchestration plan (Step 2, starting immediately after this saves)

Per operator: "orchestrate the planning phase smartly assigning each agent or small team of agents
the corresponding phase." Model assignment restated: **Opus for research/audit** (grounding each
phase in live code), **Fable for reasoning/planning** (writing the actual blueprint against §2's
contract). Waves are collision-free (different files/phases, no shared mutable state):

- **Wave 1** (parallel, Opus): ground-truth audits for P-D (RootDelegationPolicy), P-G (WASM-bridge
  wiring detail), P-H (existing telemetry/regression tooling inventory), P-I (read all 5 old
  MASTER-* docs + all 19 P0x blueprints in full, produce a diff-against-this-standard).
- **Wave 2** (parallel, Fable, after Wave 1 + the in-flight mesh resynthesis both land): write the
  actual phase blueprints P-A through P-I against §2's 20-point contract, each citing its Wave-1/
  mesh-masterwork grounding.
- **Wave 3** (single Fable pass): the canonical roadmap update — supersede-banner the 4 old
  MASTER-* docs, fold P-A..P-I into `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`, build
  the master index (§6 below, promoted to a real `docs/design/CORE-ROADMAP-INDEX.md`).

Not dispatched yet — this document is the Step-1 save. Wave 1 dispatches next.

---

## 5. What this document deliberately does NOT do

Per the standard it sets (§2 item 19: reuse-first) this document does not re-derive P06, the
mesh-masterwork verdicts, or the Hermetic principles — it points at them. It does not yet contain
the actual phase blueprints (that's Wave 2's output, saved separately and indexed here once real).
