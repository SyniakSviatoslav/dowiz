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
   51 -> 14 words), then temporaries in registers, condition fusion, a real calling
   convention and one integer-exact peephole pass (T101-T104, through a per-fn op-list
   IR per D11-G);
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

Reordered 2026-09-06 by D14 item 1 (docs/DECISIONS-RESEARCH-2026-09-06.md §5 Q1, session 16):
the disassembly of the three honest kernels showed K2H's 3.8x and K3H's 4.0x are prologue/join/
left-nest overhead, not the 16 KiB frame or a missing compare fusion (REPORT-honest.md's
attribution was wrong) — so three single-variable commits move before the IR rung and shrink
what it has to reproduce byte-for-byte. The done rungs (T118 -> T122 -> T43 -> T47/T47b/T80 ->
T48b -> T101-T103 + T105-T108 -> T109-T117 G1-G8 stage 2 -> T123-T125 -> T130/T118b -> T90 ->
T104b -> P3 cmp_try -> D13's twelve retro proposals, all DONE 2026-09-06) are in HISTORY.md;
this list is only what remains and is the ONE ordering (SESSION-HANDOFF points here, HISTORY's
"Ordering for T84-T95" is superseded).

1. B1 DONE 2026-09-06 (308f2db: fixpoint 1a3b2cc2, K2H 3.8x -> 2.6x, K3H 4.0x -> 2.4x) -> B2 DONE 2026-09-06 (fixpoint a903d33b,
   bin_words 74222 -> 67775, K2H 2.0x MET) -> B5 (loop rotation, bottom test): three
   single-variable codegen commits, each through `tools/chain.sh --codegen`, constructs
   re-frozen, an honest.sh row (D14 item 1; expected K2H 3.8x -> ~2.1x with no change to the
   expression model).
2. The register model, ONE commit (operator 2026-09-06, session 18, supersedes D12-B/D14 item 2's
   R1-R5 rungs): docs/REGISTER-MODEL-BLUEPRINT.md. The stack machine is deleted -- no
   `sub sp/str x0,[sp]` / `ldr/add sp` anywhere in emitted code (new invariant push_words == 0 in
   invariants.sh + a PERF row); expression values are tags (CONST / SYM / REG / CS / SLOT / MULC /
   FLAGS) over a window x0..x7, values that outlive the window go to callee-saved temps x(19+vc)..x26,
   then to x15 frame slots sized by the planning pass (facts vc/alloc/cs_hi/tsp published by the
   B1 mechanism); one stream edit remains (retarget of the last word's rd), every peephole and
   word-pattern decoder is deleted (pop2 / left_single_* / madd_try / shl_try / mulc_try /
   addshift_try / cmp_try / fold_try / cond_branch_word). Gates: chain --codegen GREEN, push_words 0,
   k4 <= 13, k3h <= 10, k1h <= 8, k2h <= 30, k4_ms <= 3.0; constructs c55-c61 (window cap, nesting,
   FLAGS, call mix, x0 eviction, nested ctor, array-of-calls). Expected: K4 14 -> 7, K1H 10 -> 5,
   K3H 24 -> 7, K2H 51 -> ~21 words, bin_words 68229 -> < 55000.
3. Freeze codegen for one 24 h fuzz window once the register model (item 2) lands, so `fuzz_seeds_on_bin`
   reaches 10^5 on one md5 (TG-DONE 8, D14 item 12) — scheduled here, before more codegen work
   resets the per-binary seed counter again.
4. K8 DONE 2026-09-06 (REPORT-honest.md K8 row: bebop 0.31-0.34 ms/rep vs Rust csel 0.069, 4.5-5.7x;
   control with a predictable bit 0.15 ms/rep = the branch is ~55 % of K8): T52 (pure `if` -> csel)
   PROCEEDS as a `csel` on FLAGS/REG tags in the commit right after the register model (item 2; pure arms = no calls, no statements), T53/T54 stay DELETED (HISTORY.md).
5. B4, per-fn computed frame size (D14 item 4): `80 + 8*while_marks + 8*spill_slots` (sized from vc:
   today's x15 region is 64 slots while sym_bind admits 128 symbols -- IR-RUNG-BLUEPRINT §0; the register model publishes vc/alloc/cs_hi/tsp per fn, REGISTER-MODEL-BLUEPRINT §5, and traps exit 89 on slot overflow until B4 sizes the region), plus the
   heap only when the body needs it; a mis-estimate is exit 81, TRAP-82 stays the fuzz gate at 0.
6. Store, first move: B8 — profile the 45-90 s CSR build (sgraph phase b) before any store code
   change (D14 item 6); the real workload W = the dowiz-core order log (T66 `ordfsm.bp`/
   `money.bp`, byte-exact Rust oracles — D14 item 8); a, b, c stay frozen (D12-F: 4x / 10x /
   2.5x) and the sgraph2.sh full run + honest.sh R=11 rerun stamp validity via E7 in the
   background.
7. T48 rest (checked types into bebop.bp) rides the register model's per-symbol table (S once
   item 2 lands, not before); T61 (pool/futex builtins exist: the task is the library + a gate).
8. Design-bound, operator decision first (AskUserQuestion before code): T68-T70, T85 -> T86
   follow-ups, T73, T76, T49/T50, T56, T59.
9. Last, each a project of its own: T91 x86_64 backend, T63/T64/T83 bench-policy rows as they
   come up; T92-T95 backends, T84 glyphs, T62 network, T67 mesh, T87 f64, T88 supervisor, T89
   trust chain moved to HISTORY.md's `## PARKED` heading (D14 item 11, reverses D11-J).

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
