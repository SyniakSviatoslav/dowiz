# Bebop — THE Roadmap (single source of truth)

Status: 2026-09-05 CURRENT (D11-L split: this file = thesis, TG-DONE table, critical
path, open decisions, measured table; HISTORY.md = every pull, task body, decision
D1-D11 with evidence and the superseded 2026-08 vision text; TASKS.md = the generated
ledger; docs/exp.journal = one line per experiment; AGENTS.md = the laws L1-L17)

## Thesis (binding since D11-A, 2026-09-05)

Bebop is a self-hosting, integer-only language for AArch64 with no C anywhere in the
toolchain: a 1.5 KB assembly loader runs `bebop.bin`, which compiles `.bp` source to
raw machine words and compiles itself to a byte-exact fixpoint. Its purpose is one
measurable thing: **the language's object model is its persistent store.** A Bebop
program's persisted objects are its in-memory objects (same layout, object-relative
offsets, no pointers on disk), queries are ordinary compiled functions, and the
compiled code runs within a small factor of Rust on honest kernels. Everything in the
plan serves that sentence and is judged by a number in a committed script:

1. a one-pass compiler whose loops carry no stack-machine words (T96 done: K1 loop
   51 -> 14 words), then temporaries in registers, condition fusion, a real calling
   convention and one integer-exact peephole pass (T101-T104, through a per-fn op-list
   IR per D11-G);
2. a store (docs/LANG-DB-DESIGN.md §4 + §9.5, amended by D11-H): two superblocks +
   append-only arena of self-describing objects, `ref T` = object-relative offset,
   commit = root swap, Cheney compaction, sha256-named migrations, CSR/bucket indexes,
   edge log + tiered CSR for graphs — gated G1-G8 against sqlite, LMDB and native
   Rust on a real workload (D11-I);
3. cores only for parallel scans (T106), the sweep engine only where activity is sparse
   (T107); no runtime cells for ordinary code (T55 spike: 41x/740x against);
4. an honesty floor that cannot be talked around: every gate has an oracle and a
   mutation test, every codegen change re-freezes constructs under a written word
   budget, every branch-census increase is a committed ALLOW line, every capacity is a
   loud trap, every speed number is pinned, in-process, against an honest twin.

The 2026-08 vision (post-von-Neumann substrate, no program counter, eigentime,
SME/SVE2 fusion, reversible logic, multiversal superposition, an x9-x13 typed register
bank, "no SQL, no WAL locks") is preserved verbatim in HISTORY.md as superseded text;
its mathematics survives as 91 oracle-backed gates; its operational claims were measured
and retired by decisions D8-D10.

## TG-DONE (falsifiable; every row is a number in a committed script)

| # | criterion | script / gate | today (2026-09-05) | target |
|---|---|---|---|---|
| 1 | honest kernels, in-process pinned, bebop / Rust twin (D11-C) | bench/vs_rust/honest.sh | K1h, K2h, K3h, K4 rows measured after T118 (see Measured) | every row <= 2.0x after T101-T104; <= 1.0x is the D1(a) stretch, decided on the numbers |
| 2 | linear self-hosting fixpoint | every codegen commit (AGENTS law) | gen3 == gen4 2b6171ce | invariant, never a goal |
| 3 | one oracle per gate, none self-frozen | bench/oracles/run_all.sh | ok=99 self-frozen=0 | 0 self-frozen; 8 gates await T124 fold specs before "== oracle" |
| 4 | zero tolerated miscompiles | construct_parity (38 constructs + neg), fuzz | R3.x deleted; fuzz 150-seed batches clean | fuzz >= 10^5 programs, 0 CRASH / DIVERGE, traps only (TG-DONE 8) |
| 5 | single compiler, single language | attic, construct_parity | expr_compile.bp in attic; 38 constructs | every accepted construct in construct_parity; struct literals + `use` + `ref T` landed (T43 rest, T47, T48) |
| 6 | hardware claims measured, never projected | bench_pinned.sh, REPORT-pinned.md, Measured table below | no projected row remains | stays true |
| 7 | the store | G1-G8 (T112-T117) | library + G1 G2 G4 G5 G3 G6 green (std_golden 99: slayout, sround x2, scompact, scrash, sevolve, sconc; 100/100 SIGKILL trials); G7 sbench and G8 sgraph running | all eight green with numbers; G7/G8 thresholds a,b,c frozen by the operator before the run (D11-I) |
| 8 | fuzz at scale | bench/fuzz/fuzz.sh | 150-seed batches; gen.py widened 2026-09-05 (large loops, literals in loops, recursion 127, return/break) | 10^5 programs, 0 CRASH/DIVERGE, only TRAP-OK/TRAP-8x |

## Critical path (in order; one writer, one commit per single-variable step)

T118 traps (done) -> T122 reserved words -> T43 rest (struct literals + field access)
-> T47 `use` -> T48 checked types with `ref T` -> T101-T104 via the op-list IR
(byte-identical rung first, D11-G) -> T105 sdiv/isqrt -> T106 nn4 (3 A78) -> T107
incremental curve -> T108 .becache -> T109-T117 store (G1..G8, with the D11-H
amendments and a G2-lite spike first) -> T52-T54 predication where measured -> the
rest by ordering text in HISTORY.md. Parallel-safe now: T121 K5/K6 rows, T123
mutation-sensitivity of 15 gates, T124 fold specs, docs.

## Open decisions (operator)

- a, b, c thresholds of the store claim (D11-I) and the real workload W.
- T101-T104: the IR rung passes or falls back to x1-x7 retractions (D11-G rule).
- Whether "<= 1.0x Rust" (D1(a)) stays a target once the honest rows exist.

## Measured (pinned A78, in-process clock_ms medians; every number has a script)

| kernel (bench/vs_rust/bench_pinned.sh) | before T96 (364009e9) | after T96 (3aae4ad8) | Rust twin (black_box in loop) | after / twin |
|---|---|---|---|---|
| K1 sum 1M | 10.0 ms | 3.0 ms | 2.41 ms | 1.24x |
| K2 fib(25) | 2.85 ms | 1.5 ms | 0.277 ms (inlined) | 5.4x |
| K3 300x300 | 1.2-1.5 ms | 0.5 ms | 0.213 ms | 2.3x |
| K4 chain 2M | 32 ms | 12.0 ms | 2.85 ms | 4.2x |
| K1 loop words/iteration | 51 | 14 | 3 | |
| isqrt / fp_div, 1M calls (T105, scratch micro-bench, pinned A78) | 286 ms / 253 ms (restoring loops) | 41 ms / 22 ms (clz Newton / sdiv base-2^k) | | 7x / 10x faster |

Honest twins (bench/vs_rust/honest.sh, D11-C): see bench/vs_rust/REPORT-honest.md
(first run recorded after T118).

| tensor query, 1M points (bench/tq_sqlite/run.sh, T100) | sqlite 3.46 | bebop | bebop faster by |
|---|---|---|---|
| nearest, full scan | 183 ms (python) / ~158 ms native | 18.4 ms | 8.6-9.9x |
| nearest, 3x3 cell index | 55 us (C API incl. ~19 us ctypes) / ~35 us native | 4.0 us | ~9x native |
| nn4 bucketed scan, 1 A78 vs 3 A78 pinned (bench/tq_sqlite/nn4.sh, T106) | seq 219 ms | par 99 ms | 2.21x on 3 cores |

| store vs sqlite 3.46.1 C API via ctypes (bench/vs_rust/sbench.sh, G7/T116, 1M records) | store | sqlite | ratio |
|---|---|---|---|
| insert 1M + index + commit | 880 ms | 15147 ms | 17x |
| PK lookup (10^5) | 450 ns | 10.3 us (ctypes; ~2 us via python's module) | 22.8x / >= 4x native |
| 3x3 cell-window scan (10^4) | 2.7 us | 83 us | 30.7x |
| update 10^5, one transaction | 157 ms | 1000 ms | 6.4x |
| reopen + first record | 590 us | 3600 us | 6.1x |
| logical size after update / after compaction | 85.2 MB / 72.4 MB | 34.1 MB | 2.5x / 2.1x LOSS |
| compaction vs VACUUM | 747 ms | 544 ms | 0.7x |

| substrate (bench/substrate_spike/run.sh, T55 spike) | ms | vs linear |
|---|---|---|
| 12-op fn x 300k, bebop linear | 18 | 1.0x |
| same as runtime cells (bebop sweep) | 738 | 41x slower |
| same sweep engine in Rust (model floor) | 39 | 39x slower than Rust linear (1.0 ms) |

| incremental curve, 2^16-cell DAG (bench/substrate_spike/incr.sh, T107) | sweep / full, us per rep | crossover |
|---|---|---|
| bebop k=1 / 16 / 256 / 4096 | 15/1031, 234/984, 1828/1078, 5281/1109 | k = 256 (0.39% of N) |
| Rust twin k=1 / 16 / 256 / 4096 | 4/132, 50/127, 525/129, 1446/135 | k = 256 (0.39% of N) |

| platform | measured |
|---|---|
| usable A78 cores in this shell | 3 (cpus 4-6; cpu 7 refuses taskset) |
| DRAM bandwidth, one A78 / three | ~12 GB/s / ~12 GB/s |
| process RSS K1-K4, bebop / Rust | 16-17 MB / 16-17 MB |
| self-compile pinned (2026-09-05 re-measured, core 4: T109 binary / T126 binary) | 294.5 s, 94 MB / 292.9 s, 111 MB (the 108.7 s row of 2026-09-04 is not reproducible today; same box, same core) |
| compile of a std gate, cold / warm .becache hit / trivial-program floor (T108, becache_gate.sh) | 346 ms / 113 ms / 106 ms |
| page-cache read fault / CoW fault / msync 1 page / rename (proot) | 0.3 us / 3.5 us / ~100 us / ~270 us |

## Where everything else is

HISTORY.md (all pulls, task bodies, decisions D1-D11 and their evidence, progress
log, the 2026-08 vision text), TASKS.md (ledger), docs/SPEEDUP-ANALYSIS.md (plan
P1-P10, physics), docs/LANG-DB-DESIGN.md (store spec, prior art, §9 tensor-graph
indexing), docs/ROADMAP-CRITIQUE-2026-09-04.md (findings F1-F43), docs/LANGUAGE.md,
docs/TRAPS.md, docs/SESSION-HANDOFF.md, docs/exp.journal.
