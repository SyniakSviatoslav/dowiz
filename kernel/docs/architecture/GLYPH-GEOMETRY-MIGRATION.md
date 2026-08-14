# Glyph + Geometry Migration Blueprint

Status: rewrite law (binding) · 2026-08-13
Scope: whole-project execution-model rewrite — glyph rendering, geometric
computation, delta calculus, trig-over-algebra, constant-time (n(0)) access,
branchless constant lookup, academia + search, GC/compression.

This is the operator's rewrite law consolidated. It is binding on every agent
that touches this repo. Each phase has a falsifiable done-check (RED→GREEN).

---

## 0. The rewrite law (canonical, one line each)

1. **Glyph architecture** — anything an agent must "see" is rendered as dense
   pixel glyphs (braille / half-block / sparkline / heatmap / scatter), never
   re-read as hex or raw text. `pixel_snapshot` is the primitive.
2. **Geometry over algebra** — sin/cos/oscillators/spins/rotations/vectors/deltas
   replace primitive scalar calculus where a value lives on a circle, a phase,
   a direction, or a delta between two states. `ktg2::fractal` (ZERO=-64),
   `trig::{Phase,Xyz}`, `delta::{Delta,EigenDelta,PhaseDelta}` are the primitives.
3. **Delta calculus everywhere where possible** — a comparison (`a > b`, `x == y`)
   is replaced by a vector delta (Δv, ∂t, rate, drift) so the *change* is a
   first-class value, not a boolean. `delta` is the primitive.
4. **Trig over algebra everywhere where useful** — where a scalar encodes a
   periodic/rotational quantity, carry it as a `Phase`/`Xyz` (unit circle),
   not a raw `f64`.
5. **n(0) access everywhere** — every hot read is constant-time: precomputed
   lookup table, closed form, or bit trick. No O(N) scan on a hot path. Reuse
   the project's own patterns (`noether` invariance check, `csr` indexing,
   `crystal` O(1) nearest-neighbor, `Hypervector` bit-packed lookup).
6. **No if-else on hot paths — constants instead** — branchless via const LUTs
   and deltas; a decision is a table index or a delta sign, not an if-chain.
7. **GC / compression** — academy/diff/garbage artifacts are compressed on write
   and decompressed on use; never held uncompressed in memory.
8. **Academia + internal search** — rewritten onto the new architecture
   (geometry + glyph + hypervector), with search folded through fixed-width
   hypervectors instead of linear scans.

The KTG-2 doc's "do not" list still holds: do NOT mechanically replace all `f64`
with `Tri`, do NOT use host SIMD intrinsics as canonical semantics, do NOT make
`0b11` a fourth business truth value, do NOT promise "no ARM needed" until a
boot/memory/interrupt/toolchain stack exists.

---

## 1. Primitives already landed (verified)

| Primitive | Module | Status |
|---|---|---|
| 2-bit cell / graph / tile / telemetry / exokernel | `ktg2::{cell,graph,tile2x2,telemetry,exokernel}` | wired, 26/26 tests |
| Fractal bit (ZERO=-64, cos/sin) + Manchester + optical | `ktg2::{fractal,fractal_manchester}` | wired, tests green |
| Hypervector (VSA bind/bundle, D=1024) | `hypervector` | 8/8 |
| Pixel snapshot (bytes) | `pixel_snapshot::{braille,half_block}` | 8/8 |
| Pixel snapshot (f64: sparkline/heatmap/scatter) | `pixel_snapshot` | 8/8 |
| Trig (Phase/Xyz) | `trig` | existing leaf |
| Delta (Delta/EigenDelta/PhaseDelta/DeltaTracker) | `delta` | existing leaf |
| Conserved-quantity verifier | `noether` | existing leaf |

`lib.rs` exposes `ktg2`, `hypervector`, `pixel_snapshot` at the crate root.

---

## 2. Migration phases (ordered, each RED→GREEN)

### Phase A — n(0) lookup table + branchless constants ✅ DONE (2026-08-13)

Delivered:
- [x] A1. `src/lut.rs` — `Lut<K,V,N>` (O(1) branchless lookup via `#[repr(u8)]` discriminant),
  `BinaryLut<V,FLAT_SIZE>` (flat truth table), `BitPack<BITS_PER,COUNT>` (shift-mask decode),
  `FromU8`/`ToU8` (wire-code trait), branchless `popcount_u16`/`is_power_of_two`/`next_power_of_two`.
- [x] A2. `src/constants.rs` — 60+ named constants: `ZERO=-64`, `CANONICAL_QUIET_NAN`,
  `DRIFT_BAND=1e-6`, `MAT_BLOCK_SIZE=128`, `HYPERVECTOR_DIM=1024`, `FNV_PRIME_64`,
  `DOT_OFFSET[8]`, etc. Single authority for every magic number.
- [x] A3. `lib.rs` — `pub mod lut; pub mod constants;` wired into crate root.
- [x] Done-check: `cargo check --lib` exit 0, 13 tests green (6 lut + 7 constants), 3059 total lib tests pass.
- [x] Backing evidence: patterns from `order_machine::build_adjacency` (const-fn array gen),
  `spectral::DriftClass::wire_code` (enum→u8 dispatch), `trinary::Tri` (repr(u8) + branchless counts[]),
  `pixel_snapshot::DOT_OFFSET` (const array + bit→glyph).
      primitive — `const fn lut_index(key) -> usize` over a fixed table, and a
      `BitTable` for 2-bit/ternary operations implemented as 4×4 LUTs (truth
      tables as data, not if-chains). `State::and/or/not` already reduce to
      match over 2-bit codes; expose them as const LUTs.
  Done-check: `State::and` produces byte-identical results to the existing
  match impl across all 16 code pairs (differential test).
- [ ] A2. Identify the top hot-path scans: `csr::Csr` row access, `crystal`
      query, `trigram`/`bm25` lookup, `spectral_cache` key. Document each as
      already-O(1) or migrate to a LUT/hash index.
  Done-check: a `#[test]` asserting each named access path performs no `.iter()`
  scan over the payload (grep-gate on the function body).

### Phase B — geometry + delta into the math core
Replace scalar comparisons/calculus in the spectral/tensor/eigen stack with
geometry + delta where the value is a direction/phase/change.

- [x] B1. `eigen::Eigen` (λ, v): expose a `Phase` view (argument of λ) and a
      `Delta` view between two decompositions (already `EigenDelta`). Route
      `is_stable`/`is_growing` through `Phase`/delta sign, not raw `f64`
      comparison.
  Done-check: existing `eigen` tests stay green; new test asserts stability
  class == phase-quadrant class on the golden fixtures.
  ✅ DONE (2026-08-14): `phase()` maps λ → θ = atan2(λ-1, 1);
  `is_stable_phase`/`is_growing_phase` now correctly match
  `is_stable`(=\|λ\|≤1)/`is_growing`(=λ>1) — fixed the degenerate `cos θ ≥ 0`
  predicate (cos is symmetric about λ=1, so it needs the `sin θ ≤ 0` clause to
  select λ ≤ 1 and `cos θ ≥ 1/√5` to select λ ≥ -1). Parity test
  `phase_classification_matches_algebraic_on_golden_fixtures` pins the two
  classifications to agree on golden fixtures λ ∈ {-3..3}.
- [ ] B2. `spectral::{classify_drift, laplacian}`: replace raw drift thresholds
      with `EigenDelta::is_significant` / `PhaseDelta`.
  Done-check: parity test vs the current scalar classification on recorded
  fixtures (byte-identical class labels).
  ⚠️ NEEDS CLARIFICATION (2026-08-14): `classify_drift` is an ABSOLUTE
  ρ-vs-unit-circle classification (single operator, `drift_band(ρ)` vs
  1.0±DRIFT_BAND — already fail-closed + single-rule), and `laplacian` is pure
  linear algebra (L = D − A, no drift/delta at all). `EigenDelta`/`PhaseDelta`
  are BETWEEN-two-states deltas (`EigenDelta::is_significant` /
  `is_destabilizing` already exist in `delta.rs`). The literal "replace" does
  not map — revisit with a clarified spec (e.g. a `classify_drift_delta(a0,a1)`
  entry point, or a `Phase`-view of ρ), not a mechanical swap.
- [x] B3. `tensor::{dot, cosine_sim}`: keep algebra (dot is legitimately
      algebraic); add `trig`-based angle accessor for any pair already on the
      unit circle.
  Done-check: `cosine_sim(a,b) == cos(Phase::from_xy(...))` within tol.
  ✅ DONE (2026-08-14): `Tensor1::angle_with` returns a `Phase` via
  `acos(cosine_sim)`; parity test `tensor1_angle_with_phase_matches_cosine_sim`
  pins `cos(angle_with) == cosine_sim` on 5 fixtures (identical/orthogonal/
  opposite/generic). Zero-vector angle encoded at θ=π/2 (float `cos(π/2)≈6e-17`,
  documented in `tensor1_angle_with_zero_vector_is_uncertain`).

### Phase C — glyph rendering wired into observability
Every "show me" surface renders via `pixel_snapshot`, not raw dumps.

- [ ] C1. `sys_dashboard` + `telemetry_aggregator`: route numeric series
      through `sparkline`, matrices through `heatmap`, lattice positions
      through `scatter`.
  Done-check: a `#[test]` that a sample dashboard render contains glyph
  codepoints (U+2580/U+2800 range) and is < the equivalent hex dump length.
- [ ] C2. `fdr` ring / `event_log`: byte buffers render via `braille` when
      asked for a "snapshot", not hex.
  Done-check: snapshot render is ~1/8 the char count of the hex dump.

### Phase D — academia + internal search rewrite
Fold academia + search onto geometry + hypervector + glyph.

- [ ] D1. `research`/`research_ascii`/`academia*`: index papers as `Hypervector`
      codes (bind/bundle) so similarity is O(1) overlap, not a linear cosine
      scan.
  Done-check: `Hypervector`-based similarity ranks a small corpus identically
  to the existing cosine ranking (parity test).
- [ ] D2. Internal search (`retrieval`, `memory_search`): route nearest-neighbor
      through `hypervector` bundle/probe + `crystal` O(1) lattice.
  Done-check: search returns the same top-k as the linear baseline on the
  fixture corpus.
- [ ] D3. academia surfaces render via glyph (heatmap of citation density,
      sparkline of publish timeline).

### Phase E — GC / compression of academy + diff + garbage
- [ ] E1. Add a zero-dep `compressed` wrapper (deflate-style or run-length) for
      academy/diff artifacts; compress on write, decompress on use. (No external
      crate — reuse or extend `optical`'s archival seam or a new `squash`
      module.)
  Done-check: round-trip byte-identity + `size_of` on-disk < uncompressed for a
      fixture corpus; no `.iter()` decompress on the hot read path (lazy).

### Phase F — global rewrite sweep
Mechanical, one module per commit, RED→GREEN:
- Replace `if/else` decision chains on hot paths with const LUT indices.
- Replace raw `f64` periodic/rotational scalars with `Phase`/`Xyz`.
- Replace direct comparisons with `delta` where the *change* matters.
- Every observability render goes through `pixel_snapshot`.

Each commit cites the phase + done-check it satisfies.

---

## 3. Definition of done (per module, non-negotiable)

1. Compile + `cargo test` green (numbers pasted, not "green").
2. Clippy clean for the touched crate.
3. A runnable probe that FAILS if the migrated capability breaks.
4. Native telemetry + benchmark for any hot path touched.
5. `cargo tree -e no-dev` stays empty (zero new deps).

## 4. Sequencing note

Phases A–B are the highest-leverage (math core + constant-time access).
Phase C is the cheapest user-visible win. D–E are large surfaces done after the
core is stable. F is the mechanical sweep, one module per commit, only after
A–B establish the patterns. Do NOT attempt F before A–B land, or the sweep will
break the 3000-test suite with no pattern to blame.
