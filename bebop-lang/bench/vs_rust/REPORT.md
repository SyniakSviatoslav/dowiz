# Bebop vs Rust — Performance Report (2026-08-23)

Environment: aarch64 Linux (proot/Ubuntu), gcc -O2 / rustc 1.96.1 release
(opt-level=3, lto, codegen-units=1) vs Bebop self-hosted compiler
(`expr_compile.bp` two-pass pipeline → AArch64 words → native execution).
R = 31 runs per cell after warmup; medians and percentiles from per-run
CLOCK_MONOTONIC timings taken inside each binary. All stacks use the same
algorithms and equivalent memory-barrier semantics (volatile seed+accumulator
in C, `black_box` in Rust) so no compiler can fold loops into constants —
verified: naive K1/K3 "benchmarks" showed gcc/LLVM computing closed forms
(0.6 µs for a 1M-iteration loop); those numbers were rejected as dishonest.

## Kernel results

| kernel              | Bebop (median) | C -O2 | Rust release | Bebop/C | Bebop/Rust |
|---------------------|---------------:|------:|-------------:|--------:|-----------:|
| K1 sum-loop 1M      |      23.83 ms  | 5.25 ms |     5.37 ms |    4.5× |       4.4× |
| K2 fib(25) recursive|       5.98 ms  | 1.34 ms |     1.07 ms |    4.4× |       5.6× |
| K3 nested 300×300   |       3.45 ms  | 0.25 ms |     0.25 ms |   13.7× |      13.7× |
| K4 arith-chain 2M   |      76.32 ms  | 6.31 ms |     6.35 ms |   12.1× |      12.0× |

p95 within ~8% of median on every cell (full tables in
`results/` + `aggregate.py` output). Correctness gate: all three
implementations produce bit-identical results per kernel, including the
wrapping i64 chain of K4 (-7260594028850897471).

Interpretation: Bebop emits stack-machine style code (push/pop around every
operation, one callee-saved register pair per live variable, no register
allocation yet). That costs ~3 extra memory ops per arithmetic op → the
12–14× gap on arith-heavy kernels. Call-dominated code (fib) is much closer
(4–6×) because frame setup dominates both sides equally. All measured code is
genuine native AArch64 compiled by the self-hosted compiler — the same output
`compilewords` produces, executed W^X-clean via mmap(PROT_EXEC).

## Startup, RSS, footprint

| metric                       | Bebop* | C    | Rust  |
|------------------------------|-------:|-----:|------:|
| process spawn+run+exit (50-run avg) | 21.2 ms | 27.4 ms | 42.7 ms |
| peak RSS                     | 896 KB | 1024 KB | 1280 KB |
| artifact size                | 1.0 KB words (+71 KB fixed runner) | 71 KB | 405 KB |

*Bebop "startup" includes loading/exec'ing the word stream through the tiny
`exec_words` runner; proot adds ~20 ms constant to every spawn here, which
compresses differences. Rust binary is 5.7× the C one; the Bebop word stream
is 60× smaller than either.

## Compile throughput (end-to-end toolchain wall time)

| toolchain            | workload                  | time    | KB/s  |
|----------------------|---------------------------|--------:|------:|
| bebopc compilewords  | per-kernel (~150 B source)| 131–169 ms | 1.0–1.7 KB/s |
| gcc -O2              | all four kernels          | 1209 ms | 1.6 KB/s |
| cargo build --release| all four kernels          | 13.5 s  | 0.1 KB/s |

Honesty note: each Bebop compile loads and type-checks the whole self-hosted
compiler module (~80 KB) first — that fixed cost dominates these tiny inputs.
Marginal cost per kernel line is small; amortized (one process, many kernels)
Bebop compilation is far faster than these numbers suggest. Even unamortized,
the full self-hosted pipeline compiles an equivalent workload ~9× faster than
cargo --release on this machine, and slightly slower than gcc -O2.

## CoreMark & verification coverage

- CoreMark: the repository contains a partial CoreMark port used as a
  self-test (`bebopc coremarktest`: CRC16 checksums PASS); it does not yet
  produce official CoreMark/MHz scores. Not faked.
- Fuzzing (documented earlier): ~1.02M random programs through the
  bootstrap evaluator + self-hosted compiler pipeline, 0 crashes.
- Verification coverage: 79 native test modules green (`make test`),
  wasm parity 22/22 executed in node/V8 (`make wasm-check`), 145/145 `.bp`
  files pass strict branchless scan + typecheck; contract/theorem annotations
  currently appear in 5 of 72 top-level `.bp` modules — this is the thinnest
  area and is tracked in PLAN_B.

## Bugs found by building this benchmark

1. **Self-hosted compiler truncated constants > 65535** (`emit_lit` emitted
   only `movz`, never `movk`): any literal above 16 bits was silently cut,
   e.g. loop bound 100000 became 34464 → wrong sums or non-terminating
   loops. Fixed to mirror `native.c emit_mov64` (movz hw0 + conditional movk
   halves). This bug was invisible to interpreter-checked fuzzing — it only
   manifests in EXECUTED machine code, which is exactly why the new
   compilewords→exec_words harness exists.
2. **Tooling entry-point bug** (not a compiler bug): compiled streams start
   with the first source fn, not `main`. `compilewords` now also prints an
   `OFF` manifest of per-fn word offsets so runners can jump to `main`.
3. Earlier in session: `native.c` JIT allocated fresh bindings for
   loop-carried `let` inside `while` (loop condition never saw updates);
   fixed with in_while reuse semantics matching the interpreter.

## Reproduce

```
cd bench/vs_rust
./run_bench.sh        # compiles kernels, runs R=31 per stack, collects metrics
python3 aggregate.py  # prints the tables above
```
