# REPORT-honest — the D1(a) column (bench/vs_rust/honest.sh, D11-C)

Status: 2026-09-06 CURRENT (first committed run; rerun after every codegen step, replace this file)

# honest twins (D11-C), in-process pinned core 4, R=11, REPS=100 per run, bebop.bin 94e47998
| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | target >= 1.0x (T83) |
|---|---|---|---|---|
| K1H | 2.07 / 2.15 | 1.132 / 1.189 | 1.8x | UNMET |
| K2H | 1.30 / 1.45 | 0.397 / 0.438 | 3.3x | UNMET |
| K3H | 0.65 / 0.72 | 0.194 / 0.272 | 3.4x | UNMET |
| K4 | 5.26 / 5.42 | 3.259 / 3.299 | 1.6x | UNMET |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.67 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

Method (2026-09-06): every kernel runs REPS=100 reps in-process and returns the TOTAL
clock_ms (1 ms coarse) — the table divides by 100, so a row has 0.01 ms resolution; the
Rust twins run the same 100 reps with the carried accumulator through black_box at every
rep boundary and print ms per rep from Instant. K4 got an honest twin (rust_once/k4h.rs,
black_box only on input/output) — 3.26 ms here vs 2.85 ms for the old black_box-in-loop
k4.rs, so the old number was not unfair. K5 is measured cold (the .becache replay removed
before each of 3 runs). Before this change the bebop column had 1 ms resolution and K3h
read "1.0 ms vs 0.124 = 8.1x" — a ceiling, not a measurement.

Run-to-run: an earlier run the same hour at bebop.bin 70cddb59 gave K1H 1.64 / K2H 1.22 /
K3H 0.62 / K4 4.36 ms with Rust 0.879 / 0.356 / 0.170 / 2.629 — both columns move ~20%
together (DVFS on the pinned A78), the ratios stay 1.8-1.9x / 3.3-3.4x / 3.4-3.6x / 1.6-1.7x.
The ratio is the claim; TG-DONE 1 target is <= 2.0x on every row after T101-T104: K1H and K4
are inside, K2H (calls: frame 16 KiB + spills, P4) and K3H (nested loop, no condition fusion
on the inner compare, P3) are the two rows the codegen steps must move.

## B1 row (2026-09-06, session 17): prologue/epilogue/call-site right-sizing, bebop.bin 1a3b2cc2

Status: measured after B1 landed (R=11, pinned core 4, box idle, fuzzd paused). Ratio is the claim: K2H 3.8x -> 2.6x, K3H 4.0x -> 2.4x (D14 item 1 predicted ~2.7x for K2H); bin_words 74804 -> 74222 (-0.8 %), k2h_loopwords 65 -> 51. Both columns sit ~20 % above the session-15 run (DVFS), K1H/K4 unchanged in ratio.

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 2.01 / 2.20 | 1.147 / 1.232 | 1.8x | MET | 1.8x | 16.2 |
| K2H | 1.13 / 1.19 | 0.443 / 0.491 | 2.6x | UNMET | 2.6x | 16.2 |
| K3H | 0.70 / 0.79 | 0.293 / 0.359 | 2.4x | UNMET | 2.4x | 16.2 |
| K4 | 4.70 / 4.77 | 3.284 / 3.383 | 1.4x | MET | 1.4x | 16.2 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.83 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |
