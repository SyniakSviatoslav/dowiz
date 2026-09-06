Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on B3 (kernels) and B5 (snapshot readers with writers). Mandatory per operator 2026-09-06.

# B6 multi-core kernels -- row-range partitioning over the A78 cores, private accumulators, DRAM-aware scaling

## 0. Goal

Every generated kernel (mxv, vxm, mxm, scan/select, reduce) runs partitioned over the big cores with a measured scaling row; gates: K6 scan x1.4-2.2 on 3 cores (nn4 today: 219 -> 99 ms = 2.21x, verified bench/tq_sqlite/RESULT.md via ROADMAP measured table), BFS/PR rows at 1/2/3 cores, no lost updates when B5 writers run concurrently (readers on snapshots).

## 1. Scope

In: a `par` template parameter in gen_gb (B3) that emits the partitioned form; the worker pool (pool.bp pattern) with a work queue of row ranges; per-core private accumulators + merge; the Beamer push/pull switch per partition; core discovery (`/proc/cpuinfo` part 0xd41 = A78, as the bench scripts do -- verified bench/vs_rust/sbench.sh:10-11); a DRAM-ceiling stop rule (do not spawn more workers when the single-core kernel is already at >= 80 % of the measured 12 GB/s stream). Out: NUMA (none), work stealing (static ranges + one dynamic queue for skew suffice), multi-process.

## 2. Preconditions

- pool.bp idiom verified (:1-72): `sys_clone(68864, base + 131072 + i*65536)`, `sys_cond_set`, `sys_futex_wait_guard`, `sys_futex_wake`, park cell; `par_merge` (:73) uses `sys_atomic_add`; pool gate bench/vs_rust/pool_parity.sh; interp mirror = sequential (sys_clone returns 0).
- `sys_setaffinity` builtin (bebop.bp:1324) -- nn4.bp uses it to pin workers.
- Pitfall: a clone-spanning fn keeps <= 8 live symbols; spilled slots are per-thread stack (pool.bp:5-8).
- B3 kernel argument block (matrix refs, vector refs, out) is the unit of work.

## 3. Design

```
par_run(kernel, args, P):
    ranges = split rows [0,n) into P*4 chunks (over-decomposition 4x for skew) ; queue cell q = 0
    spawn P-1 workers pinned to A78 cores 4..6 (sys_setaffinity), the caller is worker 0
    worker: loop { c = sys_atomic_add(q, 1); if c >= chunks break; kernel(args, range c, acc_private[worker]) }
    barrier: done flags + futex (pool.bp idiom) ; merge: acc = ⊕ over acc_private (semiring ⊕), for vectors the merge is
             per-row disjoint (no merge) except reductions
    workers park on the never-woken cell (pool.bp rule: no exit in a clone)
```
Per-kernel rules: mxv/vxm push -- disjoint output rows per range, no merge; pull -- the same; mxm -- disjoint output rows, each worker owns an SPA (n cells x P = the memory cost, report it); scan/select -- private counts then a prefix over P for the fill pass (two-phase, as csr_build); reduce -- private accumulators merged with ⊕. Frontier BFS: each worker handles a row range of the frontier; the push/pull decision uses the global frontier size (one atomic sum per level).

Scaling rule: measure the single-core kernel's bytes/s (bytes touched / ms); if >= 0.8 x 12 GB/s, run with P = 1 (extra cores only burn energy at the DRAM ceiling -- RESEARCH-TENSOR §3). Report rows P = 1/2/3 for every kernel in the twin table.

Interaction with B5 writers: kernels run on a snapshot (PartTab of the reader's `st_snapshot`); writers append elsewhere; no shared mutable cells except the work queue and done flags (private to the kernel run).

Failure modes: a worker faults -> the whole process dies (no isolation; acceptable, as pool.bp today); skew (Zipf rows) -> the dynamic queue bounds it to one chunk; SPA memory x P -> fall back to P = 1 when n*P*8 B > 64 MB.

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/std/gen_gb.bp | B3 templates | `par` parameter: range-taking kernel body + par_run wrapper (~100 lines + template edits) |
| selfhost/prelude/gb.bp | B3 | `gb_cores()` discovery, `par_run`, private accumulator allocation (arena, before spawning: L8) |
| selfhost/std/pool.bp | :31-114 | generalised queue worker (reuse; do not fork the idiom) |
| bench/tq_sqlite/nn4.bp | existing 2.21x measurement | re-expressed on `par_run` (fold unchanged) |
| bench/vs_rust/sgraph2.sh, bench/tq_sqlite/nn4.sh | rows | P = 1/2/3 rows for scan, BFS, PR, mxm |

## 5. Steps

1. `par_run` + scan kernel partitioned; nn4 fold unchanged; rows P = 1/2/3 (expect 2.2x at P = 3 for the codegen-bound scan, lower once the register model makes it DRAM-bound -- state which regime the row is in).
2. mxv/vxm/BFS partitioned (frontier fold unchanged); reduce merge; PR rows.
3. mxm partitioned with SPA memory rule; concurrent B5 writers test (G10 + kernels running: folds equal).
Each step one chain-gated commit (battery + rows).

## 6. Constructs, oracles, twins

- Gates: pool_parity (existing), `gb_par` (folds of every partitioned kernel == the P = 1 fold, in std_golden), G10 concurrency fold.
- Rows: ms and bytes/s at P = 1/2/3 per kernel; Rust twin rows stay single-threaded plus one `std::thread::scope` row for the scan (parity expected).

## 7. Gates

```
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp          # GREEN incl. gb_par, pool_parity
bash bench/tq_sqlite/nn4.sh ; bash bench/vs_rust/sgraph2.sh   # scan P3/P1 >= 1.4 (>= 2.0 if codegen-bound); BFS P3/P1 reported; PR reported
```
RED: any fold difference between P = 1 and P = 3 (race), scaling < 1.2x on a codegen-bound kernel (queue/barrier overhead), process count above the cap during the battery (workers must park, not spin-spawn).

## 8. Risks and probes

| risk | probe |
|---|---|
| more than 30 processes during battery (threads count as procs under proot?) | `tools/reap.sh` after the run; measure with P = 3 only |
| false sharing on accumulators | pad private accumulators to 64 B cells (8 cells) |
| the A55 cores get workers | pin only to part 0xd41 cores; row with A55 included as a negative example |
| sequential interp mirror hides races | folds are order-independent; add an intentional-race probe (shared cell without atomic) expected to FAIL to prove the gate sees races |

## 9. VERDICT format

```
VERDICT: GREEN|RED
scan: P1 <ms> P2 <ms> P3 <ms> (bytes/s <v>, regime codegen|dram)
bfs: P1 <ms> P3 <ms> ; pr: P1 <ms> P3 <ms> ; mxm: P1 <ms> P3 <ms> (spa MB <v>)
folds: P1 == P3 <equal> ; with writers <equal>
procs_peak: <n>
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; pool.bp:1-72 idiom; nn4.bp as the measured precedent; gen_gb templates; $OUT; rules (proc cap, reap, <= 8 live symbols in clone fns); `<constraints>` folds identical across P, no exit inside workers, zero deps; `<output_format>` §9; `<task>` steps 1-3.
