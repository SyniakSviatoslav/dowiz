Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on A1 (register model), B2 (twins decided the join path), the A-series builtins `umulh` (plus-times over Q32) and in-process `run(bin)` (A11 dogfooding item; B3 defines the fallback without it). Feeds B4 (assign), B6 (partitioned kernels), B7 (DSL).

# B3 gb.bp + gen_gb.bp -- GraphBLAS objects, generated semiring kernels, instant start

## 0. Goal

The store's matrices become GraphBLAS objects and every operation is a GENERATED kernel per (op, semiring, mask, fmt, transpose), with instant start: first query <= 1 ms (tier-0 generic kernel), specialised kernel <= 50 ms in the background, repeats 0 ms (digest memo), and a PreJIT pool shipped in the store image so the 36 standard kernels never compile at query time. Gates: G9a round-trip across two bases, G9b LAGraph-style folds (BFS, PageRank, TC, CC, SSSP) == python oracle, latency rows.

## 1. Scope

In: `selfhost/prelude/gb.bp` (objects, formats, masks, extract/transpose/reduce-by-row in plain .bp), `selfhost/std/gen_gb.bp` (kernel generator: templates -> `.bp` text -> compile -> pool), the tier-0 generic kernels (one per op, semiring id as a runtime parameter), the kernel pool object, the memo. Out: assign/subassign (B4), multi-core partitioning (B6), the DSL (B7), rank-3 storage (a mode = another CSR, RESEARCH-GRAPHBLAS §1.3). Fixed points: CSR contract `rp[n+1]`, `ci` ascending within a row, `vv` Q32 (verified selfhost/std/csr.bp:5-8); store object model h0/h1 (`st_alloc`:112, `st_seal`:121); `csr_build` (sgraph2.bp:24) stays the transpose/build primitive.

## 2. Preconditions

- Existing mechanisms (verified): frontier BFS = SpMSpV over or-and with push/pull and alpha 14 (sgraph2.bp:318 `push_step`, :366 `pull_step`, :399 `bfs_frontier`), bitmaps `bit/setbit` :307-308, tombstone filter `nbr_live` :313, `csr_spmv` plus-times over Q32 (csr.bp:94), `spmv_fp` in spectral.bp, `csr_build` counting sort (sgraph2.bp:24), G2 root layout `{N, E, ref RP, ref CI, ref LOG, ref RP0, ref CI0, ref TB, nlog}` (sgraph2.bp:9-10).
- `st_digest(s)` (store.bp:203) and the T108 digest memo of compiled kernels (HISTORY STORE PULL) -- the pool key.
- `sys_mmap` (bebop.bp:5620) with PROT_EXEC for the pool; `sys_clone` (bebop.bp:1159) for the background compile; `sys_rename` for atomic pool publish (F2 pattern, bebop.bp cli_compile comment).
- `umulh` builtin (A-series; ~8 words, `umulh xd,xn,xm`) -- without it plus-times over Q32 stays the schoolbook `fp_mul` (~30 ops/nnz, csr.bp:14-16) and the PageRank row is not honest.

## 3. Design

Objects (store cells, all refs object-relative via `st_ref`/`st_link`, store.bp:197-198):
```
GbMatrix  {n, m, nnz, fmt, ref rp, ref ci, ref vv, ref mask, gen, ref prev}     fmt: 1 CSR, 2 bitmap, 3 iso-CSR (no vv), 4 hypersparse (ref rows = nonempty row list)
GbVector  {n, nnz, fmt, ref idx, ref val}                                        fmt: 1 dense (val only), 2 sparse (idx sorted + val), 3 bitmap
Kernel    {digest, op, semiring, mask, fmt, transpose, ref words, entry, gen}     words = the compiled .bin cells (code + literals), entry = offset of main
Pool      {count, ref kernels[]}                                                  root-reachable; published by root swap like any object
```
Semirings (id: ⊕, ⊗): 1 or-and, 2 any-pair, 3 plus-times (Q32: `umulh`), 4 min-plus, 5 plus-second, 6 max-first, 7 plus-pair. Ops: mxv (push), vxm (pull), mxm (Gustavson + SPA: dense accumulator `n` cells + touched list), eWiseAdd, eWiseMult (two-pointer merge of sorted rows), select (2-pass count/fill), apply, reduce (row / column via the transposed CSR / scalar), extract (= `csr_scan` slice), transpose (= csr_build over (dst,src)).

Generator: `gen_gb(op, sr, mask, fmt, tr)` builds the kernel source from string templates (one template per op, ~30-120 lines each; the semiring's ⊕/⊗ are text substitutions: `acc = acc + x*y` becomes `acc = if acc < x + y then acc else x + y` for min-plus, etc.), writes `$OUT/gb_<digest>.bp` via `sys_export`, and compiles it with the resident compiler. Two compile paths: (a) `cli_compile` is a function of bebop.bp -- when gb.bp is linked into the compiler image (`use`), compile in-process, no fork; (b) otherwise `sys_clone(17,0)`-fork + the seed binary (until A11's execve exists, path (a) is the one to build).

Instant start, three tiers:
1. Pool hit: `digest = st_digest(op|sr|mask|fmt|tr)` -> Kernel object -> `run`: mmap the kernel's words `PROT_EXEC` once per process (cache the address in a process table), call through `blr` with the argument block (matrix refs + vector refs + out) -- this is the `run(bin)` builtin of A11; until it exists, the kernel is a normal `.bin` executed by fork+exec (8 ms under proot, still << 50 ms).
2. Tier 0: `gb_mxv_generic(sr, ...)` etc. -- one .bp function per op with a `while` over rows and a branch table on `sr` inside the inner loop (2-5x slower than specialised; measured row). Used when the digest is absent: returns the result NOW and enqueues the compile (`sys_clone` thread: generate + compile + append Kernel + link into a new Pool + root swap under the single writer of B5's rules, or via the pool file's own root when the store is read-only).
3. Memo: the Kernel object is immutable; repeats are a lookup.

PreJIT build step: `bebop.bin gbpool <store>` enumerates the 7 semirings x the ops that accept each (<= 36), generates, compiles and appends them once; the store image ships with the pool (T108 memo semantics: digest-keyed, never recompiled).

Failure modes: a kernel compiled by a different bebop.bin (ABI drift) -- the Kernel carries the compiler md5 and is rebuilt on mismatch; a semiring/format combination without a template -> tier 0 forever (row reported); mmap PROT_EXEC refused under a hardened kernel -> fork+exec path (row reported).

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/prelude/gb.bp | new | objects, formats, masks, extract, transpose, reduce-row, pool lookup, tier dispatch (~300 lines) |
| selfhost/std/gen_gb.bp | new | templates + generator + compile driver (~400) ; templates as .bp string tables (~500) |
| selfhost/std/sgraph2.bp | :318-435 | frontier BFS re-expressed as `gb_mxv(or-and)` with the Beamer switch -- fold must not change |
| bebop.bp | emit_sys_mmap :5620 | PROT_EXEC flag accepted (already a parameter -- verify) ; `umulh` builtin (A-series) |
| bench/oracles/gb_lagraph.py | new | BFS/PR/TC/CC/SSSP folds over the same LCG graphs |
| bench/vs_rust/std_tests/gb_*.bp | new | one program per fold |
| bench/vs_rust/std_golden.sh | :39 `gate` | gates gb_bfs, gb_pr, gb_tc, gb_cc, gb_sssp, gb_roundtrip |
| bench/vs_rust/sgraph2.sh | :8-30 | rows: first-query latency (tier 0), specialised latency, pool-hit latency |

## 5. Steps

1. gb.bp objects + extract/transpose/reduce-row + G9a round-trip gate (two bases, G2 style: `st_map_ro` at another address, verified store.bp:94).
2. Templates + generator for mxv/vxm (or-and, any-pair, min-plus, plus-times) + tier-0 mxv; sgraph2 frontier BFS reimplemented on `gb_mxv` (fold unchanged -- this is the oracle for the template).
3. mxm (plus-pair TC, any-pair join), eWiseAdd/Mult, select, apply, reduce; G9b folds.
4. Pool object + PreJIT build step + in-process compile + background compile thread + latency rows.
Each step one chain-gated commit (`--codegen` only for step 4's builtin words).

## 6. Constructs, oracles, twins

- Oracles: bench/oracles/gb_lagraph.py (stdlib): BFS levels sum, PageRank Q32 after 10 iterations (exact integer arithmetic mirrored), triangle count, CC label sum, SSSP min-plus distance sum -- on the sgraph LCG graph (1M/10M) and a small 1k graph for the constructs.
- Gates: `gb_roundtrip`, `gb_bfs`, `gb_pr`, `gb_tc`, `gb_cc`, `gb_sssp`, `gb_pool` (pool hit == tier 0 == specialised folds).
- Twins (rows, not gates): BFS ns/slot vs Rust CSR (B2-style twin), PageRank ns/nnz vs Rust.

## 7. Gates

```
BEBOP_TMP=$OUT tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp        # GREEN incl. gb_* gates
BEBOP_TMP=$OUT bash bench/vs_rust/sgraph2.sh                               # frontier fold unchanged; latency rows:
#   first query (tier 0) <= 1 ms ; specialised compile <= 50 ms (background) ; pool hit <= 0.1 ms ; tier0/specialised <= 5x
```
RED: any fold mismatch; tier-0 slower than 5x specialised (template bug); pool kernel md5 mismatch not detected.

## 8. Risks and probes

| risk | probe |
|---|---|
| generated .bp trips the compiler's traps (exit 95/98/89) | every template compiled in the construct lane with a 1k graph |
| SPA accumulator (n cells) overflows L2 on 1M | measure mxm on 100k and 1M; note the DRAM regime |
| background compile thread and the single writer race on the pool root | pool lives in its own store file with its own root; B5 rules apply |
| plus-times without umulh | row marked "schoolbook" until the builtin lands |

## 9. VERDICT format

```
VERDICT: GREEN|RED
gates: gb_roundtrip gb_bfs gb_pr gb_tc gb_cc gb_sssp gb_pool -> pass/fail each
latency_ms: tier0 <v> ; specialised <v> ; pool_hit <v>
pool: <count> kernels, <bytes>
frontier_fold: unchanged|CHANGED
rows: bfs ns/slot <v> (rust <v>) ; pr ns/nnz <v> (rust <v>)
journal: <line>
open: <templates missing, deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; sgraph2.bp:318-435 as the oracle implementation of mxv; csr.bp contract; store.bp object API; $OUT; harness rules; `<constraints>` fold-preserving rewrite of frontier BFS first, kernels generated not hand-written, zero deps; `<output_format>` §9; `<task>` steps 1-4.
