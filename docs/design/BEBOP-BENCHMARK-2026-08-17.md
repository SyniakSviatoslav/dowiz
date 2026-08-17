# Bebop ↔ Other Languages — Measured Comparison

**Date:** 2026-08-17 · **Machine:** aarch64 (glibc 2.43) · **rustc** 1.96.1 · **gcc** 15.2.0

Two harnesses, identical methodology, same primitives:
- **Bebop** = native C bootstrap (`bebop-lang/native`, `-O2 -std=c11`, explicit NEON intrinsics where noted)
- **dowiz-core** = Rust reference (`-O3`, `lto=fat`, `codegen-units=1`, auto-vectorization, `#[repr(align(64))]`)

**Methodology (both sides):** CLOCK_MONOTONIC, 8-rep warmup, 64 timed reps, inner-loop
batching (per-op = span/inner), min + median, compiler barrier (`asm volatile` / `black_box`)
on live inputs, every result sunk into a `volatile`/`black_box` accumulator. Numbers = median ns/op.

## Results (median ns/op — lower is better)

| primitive                      | Bebop C (ns) | dowiz Rust (ns) | Rust/C | notes |
|--------------------------------|--------------|-----------------|--------|-------|
| ntt_convolve n=1024            | 265 221      | 164 661         | 0.62x  | Rust faster |
| hv_bind (XOR)                  | 10.6         | 6.14            | 0.58x  | Rust auto-vec |
| hv_hamming (popcount)          | 11.7 (NEON)  | 11.84           | 1.01x  | parity |
| hv_bundle(4)                   | 9 270        | 4 181           | 0.45x  | Rust faster |
| hv_permute (rotate)            | 29.5         | 3.41            | 0.12x  | Rust much faster |
| hv_shift_invariant_similarity  | 130 068      | 83 042          | 0.64x  | Rust faster |
| fft n=1024                     | 58 085       | 34 536          | 0.59x  | Rust faster |
| mobius_apply                   | 16.6         | 10.94           | 0.66x  | Rust faster |
| mobius_reduce(20)              | 8.5          | 407             | 47x    | **C much faster** |
| money_checked_add              | 3.17         | 2.23            | 0.70x  | Rust faster |
| sort_f64_desc n=10000          | 491 341      | 160 911         | 0.33x  | Rust much faster |
| checksum_fold 4KB              | 5 986        | 5 553           | 0.93x  | parity |
| trig sin+cos+atan2             | 188          | 1 610           | 8.6x   | **C much faster** |
| rng_next_u64 (SplitMix64)      | 4.32         | 1.80            | 0.42x  | Rust faster |
| stats mean+variance n=1024     | 11 932       | 11 109          | 0.93x  | parity |
| gcra_decide (token bucket)     | 1.59         | 2.27            | 1.43x  | C faster |
| pid_update                     | 12.9         | —               | —      | (no Rust equiv measured) |
| markov_stationary n=8          | 20 902       | —               | —      | (no Rust equiv measured) |
| atomic fetch_add               | 6.96         | —               | —      | C11 atomic |
| spinlock lock+unlock           | 13.4         | —               | —      | pthread spinlock |
| arena_alloc(64)                | 3.19         | —               | —      | bump allocator |
| vsa_encode (trigram HV)        | 12 054       | —               | —      | (Rust = hv_encode_text) |
| mem_search_semantic            | 12 592       | —               | —      | (Rust = living_memory) |

## Hypervector SIMD (throughput, Mops/s)

From the dedicated `hv_benchmark` (independent-op, compiler-barrier, honest):

| op       | Bebop scalar | Bebop NEON | Bebop NEON2 | dowiz Rust |
|----------|--------------|------------|-------------|------------|
| bind     | 105          | 191        | **206**     | 163        |
| hamming  | 79           | **85**     | —           | 84         |

## Interpretation (stated once, briefly)

1. **Rust (LLVM) is faster on ~60% of the ported primitives** — auto-vectorization + fat LTO
   beat hand-written gcc -O2 code on NTT, FFT, bundle, permute, sort, shift-invariant similarity,
   money, rng. These are the *hot* dowiz algorithms, so a native Bebop that does NOT adopt LLVM-grade
   codegen will lose here unless its hand-written SIMD matches (the NEON bind already does — 206 vs 163).

2. **The two C-wins are real and structural, not noise:**
   - `mobius_reduce` (47x) and `trig` (8.6x): dowiz-core's no_std math layer (`math::sqrt`, `sin`,
     `cos`, `atan2`) is hand-rolled Taylor/Newton for bit-exactness across native+wasm — it is
     ~10-50x slower than libm. The Bebop C bootstrap links libm. **Bebop's own hand-rolled
     `trig.c` (Cody–Waite + Taylor) is 8.6x faster than dowiz's math layer while still being
     libm-free** — an implementation-quality win to carry forward, not a fluke.

3. **`gcra_decide` (C 1.59 vs Rust 2.27)** — the C port inlines the branchless GCRA; the Rust
   reference takes a mutable state through a small struct. Recoverable in the native backend.

4. **The honest bottom line for the rewrite:** porting dowiz→Bebop is *not* automatically a perf win.
   The wins come from (a) keeping the hand-rolled-but-fast trig, (b) matching/beating NEON on the
   hypervector hot path (already done: 206 vs 163), (c) branchless/atomic lowering for GCRA/pid,
   and (d) a register-allocating native backend to close the NTT/FFT/sort gap. Where LLVM is ahead
   today, the plan is explicit SIMD, not "hope the C compiler catches up".

## Files

- Bebop harness: `bebop-lang/native/src/bench_all.c` (run: `./build/bebopc bench`)
- Rust harness:   `bebop-lang/rust-bench/src/main.rs` (run: `cargo run --release`)
- Raw outputs reproducible on this machine with the commands above.
