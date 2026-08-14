# Kernel-Space Migration — Progress Ledger

Goal: move everything possible into the kernel (no_std / native). This file is
the single source of truth for what is DONE vs REMAINING. Updated 2026-08-13.

## Tier audit (subagent, exact)

- **kernel-core = 174** modules — pure `core`+`alloc`, no fs/net/thread/process/
  time/Mutex/HashMap-default. Ready to extract to `#![no_std]`.
- **boundary = 85** modules — only `std::time` OR `HashMap`-default-hasher.
- **user-space = 43** modules — fs/net/thread/process.

## DONE (mechanical no_std-readiness)

1. **`extern crate alloc;`** at `src/lib.rs` root.
2. **core::/alloc:: rewrite** (129 files) — `std::f64::consts`, `std::cell`,
   `std::cmp`, `std::mem`, `std::fmt`, `std::hint`, `std::sync::atomic`,
   `std::convert`, `std::ops`, `std::marker`, `std::num`, `std::borrow`,
   `std::slice`, `std::str`, `std::char` → `core::`; `BTreeMap/BTreeSet/VecDeque/
   BinaryHeap/LinkedList/Vec/String/ToString/Box/Rc` → `alloc::` (Cow/ToOwned
   → `alloc::borrow`). Zero behavior change; 3175 tests green.
3. **Clock/WallClock abstraction** (`src/clock.rs`) — `std::time` seam; kernel
   port swaps the impl (`ktime_get_ns`), not the call sites. `now_ns()`
   (monotonic) + `now_ms()`/`now_epoch_s()` (wall clock, preserves legacy
   semantics).
4. **FxHash** (`src/fxhash.rs`) — deterministic multiply-xor hasher +
   `FxBuildHasher` + `FxHashMap`/`FxHashSet` aliases. Replaces OS-entropy
   `RandomState` (determinism goal; the alias itself stays std because `HashMap`
   the container is std-only — use `BTreeMap` for no_std).
5. **HashMap/HashSet → BTreeMap/BTreeSet** (81 files) — the 60 boundary modules
   swapped to alloc-clean, deterministic (sorted) maps. 13 key types gained
   `PartialOrd, Ord`. `laplacian_eqc_parity` reverted (calls external eqc-rs
   HashMap API).
6. **`dowiz-core` crate split — transcendental geometry layer (2026-08-14)** —
   the `no_std` `dowiz-core` crate now holds the self-contained geometry modules,
   re-exported at the kernel root so `crate::{complex,fft,spherical,modular,trig}::…`
   keep resolving unchanged:
   - `complex` — `Complex` (extracted from `spectral.rs`).
   - `trig` — `Phase`/`Xyz`/`PhaseVector`.
   - `modular` — Möbius / PSL(2,Z).
   - `fft` — radix-2 Cooley–Tukey.
   - `spherical` — Legendre/spherical harmonics/Lebedev/structure factor.
   - `math` — correctly-rounded `sqrt`/`fma` + bit-exact glibc `hypot` + ~1-ULP
     `sin/cos/atan2/acos/floor/ceil/round` (the no_std libm replacement; the
     `eig2x2_bit_capture_oracle` golden signatures stay bit-exact — see
     `crates/dowiz-core/src/math.rs` + `tests/bitdiff.rs`).
7. **`dowiz-core` crate split — sanitize + stem + eigen (2026-08-14)** — the
   first tranche of the "no_std-READY but not moved" kernel-core modules:
   - `sanitize` — `sanitize_f64`/`sanitize_f32`/`sanitize_normalized` (the
     fail-closed boundary sanitizers; previously crate-root free fns).
   - `stem` — the 50-language light stemmer (self-contained leaf, 114 tests).
   - `eigen` — `Eigen`/`EigenDecomp`/`decompose` (26 tests); routes `sqrt`
     through `crate::math`, stability classification through `trig::Phase`.
   Kernel re-exports keep `crate::sanitize_f64` / `crate::stem` / `crate::eigen`
   resolving unchanged. dowiz-core 222 lib tests + 4 bitdiff; kernel 2969 lib
   tests green.

## REMAINING (architectural, dedicated sessions)

1. **`dowiz-core` crate split — remaining kernel-core modules** — move the rest
   of the 174 kernel-core modules into `#![no_std]` `dowiz-core`. The
   transcendental geometry layer is DONE (item 6 above); the remaining modules
   are the bulk (order/money/domain, retrieval, etc.). Requires: resolve
   `crate::` cross-refs, add a no_std target (`thumbv7em-none-eabi` or
   `wasm32-unknown-unknown`), handle `format!`/prelude via `alloc::prelude`.
2. **Transcendental modules still in-kernel (no_std-READY but not moved)** —
   **DONE** (all extracted): `spectral`, `householder`, `csr`, `eigen`, `stem`,
   `arena`, `mat` (items 6–10). The only remaining spectral-cluster modules in
   the kernel are the std-coupled consumers (`spectral_cache` — a `&mut`
   eigensolve cache; `order_machine` — power-iteration ρ) which stay kernel-side
   by design (they depend on the kernel's cache/FSM machinery).
3. **Mutex → spinlock** (DONE 2026-08-14) — hand-rolled zero-dep
   `SpinLock<T>` (`src/spinlock.rs`, test-and-set on
   `core::sync::atomic::AtomicBool`) replaced `std::sync::Mutex` in the 4
   non-thread modules: `breaker/audit`, `breaker`, `ports/agent/admission`,
   `retrieval/memory_store`. Poisoning is preserved (`lock() ->
   Result<SpinLockGuard<T>, Poisoned>`, set on unwind in the guard's `Drop`), so
   every existing `.lock().map_err(|_| E::Poisoned)?` / `.ok()?` / `match …
   Err(_)` call site compiled unchanged — zero error-enum changes. The only
   std touch-point is `std::thread::panicking()` (cfg-gate it on no_std
   extraction). The other 3 (fdr/mod, span_metrics/obs, token_bucket) are
   thread-based → stay std.
4. **43 user-space ports** — fs→VFS (`std::fs` → trait), net→sk_buff,
   thread→kthread, process→kexec. These are NOT mechanical; each needs a trait
   seam like `Clock`.
5. **`dowiz-core` crate split — arena + mat (2026-08-14)** — extracted the two
   spectral-cycle leaves:
   - `arena` — `BumpArena` (bump/region allocator) + `HugePageHint`; the
     `count-allocs` counting-allocator harness stays kernel-side (it installs a
     `#[global_allocator]`, std-only) as a thin wrapper
     (`kernel/src/arena.rs` = `pub use dowiz_core::arena::*` + `counting_alloc`).
   - `mat` — `Mat`/`MatrixError`/`matmul_contig(_in)`; pure `core`+`alloc`.
   Kernel re-exports keep `crate::arena::BumpArena` / `crate::mat::Mat`
   resolving unchanged. dowiz-core 230 lib tests (+7 arena); kernel 2971.
6. **`dowiz-core` crate split — csr + householder + span seam (2026-08-14)** —
   broke the `csr`⇄`spectral` cycle and extracted the Householder eigensolver:
   - `span` — the no_std no-op span seam (`info_span!` + `SpanHandle` +
     `SpanGuard`, tracing-grammar parity). Replaces `fdr::info_span!` for the
     eigenvalue family; its `Drop` reports nothing (no observer in kernel-space).
     This is the "fdr span seam (like the Clock seam)" the ledger called for.
   - `csr` — `Csr`/`LaplacianKind`/`recall_at_k`/`precision_at_k`/`NormalizedTile`
     etc.; `sqrt` → `crate::math`; `Csr::energy()` dropped and re-homed as
     `spectral::csr_energy(&Csr)` (the only production csr→spectral edge).
   - `householder` — the dense QR eigensolver (`eigenvalues_contig`/`eigh_contig`/
     `Matrix32x32`); `use crate::spectral::Complex` → `crate::complex`; FMA
     runtime-detection stays `cfg(feature = "std")`-gated (known-but-false via
     `check-cfg`). `eig2x2_bit_capture_oracle` golden signatures unchanged.
   Cross-module parity tests (csr×spectral, householder×spectral) relocated to
   `kernel/tests/{csr,householder}_spectral_parity.rs`. dowiz-core 257 lib
   tests (+18 csr, +9 householder); kernel 2934; +2 integration tests (8 total).
7. **`dowiz-core` crate split — spectral + sort (2026-08-14)** — extracted the
   whole spectral engine (the last transcendental-cycle module) and the float-key
   sort helpers:
   - `spectral` — `charpoly`/`roots`/`eigenvalues` (Faddeev-LeVerrier +
     Durand-Kerner), `eigh`/`topk_symmetric(_in)`, `spectral_radius`/`slem`/
     `spectral_gap`/`graph_energy`/`csr_energy`/`graph_spectrum`/`laplacian`/
     `algebraic_connectivity`/`classify_drift(_phase)`/`spectral_drift`/
     `graph_energy_report`/`dominant_period` + `DmdRank1Rls`. Edge-breaks:
     `eigh`/`topk_*` return `(Vec<Vec<f64>>, Vec<f64>)` instead of
     `spectral_cache::Decomp` (a tuple alias), `fdr::info_span!` → `span::`,
     `order_machine` cross-check test relocated to
     `kernel/tests/order_machine_fsm_crosscheck.rs`. The std-coupled consumers
     (`spectral_cache`, `order_machine`) stay kernel-side.
   - `sort` — `sort_by_f64_desc`/`asc` (from the kernel crate root), re-exported
     so all 17+ `crate::sort_by_f64_*` call sites resolve unchanged.
   dowiz-core 317 lib tests (+60 spectral); kernel 2873; +3 integration tests.

## Honest scope note

The mechanical tier (boundary) is **fully migrated**. The transcendental geometry
layer, sanitize/stem/eigen, arena/mat, csr/householder (+ the `span` seam), and
the full `spectral` engine (+ `sort`) are **extracted** to `dowiz-core`; the
Mutex→spinlock sites are **done**. The remaining items are architectural: the
bulk crate split (move the rest of the 174 modules) and the 43 I/O ports. All are
large, low-mechanical-effort, high-care efforts — not bulk-editable. This ledger
marks the exact boundary so a future session resumes cleanly.
