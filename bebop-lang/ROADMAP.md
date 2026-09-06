# Bebop — THE Roadmap (single source of truth)

Status: 2026-09-06 CURRENT (D11-L split: this file = thesis, TG-DONE table, critical
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
   51 -> 14 words), then the register model (2026-09-06: tags over x0..x7, no runtime push/pop, retarget
   instead of peepholes -- docs/REGISTER-MODEL-BLUEPRINT.md), csel, LIN recurrence folding,
   pointer-free data and a flat per-fn IR only where the self-compile measurement earns it;
2. a store (docs/LANG-DB-DESIGN.md §4 + §9.5, amended by D11-H): two superblocks +
   append-only arena of self-describing objects, `ref T` = object-relative offset,
   commit = root swap, Cheney compaction, sha256-named migrations, CSR/bucket indexes,
   edge log + tiered CSR for graphs — gated G1-G8 against sqlite on a real workload
   (D11-I; D14 item 7 drops LMDB and native Rust from this sentence — neither is
   measured by any script in the tree);
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
| 1 | honest kernels, in-process pinned, bebop / Rust twin (D11-C) | bench/vs_rust/honest.sh | 2026-09-06 (REPORT-honest.md, 94e47998, REPS=100): K1H 1.8x, K2H 3.3x, K3H 3.4x, K4 1.6x | every row <= 2.0x after T101-T104; <= 1.0x is the D1(a) stretch, decided on the numbers |
| 2 | linear self-hosting fixpoint | every codegen commit (AGENTS law) | gen3 == gen4 c3f58e8e (2026-09-06) | invariant, never a goal |
| 3 | one oracle per gate, none self-frozen | bench/oracles/run_all.sh | ok=99 self-frozen=0; every gate mutation-sensitive (T123 sweep 2026-09-06, tools/mutate_gate.sh) | 0 self-frozen; T124 fold specs written (docs/FOLDS.md) |
| 4 | zero tolerated miscompiles | construct_parity (52 constructs incl. neg), fuzz | R3.x deleted; DIVERGE-20056 (2026-09-06) was an undefined program (loop-release rule now in LANGUAGE.md, bpref raises, gen.py never emits it); DIVERGE-42122 (2026-09-06) WAS a miscompile — a 9+-param callee whose body never touched x15 wrote its 9th param into the caller's spill slot (OPT-G1 scan started after the param stores) — fixed, construct c53_param9 | fuzz >= 10^5 programs, 0 CRASH / DIVERGE, traps only (TG-DONE 8) |
| 5 | single compiler, single language | attic, construct_parity | expr_compile.bp in attic; 38 constructs | every accepted construct in construct_parity; struct literals + `use` + `ref T` landed (T43 rest, T47, T48) |
| 6 | hardware claims measured, never projected | bench_pinned.sh, REPORT-pinned.md, Measured table below | no projected row remains | stays true |
| 7 | the store | G1-G8 (T112-T117) | library + G1-G6 green in std_golden (99 gates; 100/100 SIGKILL trials); G7 sbench measured vs sqlite (17x insert, 450 ns PK lookup, 30x window scan, 2.5x size loss); G8 stage 1 measured (BFS 187 ns/edge vs sqlite 10.8 us), stage 2 running | all eight green with numbers; G7/G8 thresholds a,b,c frozen by the operator before the run (D11-I) |
| 8 | fuzz at scale | bench/fuzz/fuzz.sh | 150-seed batches; gen.py widened 2026-09-05 (large loops, literals in loops, recursion 127, return/break); 2026-09-06 generator fixed (fresh loop names, big-loop vars never reassigned, loop-bound literals released) -> 1.5 programs/s, 0 timeouts on 20 seeds; 2026-09-06 in-process shards (fuzz_batch.py): 3.5 programs/s on 3 cores, 300-seed batch found 42122 | >= 10^5 seeds ON THE PROMOTED BINARY (D12-C: docs/PERF.md `fuzz_seeds_on_bin`, keyed by md5; 28.5k on d785e062, e14dd55e counting), 0 CRASH/DIVERGE/COMPILEFAIL, TRAP-82 = 0 (ALERT class since D12-C), capacity traps 80/81/83 only |

Row 1's target (D14 item 10, wasmtime's own baseline-vs-optimising numbers): one-pass
(baseline) compiler tiers land at 1.1x-1.5x of an optimising tier generally, so <= 2.0x per
honest row is the real target for this compiler shape; D1(a)'s <= 1.0x stays report-only
(D12-G), not a gate.

## Critical path (in order; one writer, one commit per single-variable step)

Rewritten 2026-09-06 (session 18) by the main session after the four research reports
(docs/RESEARCH-DEPS, -TENSOR, -NOPOINTERS-SQL, -GRAPHBLAS -2026-09-06.md) and the operator's
decisions of the same day: the operand-tag-stack IR rung is replaced by the register model in one
commit; the Salsa-like per-fn memo is required; harness dogfooding is measured-first; the pointer-
free data model, u32 cells and the tensor-graph DB over the store are on the path; and -- operator
2026-09-06, binding -- **multi-writer parallelism, instant start (no first-shape compile latency),
multi-core execution, rank-n tensors and GraphBLAS-speed transformations are MANDATORY properties of
the store, not optional "after" items**. Every task below has an executable blueprint in
docs/blueprints/NN-<name>.md written by the main session (AGENTS.md L22); the Sonnet worker executes
them in this order, one chain-gated commit per step, and never two codegen tasks in flight.
Duplicates removed: B4 (frame size) is inside A6; T48 types are inside A8; the NEON items of the
tensor report are one task A9; the specialise-then-run twin, the SpGEMM-join twin and B8 (profile the
CSR build) are one measured-first task B2; the compiled planner (14a-3), the DSL (6b-4) and item 6 are
one task B7; group commit / st_verify / recovery rows are inside B1; "multi-writer only if W demands"
is superseded by B5 (mandatory).

### Phase A -- compiler substrate (every task through `tools/chain.sh --codegen`, constructs re-frozen)

| # | task | blueprint | gate (the number) | depends on |
|---|---|---|---|---|
| A1 | register model, one commit (in flight) | docs/REGISTER-MODEL-BLUEPRINT.md | push_words == 0; k4 <= 13, k3h <= 10, k1h <= 8, k2h <= 30 loop words; k4_ms <= 1.15x Rust twin; c55-c64 | -- |
| A1b | fntab relayout + fn cap 512 (the old compiler's `offs/fnames/fpos/sizes/starts = zeros(256)` and the 3*cnt+1 zones; bebop.bp is at ~250 fns and every later task adds fns) -- one byte-identical commit (gen2 == gen3 == gen4 md5 unchanged) | docs/blueprints/A2-csel-and-const-hoist.md step 0 | fixpoint md5 unchanged; a 300-fn synthetic program compiles; exit 89 above 512 | A1 |
| A2 | T52 `csel` on FLAGS/REG tags for pure `if` arms; + hoisting of 64-bit loop-invariant constants as the optional second commit if K8H > 1.2x after csel | docs/blueprints/A2-csel-and-const-hoist.md | K8H <= 1.2x Rust (honest.sh row) | A1b |
| A3 | LIN tag: folding of linear recurrences over k = 2/4 iterations on tags (exact in wraparound i64) | docs/blueprints/A3-lin-recurrence-folding.md | k1h_ms <= 0.5x Rust, k4_ms <= 0.6x Rust, bpref parity on std_tests | A1 |
| A4 | 24 h fuzz freeze window (process, no code): fuzzd on the promoted md5 until fuzz_seeds_on_bin >= 10^5 | docs/blueprints/A4-fuzz-freeze.md | 0 CRASH/DIVERGE, TRAP-82 = 0 | A1-A3 |
| A5 | pointer-free step 1: x17 arena-relative addressing over one PROT_NONE reserve (`zeros` returns an index; `ldr xd,[x17,xidx,lsl #3]`; file maps placed inside the reserve so store bases are indices too; the frame heap is carved from the reserve here so aggregate indices are never negative; arena image = file) | docs/blueprints/A5-arena-relative-addressing.md | chain GREEN; c65_index_roundtrip, c66_ptrfree; K5 <= +8 %, K6 ns/row <= +20 % | A3 |
| A6 | pointer-free step 2: delete x14 (aggregates already index the reserve after A5) + fn-mark LIFO release of literal-only fns + computed frames (B4): T118 / exit 81 deleted, frame = 80 + 8*marks + 8 + 8*slots, emit_bl saves x15 only | docs/blueprints/A6-aggregates-in-arena-and-frames.md | c67_deeprec (10^5 recursion), fuzz TRAP-81 = 0, TRAP-82 = 0, RSS row | A5 |
| A7 | pointer-free step 3: byte arena + `str` as a value `(off<<32 | len)` + raw-byte parser + crc32x per page (= raw ingest) | docs/blueprints/A7-byte-arena-and-str-values.md | c68_strval; 100 MB ingest twin: raw <= 1.5x best Rust, maxrss <= 1.5x file size | A6 |
| A8 | typed tables: T48 checked types in bebop.bp (`[i64]`, `[u32]`, `str`, `ref T`) + u32 cells for CSR/store | docs/blueprints/A8-typed-tables-u32.md | G7 file size <= 1.2x sqlite; K6 ns/row -2x; typecheck oracle == bpref | A7 |
| A9 | NEON/hardware builtins: `scan` (parser loops), `cmp_mask`/`sum64`/`fill` (scans, CSR build), `umulh` (Q32 plus-times) | docs/blueprints/A9-neon-builtins.md | K5 -10 % (scan); K6 ns/row <= 4 with a Rust scan twin; each builtin a construct + bpref stub | A1 (scan any time), A8 (cmp_mask on u32) |
| A10 | per-fn memo (Salsa-like): fn words memoised by hash(text, facts), `bl` re-link from the layout table | docs/blueprints/A10-per-fn-memo.md | one-fn-edit self-compile <= 0.3 s, fixpoint md5 unchanged | A1 |
| A11 | harness dogfooding: (a) spawn census + perf rows chain_spawns (codeless), (b) bebop-native std-runner lane (execve/wait4/pipe2/dup3/kill/run builtins) with std_golden.sh as oracle for 3 chains, (c) chain.bp after a golden-runner decision | docs/blueprints/A11-harness-dogfooding.md | (a) decision gate 20 % of battery wall; (b) byte-identical lane summaries x3 | A1; (b) after A7 (str values) |
| A12 | flat per-fn index IR (rows {op,a,b,aux}, blocks as ranges; passes fold/lin/cse/sroa/inline/tre/dce) then rung 2: graph register allocation (Hack/Goos) replacing the window | docs/blueprints/A12-flat-ir-and-graph-ra.md | rung 1: K5 -15 % (measured by hand first), K2H ~1.0x; rung 2: K5 -3 % more, bin_words -2 % | A5-A8 (alias facts from typed tables) |

### Phase B -- the tensor-graph DB over the store (mandatory properties: multi-writer, instant start, multi-core, rank-n, GraphBLAS-speed transformations; zero dependencies)

| # | task | blueprint | gate (the number) | depends on |
|---|---|---|---|---|
| B1 | durability: G5b torn-write harness (SQLite atomiccommit sector model) + `sys_fsync` of the directory after the compaction rename + group commit + `st_verify` + recovery row | docs/blueprints/B1-durability-torn-write.md | 1000 torn trials, 0 invalid reopens (same harness over sqlite WAL); commits/s row; recovery row | none (python part any time; sys_fsync after A1) |
| B2 | (only on the promoted A1 binary; before that the rows measure the stack machine) measured-first twins that decide the thesis: (i) 2-way join as SpGEMM (1M x 1M, uniform + Zipf) vs sqlite native vs Rust HashMap/sort-merge; (ii) specialise-then-run scan vs Rust generic scan (latency to result incl. compile); (iii) B8 profile of the CSR build | docs/blueprints/B2-decisive-twins.md | join >= 10x sqlite AND >= 0.7x best Rust on both key distributions; scan twin rows first/repeat; CSR build profile row | A1 (register model) |
| B3 | (needs the in-process `run(bin)` builtin of A11(b) FIRST -- land it as a standalone ~15-word builtin before B3; until then the pool falls back to fork+exec, ~8 ms, and the <= 1 ms gate is measured only after it) `gb.bp` + `gen_gb.bp`: GbMatrix/GbVector as store objects, formats CSR/bitmap/iso, masks, and the generated semiring kernels (mxv push/pull, mxm Gustavson+SPA, eWiseAdd/Mult, select, apply, reduce, extract, transpose) with **instant start**: a PreJIT kernel pool (all <= 36 (op, semiring) kernels compiled ahead into the store image, digest-keyed) plus a tier-0 generic kernel (semiring by id, no compile) used until the specialised one exists, compiled in the background | docs/blueprints/B3-graphblas-kernels-prejit.md | G9a round-trip; G9b LAGraph-style folds BFS/PR/TC/CC/SSSP == python oracle; first-query latency <= 1 ms (tier 0) and specialised <= 50 ms; each kernel a construct | B2 gate, A9 (umulh, cmp_mask, sum64) |
| B4 | purely functional tensor updates: tail COO + L0 + L1 with row-block CoW (2-level blocktab), eWiseAdd block merge at commit, `prev` for time-travel, compaction = GC of unreachable versions (= sgraph stage 3) | docs/blueprints/B4-functional-tensor-updates.md | G9c: 1M single-row updates amortised <= 0.5 us, max stall <= 10 ms, folds == oracle every 10^4, kill -9 during merge (G5 harness); update twin vs sqlite WAL UPDATE | B3 |
| B5 | multi-writer (mandatory): partitioned writers (one writer thread per matrix partition / arena), per-partition roots under one global root swap, cross-partition atomic commit by in-process 2PC over futex, STM validation of read-sets for cross-partition transactions, single-writer mode kept as the degenerate case | docs/blueprints/B5-multi-writer.md | G10: 3 writers x 10^5 updates on disjoint partitions, linearisable folds == oracle; cross-partition commit atomic under kill -9 (G5 harness); throughput >= 2x single writer on 3 A78 | B4 |
| B6 | multi-core execution (mandatory): row-range partitioning of mxv/mxm/scan/reduce over 3 A78 via clone/setaffinity work queues, reduce merge, NUMA-free but DRAM-aware (stop scaling at the bandwidth ceiling; step 0 measures whether clone threads count toward the proot/boxguard proc cap, else the battery goes RED under PROC_CAP=30) | docs/blueprints/B6-multi-core-kernels.md | K6 scan x1.4-2.2 on 3 cores; BFS/PR rows; no lost updates with B5 | B3, B5 |
| B7 | associative-array DSL + planner (`q { from T where p group by k agg s join U on k }` -> AST -> access path + join order (k <= 4) -> gen_gb -> kernel) + compiled Q6/Q1/join twins; rank-n = mode-ordered CSR chosen by the planner | docs/blueprints/B7-dsl-planner.md | Q6 >= 10x, Q1 >= 5x sqlite native; first/repeat latency rows; rank-3 construct | B3, B6 |
| B8 | (last: needs A7 for ingest and every B item) W end to end: the dowiz-core order log (T66 ordfsm/money oracles) on the tensor-graph DB -- ingest (A7), FSM as a 12x12 matrix, queries via B7, updates via B4/B5, durability via B1; the acceptance of the thesis | docs/blueprints/B8-workload-W.md | byte-exact Rust oracles; the G-rows of this workload in REPORT-honest / RESULT-sgraph | B1-B7 |

### Still open after A/B (design-bound, operator decision first): T61 (pool library + gate), T68-T70, T85 -> T86, T73, T76, T49/T50, T56, T59. Project-sized items stay under HISTORY.md `## PARKED` (D14 item 11).

Parallel-safe at any time: docs, oracles, fuzz batches, honest.sh rows, T78/T79/T81/T82 tooling.

## Open decisions (operator)

- (decided 2026-09-06, HISTORY D13: all 12 retro proposals of docs/RETRO-SESSIONS-2026-09-06.md §5
  are work items; process-count gate, FREEZE-on-codegen, pkill block first)
- (decided 2026-09-06, HISTORY D12: evals E1-E14, P2 = IR rung, TRAP-82 ALERT, hygiene
  commit, a/b/c = 4x/10x/2.5x, 1.0x stays the long target, K8 before csel, T48 into bebop.bp)
- (decided 2026-09-06, HISTORY D14, docs/DECISIONS-RESEARCH-2026-09-06.md: order B1 -> B2 ->
  B5 -> the operand-tag-stack IR rung (not an op-list) -> K8 -> B4 computed frames; window
  x1-x7 with a forced spill path; T53/T54 DELETED, T52 conditional on K8; store's first move
  is B8 (profile the CSR build), workload W = the dowiz-core order log (T66); LMDB and "native
  Rust" leave the thesis sentence; D1(a) 1.0x stays report-only, <= 2.0x is the real TG-DONE 1
  target; the 11 project-sized tasks of report §4 move to HISTORY.md's `## PARKED` heading
  (reverses D11-J); codegen freezes for a 24 h fuzz window after the IR rung lands (TG-DONE 8))

## Measured (pinned A78, in-process clock_ms medians; every number has a script)

Per-commit series since D12-A (2026-09-06): docs/PERF.md (generated by tools/perf.py at the end of
every tools/chain.sh run from bench/perf.csv: self-compile wall/utime/stime/maxrss + energy proxy,
bebop.bin/stub/per-fn words with a budget, K1H-K4 ms med/p95 interleaved against the previous
binary + loop words, per-construct words, fuzz seeds per binary; `?` = invalid window, `!` = alert).

| kernel (bench/vs_rust/bench_pinned.sh) | before T96 (364009e9) | after T96 (3aae4ad8) | Rust twin (black_box in loop) | after / twin |
|---|---|---|---|---|
| K1 sum 1M | 10.0 ms | 3.0 ms | 2.41 ms | 1.24x |
| K2 fib(25) | 2.85 ms | 1.5 ms | 0.277 ms (inlined) | 5.4x |
| K3 300x300 | 1.2-1.5 ms | 0.5 ms | 0.213 ms | 2.3x |
| K4 chain 2M | 32 ms | 12.0 ms | 2.85 ms | 4.2x |
| K1 loop words/iteration | 51 | 14 | 3 | |
| isqrt / fp_div, 1M calls (T105, scratch micro-bench, pinned A78) | 286 ms / 253 ms (restoring loops) | 41 ms / 22 ms (clz Newton / sdiv base-2^k) | | 7x / 10x faster |

Honest twins (bench/vs_rust/honest.sh, D11-C, 2026-09-06, bebop.bin 94e47998, REPS=100 in-process,
pinned core 4, R=11 medians; bench/vs_rust/REPORT-honest.md has the p95 column and the method):

| kernel | bebop ms | Rust honest twin ms | bebop / Rust |
|---|---|---|---|
| K1H s = s*3 + i, 1M | 2.07 | 1.132 | 1.8x |
| K2H fib(25), inline(never) | 1.30 | 0.397 | 3.3x |
| K3H 300x300 nonlinear accumulator | 0.65 | 0.194 | 3.4x |
| K4 (v + i*7)*3 - 11, 2M (twin black_box on input/output only) | 5.26 | 3.259 | 1.6x |
| K5 self-compile of bebop.bp, cold, median of 3 | 1.67 s | | |

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
| durable commit (msync appended pages + superblock) vs WAL NORMAL / FULL | 506 us | 78 us / 567 us | 0.15x / 1.1x |

| graph in the store, 1M nodes 10M edge slots (bench/vs_rust/sgraph.sh + sgraph2.sh, G8/T117) | store | sqlite 3.46.1 | ratio |
|---|---|---|---|
| BFS, ns per edge (bebop 100 sources; sqlite level-synchronous, 3 sources) | 187 | 10758 | 57x |
| BFS on L1, queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot, 3 sources | 192 vs 45 (4.3x) | | |
| build the CSR | 44.8 s | 242 s | 5.4x |
| 1M edges through the edge log, 100 L0 rebuilds, 5 compactions | 30 us / edge, max stall 747 ms | | |
| tombstone 10% + commit / BFS with tombstones + log / compaction | 131 ms / 240 ns per slot / 795 ms | | |

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
| self-compile after the 2026-09-06 speed-ups (third pass removed, slen, str_len hoist a27b594), cold / warm .becache hit | 1.5-1.7 s / 0.07 s |
| compile of a std gate, cold / warm .becache hit / trivial-program floor (T108, becache_gate.sh) | 346 ms / 113 ms / 106 ms |
| page-cache read fault / CoW fault / msync 1 page / rename (proot) | 0.3 us / 3.5 us / ~100 us / ~270 us |

## Where everything else is

HISTORY.md (all pulls, task bodies, decisions D1-D11 and their evidence, progress
log, the 2026-08 vision text), TASKS.md (ledger), docs/SPEEDUP-ANALYSIS.md (plan
P1-P10, physics), docs/LANG-DB-DESIGN.md (store spec, prior art, §9 tensor-graph
indexing), docs/ROADMAP-CRITIQUE-2026-09-04.md (findings F1-F43), docs/LANGUAGE.md,
docs/TRAPS.md, docs/SESSION-HANDOFF.md, docs/exp.journal.
