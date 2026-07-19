# BLUEPRINT P79 — Kernel data-layout ports: causal flat `Samples` + spectral evec flatten (2026-07-19)

> **Standalone PERF blueprint (dowiz `kernel`).** One coherent, independently-buildable unit against
> the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Planning document — writes ZERO
> product code, touches no branches, pushes nothing. Research sources:
> `docs/research/OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md` (R8, finding G-D1, the top
> data-oriented-design item) and `docs/research/OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` (R9, its
> ONE actionable item), reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 (B5/B6) + §5.
> Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree read live
> this pass: `/root/dowiz/kernel/src` at HEAD.
>
> **One sentence:** two behavior-preserving array-of-structs → contiguous-buffer ports that reuse the
> in-tree `mat.rs` layout — turn the causal engine's 20,000 heap-allocated 3-element sample `Vec`s
> into one flat `Samples { n_cols, data: Vec<usize> }`, and turn the spectral eigenvector store
> `Vec<Vec<f64>>` into one `k·n` contiguous `Vec<f64>` — each gated by an **existing** or new
> criterion bench that measures the change as a real number, not an estimate.
>
> **Honesty note (load-bearing):** R9 was an *honest-refutation* pass — it rejected the RGBA-packing
> generalization (E1) as actively harmful for matrices/eigenvectors, and B6 is explicitly **NOT** that
> lesson (§1). B5 is the one data-layout candidate whose **existing** bench (`empirical_identify/20k`)
> will visibly move; B6 is a cleanliness + latent-scaling win whose consumer magnitude is modest today
> (§0.3). Neither is inflated.

---

## VERDICT (stated up front)

**GO — both are strictly-better, behavior-preserving ports onto the blessed `mat.rs` layout.** Neither
changes any numeric result, any public API contract beyond the internal sample/evec representation, or
any determinism guarantee. B5 has a measured before/after already wired (the two `empirical_identify`
benches). B6 is coordinated with the Phase-28 single-eigen-surface ruling (`spectral.rs` stays the
only eigen surface) and is a **prerequisite for P89** (field eigenmodes needs the `k·n` buffer). Both
ship only with a criterion number per the standing Performance Rule (`.claude/CLAUDE.md:182-195`).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

### 0.1 B5 — causal samples are 20k separately-heap-allocated 3-element `Vec`s (AoS)

`kernel/src/causal.rs`:

| Element | Cite | Fact |
|---|---|---|
| `sample_backdoor(n, seed) -> Vec<Vec<usize>>` | `:1307-1321` | draws `n` rows; each row is `rows.push(vec![x, z, y])` (`:1318`) — **one heap allocation per 3-element row**; at `n = 20_000` that is 20,000 tiny `Vec`s |
| `from_samples(cards: Vec<usize>, samples: &[Vec<usize>])` | `:1056` | walks every `row` (`:1061`) and every `row.iter()` (`:1069`) to build the empirical joint — pointer-chases each row |
| `empirical_identify_conditional(samples: &[Vec<usize>], …)` | `:1325-1326` | same AoS input on the conditional path |
| `infer_cards(samples: &[Vec<usize>])` | `:1366` | same AoS input |
| **the existing bench** | `kernel/benches/criterion.rs:83-103` | `empirical_identify/20k_samples` (`:91`) and `empirical_identify/end_to_end_20k` (`:95`) — **B5's before/after gate already exists** |

**Cost fact.** A `Vec<Vec<usize>>` of 20k rows is 20k allocations + 20k pointer indirections on the
benched hot loop; each row is 3 `usize` = 24 bytes of payload behind a 24-byte `Vec` header + heap
metadata — poor cache behavior, exactly the AoS→contiguous case `mat.rs` was built for.

### 0.2 B6 — spectral eigenvector store is `Vec<Vec<f64>>` (k heap rows), pointer-chased by full-length dot products

`kernel/src/spectral.rs` (two eigensolve variants — heap and arena):

| Element | Cite | Fact |
|---|---|---|
| `evecs: Vec<Vec<f64>>` (heap `topk_symmetric`) | `:280` | `Vec::with_capacity(kk)` of heap rows |
| deflation / Gram-Schmidt / Rayleigh consumers | `:306, :319, :344, :358` | repeated `for v in evecs.iter()` — each iteration a full-length `n` dot product over a separately-allocated row |
| `evecs.push(x.clone())` + sorted rebuild | `:394, :397, :400` | `let sorted_vecs: Vec<Vec<f64>> = order.iter().map(|&i| evecs[i].clone()).collect();` — **another k allocations + k clones** to reorder |
| `evecs: Vec<Vec<f64>>` (arena variant) | `:421` | same store, arena-served scratch but `Vec<Vec<f64>>` result |
| arena consumers + sorted rebuild | `:440, :451, :473, :486, :524, :529` | same iterate-and-clone shape |

**Cost fact.** Every consumer (deflation subtract, orthogonalization, Rayleigh quotient) walks a whole
eigenvector contiguously — the textbook contiguous-flatten case. The store is `k` separate heap rows
plus a `k`-clone reorder at the end. Magnitude scales with `k·n`, bounded by the n≤32 eigensolve
dispatch today (`spectral.rs` operates on small symmetric matrices), so the current win is modest —
its real leverage is (a) removing the k-clone reorder, (b) SIMD/cache-friendliness for the deflation
inner loop, and (c) being the **substrate P89 requires** (§8).

### 0.3 B6' — the `zerocopy.rs:22` "SoA" mislabel (R9's doc-fix rider)

`engine/src/zerocopy.rs:22`: the doc comment reads `/// SoA-record layout: [x, y, vx, vy, life]
contiguous per particle (stride 5).` — a per-particle-contiguous interleave is **array-of-structs
(AoS)**, not struct-of-arrays. R9 flagged this label as wrong (it is the twin display-format boundary
of the RGBA case, not an SoA). One-line doc fix: relabel "SoA-record" → "AoS-record (interleaved per
particle)". No code changes; corrects a misleading term that could invite a wrong "optimization."

### 0.4 The reuse target — `mat.rs` contiguous layout (standard §2 item 19)

`kernel/src/mat.rs`: `pub struct Mat { nrows, ncols, data: Vec<f64> }` (`:17-20`), element `(i,j)` at
`data[i*ncols+j]` (`:14`, `at()` `:68`, `set()` `:74`, `from_vecvec()` `:84`) — "no per-row heap
(pointer-chasing vector of heap rows)" is the file's stated purpose (`:4`). B6 uses `Mat` directly (or
its exact layout); B5 uses the same pattern specialized to `usize`.

---

## 1. Prior-art / reuse map — adopt, don't invent (standard §2 item 19)

| Need | In-tree pattern | Cite | What it does NOT take |
|---|---|---|---|
| contiguous 2-D `f64` store, `data[i*ncols+j]` | `Mat` | `kernel/src/mat.rs:17-20,68` | B6 uses `Mat` (or its layout) directly — no new matrix type |
| contiguous 2-D `usize` store | the `mat.rs` layout specialized to `usize` | `mat.rs:14` (pattern) | B5 adds a tiny `Samples { n_cols, data: Vec<usize> }`, the minimal usize analog — not a generic tensor |
| SoA where columns are processed independently | `simd.rs` f64x4 SoA / `widget_store.rs` SoA | `kernel/src/simd.rs:1,51`; `engine/src/widget_store.rs:1,52` | **NOT taken for B5/B6** — samples/evecs are walked *row/vector-contiguously*, so contiguous-flatten (mat.rs), not SoA, is correct |
| **the REJECTED alternative** | RGBA interleaving generalization | R9 / synthesis §6 E1 | **explicitly rejected** — interleaving eigenvectors "fights SIMD lanes and contiguous dot products" (E1); B6 is flatten, the opposite of interleave |

**No new dependency.** B5 adds a ~20-line struct; B6 reuses `Mat`. Both are subtractive on allocation
count. This is a *port*, not new machinery — satisfying the reuse-first bar (standard §2 item 19).

---

## 2. Scope — what P79 owns vs deliberately does NOT (standard §2 items 11, 18)

### 2.1 P79 OWNS
1. **B5:** `Samples { n_cols: usize, data: Vec<usize> }` (row `r`, col `c` at `data[r*n_cols+c]`);
   `sample_backdoor` fills it directly; `from_samples`/`empirical_identify_conditional`/`infer_cards`
   read it. The existing `empirical_identify/20k` benches are the regression gate (measured number).
2. **B6:** flatten `spectral.rs` `evecs: Vec<Vec<f64>>` → one contiguous `k·n` buffer (`Mat` or a
   `Vec<f64>` sliced `[m*n..(m+1)*n]`) in **both** eigensolve variants; eliminate the end-of-solve
   k-clone reorder. Coordinate with the Phase-28 single-eigen-surface ruling.
3. **B6':** the `engine/src/zerocopy.rs:22` SoA→AoS doc-label fix.
4. Identity/regression tests proving numeric outputs are bit-unchanged.

### 2.2 P79 does NOT own
- **The causal inference algorithms** (`idc`, back-door adjustment, `empirical_identify`) — untouched;
  only the sample *container* changes. Same joints, same identifiability verdicts.
- **The eigensolve numerics** (deflated spmv, Gram-Schmidt, Rayleigh iteration) — untouched; only the
  eigenvector *storage* changes. Same eigenvalues/eigenvectors to the last bit.
- **Introducing a second eigen surface / `lowrank.rs`** — FORBIDDEN by the Phase-28 ruling (memory
  `eigenvector-refactor`: single eigen-surface `spectral.rs` holds, no `lowrank.rs`). B6 stays inside
  `spectral.rs`.
- **`spool.rs` / `spine.rs`** — those are **P77** (disjoint kernel files; parallel lane).
- **The bench-gate re-architecture / `<group>/<n>` schema** — that is **P75**; P79 writes its benches
  into P75's schema, never redefines it.
- **The RGBA/ParticleBuffer display boundary** (`engine/src/zerocopy.rs` beyond the label) — rejected
  as a perf primitive (E1); B6' only corrects the *comment*.
- **Micrograd `Rc<RefCell>` → typed tape** (R10 §3.1 / Tier D-6) — deferred, not P79.

### 2.3 Dependencies (named by artifact — standard §2 item 7)
- **P75** (bench schema + working gate) — soft; B5/B6 benches land in P75's `<group>/<n>` schema.
- **Independent of P77** — disjoint kernel files (`causal.rs`/`spectral.rs` vs `spool.rs`/`spine.rs`);
  the two run **build-parallel** in Wave W1 (synthesis §5; ledger §3 Wave 1: `P77 ∥ P79`).
- **Phase-28 single-eigen-surface ruling** (memory `eigenvector-refactor` /
  `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`) — B6 must keep `spectral.rs` the sole eigen
  surface; coordinate the buffer shape with `spectral_cache::Decomp` (the `topk_symmetric` return type,
  `spectral.rs:270`).
- **Feeds P89** (field eigenmodes via `spectral.rs`) — the `k·n` evec buffer is P89's stated
  prerequisite (ledger §1 P79 row: "P89 prerequisite (evec k·n buffer)"; §3 Wave 3: `P89 (after P79+P81)`).

---

## 3. Predefined types & constants (standard §2 item 4)

```rust
// kernel/src/causal.rs  (B5 — the minimal usize analog of mat.rs)

/// Row-major contiguous sample matrix: one allocation for the whole batch.
/// Row `r`, column `c` at `data[r * n_cols + c]`. Replaces `&[Vec<usize>]` on the
/// causal hot path (from_samples / empirical_identify_conditional / infer_cards).
/// Scaling axis: rows = sample count (benched at 20_000); n_cols = variable count
/// (small, = cards.len()). Shape changes only if n_cols is ever non-uniform per row
/// (it is not — every assignment covers all variables), which the constructor asserts.
pub struct Samples {
    pub n_cols: usize,
    pub data: Vec<usize>,   // len == n_rows * n_cols
}
// helpers: Samples::with_rows(n_rows, n_cols) -> zeroed; row(r) -> &[usize];
//          push_row(&[usize]) (asserts len == n_cols); n_rows() -> data.len()/n_cols
```

B6 introduces **no new type** — it reuses `Mat` (`mat.rs:17`) or a plain `Vec<f64>` of `k·n` with the
documented `[m*n..(m+1)*n]` slicing. The eigen surface type (`spectral_cache::Decomp`) keeps its public
shape; only its internal evec storage flattens. B6' is a comment. No magic numbers.

---

## 4. Build items — spec → RED test → code (standard §2 items 2, 3, 5)

### 4.1 B5 — flat `Samples` on the causal path

- **Spec:** add `Samples` (§3). `sample_backdoor` returns/fills a `Samples` (or an adapter keeps the
  `Vec<Vec<usize>>` public signature and converts once at the boundary — decide by which minimizes
  churn while keeping the hot loop contiguous). `from_samples`, `empirical_identify_conditional`,
  `infer_cards` read `Samples::row(r)` instead of `samples[r]`. The empirical joint counts are computed
  identically — only the container changes.
- **RED `red_causal_joint_bit_identical`:** capture the `empirical_identify`/`empirical_identify_conditional`
  output (the identified `Vec<f64>`) on a fixed `(n, seed)` from HEAD; assert the post-port output is
  **bit-identical** (exact `f64` equality — deterministic RNG, deterministic counts). RED if the port
  changes any value; GREEN proves behavior preservation. Permanent regression test (standard §2 item 17).
- **RED `red_causal_20k_allocs_bounded`:** using the `arena_counting_allocs`-style harness (the
  commented `kernel/Cargo.toml:156` bench slot / `graph_rebuild_rank/{heap,arena}` pattern at
  `criterion.rs:161-194`), assert the sample container is **one** allocation, not `n_rows`. RED on the
  AoS code, GREEN after. Mechanism gate.
- **Bench (before/after gate):** `empirical_identify/20k_samples` + `empirical_identify/end_to_end_20k`
  (`criterion.rs:91,95`) are the measured number — record pre-port and post-port wall-clock in P75's
  committed trend store. **This is the "measured, not estimated" proof (standard §2 item 10).**
- **Adversarial `red_samples_ragged_rejected`:** `push_row` with a wrong-length row → assert/`Err`
  (a ragged sample can never enter `Samples`), matching `from_samples`'s existing length check (`:1062`).

### 4.2 B6 — flatten spectral eigenvector storage

- **Spec:** replace `evecs: Vec<Vec<f64>>` (`:280`, `:421`) with a `k·n` contiguous buffer; eigenvector
  `m` is `&buf[m*n..(m+1)*n]`. Deflation/Gram-Schmidt/Rayleigh loops (`:306…:486`) index the slice; the
  end-of-solve reorder (`:397-400`, `:526-529`) permutes by copying rows within the flat buffer (or
  returns an index map) — eliminating the `k`-clone rebuild. Keep the eigen surface in `spectral.rs`
  only (Phase-28 ruling).
- **RED `red_spectral_evecs_bit_identical`:** capture eigenvalues + eigenvectors from HEAD on a fixed
  symmetric input (both the heap and arena variants); assert the post-flatten result is bit-identical
  (including sign convention and the sorted order at `:400`/`:529`). RED if flattening perturbs any
  value/order; GREEN proves preservation. The existing `spectral_cache/*` benches (`criterion.rs:119-159`)
  and `UᵀU=I` orthogonality KAT (memory `eigenvector-refactor`: 1e-9 KAT) must stay green.
- **RED `red_spectral_no_kclone_reorder`:** assert the reorder no longer allocates `k` new row `Vec`s
  (allocation-count harness). RED on `:400`/`:529`, GREEN after.
- **Adversarial `red_spectral_deflation_orthogonal`:** after the port, the deflated components must
  still satisfy `UᵀU = I` to 1e-9 (the Phase-28 KAT) — proves the contiguous dot products didn't drift
  the Gram-Schmidt.
- **New bench `spectral_math/eigh_flat/{8,16,32,48}`:** eigen decomposition straddling the n=32
  QR↔Faddeev step, so the before/after is on the record (this bench group is shared with **P80**'s
  `spectral_math` — P79 lands the evec-relevant slice, P80 owns the full group; coordinate ids under
  P75's schema, do not duplicate).

### 4.3 B6' — the doc-label fix

- **Spec:** `engine/src/zerocopy.rs:22` "SoA-record layout" → "AoS-record layout (interleaved per
  particle)". No code, no test — a one-line comment correction. Grouped here because it is R9's rider
  to B6 and touching it separately would be churn.

---

## 5. Invariants to preserve (standard §2 items 6, 13)

Made test-pinned, not asserted in prose:

1. **Causal determinism & identifiability (B5):** `empirical_identify` is deterministic given
   `(n, seed)` (`sample_backdoor` uses a seedable RNG, `:1309`) and fail-closed on structural/trust
   violations (`from_samples` `:1056`, `:1062`, `:1070`). Preserved because only the container changes;
   `red_causal_joint_bit_identical` + `red_samples_ragged_rejected` are the falsifiers.
2. **Spectral orthogonality & sign/order convention (B6):** `UᵀU = I` (1e-9), deflation correctness,
   and the sorted eigenpair order (`:400`, `:529`) are preserved; `red_spectral_evecs_bit_identical` +
   `red_spectral_deflation_orthogonal` are the falsifiers. The `spectral_cache` KATs stay green.
3. **Single eigen surface (B6):** `spectral.rs` remains the ONLY eigensolve surface (Phase-28 ruling);
   no `lowrank.rs`, no second authority. Structural — the port adds no new module.
4. **Rollback as math (item 13):** both are self-terminating diffs — a failing bit-identity test means
   the port is not merged; there is no runtime state and no persisted format change to roll back
   (the `Samples`/evec buffers are transient compute scratch). Snapshot re-entry N/A.

---

## 6. DoD — falsifiable, RED→GREEN (standard §2 item 2)

| # | Done when… | Falsifier |
|---|---|---|
| D1 | causal path uses one contiguous `Samples`; joints bit-identical | `red_causal_joint_bit_identical`, `red_causal_20k_allocs_bounded` |
| D2 | `empirical_identify/20k` shows the measured allocation/cache win (real number) | `criterion.rs:91,95` before/after in P75's trend store |
| D3 | spectral evecs are one `k·n` buffer in both variants; results bit-identical | `red_spectral_evecs_bit_identical`, `red_spectral_no_kclone_reorder` |
| D4 | deflation still orthogonal to 1e-9; sorted order preserved | `red_spectral_deflation_orthogonal`, `spectral_cache/*` KATs green |
| D5 | `zerocopy.rs:22` label reads AoS | doc diff review |
| D6 | `spectral.rs` remains the sole eigen surface (no new module) | grep: no `lowrank.rs`; Phase-28 KAT green |
| D-NOREG | `cargo test --lib` green (kernel 561+ baseline, memory `eigenvector-refactor`); no new dep | `cargo test -p dowiz-kernel`; `Cargo.lock` unchanged |

---

## 7. Benchmarks + telemetry + measure-first (standard §2 item 10)

- **B5's before/after is already wired** — `empirical_identify/20k_samples` and `/end_to_end_20k`
  (`criterion.rs:91,95`). This is the strongest case in the whole perf pass for a *measured* (not
  estimated) data-layout win (synthesis §3.2 B5: "the one data-layout candidate whose existing bench
  will visibly move"). Record both curves; the delta is the headline.
- **B6's win is expected modest at n≤32** — its bench (`spectral_math/eigh_flat/*`, shared with P80)
  primarily proves *no regression* and captures the k-clone-reorder removal; its larger value is
  qualitative (P89 substrate) and structural (single flat buffer). Report the honest number; do not
  claim a large speedup the small-n bench won't show.
- Telemetry: no runtime hook; the criterion baselines in P75's CI gate are the regression detector
  (standard §2 item 14). Optionally instrument allocation counts via the existing
  `graph_rebuild_rank/{heap,arena}` harness pattern.

---

## 8. Rollout / sequencing (consistent with the master ledger)

Per `MASTER-STATUS-LEDGER-2026-07-19.md` §3 and `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §5:

- **Lane:** dowiz kernel (never touches bebop2 — parallel to the whole bebop lane).
- **Wave:** **W1 correctness fixes**, build-parallel with **P77** (disjoint files: `causal.rs`/
  `spectral.rs` vs `spool.rs`/`spine.rs`). Ledger §3 Wave 1: `P77 ∥ P79`.
- **Soft dependency:** **P75** lands first so B5/B6 benches write into the fixed `<group>/<n>` schema.
- **Feeds:** **P89** (Wave 3 — field eigenmodes) consumes B6's `k·n` buffer; **P80** (Wave 2) owns the
  full `spectral_math` bench group that B6's `eigh_flat` slice joins — coordinate ids, do not duplicate.
- **Coordinate:** the Phase-28 single-eigen-surface ruling before touching `spectral.rs` internals.

---

## 9. Open operator-decision points

P79 introduces **no new** operator decision. It carries two engineering coordinations (blueprint
decides, operator need not):

| # | Coordination | Decided by | Note |
|---|---|---|---|
| E-1 | B5 public signature: convert at boundary (keep `Vec<Vec<usize>>` public) vs change `sample_backdoor`'s return type to `Samples` | the executing worker | Prefer whichever keeps the hot loop contiguous with least caller churn; both are behavior-preserving |
| E-2 | B6 buffer type: reuse `Mat` vs a bare `Vec<f64>` + slicing inside `spectral.rs` | the executing worker, under the Phase-28 ruling | Must not create a second eigen surface; `spectral_cache::Decomp`'s public shape is fixed |

No §4/§5-class operator decision (no money/RLS/auth/red-line surface is touched — causal inference and
eigensolve are pure deterministic kernel math).

---

## 10. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** a contiguous buffer *is* the flat image of the logical
  matrix — element `(i,j)` at `data[i*ncols+j]` is self-describing (mat.rs's own doctrine, `:14`); the
  port makes the storage correspond to the access pattern (row/vector-contiguous walks).
- **Polarity / no-middle:** a sample row is either a complete assignment over all variables or it is
  rejected (`Samples::push_row` asserts `len == n_cols`) — no ragged half-row middle state; mirrors
  `from_samples`'s existing length gate (`:1062`).
- **Gender/Generation (cause→form):** the eigenvectors are *generated* by deflation from the operator;
  flattening their storage does not change the generative cause (the spmv), only the vessel — the
  `UᵀU=I` invariant is what makes the generation faithful, and the port keeps it (1e-9 KAT).

---

## 11. Standard-compliance map (all 20 points — standard §2)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (causal AoS cites; spectral both variants; zerocopy label) |
| 2 | Falsifiable DoD | §6 |
| 3 | Spec→test→code, event-driven | §4 (spec-first per B5/B6; bit-identity captured before edit) |
| 4 | Predefined types & constants | §3 (`Samples`; `Mat` reuse for evecs) |
| 5 | Adversarial/breaking tests | §4 (`red_samples_ragged_rejected`, `red_spectral_deflation_orthogonal`) |
| 6 | Hazard-safety from structure | §5 (determinism/orthogonality as test-pinned invariants; single eigen surface) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §3 (rows=sample count; n_cols=var count; evecs=k·n, n≤32 today) |
| 9 | Linux engineering discipline | REINFORCES the DOD/contiguous invariant (mat.rs); EXTENDS mat.rs to usize (B5); DOES-NOT-TRANSFER SoA (wrong here) — §1 |
| 10 | Benchmarks + telemetry + measure-first | §7 (B5 existing bench = real number; B6 honest modest) |
| 11 | Isolation / bulkhead | §2 (disjoint from bebop and from P77; kernel-internal scratch only) |
| 12 | Mesh awareness | N/A honestly — causal/spectral are node-local kernel compute, not mesh-propagated; stated |
| 13 | Rollback/self-heal as math | §5.4 (bit-identity gate; transient scratch; no persisted format) |
| 14 | Error-propagation / smart index | §7 (criterion baselines in P75 CI catch regressions); §4 ragged-row gate |
| 15 | Living-memory awareness | N/A honestly — samples/evecs are transient compute, not living memory; stated |
| 16 | Tensor/spectral where applicable | §0.4/§4.2 — the whole blueprint IS the tensor-layout reuse (mat.rs), coordinated with the spectral eigen surface |
| 17 | Regression tracking | §4/§6 (bit-identity tests permanent; REGRESSION-LEDGER entries) |
| 18 | Clear worker instructions | §13 |
| 19 | Reuse-first | §1 (mat.rs reuse; RGBA-interleave rejected E1); no new dep |
| 20 | Hermetic principles | §10 |

---

## 12. Links to docs & memory (standard §2 item 7)

- `docs/research/OPUS-PERF-BESTPRACTICES-PROPAGATION-2026-07-18.md` (R8) G-D1 (causal AoS, top DoD item).
- `docs/research/OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` (R9) §3-B (evec flatten — its ONE actionable),
  and its refutation of the RGBA generalization (synthesis §6 E1 — B6 is NOT that lesson).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 (B5/B6 rows), §5 (Wave W1, `P77 ∥ P79`), §6 (E1
  RGBA rejected, E16 verified-clean `eigenvalues n≤32 dispatch`).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P79 row: "P89 prerequisite"), §3 (Wave 1/Wave 3 P89 edge).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md` + memory `eigenvector-refactor` (Phase-28 single
  eigen-surface `spectral.rs`, `UᵀU=I` 1e-9 KAT, kernel 561 tests).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `performance-priority-over-minimal-change-2026-07-17.md`, `math-first-architecture-arc-2026-07-14.md`.

## 13. Instructions for the executing worker (zero prior context — standard §2 item 18)

**Repo:** `/root/dowiz/kernel` (B5/B6) + `/root/dowiz/engine/src/zerocopy.rs` (B6' label only). Disjoint
from P77 — safe to run in parallel with it (never edit `spool.rs`/`spine.rs` here).

1. **B5** — `kernel/src/causal.rs`: add `Samples` (§3) near `from_samples` (`:1056`). Capture golden
   `empirical_identify`/`_conditional` output on fixed `(n, seed)` from HEAD **before** editing. Port
   `sample_backdoor` (`:1307`), `from_samples`, `empirical_identify_conditional` (`:1325`),
   `infer_cards` (`:1366`) to read `Samples::row`. Add RED tests (§4.1).
2. **B6** — `kernel/src/spectral.rs`: flatten `evecs` in BOTH variants (`:280`, `:421`); eliminate the
   k-clone reorder (`:400`, `:529`). Coordinate the buffer with `spectral_cache::Decomp` under the
   Phase-28 single-eigen-surface ruling — do NOT create `lowrank.rs`. Capture golden eigenpairs from
   HEAD first. Add RED tests (§4.2); keep the `UᵀU=I` 1e-9 KAT green.
3. **B6'** — `engine/src/zerocopy.rs:22`: relabel "SoA-record" → "AoS-record (interleaved per particle)".
4. Run `empirical_identify/*` and `spectral_math/eigh_flat/*` benches; record before/after in P75's
   trend store (D2). `cargo test --lib -p dowiz-kernel` fully green (561+ baseline).
5. Add REGRESSION-LEDGER entries for the two bit-identity tests.
6. **Do NOT** change any causal algorithm, any eigensolve numeric, any public `Decomp` shape, or
   introduce SoA/RGBA-interleaving (E1 — actively harmful here). Report B6's honest (modest) number.
