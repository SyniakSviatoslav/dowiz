# Bebop vs Rust — R6.3 Bench (2026-09-02)

## Rerun 2026-09-03 (final status of the roadmap-pull session)

Same method, same box, back-to-back, medians of 31 runs, the compiler at
the while-boundary-fix state (bebop.bin md5 be922030..., fixpoint
bb2==bb3, std_golden 48/48, parity 9/0, construct 24/24):

| kernel              | Bebop (median) | Rust (median) | Bebop/Rust | Bebop p95 |
|---------------------|---------------:|--------------:|-----------:|----------:|
| K1 sum-loop 1M      |      34.00 ms  |      7.99 ms  |      4.3×  |  44.00 ms |
| K2 fib(25)          |       4.40 ms  |      2.11 ms  |      2.1×  |   5.00 ms |
| K3 nested 300×300   |       2.10 ms  |      0.56 ms  |      3.7×  |   2.30 ms |
| K4 arith-chain 2M   |      62.00 ms  |      9.12 ms  |      6.8×  |  73.00 ms |

vs the 2026-09-02 numbers: K1 5.3×→4.3×, K2 3.0×→2.1×, K3 6.9×→3.7×,
K4 9.7×→6.8×. The Rust medians also moved (5.82→7.99 on K1) — the box
remains load-sensitive; treat the direction, not the exact cells. The
R6.2 constant fold is NOT in this build (reverted per journal
1788285630); these numbers are the honest stack-machine baseline.

Environment: aarch64 Linux (proot/Ubuntu, the same box as the 2026-08-23
report). Bebop = the self-hosted compiler at the R6.1+guards state
(bebop.bin md5 6cd1ab23..., fixpoint byte-exact, std_golden 44/44,
parity 9/0, construct 24/24). Rust = rustc release build
(`bench/vs_rust/rust/target/release/kernels`, opt-level=3, lto).

## Method
- Bebop: in-process `clock_ms()` delta around the compute, ×10 scaling
  (0.1 ms units), repeat factors k2×20 / k3×50 for timer resolution.
  31 runs per kernel after a warmup run; the median reported. The kernels
  are byte-for-byte the same algorithms as the parity kernels
  (`bench/vs_rust/kernels/k1..k4.bp`) with only the timing wrapper added;
  the correctness of the unwrapped kernels is gated by parity (9/0).
- Rust: the twin's internal CLOCK_MONOTONIC ns timing (`kernels <n> 31`),
  31 runs, median. `black_box` prevents constant folding on the Rust side.
- Both run back-to-back on the same box in this session.

## Results (medians, 31 runs)

| kernel              | Bebop (median) | Rust (median) | Bebop/Rust | Bebop p95 |
|---------------------|---------------:|--------------:|-----------:|----------:|
| K1 sum-loop 1M      |      31.00 ms  |      5.82 ms  |      5.3×  |  40.00 ms |
| K2 fib(25)          |       4.10 ms  |      1.36 ms  |      3.0×  |   4.90 ms |
| K3 nested 300×300   |       2.10 ms  |      0.30 ms  |      6.9×  |   2.40 ms |
| K4 arith-chain 2M   |      63.00 ms  |      6.52 ms  |      9.7×  |  78.00 ms |

## Comparison with the 2026-08-23 report
- K2: 5.6× -> 3.0× (narrowed)
- K3: 13.7× -> 6.9× (narrowed)
- K4: 12.0× -> 9.7× (narrowed)
- K1: 4.4× -> 5.3× (widened; the August run measured 23.83 ms on the same
  kernel shape — today's median is 31 ms. The box has been reboot-cycling
  under memory pressure this session; treat the K1 delta as env-noise until
  reproduced.)

The gap narrowing on K2/K3/K4 tracks the emitter work since August
(prologue NOP elision OPT-A/G1, the R3.x parse fixes, the R6.1 model
threading with guarded bookkeeping).

## Honest caveats
1. Bebop still emits stack-machine code (push/pop around every operation);
   no register allocation yet. R6.2's model-driven constant folding is
   designed and probe-validated but NOT in this build (reverted per
   L14/L15 — journal 1788288252), so these numbers do NOT include it.
2. clock_ms resolution is 1 ms; the k3 measurement (2.1 ms) carries ±0.1 ms
   quantization after the ×50 repeat + ×10 scale — the ratio for k3 is the
   least precise cell.
3. No C column in this rerun (the August C numbers remain in REPORT.md;
   the C twins were not rebuilt on today's unstable box).
4. Correctness gate: the unwrapped kernels pass parity 9/0 with the frozen
   EXPECT values, including the wrapping i64 chain of K4.

## Raw data
- `results630/k{1,2,3,4}t.bebop.txt` — 31 in-process deltas (0.1 ms units).
- `results630/k{1,2,3,4}.rust.txt` — 31 internal ns timings.
- `results630/summary.json` — medians, ratios, p95.
- `bench630/k{1,2,3,4}t.bp` + `bench630/run_bench.sh` — reproducible.
