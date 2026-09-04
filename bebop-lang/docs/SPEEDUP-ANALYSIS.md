Status: 2026-09-04 CURRENT (read-only analysis by the session-8 analyst agent; numbers measured in-sandbox that day; supersedes the "Predicted speedup and memory" table of ROADMAP.md per decision D8(5))

# Bebop speed analysis — where the time goes, what can beat Rust/SQL, and by how much

Date 2026-09-04. Read-only analysis of /root/dowiz/bebop-lang at bebop.bin md5 364009e9…
All new numbers below were measured today, pinned with `taskset -c 4` (Cortex-A78, part 0xd41)
unless stated; cycles assume 2.4 GHz (ROADMAP's own figure; no PMU under proot, so cycle
counts are ns x 2.4 and carry that caveat). Rule of the report: every speed claim reduces to
counted words, bytes moved, cache lines, cores, or an asymptotic argument with the constant
estimated. "Metaphor" is flagged wherever the roadmap's language names no mechanism.

Method notes (so nobody fools themselves):
- Hand-assembled `.bin`s (GNU `as` + `objcopy`, 8-byte zero entry trailer as `cli_compile`
  writes it, bebop.bp:3499-3519) were run through `seed/build/seed` to measure the exact
  instruction shapes the compiler emits today and the shapes T96 would emit. This measures
  the CPU, not the compiler; it is the cleanest way to get a ceiling without touching bebop.bp.
- Line numbers in this report are git HEAD line numbers of bebop.bp. While this analysis ran,
  another process landed an uncommitted +17/-2 diff in bebop.bp (21:14, hunks in `push`/`pop`
  1330-1347 = T96 step (1) pair elision, plus fntab[3660] label tracking in emit_cond /
  emit_while_stmt). Citations past line 1330 shift by up to +17 in the working tree; bebop.bin
  md5 364009e9 (the binary measured) predates that diff.
- Process wall-clock includes ~8 ms proot/seed floor and ~15 ms Rust startup; the D3 column
  (in-process `clock_ms`) is the only one that measures code. bench_pinned.sh's "ratio"
  column is startup-dominated (today: K2 0.82x, K3 0.60x "faster than Rust" by wall-clock —
  that is Rust's 15 ms startup, not Bebop code).

---------------------------------------------------------------------------------------------
## 1. WHERE THE TIME GOES NOW

### 1.1 Emitter cost model (bebop.bp, live compiler)

| construct | fn (bebop.bp line) | words emitted | memory ops |
|---|---|---|---|
| push | `push` 1317 | 2: `sub sp,sp,#16` + `str x0,[sp]` | 1 store + sp write |
| pop into xR | `pop` 1330 | 2: `ldr xR,[sp]` + `add sp,sp,#16` | 1 load + sp write |
| variable read | `emit_var` 1507 | 3: `mov x0,xR` + push | 1 store |
| literal | `emit_lit` 1399 | 1-4 (movz/movk) + push = 3-6 | 1 store |
| binop a op b | `emit_binop_plain` 1484 | pop x1, pop x0, op, push = 7 | 2 loads + 1 store + 4 sp writes |
| compare | `emit_cmp_op` 1618 | pop, pop, cmp, cset, push = 7 | same |
| `let x = e;` bind | `emit_let_stmt` 2287 -> `bind_reg` 1527 | pop x0 + `mov xR,x0` = 3 | 1 load |
| while header | `emit_while_stmt` 2246-2260 | cmp-expr, pop x0, `cmp x0,#0`, `b.eq` = +4 | 1 load |
| body tail `0` | `emit_while_stmt` 2265-2279 | lit + push + pop = 5 (pure waste, every iteration) | 1 store + 1 load |
| call | `emit_bl` 396 | 3 str + bl + 3 ldr + add = 9 per call site | 6 memory ops |
| fn prologue/epilogue | `emit_prologue` 2215 / `emit_epilogue` 2230 | 10 + 8, incl. `sub sp,sp,#0x4000` = a **16 KB frame per call** | |

`fold_try` (1449) catches exactly two shapes: const-const add/sub/mul (depth 2 cells
fntab[3655-3659]) and `var +/- imm12` (depth 1). In K1 it fires once (`i - 1` -> 5 words
instead of 10). It never sees `x * 2`, `i > 0` (cmp with literal), or any left-nested
operand. The fixed-cell model (1421-1434) caps the fold at depth 2 by design.

### 1.2 K1 loop, instruction census (compiled today, `sa/k1.bin`, loop 0x5c..0x124)

| class | words / iteration | note |
|---|---|---|
| `str x0,[sp]` | 9 | one per push |
| `ldr xR,[sp]` | 9 | one per pop |
| `sub/add sp,sp,#16` | 18 | a **serially dependent chain** (each sp write depends on the previous) |
| `mov` (reg/imm) | 7 | var reads, binds, body tail `0` |
| real work (`cmp`, `cset`, `cmp #0`, `add`, `sub #1`, `b.eq`, `b`) | 8 | Rust needs 3 (`add`, `subs`, `b.ne`) |
| **total** | **51** | REPORT-630 said 50; census today 51 |

Same census: K3 inner 81 words (15 str / 15 ldr / 30 sp), K4 76 (14/14/28), spike inlined
linear loop 265 words for 17 source ops = **15.6 words and 3.5 memory ops per source op**
(58 str / 60 ldr / 98 sp writes per iteration).

### 1.3 Measured cycle accounting (hand-assembled shapes, 50M iterations, A78)

| shape | words/iter | ns/iter | cyc/iter | what it isolates |
|---|---|---|---|---|
| v5 = exact current K1 loop | 51 | 10.4 | 25 | today (matches compiled K1: 10.0 ms / 1M) |
| v2 = same but pre/post-indexed `str x0,[sp,#-16]!` | 33 | 10.0 | 24 | fewer words, **no gain**: sp writeback still serial |
| v3 = same stores/loads at fixed `[sp,#0/#16]`, no sp arithmetic | 33 | 7.6 | 18 | sp chain = 7 cycles; store->load forwarding chain = the rest |
| v1 = T96 steps 1-3 (vars in x19/x20, x0/x1 scratch, zero memory) | 15 | 2.1 | 5 | **T96 ceiling for K1** |
| v0 = LLVM shape (`add; subs; b.ne`) | 3 | 0.5 | 1.2 | Rust without in-loop black_box |

Reading: time is NOT instruction count. 51 words in 25 cycles is 2 IPC on a 4-wide core;
the loop is latency-bound on two chains: (a) sp updates (18 dependent ALU ops), (b)
store->load forwarding (~4-5 cycles each, 9 per iteration, partially overlapped). Removing
memory entirely (v1) gives 5x; making memory ops cheaper (v2) gives nothing.
On the A55 (in-order) the same shapes: v5 26.5 ns, v1 4.6 ns, v0 1.1 ns — the stack
machine is 2.5x worse again on the little cores.

### 1.4 T96 step ceilings (D2), derived from the census + the measurements above

| step | mechanism (where) | K1 words after | est. K1 ms | K4 words after | est. K4 ms |
|---|---|---|---|---|---|
| today | — | 51 | 10.0 (meas.) | 76 | 32 (meas.) |
| (1) push-pop pair elision (`pop` that sees its own `push`) | pop 1330 rewinds n[0] like fold_try 1449 | 31 | ~6.5 | ~50 | ~21 |
| (2) binop right operand = bound var -> `mov x1,xR` | emit_binop 1477 / emit_var 1507 | 21 | ~4 | ~40 | ~15 |
| (3) `let` binds from x0 (no push/pop) | emit_let_stmt 2350-2352 | **15** | **2.1** (v1 meas.) | ~24 | **9.1** (v1_k4 meas. 4.54 ns/iter) |
| + temporaries in x1-x7 (D8(1) next step) | new: value-stack slots map to regs | 15 | 2.1 | 20 | 5.0 (v4 meas. 2.52 ns) |
| + cmp-with-literal -> `cmp xR,#imm; b.cond` | emit_while_stmt 2256-2260 | 11 | ~1.6 | 12 | 4.7 (v6 meas. 2.36 ns) |
| + strength reduction (`*3`,`-11` -> madd) | optimizer pass, not one-pass | 3 | 0.5 | 4 | 2.4 (v0 meas.) |

K4 after every one-pass step stays at ~2.4 ns/iter because the recurrence
`v = (v + i*7)*3 - 11` is a latency chain mul(3)+add(1)+mul(3)+sub(1) ~ 6 cycles; only
strength reduction into one `madd` (3-cycle chain) reaches Rust's shape. K2: 92 words per
`fib` call (prologue 10 + epilogue 8 + 2 x 9 call-site words + stack traffic) and a 16 KB
frame per level (`sub sp,sp,#0x4,lsl #12`, emit_prologue 2217) -> 25 frames = 400 KB of
stack touched, exceeding L1D (64 KB). Measured 11.7 ns = 28 cycles per call.

What a real register allocator / scheduler would add beyond the table: nothing on K1/K3
(the loops are 3-4 instructions once memory is gone; the OoO core schedules them), ~2x on
K4 (madd fusion), ~3x on K2 (frame size, args in x0-x7, callee-saved only when used). A
linear-scan allocator over a per-function op list is the natural next tier after T96; an
instruction scheduler is irrelevant on the A78 (160-entry ROB does it) and only matters on
the in-order A55.

---------------------------------------------------------------------------------------------
## 2. VS RUST, SINGLE CORE — honest ceiling table

What the Rust twins actually are (bench/vs_rust/rust_once/k1-k4.rs, rust/src/main.rs):
- K1, K3: `black_box(s + i)` INSIDE the loop forces the accumulator through memory every
  iteration: measured 2.4 ns/iter (K1) — 5x slower than LLVM's honest loop (v0, 0.5 ns).
  Without the in-loop black_box LLVM folds K1 and K3 to closed forms (REPORT.md line
  "0.6 us for a 1M-iteration loop"). So the K1/K3 targets are against a deliberately
  crippled Rust; beating them is not beating Rust.
- K2: LLVM inlines/unrolls `fib` 2-3 levels: 0.277 ms = 1.1 ns per logical call. A fair
  `#[inline(never)]` fib(25) would be ~0.7 ms (242,785 calls x ~7 cycles).
- K4: wrapping arithmetic with black_box; LLVM keeps the loop (2.85 ms = 3.4 cycles/iter).
  This is the only honest twin of the four.

| kernel | Bebop today (in-proc, pinned) | Rust twin today | ratio | after T96 (1)-(3) | after +temps in regs +cmp fold (one-pass ceiling) | needs optimizer pass | vs honest LLVM |
|---|---|---|---|---|---|---|---|
| K1 sum 1M | 10.0 ms | 2.41 ms | 4.1x | **2.1 ms (1.15x FASTER than twin)** | 1.6 ms | closed form / 0.5 ms loop | 4x slower than loop; infinitely slower than closed form |
| K2 fib(25) | 2.85 ms | 0.277 ms | 10.3x | ~2.0 ms | ~1.0 ms (small frame, args in regs) | inlining -> 0.4 ms | ~1.4x vs inline(never) Rust |
| K3 300x300 | 1.2-1.5 ms | 0.213 ms | 6.5x | ~0.32 ms (1.5x) | ~0.22 ms (1.0x) | closed form | same as K1 |
| K4 chain 2M | 32 ms | 2.85 ms | 11.2x | 9.1 ms (3.2x) | 4.7 ms (1.65x) | 2.4 ms (0.85x) via madd | 0.85x — Rust twin is honest here |

So: >= 1.0x on K1/K3 is reachable with T96 alone (against the twin as defined); K4 needs
one optimizer pass (strength reduction of mul-by-const chains into madd/shift — integer-exact,
trivially implementable in .bp as a peephole over the op list, no LLVM); K2 needs the
calling convention fixed (frame size, register args). D8(1)'s "1.5-3x ceiling for a one-pass
compiler" is confirmed by measurement for K4 (1.65x) and is pessimistic for K1/K3.

LLVM techniques on K1-K4 and their .bp implementability (all integer-exact):
| technique | what LLVM does | in-roadmap-rules implementable? |
|---|---|---|
| closed-form (SCEV) | K1 -> n(n+1)/2, K3 -> polynomial | yes but pointless: the twin forbids it via black_box; do not chase |
| strength reduction | `i*7*3` -> `i*21`, `v*3+c` -> madd | yes: peephole on emitted op list (K4 2x) |
| LICM | hoist `x*2` out of K3 inner loop | yes once temporaries live in registers |
| unrolling | 4x unroll of K1/K4 | no benefit on latency-bound chains (K4) nor on 1.2-cycle loops (K1) |
| vectorization | none applies (serial recurrences) | n/a |
| inlining | fib 2-3 levels | possible for self-recursion depth 1 with a size budget; large emitter change |

---------------------------------------------------------------------------------------------
## 3. VS RUST, ORDERS OF MAGNITUDE — every mechanism, with physics

Physical constants measured today on this box (Rust probe `bw.rs`, big cluster 4-7):

| working set | 1 thread sum | 4 threads sum | nn-scan 1 thr | nn-scan 4 thr |
|---|---|---|---|---|
| 1 MB (L2) | 20 GB/s | spawn cost dominates (0.79 ms) | 8.2 GB/s | x0.07 |
| 4 MB | 10.7 GB/s | x0.31 | 8.2 GB/s | x0.54 |
| 16 MB | 13.5 GB/s | 9.4 GB/s (x0.70) | 9.3 GB/s (1.68 ms) | x1.12 |
| 64 MB | 12.3 GB/s | 11.6 GB/s (x0.94) | 9.1 GB/s | x1.17 |
| 256 MB | 12.0 GB/s | 12.4 GB/s (x1.03) | 8.9 GB/s | x1.38 |

Facts: DRAM is ~12 GB/s and ONE A78 saturates it; four A78 give 1.0-1.4x on streaming;
thread spawn+join under proot costs ~0.7 ms (std::thread; sys_clone from bebop will be
similar: it is the same kernel path plus proot's ptrace interception); L2 is ~1 MB per core
(20 GB/s); A55 cluster: 5.4 GB/s single, 8.3 GB/s x4, scalar code 2.2x slower than A78.

Ranked mechanisms. "Structural" = Bebop's design makes it the default; "language-independent"
= Rust does the same with the same effort.

| # | mechanism | physical basis | max gain on this box | in repo today | missing | class |
|---|---|---|---|---|---|---|
| M1 | **Query compiled to native code** (no VDBE/interpreter per row) | sqlite ~180 ns/row (VDBE ~30 opcodes + record decode); native scan 1.4 ns/row (DRAM-bound) | **10-130x on scan-class queries** — measured: sqlite scan 180 ms, bebop scan 49 ms (3.7x today), Rust 1.4 ms (128x) | the compile->publish->run path (cli_compile 3464, morph.bp, morph_loop.sh); T32 qjit NOT written | qjit.bp; T96 so the scan is not 35x behind Rust | structural (the language IS the query compiler) but Rust+codegen could do it too |
| M2 | **Index / asymptotic** O(N) -> O(window) | touching 6 rows instead of 1M | **1400x measured** (sqlite scan 180 ms -> cell-index 0.13 ms) | grid_cell tq.bp:29-38 computes the cell but `query` tq.bp:75-101 still scans all N; csr.bp:31-74 CSR buckets; csheaf.bp:44-51 hash probe | wire cell -> CSR bucket -> window scan (see §6 rank 1) | language-independent; without it T100 loses by 10^3 |
| M3 | **Incremental recomputation via activity bits** (work ∝ delta, not N) | sweep cost = fired cells x 205 ns (bebop) / 10.9 ns (Rust model) vs 0.3 ns per op linear | asymptotic; crossover: wins only when changed fraction k/N < 0.3/10.9 = **3%** (Rust-quality engine) or < 0.15% (today's) | substrate.bp sweep, spike.bp tzcnt drain, dispatcher.bp; csheaf.bp `check()` validates only incident edges (O(degree)) | a gate that changes k of N cells and measures vs full recompute; the constant must drop 20x first (T96 on the sweep loop) | structural; the roadmap's only true asymptotic win, but D8(4) already limits it to sparse regimes |
| M4 | **Content-addressed memoization** (digest -> artifact) | repeat work O(1) instead of O(work) | infinite on exact repeats, 0 otherwise | `.becache` only in the retired C pipeline (OPTIMIZATIONS.md:19-23); live bebop.bin never uses it (std_golden.sh recompiles 94 times); cache.bp DecompCache is an O(S) linear scan (spectral.bp:355-367), ptrless.bp is a 3-entry select | key the compiled-query artifact by digest(source+data-schema) in the qjit path | language-independent (sqlite has a prepared-statement cache) |
| M5 | **Multi-core sharding** (T98) | 4 A78 + 4 A55 ≈ 5.8 A78-equivalents for compute-bound scalar work; **1.0x for DRAM-bound streams** | ≤ 4-5.8x compute-bound; 1.0-1.4x memory-bound; minus 0.7 ms spawn unless workers persist | pool.bp par_sum/par_merge (sys_clone 68864, futex park), pool_parity 5/5 | shard a real kernel; persistent parked workers; NOT k1/k2 (serial recurrences cannot shard) | language-independent; never an order of magnitude here |
| M6 | **NEON 16 x i8 lanes on bit-packed data** | `eor+cnt` = 128 values per 2 instructions vs SWAR 64 values / 12 ops | ~5x over scalar SWAR; **1.0x vs Rust** (BENCHMARK-2026-08-17: hamming 85 vs 84 Mops; LLVM emits cnt too) | hvham/hvham2 emitters bebop.bp:584-663 (ldp q, eor, cnt, add, uaddlv); hv.bp bit-packed HV | apply hvham to hv.bp/deltasync/attn (still scalar hv_pop1); sdot/udot for ternary dot products unused | language-independent; only pays if the DATA is 1-2 bits/value |
| M7 | **Bit-packed ternary / RNS lanes** (T1/T2) | 32 blades x 2 bits in one i64; 4 residues x 15 bits | many values per word at rest; arithmetic is still scalar loops (tern.bp gprod 576 ops for 8 values); rns.bp does NOT pack (4 separate i64) | tern.bp:49-58 pack; qlora.bp:44-65 nibble pack | SWAR arithmetic on the packed form (add-with-carry-isolation), or NEON i8 lanes | representation choice, available to Rust; today negative |
| M8 | **Generative memory** (compute instead of read) | DRAM 12 GB/s = 1.5 G i64/s; a core generates 2-4 G trivial values/s | ≤ 2-3x, only for streamed-once data larger than L2; 1.0x if data fits L2 (20 GB/s) | lsys.bp, lod.bp (both fully expand; lod is an equality proof, not LOD), phant.bp | a query answered from the RULE without expanding (that is M9) | metaphor as stated in T4/T5; constant factor at best |
| M9 | **Answer from structure without touching data** | boundary O(√N) vs area O(N) (tdgstokes.bp:25-40 vs 43-57); local consistency check O(degree) (csheaf.bp:60-83); nilpotent product = 0 short-circuit (grass.bp:18-33 masks zero terms before multiply) | asymptotic where the query is closed under the structure; demonstrated at N=64, never at scale | those three files | a data set where the structural answer replaces a scan (audit = boundary sum of a 1M-cell table) | structural candidate; unproven at scale |
| M10 | **Persistence = the artifact** (no deserialization) | mmap RX of the .bin (seed.S:36-46); string cells embedded (compile_program_to 3065-3089) | vs serde/JSON: 10-100x; **vs sqlite: 1.0x** (sqlite pages are also mmap-able and never "parsed") | arrays are NOT in the artifact: zeros() bump-allocs the anonymous 256 MB arena (emit_zeros 3381, seed.S:55-68) | T26 regrec (mmapped .bt as the register image) | language-independent |
| M11 | Reversible arena (T59) / CoW + nilpotent tokens (T33) | undo log O(delta) vs snapshot O(N) | same class as any WAL/undo log; mvcc.bp 48-65 is CoW append, single-threaded; stm.bp validates O(8) cells | rev.bp XOR-delta undo (128 lines), mvcc.bp, stm.bp | multi-core version (M5) — today both are single-threaded simulations with an LCG interleaver | constant factor; sqlite's WAL is also O(delta) |
| M12 | Integer fixed point vs f64 | A78: i64 mul 3 cyc, f64 fmul 3 cyc; hardware `sdiv` ~10 cyc, `fsqrt` ~15 cyc | **negative**: fp_div/isqrt are 32-step software loops (tq.bp fp_sqrt, tdg.bp fp_div) = 100-300 ops where hardware needs 1 | everywhere in the tdg/tq stack | use `sdiv`/`udiv` (emitted already, op 5) and a hardware-assisted isqrt (Newton from `clz`) | language-independent; today a 10-30x LOSS per division/sqrt |
| M13 | No GC / arena | Rust has no GC; Vec is one allocation | 1.0x | — | — | not a mechanism |
| M14 | Holographic loading (T60) / WHT redundancy | 4 copies x 8 words; recovery from 3 | 0x speed; 4x more bytes | holo.bp | — | robustness, not speed; metaphor if listed under speed |
| M15 | Compile-time levelizer of the dataflow DAG (D8(2)) | an OoO core levelizes the DAG every cycle in hardware (ROB 160 entries) | 1.0x on A78; a few % on the in-order A55 | spike shows "levels IS linear code" | — | metaphor on OoO hardware |

The three-to-five that are real on THIS box: M1 (native query, 10-100x vs sqlite scans),
M2 (index, 10^3 vs a scan — but sqlite has it too), M3 (activity-proportional
incremental work, asymptotic, only at k/N < 3%), M9 (structure answers the audit query,
√N-class, unproven at scale), M5 (cores, ≤ 5.8x, never 10x). Everything else is
constant-factor, language-independent, or negative.

---------------------------------------------------------------------------------------------
## 4. VS SQL (sqlite3 = the T100 oracle)

### 4.1 Measured today (python sqlite3 module, 1M rows (id,u,v) i64, 18 MB file, core 4)

| query on 1M points | sqlite3 | Bebop today | Bebop after T96 (est.) | Rust | memory floor |
|---|---|---|---|---|---|
| nearest by squared euclid, full scan | **180.5 ms** (180 ns/row) | **49.3 ms** (nn.bp brute scan, 185-word loop, 49 ns/pt) | ~12 ms (≈45 words/pt ≈ 12 ns) | **1.41 ms** (9-11 GB/s) | 16 MB / 12 GB/s = 1.3 ms |
| nearest with a 3x3 cell index (cell column + B-tree; window = 6 rows) | **0.13-0.28 ms** (mostly python + VDBE setup; C API ~20-50 us) | tq.bp geodesic scan: est. **~4 s** (see below) | ~1 s | ~0.25 s with the same algorithm | index: 6 rows = 96 bytes |
| nearest with R-tree window | 0.38-0.62 ms | — | — | — | — |
| build (insert 1M) | 2.8 s (python executemany) | zeros(2M)+LCG fill ~15 ms | ~5 ms | ~5 ms | 16 MB write |

tq.bp's actual method (selfhost/std/tq.bp): `query` (75-101) is an O(N) scan; per point it
calls `geodesic` (42-72) = up to 3 segments x (k=4 anchors x sqdist (2 fp_mul)) + 3-4
`fp_sqrt` (isqrt, a 32-step restoring loop, ~200 ops each) ≈ 700-900 source ops per point.
At today's 5 ns/op that is ~4 s per query on 1M points, ~1 s after T96, ~0.25 s with
Rust-quality codegen. `grid_cell` (29-38) IS computed for every point and stored in
`cell[i]`, and the query counts window membership (88-94) — but never uses the cell to
restrict the scan. The T20 text "parametric surface O(1) lookup" describes the coordinate
computation, not the search; the search is linear.

### 4.2 Where the roadmap projection was wrong (ROADMAP "Predicted speedup" T16-T21 row)

| projected | measured / derived | verdict |
|---|---|---|
| "SQL ~10 ms" for a 1M-point query | full scan 180 ms; indexed 0.13 ms | wrong both ways: sqlite is 18x slower on scans and 75x faster with an index |
| "tensor ~0.2-0.7 ms (parametric surface O(1) lookup vs B-tree O(log N))" | tq.bp is O(N) with ~800 ops/point: ~4 s today | off by 10^4; the O(1) is the coordinate, not the lookup |
| "15-50x vs SQL" | scan class: 3.7x today, ~15x after T96, 128x at Rust quality; index class: currently 10^4 SLOWER | true only for scan-class queries after T96 |
| "no query parser, no WAL, no B-tree traversal" as the reason | parse+plan is ~20-50 us per statement (amortized to 0 with prepared statements); B-tree traversal for 6 rows is ~1 us; WAL costs nothing on reads | the real sqlite cost on scans is VDBE per-row interpretation (~180 ns/row); on point queries sqlite is already at the microsecond floor |
| "200-500 MB RSS for SQL" | 18 MB file, page cache ~20 MB, python process ~30 MB | wrong by 10x; sqlite RSS for this data is ~20 MB |

Apples-to-apples rules: same data (LCG seed, same i64 fixed point), same answer (fold over
1000 query results == python oracle), same core pinned, in-process timing on both sides
(sqlite3 CLI `.timer on` or the C API from a tiny python ctypes shim — NOT the python
wrapper's per-call overhead), 1000 queries so startup is amortized, both engines in both
classes (scan and indexed). Not fair: comparing bebop's brute scan with sqlite's
`ORDER BY d LIMIT 1` (sqlite sorts a temp b-tree — the 180 ms includes that; a `MIN()`
query is ~120 ms), or comparing an indexed engine with an unindexed one.

### 4.3 The T100 gate, exactly

- Data: N = 1,000,000 points (u,v) as i64 Q32 from `x = x*6364136223846793005 + 1442695040888963407`,
  seed 12345, in a `.bt` rank-4 tensor [N,2,1,1] (bt.bp codec) published via store.bp; the
  same rows inserted into sqlite `p(id INTEGER PRIMARY KEY, u, v, cell)` with `CREATE INDEX ic ON p(cell)`,
  `cell = ((u+2^31)>>22)*1024 + ((v+2^31)>>22)`.
- Queries: 1000 (qu,qv) from the same LCG continued; answer = nearest id by squared euclid
  (ties -> lowest id); fold = Σ (id_i * 131^i) mod 1e9+7 must equal bench/oracles/tq_sqlite.py
  (python computes the truth by brute force once, ~30 s, cached in the oracle file).
- Engines/rows: (a) sqlite scan `SELECT id FROM p ORDER BY d LIMIT 1` and `MIN`; (b) sqlite
  indexed 3x3 window; (c) bebop brute scan (nn.bp shape); (d) bebop bucketed: cell -> CSR
  row pointers (csr.bp:31-74 layout) -> 3x3 window scan; (e) Rust twin of (c) and (d).
- Metrics per row: median us/query in-process, build ms, peak RSS KB (VmHWM), fold ok.
- Pass: (d) ≤ 10 us/query AND ≥ 3x faster than (b) measured through the C API; (c) ≥ 10x
  faster than (a). Report whatever the numbers are (D8(5)).
- Expected: (a) 120-180 ms; (b) 20-50 us; (c) 12 ms after T96 (49 ms today); (d) < 2 us
  (6-20 points x 12 ns + bucket lookup) — 10-25x over (b), 10^4-10^5 over (a). That is the
  honest "orders of magnitude vs SQL": M1 (no VDBE) x M2 (index), with M2 doing 99% of it.

---------------------------------------------------------------------------------------------
## 5. THE PLAN — ordered, each step a gate with a twin and a number

| # | step | gate + pass number | why this order |
|---|---|---|---|
| P1 | T96 (1)-(3) as specified (pop-after-push elision in `pop` 1330; `mov x1,xR` in emit_binop 1477; let from x0 in emit_let_stmt 2350) | K1 ≤ 16 words/iter; K1 in-proc ≤ 2.5 ms (≥1.0x twin); K3 ≤ 0.35 ms; folds bit-exact; fixpoint | 5x on every loop; measured ceiling v1 = 2.1 ns/iter |
| P2 | value-stack slots -> x1-x7 for nested operands (the "temporaries" of D8(1)); drop the body-tail `0` push/pop (emit_while_stmt 2265-2279) | K4 ≤ 5.0 ms (≤1.75x); K3 ≤ 0.25 ms; spike linear ≤ 5 ms (was 18) | removes the last memory ops; measured v4 = 2.52 ns/iter |
| P3 | while/if condition -> `cmp xR,#imm / cmp xR,xS` + `b.cond` directly (extend fold_try to cmp; emit_while_stmt 2256) | K1 ≤ 12 words; K4 ≤ 4.7 ms | measured v6 |
| P4 | calling convention: frame = 16 + 8 x spill slots (not 16 KB, emit_prologue 2217); args stay in x0-x7 for ≤ 8 params; save only used callee-saved regs | K2 ≤ 1.0 ms (≤ 1.4x vs inline(never) Rust twin, add that twin) | frame touches 25 x 16 KB today |
| P5 | one peephole pass over the per-fn word stream (already precedent: pop_back/flush 1340-1375): mul-by-const -> shift/madd, `x*c1*c2` -> `x*(c1c2)`, LICM of loop-invariant `mov xR,#imm` | K4 ≤ 3.0 ms (≈ 1.0x twin); construct folds unchanged | the only path to 1.0x on K4; integer-exact |
| P6 | T100 as in §4.3 with the bucketed index (tq.bp grid_cell + csr.bp CSR) | bebop indexed ≤ 10 us/query, ≥ 3x sqlite C-API indexed; bebop scan ≥ 10x sqlite scan | the real 10x-over-SQL number |
| P7 | replace fp_div/isqrt software loops in tq/tdg with `sdiv` + clz-seeded Newton isqrt (op 5 already emits sdiv) | tq gate fold unchanged; tq 1M geodesic query ≤ 50 ms | 10-30x on every division; prerequisite for any geodesic query at scale |
| P8 | T98 re-scoped: persistent parked workers (pool.bp pattern) sharding the nn scan (not k1/k2, which are serial) | `nn4`: 4 A78 ≥ 3.0x on the bebop scan while it is compute-bound (today 49 ms -> ≤ 16 ms); record the 1.0-1.4x it becomes once memory-bound after P1 | measured Rust: DRAM-bound scans get ≤ 1.4x from 4 cores |
| P9 | incremental-substrate gate: N = 2^16 cells, change k cells, sweep vs full recompute; both in Rust twin | record the crossover k/N; pass = crossover ≤ 5% after P1-P2 on the sweep loop | turns M3 from a claim into a curve |
| P10 | `.becache` for the live compiler: key = digest(source) -> .bin replay in cli_compile | std_golden warm run ≥ 5x faster; folds identical | M4, cheap, makes T32 qjit memoized by construction |

Delete or re-scope (cannot produce speed on this hardware under these rules):
- T25/T26/T35 typed Z2 bank on x9-x13: reserves 5 registers for algebraic state while ordinary
  expressions stay a stack machine. Speed needs those registers as temporaries (P2). Re-scope:
  the bank is a library convention on the arena, not an ABI reservation.
- T55 substrate codegen for K1-K4 and TG-DONE 1 "one conditional branch": measured 41x/740x
  against; D8 already moved it; delete the K1-K4 rung entirely.
- T57 sweep prelude, T58 eigentime scheduler, T74 WFE: eigentime is an iteration counter
  (seigtime.bp:119-210, cycle detection of a power map) — a metaphor as a scheduler; WFE saves
  energy, not time.
- T60 holographic artifact: 4x bytes for redundancy; no speed. Keep under robustness only.
- T98 as written ("k1/k2 substrate kernels sharded"): k1 is a serial sum (shardable only as a
  reduction), k2 is a recurrence; re-scope to P8.
- "Predicted speedup and memory" table: delete (D8(5)); every row above is measured or "est."
  with the measurement that bounds it.
- T92-T95 (Verilog/WGSL/WASM/SPIR-V emitters), T84 glyphs, T85 proof kernel: zero speed.
- T15 "bare metal 8-12x": proot costs ~8 ms per process and ~0.7 ms per thread spawn, not
  per-instruction time; in-process loops run at silicon speed today (v0 = 1.2 cyc/iter).
  Forward-port trigger: only for thread-spawn-heavy or syscall-heavy workloads.
- Not on this hardware: SVE/SME (absent), perf counters (proot), >12 GB/s (DRAM), >5.8x cores.

---------------------------------------------------------------------------------------------
## 6. CROSS-LINKS — what the code makes cheap that the roadmap never wires

Four read-only sweeps covered every concept gate (runtime/JIT/eigentime; generative/packed
lanes; tensor/spectral/sheaf/txn; grading/rewrite/dowiz twins + compiler internals). First
the inventory facts that matter, then the ranked combinations.

### 6.1 Inventory facts (bluntly)
- Duplication instead of linking: the spectral engine (spmv_fp/topk/deflation) is copied
  verbatim into cache.bp, seigtime.bp, srepl.bp, msuper.bp, scoord.bp (5 copies, ~250 lines
  each); ntt.bp into fno.bp; wht.bp twice into fno.bp; bt.bp into store.bp; the Grassmann
  kernel into mvcc.bp and stm.bp; the L-system loop into lsys/entcol/lod/phant; tern.bp+rns.bp
  into rnsrot.bp. There is no `use`; T47 is the missing link that would make cross-linking
  possible at all.
- The live compiler uses none of its own memoization: `.becache` exists only for the retired C
  `bebopc`; std_golden.sh recompiles 94 gates every run.
- The only O(1)-class index in std is csheaf.bp `probe` (44-51: slot = d & 15, linear probe);
  spectral.bp's "content-addressed DecompCache" is a linear scan over S slots.
- mvcc.bp/stm.bp are single-threaded simulations (one LCG picks the next actor); the real
  threads live only in pool.bp/pool_compile.bp. They have never been run together.
- csr.bp `csr_from_edges` is O(n*m) construction (31-38); its SpMV row lookup is O(degree)
  (100-114) — the bucket structure the tensor query needs already exists here.
- tdgstokes.bp computes BOTH the boundary sum (O(perimeter), 25-40) and the interior sum
  (O(area), 43-57) and asserts equality; stm.bp re-implements a scalar Stokes check inline
  (158) instead of calling it.
- morph_loop.sh claims content-addressed names; the files are `morph_k1_$i.bin` (iteration-
  indexed). ptrless.bp is a 3-entry digest select. fiber.bp is a 4-cell update loop (no
  scheduler). lsm.bp is a Liquid State Machine, not an LSM tree. lod.bp is an equality proof
  between materialized and regenerated expansions, not a level-of-detail index.
- NEON exists only in hvham/hvham2 (bebop.bp:584-663: ldp q0-q3, eor, cnt, add, uaddlv);
  hv.bp/deltasync/attn/ringvsa still use scalar hv_pop1. sdot/udot (asimddp in cpuinfo) unused.
- The .bin embeds code + string cells only; every array is bump-allocated from the anonymous
  256 MB arena (emit_zeros 3381-3406, seed.S:55-68) and zero-filled on each zeros() — a 1M-
  element array costs an 8 MB store pass before any data arrives.

### 6.2 Ranked combinations (asymptotic class first; "evidence" = the code that already exists)

| rank | combination | what it computes | class | evidence (file:line) | missing | gate with oracle | verdict |
|---|---|---|---|---|---|---|---|
| 1 | **tq.grid_cell + csr row pointers + csheaf.probe + compiled query (morph path)** | nearest/range query = bucket lookup (O(1)) + window scan (O(density)) executed as native code with no per-row interpreter | O(N) -> O(N/cells), x no VDBE (M1 x M2) | tq.bp:29-38 (cell), tq.bp:88-94 (window count exists, unused for pruning), csr.bp:31-74 (rp/ci layout), csheaf.bp:44-57 (probe), cli_compile 3464 + morph.bp (publish) | one function: build rp[] by cell (counting sort O(N)), query scans rp[c]..rp[c+1] over 9 cells | T100 §4.3 rows (d) vs (b): ≤ 10 us vs sqlite C-API; fold == python | **mechanism, the mega-important one**: 10^4 over sqlite's scan, 3-25x over sqlite's own index, and the gap to Rust is only codegen |
| 2 | **activity bits (substrate/spike drain) + csheaf local check + mvcc CoW** = incremental validation of a delta | after k writes, re-validate only the incident edges of the k touched nodes and re-fire only their dependents | O(k * degree) instead of O(N) per commit | substrate.bp:34-58 sweep, spike.bp:17-38 tzcnt drain, csheaf.bp:60-83 check() O(degree), mvcc.bp:48-65 append | a store larger than 16 slots; an activity word per 64 cells (bitset.bp layout); the sweep constant must drop from 205 ns to ~10 ns (P1/P2) | P9: N=65536 cells, k ∈ {1,16,256,4096}: sweep ms vs full recompute ms; Rust twin of both; pass = crossover recorded | asymptotic, honest regime k/N < 3%; the substrate's real home |
| 3 | **Stokes boundary (tdgstokes.boundary) as the commit audit of stm** | audit a transaction over an R-cell region by summing its perimeter instead of its area | O(√R) vs O(R) | tdgstokes.bp:25-40 vs 43-57; stm.bp:158 inline scalar check | a region larger than 8x8; stm calling `boundary` on the touched region | gate `stokes1m`: 1024x1024 grid, region 256x256: boundary ops vs interior ops == same flux; python oracle | mechanism, √N class; needs one call to wire |
| 4 | **digest-keyed compiled artifacts** (.becache concept + cli_compile + morph) for queries (T32 qjit) | query text -> digest -> .bin replay; compile once, run natively forever | O(compile) -> O(1) per repeat | OPTIMIZATIONS.md:19-23 (12x warm replay in the old pipeline), cli_compile 3464-3538 atomic publish | 30 lines in cli_compile: digest(source) -> path lookup before emit_words | std_golden warm ≥ 5x; qjit gate: 1000 predicates, 10 distinct: ≤ 10 compiles | mechanism; language-independent but free here |
| 5 | **hvham NEON builtin applied to hv.bp / deltasync / attn / ringvsa** | Hamming/XOR-popcount over 1024-bit HVs at 128 values per 2 instructions | constant 5x over scalar SWAR; 1.0x vs Rust | bebop.bp:584-663; hv.bp:93-106 scalar hv_pop1; k7neon.bp already does it | replace hv_pop1 loops with hvham2 calls | k7 vs k7neon in-proc ms ratio ≥ 4x; fold unchanged | constant factor; do it, do not call it a breakthrough |
| 6 | **pool.bp workers + mvcc/stm** = the first real concurrent transaction test | two clone'd threads committing through stm validation on a shared arena | correctness, then ≤ 4x throughput | pool.bp:31-60 (clone/futex/atomic_add), stm.bp:126-136 validation | shared store cells in the arena; atomic commit flag (sys_atomic_add) | gate `stm4`: 4 threads x 10k commits, fold == sequential fold, lost updates = 0 | mechanism for correctness; speed ≤ 4x |
| 7 | **grid buckets + 4 cores** (rank 1 + P8) | each core owns a stripe of cells; queries dispatched by cell | ≤ 4x on compute; 1.0x when DRAM-bound | pool.bp, rank-1 structure | — | `nn4` | constant; only after rank 1 |
| 8 | **bitset.bp + CNT tier** (agent finding) | membership counts over a bitset via NEON cnt instead of one division per bit | 100x over today's bitset.bp (which divides per bit because `>>` is unused there) | bitset.bp:24-27; hvham cnt tier | rewrite bitset_word with shifts (op 16 exists) then hvham for counts | bitset fold unchanged; ns per 64K-bit count | constant; today's bitset is a 100x loss |
| 9 | **rewrite.sd_canon digests + cache_store** = hash-consed normal forms | canonical fingerprint of a diagram cached so identical sub-terms rewrite once | O(terms) -> O(distinct terms) | rewrite.bp:84-162 canon, spectral.bp:351-373 cache | a table keyed by canon digest | rewrite gate with 1000 terms, 50 distinct: ≤ 60 normalisations | mechanism at toy scale; niche |
| 10 | **regrec (T26) + mmap of the .bt** = zero-copy load of a table | ldp x9,x10 from the mapped base = the record | 1.0x vs sqlite pages; 10-100x vs serde | seed.S mmap; store.bp sys_export | T26 itself | RSS + ms for 1M records load | constant; language-independent |
| 11 | L-system rules + WHT (holo) "compute instead of read" | regenerate a deterministic table from a seed | ≤ 2-3x vs DRAM for streamed-once data; 0x when it fits L2 | lsys.bp, holo.bp:201-215 | a consumer that needs generated data exactly once | bandwidth GB/s vs generated values/s | **metaphor** as a speed claim; a 2x at best |
| 12 | eigentime as scheduler; WFE at quiescence; "one conditional branch" | iteration-count-until-periodicity; sleep; runtime cells for straight-line code | none | seigtime.bp:119-210; T74; T55 spike 41x/740x against | — | — | **metaphor**; delete from the speed story |
| 13 | .bt incidence tensor as compile-time levelizer (D8(2)) | topological levels of the expression DAG emitted straight-line | 1.0x on A78 (OoO does it); few % on A55 | lower.py depth computation | — | spike linear-inlined vs levelized: expect 1.0x | **metaphor** on OoO cores; a linear compiler already emits levels |
| 14 | Z2 nilpotent tokens + cores = "lock-free MVCC" | odd-sector token product = 0 detects a conflict | correctness device, O(1) per check; no throughput mechanism without the cores of rank 6 | zgrade.bp:88, mvcc.bp:28-46 | rank 6 | rank 6's gate | mechanism for detection; "lock-free across 4 cores" is unbuilt |

The one surprising, well-argued point: the repo already contains every piece of a native
bucketed spatial index (rank 1) — the cell coordinate, the CSR row-pointer layout, a hash
probe, an atomic publish path and a compiler that emits the scan as machine code — and the
gate that would show "orders of magnitude vs SQL" is the only gate that was never written;
instead tq.bp scans N with an 800-op geodesic, which is the shape that loses by 10^4.
The falsifier is P6: if bebop's bucketed query is not ≥ 3x faster than sqlite's C-API
indexed query at ≤ 10 us, the "language IS the database" speed thesis is dead on this box
and only the scan-class claim (M1, 10-100x) survives.

### 6.3 Where the roadmap's language is metaphor, not mechanism (flagged once, here)
"post-von-Neumann substrate" (a bit-scan loop over ≤ 26 cells), "eigentime" (iteration
count), "fibers" (a 4-cell loop), "LOD zoom" (equality proof), "entropic collapse" (counter
reset), "self-replication" (recompute before/after edit), "multiversal superposition" (top-2
eigenvectors of a 4x4 built to have a gap), "holographic artifact" (4x redundancy),
"content-addressed morph" (iteration-indexed filenames), "O(1) insert/search" (O(N) scan),
"no GC = speed", "bare metal 8-12x", "15-50x vs SQL" (true only for scans, after T96).
The mechanisms that are real: CSR, hash probe, tzcnt drain, CoW append, nilpotent product
test, Stokes boundary sum, FWHT/NTT/Haar, DPLL, Kahn/topological nilpotent-matrix test,
NEON hamming, clone/futex/LSE pool, atomic tmp+rename publish, digest-keyed replay.
