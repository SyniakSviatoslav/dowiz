# BLUEPRINT — Items 39 + 40: SIMD Quantized Kernels + Per-Layer Golden-Checksum Oracle (hard-fail)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Why paired:** item 40's golden checksum guards the exact bytes item 39's SIMD kernel produces;
  they share the item-37 oracle as their common truth source and land in strict sequence (39 → 40).
  One doc keeps the differential-vs-golden distinction in one place. They remain two deliverables
  with two acceptance sets.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — item 39's SIMD is
  the ONE place speed is pursued, and it is pursued **only** under a bit-exact differential leash;
  item 40 is pure safety: **hard-fail to a safe state on any checksum mismatch**, never emit
  suspect output.
- **Sources read this session:** roadmap §H items 39 (lines 561–568) + 40 (lines 569–577);
  `kernel/src/simd.rs:1-9` (the **bit-identity design rule**: "vectorise ACROSS the batch of
  independent rows, never WITHIN a single row's reduction" — because f64 add is non-associative),
  `:36 softmax_scalar` (scalar reference + fallback), `:64-66` (`#[target_feature(enable="avx2")]`,
  `is_x86_feature_detected!`); `kernel/src/householder.rs:33-43` (`_mm256_fmadd_pd` dot kernel — the
  intrinsics-not-autovec precedent); `kernel/src/fdr/ring.rs:65` (`crc32`, IEEE-802.3 reflected,
  hand-rolled table-on-first-use `:46`; known vector `0xCBF4_3926` `:336`) — **the single CRC32,
  reused (P2, no second CRC)**; CHECKLIST.md items 1/3/4; the item-9 breaker composition (synthesis
  §3: mismatch is a `CommitError`-class must-alarm; design does NOT gate on item 9).
- **Dependency gate:** **item 39 after 36+37+38** · **item 40 after 39.** Item 9 (breaker) *composes*
  with item 40's fail path when it exists but **gates nothing here**.

---

# PART A — Item 39: SIMD Quantized Kernels via `core::arch`

## A1. Scope / goal + non-goals

**Goal.** AVX2 integer quantized-dot / matmul kernels (`_mm256_madd_epi16` /
`_mm256_maddubs_epi16`-class), runtime-detected (`is_x86_feature_detected!("avx2")`), with item
37's **scalar oracle as the fallback and the differential target**. Named Q5 dividend: **integer
arithmetic is associative**, so within-row vectorization is *legal* here — unlike the f64 lanes'
across-rows-only rule — **but the chosen lane order is still fixed and documented**, and debug builds
carry `debug_assert_eq!` against the oracle (the `ring_mul` standard).

**Non-goals.** NOT compiler autovectorization — hand-written `core::arch` intrinsics, so the
processor does *exactly* the math written (the dialogue's part-5 §2 requirement). NOT a new
dependency (empty allowlist). NOT AVX-512/AMX in v1 (a later widening; AVX2 is the house-proven
lane). NOT bit-*loose* — a 1-bit divergence from the oracle is a build failure.

## A2. Grounding — the critical distinction from the existing f64 lane

`simd.rs`'s rule (`:1-9`) is "vectorise ACROSS independent rows, never WITHIN a row's reduction,"
**because f64 addition is non-associative** — reordering a float sum changes the last bit. **Integer
addition IS associative**, so within-row lane reduction is *mathematically legal* for the quantized
kernels. This is a genuine, correct relaxation the arc is entitled to — but the ruling still demands
the lane order be **fixed and documented**, and the SIMD result be **bit-exact to the scalar oracle**
(item 37). The relaxation buys legality of within-row SIMD; it does not buy freedom from the
differential leash.

## A3. Implementation plan (item 39)

1. **Integer kernels via `core::arch`** (e.g. `kernel/src/inference/simd_i8.rs`): AVX2
   quantized-dot / matmul, `#[target_feature(enable = "avx2")]`, gated behind
   `is_x86_feature_detected!("avx2")`, **scalar oracle fallback** otherwise (the `simd.rs`/
   `householder.rs` pattern verbatim).
2. **Resolve the signedness / saturation hazard explicitly.** `_mm256_maddubs_epi16` is
   **unsigned×signed with i16 saturation** — a determinism hazard if the i16 intermediate can
   saturate. Two documented paths:
   - **Preferred: `_mm256_madd_epi16` (i16×i16 → i32, no saturation).** Widen i8→i16 first; no
     saturation possible, so the SIMD path is *provably* the same integer math as the oracle.
   - If a `maddubs` path is chosen for throughput: offset symmetric activations to u8 (zero-point
     shift) and correct in the i32 accumulator, AND **prove the i16 intermediate cannot saturate**
     for the item-35 bounded domain — else the path is refused.
3. **Pin and document the lane order** (even though associativity permits reorder) — for
   auditability and bit-exact-to-oracle.
4. **`debug_assert_eq!` against item 37's oracle on every call** (item 3 / `ring_mul` standard) —
   continuous verification, compiled out of release at zero production cost.
5. **Differential corpus** (large randomized + i8 boundary values ±127/0) vs the oracle, bit-exact on
   BOTH paths (scalar fallback + AVX2).
6. **Register in HOT-PATHS.tsv** and add a **bench to `baseline.json`** so the bench-gate guards it.

## A4. Required proofs (5-point checklist) — item 39

- **1 (oracle):** differential corpus vs item 37 oracle, **bit-exact**, scalar AND AVX2 paths.
- **3 (debug/differential):** `debug_assert_eq!` vs the oracle per call, compiled out of release.
- **4 (asm):** assembly spot-check on every compiler-version bump (item 14 toolchain-keyed audit
  format) — confirm the intrinsics lower to the expected integer-SIMD instructions (`vpmaddwd`,
  etc.) and that **no data-dependent branch** entered the kernel body.
- **2 (dudect):** the kernel is data-oblivious by construction (fixed lane order, no data-dependent
  memory access); the *gate* scope is item 43's call — public plane ⇒ cheap-but-optional for the toy
  pilot.
- **5 (kani):** N/A — SIMD intrinsics are not Kani-checkable; the exhaustive/differential oracle
  carries correctness (per the item-7 rescope logic: use the engine that fits the property).

## A5. Falsifiable acceptance criteria — item 39

1. Differential corpus (randomized + i8 boundary) vs item 37 oracle: **BIT-EXACT on scalar AND AVX2
   paths, zero divergence.** **RED→GREEN:** a deliberately-wrong lane order or a dropped tail element
   turns it RED.
2. `debug_assert_eq!` vs oracle wired per call; a planted kernel bug is caught in a debug run.
3. The AVX2 path's saturation/signedness is **proven bit-exact to the scalar oracle** (or the
   non-saturating `_mm256_madd_epi16` path is chosen and the choice documented).
4. Assembly spot-check recorded (item 14 format): no data-dependent branch in the kernel; expected
   integer-SIMD instructions present.
5. A bench is in `baseline.json`; the bench-gate guards it; the HOT-PATHS row is registered.

---

# PART B — Item 40: Per-Layer Golden-Checksum Oracle + Hard-Fail

## B1. Scope / goal + non-goals

**Goal.** Build-time golden CRC32 per layer over **pinned test vectors** (reusing `fdr::crc32` — P2,
NO second CRC), a runtime self-check, and **hard-fail to a safe state on mismatch**. A checksum
mismatch is **hardware/memory-fault evidence** (bit flip, ECC error, corrupted `.rodata`, stuck
compute unit), *not* a model error — so the engine stops rather than emit suspect output.

**Non-goals.** NOT a per-real-inference integrity check (see the honest limit, B5). NOT a second CRC
implementation (`fdr::crc32` is reused). NOT gated on item 9's breaker — the composition is *named*
so nobody re-derives it, but the interim fail path stands alone.

## B2. Grounding

- **The CRC32 already exists, once.** `kernel/src/fdr/ring.rs:65` `crc32` (IEEE-802.3 reflected,
  table-on-first-use `:46`, known-answer `crc32(b"123456789")==0xCBF4_3926` `:336`). Item 40 **reuses
  it** — P2 Correspondence, no second CRC (roadmap explicitly: "reusing `fdr`'s hand-rolled CRC32").
- **The FDR entry path exists** (`fdr/ring.rs`, CRC32-checked NDJSON, kill-9 recovery) — the interim
  fail records a typed FDR entry there.
- **The breaker composition is named, not built.** Synthesis §3: a mismatch is a `CommitError`-class
  "must alarm" event; until item 9 lands, the fail is a typed trap + FDR entry; when it lands, the
  mismatch routes through `Result<Permit, Tripped>`. **Design does NOT gate on item 9.**

## B3. Implementation plan (item 40)

1. **Build-time goldens from the oracle.** For a set of **pinned test vectors** (a subset of item
   34's D), run item 37's oracle and record `golden_crc[layer] = fdr::crc32(oracle_layer_output_bytes)`
   as committed constants.
2. **Runtime self-check.** The engine, on the *same pinned vectors*, recomputes each layer's output
   CRC32 (`fdr::crc32`) and compares to `golden_crc[layer]`.
3. **Hard-fail to a safe state on mismatch.** A mismatch → the engine **traps to a safe state**
   (typed `Err(ChecksumFault{layer})` / panic-handler path per the dialogue's part-5 §3), **never
   returns a computed output**. An FDR entry is written recording the faulting layer.
4. **Interim vs breaker composition.** Interim (no item 9): typed trap + FDR entry. When item 9
   lands: `Result<Permit, Tripped>` (named, not built here).
5. **Silent when healthy.** An uncorrupted run performs the check and is otherwise silent (no FDR
   spam, no false trip).
6. **CI re-executes the planted fault** (P7 — the verifier proves it can reject), never presence-
   checks.

## B4. Required proofs (5-point checklist) — item 40

- **1 (oracle):** the goldens ARE the item-37 oracle's outputs — the self-check is a differential
  against the schoolbook, frozen as CRCs.
- **3 (differential):** the runtime self-check is a continuous differential vs the golden.
- **P7 (planted-fault, the load-bearing proof):** a planted single-bit corruption — **weights AND
  activation, separately** — demonstrably trips the fail path and writes the FDR entry; an
  uncorrupted run is checksum-silent; **CI re-executes the planted fault.**
- **2/4/5:** N/A (CRC compare has no secret-timing surface here — the pinned vectors are public; no
  new asm; correctness is the CRC known-answer + the planted-fault demo).

## B5. Falsifiable acceptance criteria — item 40

1. Build-time golden CRC32 per layer, computed via `fdr::crc32` (NO second CRC impl — verified by
   grep: still one `crc32` in the tree).
2. A **planted single-bit weight corruption** → self-check **hard-fails to safe state** + writes an
   FDR entry. **RED→GREEN.**
3. A **planted single-bit activation corruption** (separately) → hard-fails + FDR entry.
   **RED→GREEN.**
4. An **uncorrupted run is checksum-SILENT** — no false trip, no FDR entry.
5. **CI re-executes the planted fault** (never presence-checks) — deleting the planted-fault test or
   emptying the golden turns the gate RED.
6. The item-9 composition (`Result<Permit, Tripped>`) is **named in the module doc**; the interim
   typed-trap + FDR path works with item 9 absent.
7. **Honest limit documented (architect, failure-first):** the golden self-check catches **persistent**
   faults (corrupted `.rodata` weights, a stuck compute unit) via the pinned-vector self-check; a
   **transient single-inference bit-flip on an arbitrary real input has no precomputed golden and is
   NOT caught by this mechanism** — that class needs redundant computation (dual-run compare), which
   is **named out-of-scope** here (a follow-on for the real-product pilots). This limit is stated so
   the mechanism is not over-claimed as full runtime integrity.

## B6. Dependency gate + operator-decision-needed (both items)

- **Gate:** item 39 after 36+37+38; item 40 after 39; item 9 composes but gates nothing.
- **Operator-decision-needed — FLAGGED:**
  1. *(item 39)* the `maddubs` (saturating) vs `madd_epi16` (non-saturating) path is an engineering
     choice with a proof consequence — **architect recommendation:** non-saturating `madd_epi16`
     (removes the saturation-determinism hazard entirely); flagged, not invented.
  2. *(item 40)* the **runtime self-check cadence** — init-only, periodic canary, or a per-inference
     flag. **Architect recommendation:** init self-check (always, catches corrupted-weights-at-load)
     + periodic canary re-check (catches drift/stuck-compute), with a per-inference-layer-checksum
     mode behind an opt-in flag for max-assurance deployments. Flagged; the "safety over speed"
     ruling favors *more* checking, bounded by the honest limit in B5.7.
