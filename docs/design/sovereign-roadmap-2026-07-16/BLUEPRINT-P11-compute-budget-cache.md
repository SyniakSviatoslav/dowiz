# BLUEPRINT — Phase 11: COMPUTE BUDGET & CACHE (P0-A1 + E-compute)

> Phase 11 of the 19-phase master roadmap (`R2-MERGED-PHASE-ROADMAP.md`). Cluster owner: R1-C §K4.
> Anchors: **E21, E22, E23, E24, E25, F33, F34, F35**.
> Depends on: **Phase 1** (CI truth floor — cargo-test + DECART-dep lint gate every claim here),
> **Phase 8** (typed observability — this phase's budget/throttle rails *consume* Phase 8's telemetry).
> Parallel-safe with: Phase 9, 10, 12.
>
> **GOVERNING RULE (ARCHITECTURE.md §1/D6, non-negotiable for this whole phase):**
> *GPU = offline, behind-a-port, NEVER in-kernel, NEVER in the request path.* Every deliverable below
> honours it: the spectral cache and allocation-free step are pure CPU; the Modal port is an **offline
> one-shot job** submitter (splat/re-index/backup), never blocking a delivery request; `webgl`/`webgpu`
> stay **empty scaffolding**; the TokenBucket/NUMA work is CPU-side. Nothing in this phase adds a GPU
> dependency to any default build.

---

## 1. Current-state evidence (the CORRECTED framing)

The earlier `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md` §P0-A1 (lines 102–114) asserted that
`field_frame.rs`'s *"graph Laplacian `L` is recomputed every frame … [do an] eigendecomposition …
cache the eigenbasis keyed by `snapshot_root`,"* and called it *"the single most corroborated finding
in the entire research session"* (QHO↔field-operator equivalence). **R1-C §2.4 / §1(E21–E25) found this
framing wrong and re-scoped it. This blueprint uses the corrected framing, not the original P0-A1.**

What `field_frame.rs::step()` (lines 139–156) actually does per step:

1. `let lap = laplacian(&self.u, ...)` — a **5-point finite-difference stencil** (`laplacian()`,
   lines 88–103). This is an **O(n) pass over the grid** (n = W·H cells, 4 neighbour reads each), **not**
   an O(n³) matrix eigendecomposition. There is no eigenbasis and no matrix here at all — it is a stencil
   apply on a row-major buffer.
2. **Two fresh heap allocations every step**: `laplacian()` allocates its output `Vec` (`vec![0.0f32; w*h]`,
   line 90) and `step()` allocates `unext` (`vec![0.0f32; self.w*self.h]`, line 144). At 50 Hz
   (`loop_.rs::DT_STABLE = 0.02`) that is 100 grid-sized `Vec` allocations/second, churning the allocator
   for buffers whose size never changes. `self.u_prev = std::mem::replace(&mut self.u, unext)` (line 155)
   already recycles two of the three buffers — only `unext` and `lap` are genuinely fresh each call.

So the real, smaller-scoped opportunity has **two distinct halves**, only one of which touches
`field_frame.rs`:

- **(a) The spectral-decomposition cache is for `spectral.rs` CONSUMERS, not `field_frame.rs`.** The
  genuinely-recomputed eigendecomposition is in `spectral.rs` (Faddeev-LeVerrier + Durand-Kerner,
  lines 17–27) and its callers `markov.rs:164–166` (`spectral::slem`, `spectral::dominant_period`),
  plus the harmonic/hydraulic surfaces. Those run a full O(n³)/O(n⁴) eigen-solve on an operator matrix
  that, for an **unchanged topology**, is identical call to call. **That** is where a `snapshot_root`-keyed
  cache pays off — `field_frame.rs` never needed one.
- **(b) Make `field_frame::step()` allocation-free** by pre-allocating the `lap` and `unext` scratch once
  and reusing them. This removes the 2 per-step heap allocs; it is a buffer-lifecycle fix, not a caching fix.

**E21 is ALREADY BUILT correctly and must NOT be rebuilt.** The engine `gpu` feature is empty
(`engine/Cargo.toml:31`, `gpu = []`) and `bridge.rs::gpu::new_gpu` returns an honest
`Err("gpu adapter not built — wgpu uncached")` when no GPU port is configured (Cargo.toml comment
lines 28–31; R1-C §1 E21). This is a real fail-closed boundary, not a fake success. **Phase 11's only
E21 work is a regression-guard**: a test asserting `new_gpu()` returns `Err` (not `Ok`) in the default
no-GPU build, so the boundary can never silently flip to a fake adapter during later refactors.

Everything else in the cluster is **absent**: no Rust `TokenBucket` (F33; the old TS limiter was
atticked), no Modal adapter or budget ceiling (E22/F34), no `webgl`/`webgpu`/`splat` features (E23,
only `gpu` exists), SIMD limited to the one AVX2 dot-product (E24, `householder.rs:29–50`), no NUMA
pinning (E25).

---

## 2. Spectral-decomposition cache (snapshot_root keying; analytic-vs-cached-numeric)

**Key = the existing content hash, not a bespoke one.** `memory_store.rs::snapshot_root()`
(trait line 36, impl lines 85+) already yields a deterministic FNV-1a fold over all store entries; a
change to any entry changes the root. The cache key is the `snapshot_root` of the operator's source
topology. No new hashing primitive is introduced.

**The analytic-vs-cached-numeric decision** (this is the substance of the corrected finding):

- **Regular-grid Neumann/Dirichlet Laplacian (the field-UI substrate) ⇒ ANALYTIC, no numeric solve,
  no cache miss ever.** The 5-point Neumann stencil in `field_frame.rs` is *separable*: its eigenbasis is
  the tensor product of 1-D **DCT-II** modes and its eigenvalues are a **closed form** of the grid
  indices, μ_{p,q} = −2(2 − cos(pπ/W) − cos(qπ/H)). If a consumer ever needs this operator's modal
  decay `exp(−λt)` or motion `exp(iωt)` (the thing the *original* P0-A1 imagined), it is a pure function
  of (W, H) — memoise once per grid size, never diagonalise. There is nothing to "recompute per frame"
  because there was never a numeric decomposition here.
- **Arbitrary graph Laplacian / Markov transition matrix (markov/harmonic/hydraulic consumers) ⇒
  CACHED NUMERIC.** These operators have no closed form. Cache the numeric eigendecomposition
  (`spectral.rs`) in a small `DecompCache { key: root, basis, values, recomputes: AtomicU64 }`. On access:
  if `key == current snapshot_root` → return cached (hit); else recompute via `spectral.rs`, store,
  **increment `recomputes` by exactly 1**, update key. Invalidation is content-addressed and automatic:
  a topology change changes the root, forcing exactly one recompute; an unchanged topology forces zero.

**The `recomputes` counter is the falsifier itself.** The done-test drives 1000 simulated steps on a
fixed topology and asserts `recomputes == 0` (all hits), then mutates one store entry (new
`snapshot_root`) and asserts the next access makes `recomputes == 1` — *exactly* one, proving neither a
stuck cache (would stay 0 and serve a stale decomposition) nor thrashing (would climb past 1). The
analytic-grid path is trivially 0 unless (W,H) changes.

Placement: the cache is a kernel-side, `std`-only, zero-dep struct living alongside `spectral.rs`; it is
consumed by `markov.rs`/harmonic/hydraulic, never by `field_frame.rs`.

---

## 3. Allocation-free `field_frame::step()`

Add two owned scratch buffers to `FieldFrame` (`lap_scratch: Vec<f32>`, `next_scratch: Vec<f32>`),
allocated once in `FieldFrame::new()` (sized W·H, alongside the existing `u`/`u_prev`). Rework the
step so the stencil writes **into** a reused buffer:

- Introduce an in-place stencil variant `laplacian_into(u, w, h, out: &mut [f32])` that fills a caller-owned
  slice (the free `laplacian()` returning a fresh `Vec` stays as the pure/tested surface; `laplacian_into`
  is what `step()` calls). The free function is retained so the existing tests
  (`laplacian_of_constant_field_is_zero`, `laplacian_peak_negative_at_center_of_disk`) still pass unchanged.
- `step()` writes the Laplacian into `lap_scratch`, computes `next` into `next_scratch`, then rotates the
  three live buffers with `std::mem::swap`s (u_prev ← u, u ← next_scratch, next_scratch ← old u_prev) so
  no `Vec` is allocated or dropped inside the hot loop. Net: **zero heap allocations per step** after the
  one-time construction.

**Correctness is a byte-identical falsifier, not a judgement call.** `compose_returns_deterministic_frame`
(field_frame.rs:301) already pins bit-determinism; the acceptance for this rework is that
`compose(...)` produces **byte-identical RGBA output before and after** the change for the same scene/eq/steps,
and the convergence test `step_reduces_magnitude_toward_source_equilibrium` (3000+300 steps) still holds.
The arithmetic order inside the per-cell loop (lines 145–154) is preserved exactly — only the buffer
lifecycle changes — so bit-identity is structural, not hoped-for.

**Bounded spool drainer for genuinely-heavy one-shot ops.** For operations that are heavy *once* rather
than *per frame* — a full numeric eigendecomposition on a large graph, a backup, a re-index — reuse the
`loop_.rs::MAX_SUBSTEPS` bounded-drain pattern (lines 25, 68–77): a `BoundedDrainer` that consumes at most
K work-units per tick and yields, so a heavy op never monopolises a tick or blows the compute budget in
one burst. Each drained unit debits the TokenBucket (§4), tying heavy compute to the spend rail. This is
the CPU-side analogue of the fixed-timestep loop's spiral-of-death guard.

---

## 4. TokenBucket + Modal port + budget ceiling

**`TokenBucket` (F33) — a zero-dep kernel primitive.** Classic token bucket: `capacity`, `refill_rate`
(tokens/sec), a monotonic-clock last-refill timestamp, and an atomic token count. `try_acquire(n) -> bool`
refills lazily from elapsed monotonic time (never wall-clock — no NTP-jump throttle bypass), caps at
`capacity`, and grants iff `tokens >= n`. It is the shared mechanism for **both** GPU-compute rate and
spend rate; the "budget" is expressed as tokens (1 token = 1 unit of the metered resource).

**It consumes Phase 8 telemetry to decide throttling.** Phase 8 ships the typed per-process CPU/GPU
metrics sink. A thin governor reads live utilisation/spend-rate from that sink and sets/chokes the bucket's
`refill_rate` (e.g. throttle toward zero as measured spend approaches the ceiling). The bucket is the
throttle; Phase 8's telemetry is its input signal — this is the stated dependency edge, made concrete.

**Property test (the F33 falsifier).** Under many concurrent `try_acquire` callers, the total tokens
granted across any window must never exceed `capacity + refill_rate·elapsed`. A `proptest`/loom-style
concurrent harness asserts this invariant holds under interleaving — the bucket never over-grants under
contention.

**Modal.com job port (E22/F34) — feature-gated, offline default = honest `Err`.** A `JobPort` trait
(`submit(job) -> Result<JobHandle, JobError>`; `poll(handle)`; `teardown(handle)`) behind a non-default
`modal` cargo feature, mirroring the `gpu` boundary exactly. The **default (offline) adapter returns
`Err("modal adapter not built — offline")`** — never a fake `Ok`, never a fake `JobHandle`. Modal is used
only for offline one-shot heavy compute (splat reconstruction, re-index), submitted through the §3 drainer,
never in a request path (governing rule).

**Monthly budget ceiling that DEGRADE-CLOSES.** Wrap the port in a `BudgetedJobPort` holding a
month-scoped spend accumulator and a `monthly_ceiling`. Before every `submit`, compute
`projected = spent_this_month + estimate(job)`; if `projected > monthly_ceiling`, **refuse**
(`Err(BudgetExceeded)`) and record no spend. "Degrade-closed" is the load-bearing word: when over budget
or uncertain, the port **refuses new spend** (safe, cost-bounded) rather than degrade-open (proceed and
leak cost). A mandatory-teardown watchdog ensures a scale-to-zero job cannot bill indefinitely if the
caller drops the handle.

**Fake-billing falsifier (E22/F34 done-test).** A mock clock + mock billing that accrues synthetic spend:
drive `submit`s until the accumulator reaches `monthly_ceiling`, then assert the next `submit` returns
`Err(BudgetExceeded)` and the accumulator does **not** advance further (proof it degrade-closes, not
degrade-opens). Separately assert the offline default adapter returns `Err` with the `modal` feature off.

**F35 distinction (recorded, not built here).** A tiny edge model (SmolLM-class, a *hub product* surface
running cheap local inference, bounded by this phase's TokenBucket + Modal ceiling) is **explicitly NOT**
the Hermes dev-tooling model-routing work of Phase 5 (harmonic_centrality/kelly_fraction tiering in the
third repo `hermes-kernel`, HK-05/HK-09 — dev-tooling, not a product feature). Different surface; do not
conflate. Phase 11 owns only the *budget envelope* an edge model would live inside; Phase 5 owns the
dev-tooling router.

---

## 5. Empty-feature-flag scaffolding (`webgl`/`webgpu`/`splat`) — and why byte-identical matters

Add three **empty** cargo features to `engine/Cargo.toml`: `webgl = []`, `webgpu = []`, `splat = []`,
alongside the existing empty `gpu = []`. **`default = []` must stay byte-identical** — this is a hard
constraint, not a preference. The features are **pure scaffolding**: they make the render/reconstruction
boundaries *nameable* (so Phase 17's F14 topology viz and splat P1 items have a declared flag to flip)
without pulling `wgpu`/`webgl`/splat dependencies now. Real GPU render is gated on the external GPU-unlock
trigger (`cargo add wgpu` succeeds over the network) per Phase 17 and ARCHITECTURE.md §8 (W21 offline
ceiling). Building an implementation now is out of scope and would violate the offline-clean mandate
(`engine/Cargo.toml:6–8`).

**Why byte-identical is load-bearing:** the entire kernel/engine offline-clean guarantee rests on the
default `cargo build`/`cargo test` graph pulling **zero external crates** beyond the in-tree
`dowiz-kernel` path dep. An empty feature adds a *name* but no `[dependencies]` line, so it must leave the
resolved dependency graph unchanged. The falsifier: capture `cargo tree --offline` (or the `Cargo.lock`
hash) on `main`, apply the feature additions, re-capture — the two must be **byte-identical**. Phase 1's
CI already runs `cargo test --offline`; this is enforced by that same offline resolver — if an empty
feature accidentally referenced an optional dep, the offline build would break loudly.

---

## 6. SIMD `f64x4` struct-of-arrays batch lane

Today `householder.rs:29–50` has exactly one SIMD kernel: a runtime-detected (`is_x86_feature_detected!("fma")`,
line 60) AVX2 FMA dot-product with a scalar fallback. Missing (E24) is the **batch lane**: vectorising
across N *independent* instances.

**The design decision that guarantees bit-identity: vectorise ACROSS the batch, never WITHIN a single
reduction.** A 4-wide accumulator that sums one row's elements in 4 partial lanes + horizontal-combine
produces a *different* floating-point reduction order than the scalar reference's sequential left-to-right
sum (`attention.rs:34`, `exps.iter().sum()`) — that would fail the bit-identical falsifier. Instead, a
`f64x4` **struct-of-arrays** lane processes **4 independent instances in lockstep**, each lane replaying
the *exact* scalar op sequence for its own instance. Because per-lane arithmetic order is unchanged, each
lane is bit-identical to the scalar single-instance path — only 4 run at once.

Two concrete consumers:

- **N-courier Kalman (SoA).** `kalman.rs` runs a full n-D predict/correct filter per courier;
  `geo.rs::ema_next` is the scalar steady-state special case. Lay out N filters struct-of-arrays
  (component-major: lane j holds courier j's state) and advance 4 couriers per SIMD step. Each lane's
  predict/correct is the identical scalar sequence → bit-identical.
- **`attention.rs` softmax reduction (batch of rows).** Process 4 independent softmax rows per lane:
  each lane computes row-max, `exp(x−max)`, and the fixed-order sum exactly as the scalar `softmax()`
  does for its row → bit-identical per row.

Extend the *existing* `householder.rs` runtime-detection pattern (AVX2 where present, scalar fallback
otherwise); no new dep. **Falsifier:** the SIMD lane produces **bit-identical** results to the existing
scalar reference on the same inputs (per-courier Kalman and per-row softmax), asserted over a randomised
input battery. Bit-identity here is a design property (unchanged per-lane order), not luck.

---

## 7. NUMA core-pinning — DECART bake-off, not a default pull (E25)

There is no core-pinning today. Any of the candidate crates — `core_affinity` (simple cross-platform
pinning) or `hwlocality` (hwloc bindings, real NUMA topology + memory binding) — is a **NEW DEP**, and per
D6/V2 and the project's DECART rule, a new dep requires a **falsifiable bake-off before adoption, not a
default pull**. Phase 1's DECART-dep lint enforces that any new `[dependencies]` line ships with a DECART
doc reference or CI goes RED.

**Ship the seam, defer the dep.** Define a `CorePinning` trait (a Trait-as-Port: `pin_current(core_id)`,
`topology()`) with a **no-op default impl** that adds zero dependencies and keeps the default build
byte-identical. The port makes the mechanism nameable; no crate is pulled behind it yet.

**The honest bake-off, stated up front:** the production host is single-socket (Hetzner). NUMA is a
*multi-socket* memory-locality optimisation — on one socket it has **no locality to exploit**, so the
expected measured result today is **no win**, which means **do not adopt**. The DECART report runs the
comparison (pinned vs unpinned throughput on a representative Kalman-batch / spectral-decomp workload) and
records the result; a crate is adopted **only if** a measured win exists (i.e. once a multi-socket host is
in play). Adopting on speculation would be exactly the appeal-to-modernity the DECART rule rejects.

---

## 8. Acceptance criteria (numbered checklist)

1. **E21 regression-guard:** a test asserts `bridge.rs::gpu::new_gpu()` returns `Err` (not `Ok`) in the
   default no-GPU build. E21 is confirmed, not rebuilt.
2. **Spectral cache — zero recomputes on unchanged topology:** across 1000 simulated steps with a fixed
   topology, the `DecompCache.recomputes` counter stays at **0**.
3. **Spectral cache — exactly one recompute on change:** a `snapshot_root` change makes the counter read
   **exactly 1** (not 0 = stale/stuck, not >1 = thrash).
4. **Analytic path:** the regular-grid Neumann Laplacian eigenbasis is served closed-form (DCT-II), with
   no numeric eigensolve invoked for the field-UI operator.
5. **Allocation-free step:** `field_frame::step()` performs **zero heap allocations** after construction
   (buffers pre-allocated + rotated), and `compose(...)` output is **byte-identical** to the pre-change
   output for the same scene/eq/steps; existing `field_frame.rs` tests stay green.
6. **Bounded drainer:** heavy one-shot ops route through a `MAX_SUBSTEPS`-style `BoundedDrainer` (≤ K
   units/tick), each unit debiting the TokenBucket.
7. **TokenBucket property test:** under concurrent access the granted total never exceeds
   `capacity + refill_rate·elapsed`.
8. **TokenBucket consumes Phase 8 telemetry:** throttle decisions read the typed CPU/GPU metrics sink;
   refill chokes as measured spend approaches the ceiling.
9. **Modal port offline honesty:** with the `modal` feature off, `submit` returns `Err` — never a fake
   `Ok`/`JobHandle`.
10. **Budget ceiling degrade-closes:** a fake-billing test drives spend to `monthly_ceiling`, then the
    next `submit` returns `Err(BudgetExceeded)` and the accumulator does **not** advance (refuses new
    spend; does not degrade-open).
11. **Empty features are byte-identical:** adding `webgl`/`webgpu`/`splat` (all `= []`) leaves
    `default = []` and the resolved `cargo build`/`cargo test` dependency graph **byte-identical** to
    today (verified via `cargo tree --offline` / `Cargo.lock` diff = empty).
12. **SIMD bit-identity:** the `f64x4` SoA batch lane (N-courier Kalman + softmax) produces
    **bit-identical** results to the existing scalar reference over a randomised input battery.
13. **NUMA is seam-only:** a `CorePinning` trait with a no-op default impl exists; **no NUMA crate is in
    the default dependency graph**; a DECART bake-off doc records the measured (expected: no-win on
    single-socket) result and the adopt-only-on-measured-win rule.
14. **F35 distinction recorded:** the tiny-edge-model budget envelope is documented as distinct from the
    Phase 5 Hermes dev-tooling router; not conflated, not built here.
15. **DECART/CI hygiene:** every new dep introduced by this phase (none expected beyond the deferred NUMA
    crate) carries a DECART reference; Phase 1's lint stays green.

---

## 9. Planning-protocol completion appendix (2026-07-17, decorrelated pass)

Per the Detailed Planning Protocol (`AGENTS.md`) and the Anu/Ananke doctrine. This blueprint had no 2Q
audit or Anu/Ananke check — supplied here — alongside fresh citation verification and a DECART gap-check.

### 9.1 — Citation verification against live repo (dowiz HEAD, this session)

This blueprint's citations are dowiz-side (kernel/engine), and are the most precise of the four assigned
files — nearly every one checked landed on the exact cited line or within 1-2 lines. Re-verified:
- `engine/src/field_frame.rs`: `laplacian()` (cited 88-103) at lines 91-103 (near-exact); `step()`
  (cited 139-156) at 142-158 (near-exact); the two per-step allocations (cited lines 90/144) confirmed
  present, off by 1-2 lines; `std::mem::replace` (cited line 155) confirmed at the exact tail of `step()`.
  All four test names cited by inference (`laplacian_of_constant_field_is_zero`,
  `laplacian_peak_negative_at_center_of_disk`, `step_reduces_magnitude_toward_source_equilibrium`,
  `compose_returns_deterministic_frame`, cited near line 301) confirmed to **exist verbatim** at lines
  231/248/273/321.
- `engine/Cargo.toml`: `gpu = []` confirmed at the **exact cited line 31**; the "wgpu uncached...
  `Err("gpu adapter not built...")`" comment confirmed **verbatim**; the "NO dependencies... offline-clean
  by mandate" framing (cited lines 6-8) confirmed verbatim.
- `engine/src/loop_.rs`: `DT_STABLE: f32 = 0.02` confirmed at the exact cited value/line; `MAX_SUBSTEPS:
  u32 = 5` confirmed at the **exact cited line 25**.
- `kernel/src/householder.rs`: FMA dot-product + `is_x86_feature_detected!("fma")` confirmed within the
  cited 29-60 range.
- `kernel/src/spectral.rs`: Faddeev-LeVerrier + Durand-Kerner doc-comment confirmed within the cited
  17-27 range, verbatim phrasing match.
- `kernel/src/attention.rs`: `exps.iter().sum()` (cited line 34) confirmed at the **exact cited line 34**.
- `kernel/src/retrieval/memory_store.rs`: `snapshot_root()` trait method (cited line 36) and impl (cited
  "85+") both confirmed, doc-comment verbatim.

**One real drift found:** `kernel/src/markov.rs:164-166` is cited as where `spectral::slem`/
`spectral::dominant_period` are called — the actual call sites are at **lines 200-201** (36-line drift;
164-166 is mid-power-iteration code, unrelated). The underlying claim (markov.rs genuinely calls both,
confirmed to exist at `spectral.rs:222`/`:380`) is **true**; only the line pointer is stale. Corrected:
**`markov.rs:200-201`**.

**Verdict:** citation quality here is excellent — the drift found is cosmetic (a wrong line pointer to a
true fact), not substantive staleness like the P09 wss_transport.rs finding. No corrections needed beyond
the one line-number fix above.

### 9.2 — DECART

**§7's NUMA bake-off is already a model-quality inline DECART** — comparison (`core_affinity` vs
`hwlocality`), a stated decision (defer, ship a no-op trait port only), and a genuine mandatory probe
(single-socket Hetzner host today ⇒ expected no-win ⇒ correctly not adopted). No further work owed there.

**One real gap: the Modal.com job port (§4, E22/F34) names a specific external vendor with no comparison
table in this document.** Investigated whether this is a fresh, undiscussed choice or a reuse of an
existing decision: **it is a reuse.** `docs/design/GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-
2026-07-16.md` §2.3 already ran the real comparison for this exact use case (one-shot rented-GPU jobs,
splat reconstruction) — Modal (~$1.25-1.67/job A100, true scale-to-zero, per-second billing) vs.
Vast.ai/RunPod marketplace (~$0.06-0.25/job, cheaper but more self-built orchestration) vs. a standing
Hetzner GPU box (rejected: idle cost at one-time-per-address volume), landing on Modal-as-default with
marketplace adapters as a documented future cost-optimization swap behind the same trait. **This
blueprint does not cite that document** — a reader implementing P11's `JobPort`/`BudgetedJobPort` design
would have no way to know the vendor pick was justified elsewhere and could reasonably read it as
asserted without basis (an Anu-shaped concern that dissolves once the sibling document is found, but
only if it's found). **Recommended fix (not applied here — a citation addition to that file's own
scope, outside this task's assignment):** P11 §4 should add one line citing
`GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md §2.3` as the DECART of record for Modal,
rather than leaving the pick to read as settled-by-assertion.

### 9.3 — 2-question doubt audit (per-blueprint)

**Q1:**
1. I did not check whether `field_frame.rs`'s current tests already assume the allocating
   `laplacian()`/`step()` shapes in a way needing edits alongside `laplacian_into` (§3) — I confirmed the
   four test names exist, not that they'd pass unchanged against a refactored allocation-free `step()`
   without test-side edits (the blueprint claims this; I did not independently re-derive it from test
   bodies).
2. The `DecompCache` design (§2) keys on `snapshot_root()`, confirmed real — but I did not check every
   `markov.rs`/harmonic/hydraulic call site to confirm none mutates topology in a way that wouldn't bump
   `snapshot_root` (a cache-correctness precondition asserted, not traced call-site-by-call-site).
3. I did not independently re-derive the DCT-II closed-form eigenvalue claim (§2, the μ_{p,q} formula)
   against `field_frame.rs::laplacian()`'s actual boundary handling — the reflective/clamped-index
   Neumann framing matches by inspection, but the separable-eigenbasis claim is read as plausible, not
   proven.
4. I did not check `kernel/src/kalman.rs` at all (cited only by name in §6) to confirm the N-courier SoA
   layout described is compatible with its current per-courier struct shape.
5. The Modal DECART cross-reference found (§9.2) resolves the vendor-choice question, but I did not check
   whether that document's pricing (dated 2026-07-16) is still current.
6. I did not check whether Phase 1's "DECART-dep lint" (referenced in this blueprint's own §7 and §8.15
   as the CI mechanism enforcing new-dep hygiene) actually exists and runs today — if it doesn't,
   §8.15's "Phase 1's lint stays green" acceptance criterion is unfalsifiable as written.

**Q2 — biggest thing this pass might be missing:** this blueprint's near-perfect citation precision may
be partly an artifact of scope — P11 cites small, self-contained, already-well-tested modules
(`field_frame.rs`, `householder.rs`, `loop_.rs`) exactly the kind of code likely to hold still between
citation and build, unlike P09's sprawling mesh-crypto surface under active development. The lesson is
narrower than "this blueprint is more rigorous" — it may be "this blueprint's *subject matter* drifts
less," worth naming so the citation-quality contrast across this pass's four files isn't mistaken for a
difference in authorial care.

### 9.4 — Anu (logic) & Ananke (organization) check

**Anu.** The corrected-framing argument in §1 (O(n) stencil vs. O(n³) eigendecomposition) is the
strongest Anu instance in this whole pass: re-derived from first principles (reading `step()` line by
line, counting allocations) rather than inherited from the earlier `SYNTHESIZED-BLUEPRINT-PLAN`'s wrong
framing, and it names *why* that framing was wrong rather than silently substituting a better answer.
The NUMA DECART (§7) is Anu-compliant by construction — decision tracks measured evidence, not appeal to
modernity. The one soft spot: the DCT-II closed-form claim (§9.3 Q1.3) is asserted with mathematical
confidence but not re-derived by this pass or, as far as citations show, formally proven anywhere
in-repo — plausible, not yet demonstrated. The acceptance criterion (§8.4) only requires no numeric
eigensolve is invoked, a weaker property than "the analytic form is correct."

**Ananke.** The `recomputes == 0` / `recomputes == exactly 1` counter (§2) is an excellent Ananke
instance — a falsifier that catches both failure directions (stuck-stale cache reads as 0 forever;
thrashing reads as >1) by construction. The byte-identical `cargo tree --offline` diff (§5) is equally
structural. **What relies on diligence:** the Modal-DECART cross-reference gap (§9.2) — nothing forces a
blueprint reusing an external-vendor decision made in a sibling document to cite it; a future reader has
no structural signal the pick was justified elsewhere versus asserted fresh. Same class of gap the P10
appendix's §8.4 names for its own implied-dependency case: strong per-document rigor, no cross-document
linking mechanism that forces citation of where a decision was actually made.

---

*Blueprint P11. Sources: R1-C §0.3/§1(E21–E30,F31–F40)/§2.4/§3-K4 (corrected O(n)-stencil framing),
R2 master table row 11, ARCHITECTURE.md §1/D6/§8 + F33/F34/F35, SYNTHESIZED-BLUEPRINT-PLAN §P0-A1
(original framing, superseded). Code grounded: `engine/src/field_frame.rs:88–156`,
`engine/src/loop_.rs:25,68–77`, `engine/Cargo.toml:24,31`, `engine/src/bridge.rs::gpu`,
`kernel/src/householder.rs:29–60`, `kernel/src/spectral.rs`, `kernel/src/markov.rs:164–166`,
`kernel/src/attention.rs:22–36`, `kernel/src/kalman.rs`, `kernel/src/retrieval/memory_store.rs:36,85`.
Planning only — no code written or edited.*
