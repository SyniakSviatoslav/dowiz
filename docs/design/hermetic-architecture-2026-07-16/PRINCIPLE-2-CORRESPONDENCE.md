# Principle 2 — CORRESPONDENCE ("as above, so below")

> One of 7 parallel Hermetic-principle passes (Kybalion → architecture). Grounding is code and
> canon only; every claim carries a `file:line`. A later Fable pass synthesizes all 7. This pass
> is self-contained. No mysticism.
>
> Grounded in: `docs/design/ARCHITECTURE.md` (canon, 147 anchors), `physics-ui-capture-blueprint.md`
> §1 ("ONE Laplacian `L`"), `living-interface-2026-07-16/R-LM-...md` §2 (3-tier mesh/hub/node), and
> a live read of `kernel/src/*` + `/root/bebop-repo/bebop2/*`.

---

## 1. The architecture-principle statement (concrete)

**CORRESPONDENCE (software form):** *A mechanism proven correct at one scale must be the SAME
mechanism — one implementation, one entry point — when the same conceptual problem recurs at a
different scale or context, unless a stated, falsifiable reason forces divergence; and where
divergence is forced, the divergent implementations must be pinned to each other by a parity check
so they cannot drift.*

The corollaries that make it checkable:

- **One concept → one primitive.** If two modules both need "a graph", "the Laplacian `L`", "a
  PageRank", "a cryptographic hash chain", or "fold events into state", they must call the *same*
  code, not two copies of the same math.
- **A dead canonical primitive is a violation, not a neutral.** If the architecture doc names an
  operator as *the* unifying primitive (`csr::laplacian_spmv`) and nothing calls it, the claimed
  correspondence exists only in prose — the "above" and the "below" are wired to *different* code.
- **Justified divergence must be gated.** Two implementations of one math object are acceptable
  only when (a) a stated reason requires it (perf, representation, wasm/API compat) and (b) a
  parity test binds their outputs. Divergence without a parity gate is a latent drift bug of the
  same class as a duplicated dependency version.

This principle is *already latent in the codebase's own vocabulary*: `bebop2/core/src/field.rs:26`
declares `EIGEN_AUTHORITY = "linalg::eigenvalues"` — "the SINGLE authoritative eigensolver every
spectral consumer must route through"; `kernel/src/mat.rs` calls itself "the ONE backing store and
the ONE matmul". The repo already *wants* Correspondence; this audit measures how far the code has
actually gotten there.

---

## 2. The two hypothesis instances, tested against real code

### 2a. "ONE Laplacian `L` across FIVE subsystems" (`physics-ui-capture-blueprint.md §1`) — **REFUTED at the code level; TRUE only as design.**

The blueprint's thesis: a *single* GPU Laplacian-SpMV kernel serves memory-recall, salience-decay,
UI-layout, UI-motion, and UI-blur, and it identifies that kernel as `csr::laplacian_spmv`
(`physics-ui-capture-blueprint.md §1`; R-LM §1.3 pins it to `csr.rs:307-359` as "the exact `y = L·x`
per-frame operator that `physics-ui-capture §1` unifies … around").

What the code actually shows:

- **`csr::laplacian_spmv` has ZERO production callers.** Every reference in `kernel/src/` is either
  its own definition (`csr.rs:307`) or a unit test (`csr.rs:695,709,752,778`). No UI, no retrieval,
  no decay, no layout path calls it. The "one operator serving five subsystems" is served, today,
  to *no* subsystem.
- **The Laplacian that is actually wired is a different, non-shared one.** `spectral::laplacian`
  (`spectral.rs:287`) materializes `L = D − A` as a **dense `Vec<Vec<f64>>`**, and *that* is what
  the live drift-gate consumes: `hydra::candidate_drift` → `spectral::classify_drift`
  (`hydra.rs:56-60`, `hydra.rs:22`), and the wasm surface exposes the dense path
  (`spectral_*_js`, R-LM §1.6). So the "canonical" `L` and the "live" `L` are two separate
  implementations of `L = D − A`.
- **The UI engine cannot call either.** `engine/` is pure-CPU, zero-dep; `bridge.rs`'s
  `VertexBridge` "only *counts* a hypothetical call — it never reaches a real GPU buffer"
  (`physics-ui-capture §2`, the FE-01 gap). No `wgpu`/`wgsl` exists in either repo
  (`physics-ui-capture §2`, verified). Blur-as-heat-kernel `e^{−tL}` is unimplemented.

Counting across both repos, `L = D − A` is implemented **at least four independent times**:
`csr::laplacian_spmv` (sparse, matrix-free, dead); `spectral::laplacian` (dense, live);
`bebop2/core/field.rs:82` (dense `L` built for Jacobi, "never stored long-term"); and
`crates/bebop/field_physics.rs::step_wave`, the `MÜ+ΓU̇+c²LU=S` operator (`physics-ui-capture §2`).
**Verdict:** the *math* is genuinely one operator; the *code* is four. The unification is a
truthful design north-star, not a realized correspondence. This is the same aspirational-vs-wired
split Principle 1 audits — here it falsifies the flagship "as above, so below" claim.

### 2b. R-LM 3-tier "same rendering primitive over a different graph" (mesh/hub/node) — **REFUTED as built; the design is honest that it is not yet buildable.**

R-LM §2 asserts "all three tiers are the SAME rendering primitive over a DIFFERENT graph … One
`Graph → Laplacian L → { spectral-embedding = positions, PPR-diffusion = activity, graph_spectrum =
health }` pipeline". Against code:

- **The three tiers name three DIFFERENT graph representations.** MESH = `hydra::topology_adjacency`
  → dense `Vec<Vec<f64>>` (`hydra.rs:41-49`). HUB is *described* as `Csr` (R-LM §2 table) but the
  fixture it points at, `retrieval/diffusion.rs`, builds a **dense** row-stochastic matrix
  (`wiki_row_stochastic() -> Vec<Vec<f64>>`, `diffusion.rs:109-124`) and runs the **dense**
  `Ppr` engine (`wiki_ppr()` → `Ppr::new(...)`, `diffusion.rs:126-129`; `Ppr { w: Vec<Vec<f64>> }`,
  `ppr.rs:20-22`) — **not** `csr::personalized_pagerank`. NODE = an ego-graph (not yet built).
- **The "positions from `L`" step does not exist.** R-LM §1.4 flags it in the open: `spectral.rs`
  computes eigen*values* only (`charpoly`/Householder, `spectral.rs:113-214`); "there is **no
  eigen*vector* / spectral-embedding / coordinate function** in the kernel today." Positions are the
  load-bearing output of the shared pipeline, and the primitive that produces them is missing.

So the *design* explicitly wants Correspondence ("zooming does not switch renderers, it re-seeds the
operator", R-LM §2) and even labels itself "DESIGN ONLY — no code was written" (R-LM header). As
built, the three tiers would use dense-adjacency + dense-PPR + a not-yet-existing embedding — three
representations of the claimed-single pipeline. **Verdict:** the fractal "a hub is a neuron in the
mesh; a record is a neuron in the hub" (R-LM §4.3) is a conceptual analogy today, not shared
structural code. It is buildable and honestly scoped (FE-12 / RW-01), but it is not the state of the
tree.

---

## 3. Audit findings (same concept, divergent code)

### F-1 — The graph Laplacian `L` is 4 implementations; the "canonical" one is dead. **Severity: MEDIUM (violation).**
Evidence in §2a. The live drift-gate `L` (`spectral.rs:287`, dense) and bebop's spectral `L`
(`bebop2/core/field.rs:82`, dense) are *two live* numeric Laplacians in two repos that must agree on
the same graph but share no code — a genuine drift surface. `csr::laplacian_spmv` (`csr.rs:307`)
being dead is not a drift risk *today* (nothing can drift from an uncalled function), but it is a
correspondence violation of the documentation kind: the architecture's headline unification points
at code no path executes. **Classification: violation** — the concept ("the Laplacian") is solved by
non-shared code, and the one designated to be shared is bypassed.

### F-2 — Personalized-PageRank / power-iteration is implemented 3×; the design points at the bypassed one. **Severity: MEDIUM (violation).**
- `csr::personalized_pagerank` — CSR, "fixed K, fixed summation order, bit-reproducible"
  (`csr.rs:228-264`) — called only from `evals.rs:281` and its own tests. Not on any live recall path.
- `retrieval/ppr.rs::Ppr::rank` — **dense** `Vec<Vec<f64>>` power-iteration (`ppr.rs:42`) — the one
  actually driving "what relates to X" (`diffusion.rs:126-135`).
- `markov.rs::analyze` — a third damped-stationary power-iteration on an empirical transition matrix
  (`markov.rs:81`, "power iteration" `markov.rs:123`).
Two of these (CSR PPR and dense `Ppr`) compute the *same* personalized-PageRank on the *same class*
of small relatedness graph, in one kernel, with no shared core and no parity test binding them — the
textbook drift pair. R-LM §1.3/§3.1 tells the viz to use `csr::personalized_pagerank`, while the
fixture it cites runs the dense `Ppr`. **Classification: violation** (live-vs-live duplication).

### F-3 — Graph adjacency is represented ≥5 ways. **Severity: LOW–MEDIUM (mostly JUSTIFIED divergence, with a real conversion-drift edge).**
One concept ("a graph"), these structures: CSR `{row_ptr,col_idx,val}` (`csr.rs:45-54`); dense
`Vec<Vec<f64>>` (`spectral.rs`, `hydra.rs:41`, `ppr.rs:22`, `absorbing.rs:45`); `Vec<Vec<usize>>`
adjacency list (`harmonic.rs:31`); `CGraph { parents, bidirected }` (`cgraph.rs:37-42`). **This is
largely a justified divergence**: BFS centrality genuinely wants an adjacency list (`harmonic.rs:17-18`
documents the choice), do-calculus genuinely wants separated directed/bidirected edges
(`cgraph.rs:29-42`), dense eigensolve wants row-major, sparse SpMV wants CSR. Forcing one
representation would *harm* correctness/perf — Correspondence does **not** demand it here. The residual
violation is narrower: **`from_edges` edge→structure conversion is hand-rolled in each module**
(`csr.rs` `from_edges`, `field.rs:52`, `diffusion.rs:111`, `harmonic.rs:31-36`) with no single
conversion hub, so an edge-parsing rule (self-loops, dedup, direction) can be applied inconsistently
across them. `kernel/src/mat.rs` is the codebase's *own* partial repair — it declares itself "the ONE
backing store and the ONE matmul" (`mat.rs:6-8`) but concedes "the `&[Vec<f64>]` entry points in
`spectral`/`absorbing` … stay for wasm/API compat" (`mat.rs:8-9`): a **documented, half-finished
consolidation**. **Classification: justified divergence in the representations, LOW-MEDIUM drift risk
in the un-shared conversion + the still-live `Vec<Vec<f64>>` API seam.**

### F-4 — The cryptographic hash chain IS a genuine, enforced correspondence. **Severity: none — this is the model to copy.**
Two append-only, tamper-evident chains at different scales — the event substrate
(`event_log.rs` `MeshEvent` chain, `event_log.rs:257-273`) and the knowledge spine
(`spine.rs` `SpineRecord` chain, `spine.rs:144-149`) — **share one hashing primitive**: `spine.rs`
computes `record_hash`/`payload_hash` via `crate::event_log::sha3_256` (`spine.rs:92`, `spine.rs:117`,
comment `spine.rs:12`). There is exactly **one** `sha3_256` definition in the kernel
(`event_log.rs:30`; the only other hits are its own KATs). This is Correspondence realized: the same
FIPS-202 Keccak and the same `prev`-binding discipline recur at the event scale and the record scale
from one implementation. **Classification: confirmed correspondence.**

### F-5 — The eigensolver: divergence that IS pinned by parity (the right pattern), with one residual gap. **Severity: LOW (justified + gated).**
dowiz has two eigen paths behind **one dispatch entry point**: `spectral::eigenvalues`
(`spectral.rs:195`) routes `n≤32` to `householder::eigenvalues_contig` (`spectral.rs:204`,
`householder.rs:338`) and larger/exact cases to `charpoly`+`roots` (Faddeev–LeVerrier + Durand–Kerner),
and a **parity test** asserts the two agree ("eig mismatch: householder vs faddeev",
`householder.rs:386`). bebop goes further: `EIGEN_AUTHORITY` sentinel (`field.rs:29`, `energy.rs:31`),
a dedicated `tests/eigensolver_parity.rs` cross-checking the solvers (`eigensolver_parity.rs:7,184`),
and `dmd` deliberately routed onto `field::jacobi_eigen` "to avoid a second Jacobi fork"
(`field.rs:308`; `dmd.rs:76`). This is exactly the "divergence forced by a stated reason
(eigenvalues vs eigenvectors, small vs large) + parity gate" the principle allows. **Residual gap:**
`jacobi_eigen` is the *sole* source of eigen*vectors*, and the parity harness binds eigen*values*
only (`dmd.rs:82-107` cross-checks values; vectors are validated by `A·v=λ·v` residual, not against a
second solver) — so the eigenvector output has no independent second party. This is the
"3-eigensolver dual-authority" item from project memory: acknowledged and mostly gated, not yet
fully closed. **Classification: justified + gated divergence; minor un-pinned eigenvector edge.**

### F-6 — Event-sourcing / fold-replay corresponds at the `decide` seam but NOT at the `fold` reducer. **Severity: LOW–MEDIUM (partial correspondence).**
The `decide` boundary genuinely corresponds: `event_log::commit_after_decide` takes `decide` as a
**generic parameter** `D` (`event_log.rs:300-319`), so every stateful commit shares one "validate
before persist" seam, and `order_machine`/`intake` both speak the "decide/fold Law" (`order_machine.rs:1`,
`intake.rs:47`). But the **fold reducer is per-subsystem, not one primitive**: `order_machine::fold_transitions`
folds a transition sequence into an `OrderStatus` and is "the deterministic reducer the WS event bus
replays against" (`order_machine.rs:137-151`); `intake.rs` has its own decide-fold (`intake.rs:47`);
and `event_log`/`spine` **never fold events into a projected state at all** — they are append + verify
chains (`event_log.rs:257`, `spine.rs:199-211` `verify_chain`). So "event-sourcing = fold-replay
everywhere" overstates the sharing: the order machine folds, the event log accretes. Part of this is
justified (an FSM state-fold and a hash-chain append are genuinely different operations); part is a
real gap (no shared replay/projection primitive, so a second subsystem needing state-reconstruction
will hand-roll a third fold). **Classification: partial correspondence — genuine at `decide`,
divergent at `fold`.**

### F-7 — "Hub" (dowiz `hydra.rs`) and "mesh-node" (bebop2) share NO topology code. **Severity: LOW (a GAP, not a drift).**
The MESH-tier graph the R-LM design attributes to the mesh lives in the **dowiz kernel**
(`hydra::topology_adjacency`, dense `Vec<Vec<f64>>`, `hydra.rs:41`). The actual **bebop2/mesh-node**
crate has **no graph/topology primitive** — it is `SignedFrame` gossip + DoD admission
(`mesh-node/src/node.rs:69` `admit_inbound`, `MeshEventSink`), zero adjacency. M7's "mesh heals via
Dijkstra/A* + Union-Find/MST" (`ARCHITECTURE.md M7`, F45/F46) is **unimplemented** — grep found no
`Dijkstra`/`UnionFind`/`mst` in `mesh-node` (only `crates/bebop/cost_estimate.rs` + a design doc).
So "a hub is a neuron in the mesh" is, structurally, two unrelated modules: a hub has a dense
adjacency graph; a mesh-node has none. **Classification: correspondence gap** (you cannot have
drifted from what is not built), but it means the fractal claim is conceptual until M7 lands and
until hub and mesh graphs share a representation.

---

## 4. Verdict

**Correspondence is the principle this codebase most *aspires* to and least *implements*.** The repo
has internalized the idea — `EIGEN_AUTHORITY`, `mat.rs`'s "ONE backing store", the "ONE Laplacian",
the "same primitive at three scales" — which is why the violations are measurable against the
project's own stated intent rather than an outside standard.

- **Where it holds, it is excellent and worth copying:** the shared `sha3_256` across the event and
  spine hash chains (F-4), and the parity-gated eigensolver divergence (F-5), are exactly the
  principle in force — same primitive at two scales, or forced divergence pinned by a parity test.
- **Where it fails, the failure is uniform in shape:** a math object (Laplacian `L`, PageRank) is
  implemented several times, and — the sharper tell — the copy the architecture *names as canonical*
  is the dead or bypassed one (`csr::laplacian_spmv` uncalled, F-1; `csr::personalized_pagerank`
  bypassed by the dense `Ppr` the design's own fixture runs, F-2). The two flagship "as above, so
  below" claims (the ONE-`L` UI thesis, the 3-tier fractal) are **design-true but code-false today**,
  and both docs are honest that they are design (R-LM header; the FE-01/FE-12 gaps).
- **Not every divergence is a sin:** the ≥5 graph representations (F-3) are mostly a *justified*
  divergence — different algorithms legitimately need different structures, and forcing one would be
  the wrong reading of the principle. The real, actionable drift risk there is narrow: hand-rolled
  `from_edges` conversions with no single hub, and the still-live `Vec<Vec<f64>>` API seam `mat.rs`
  set out to retire but only half-retired.

**Highest-value corrections, in priority order:** (1) bind the two *live* dense Laplacians
(`spectral::laplacian` and `bebop2/core/field.rs`) with a cross-repo parity test, or route both
through one vendored operator — this is a live-vs-live drift pair, unlike the dead `csr` one; (2)
collapse the CSR-vs-dense PPR duplication (F-2) to one engine or parity-gate it, and fix the R-LM
design to point at whichever survives; (3) treat the ONE-`L` and 3-tier fractal claims as
**explicitly aspirational** in `ARCHITECTURE.md` until `csr::laplacian_spmv` has a live caller and the
spectral-embedding primitive (R-LM §1.4 gap) exists — otherwise the canon asserts a correspondence
the tree does not contain.
