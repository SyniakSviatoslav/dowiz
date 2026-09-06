Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on B1 (fsync, harness), B4 (per-matrix versions), B3 (objects). Mandatory per operator 2026-09-06. Feeds B6 (readers on snapshots) and B8.

# B5 multi-writer -- partitioned writers, one global root swap, in-process 2PC, bounded retry

## 0. Goal

Several writer threads commit concurrently without lost updates: partition-local transactions never wait on each other; cross-partition transactions commit atomically. Gates G10: 3 writer threads x 10^5 updates on disjoint partitions, folds == oracle; cross-partition commit atomic under kill -9; throughput >= 2x a single writer on 3 A78; zero lost updates (G6 style: today G6 = 4 writers x 10^4 atomic increments, 0 lost -- verified HISTORY STORE PULL).

## 1. Scope

In: partition objects (each with its own append cursor region and partition root), the global root = table of partition roots, `st_begin_p/st_commit_p` (partition tx), `st_commit_2pc` (cross-partition), reader validation (generation compare), the inter-process lock (`O_EXCL` lock file per partition), conflict = abort + bounded retry. Out: distributed transactions, row-level locking, any change to object headers. Fixed points: single-writer semantics remain the degenerate case (one partition) -- every existing gate (G5/G6/G7/G8) must be unchanged with one partition; the superblock stays 16 cells (its `root` cell now names the partition table).

## 2. Preconditions

- Today: one append cursor `used` in the superblock (`sb+4`, verified store.bp:100-110 `st_begin` reads `base[sb+4]`), one writer; in-process mutual exclusion by `sys_atomic_add` + futex (pool.bp:73 `par_merge` pattern; G6 in std_golden); `sys_clone` flags 68864 with 64 KiB stacks above the arena cursor (pool.bp:31-47), `sys_cond_set`/`sys_futex_wait_guard`/`sys_futex_wake` (pool.bp:12-15), `sys_atomic_add` (bebop.bp:1279).
- stm.bp (`begin`:90, `commit`:113: conflict = nilpotent product) and mvcc.bp (`upd`:48, `acq`:67, `rel`:79, `rdchk`:103) are the in-tree references for read-set validation and versions.
- LMDB/SQLite facts (RESEARCH-NOPOINTERS-SQL §2.1): the deployed SQL class is single-writer; multi-writer here is partition-level, honestly stated.

## 3. Design

Partitioning: the arena file is divided into P regions (P <= 8, one per A78/A55 pair at most; default P = 3 on this box): region p = `[base_p, end_p)` cells; each region has its own `used_p` cursor and its own partition root `root_p`. The global superblock's `root` points at a `PartTab {P, (root_p, used_p, gen_p) x P}` object written by the committing thread of every commit (16 + 3P cells, append-only like everything).

```
st_begin_p(base, tx, p):    cursor from PartTab.used_p (from the live superblock); tx[6] = p
st_alloc(base, tx, ...):    unchanged (bumps tx[2] inside region p; exit 80-class trap at end_p)
st_commit_p(base, tx, root_p):
    lock_p  (in-process: futex word per partition, acquire by atomic CAS-loop with sys_atomic_add semantics as in G6;
             inter-process: O_EXCL file <store>.lock.p, created at open, removed at close)
    PartTab' = copy of the live PartTab with (root_p, used_p, gen_p+1)      -- allocated in region p (small)
    superblock toggle (st_sb_write_m) with root = PartTab' , global gen+1    -- ONE global writer lock around the toggle
    msync region p range + superblock pages (B1 order) ; unlock
st_commit_2pc(base, txs[], roots[]):    for each p: lock_p ; msync region p appended range   (prepare)
                                        PartTab' with all P entries updated ; one superblock toggle ; msync ; unlock all (commit)
readers:                                st_snapshot -> PartTab -> root_p ; a cross-partition read records (p, gen_p) and re-validates
                                        against the PartTab of its snapshot only (snapshot isolation: no validation needed
                                        for a single snapshot; validation is for read-modify-write across snapshots = STM rule)
conflict:                               a writer whose read-set gen_p changed before commit aborts (st_abort) and retries <= 8 times, then error 91
```
The global toggle is serialised by one short lock (microseconds); partition appends and msyncs run in parallel -- that is where the >= 2x comes from. Write amplification: one PartTab per commit (16 + 3P cells) -- negligible. Ceiling: msync-bound durable commits (~500 us each) scale with partitions until the device queue saturates; non-durable commits scale until DRAM bandwidth.

Inter-process: a second process opening the store takes `O_EXCL` lock files; the futex words are per process, so cross-process writers on the SAME partition are excluded, on different partitions allowed. The global toggle across processes uses the same lock file discipline on `<store>.lock.sb`.

Failure modes: kill -9 between prepare and commit -> superblock still names the old PartTab: all partitions roll back together (their appended cells are garbage above `used_p`, reclaimed by the next append -- as today); kill -9 after the toggle but before msync of a region -> B1's torn-write rule: the region's data was msync'ed in prepare BEFORE the toggle, so the superblock never names unsynced data; lock file left behind after a crash -> opened with a pid check (`/proc/<pid>` absent -> stale, removed).

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/prelude/store.bp | `st_begin`:100, `st_commit_m`:138, `st_commit_sync`:176, `st_snapshot`:150 | partition-aware variants (`_p`), PartTab, 2PC, locks (~180 lines); single-partition path byte-identical in behaviour |
| selfhost/std/pool.bp | :31-72 | writer-thread helper (spawn P writers with region ids) |
| bebop.bp | `emit_sys_open` (O_EXCL flag value) | verify the flag is passable; else +1 builtin word |
| bench/vs_rust/std_tests/smw.bp | new | G10 program: P writers, disjoint partitions, cross-partition tx every 100th, folds |
| bench/oracles/smw.py | new | deterministic fold |
| bench/vs_rust/scrash.sh | :12-30 | `--mw` variant: SIGKILL during 2PC |
| bench/vs_rust/sbench.sh | rows | `commits/s` with P = 1, 2, 3 (durable and non-durable) |

## 5. Steps

1. PartTab + `_p` transaction API with P = 1; every existing store gate green and byte-identical folds (the degenerate case).
2. P writer threads on disjoint partitions (in-process locks), G10 folds, throughput rows P = 1/2/3.
3. `st_commit_2pc` + read-set validation + bounded retry; kill -9 variant; inter-process lock files.
Each step one chain-gated commit.

## 6. Constructs, oracles, twins

- Gates: `smw` (std_golden), `scrash --mw` (TRIALS 50 in battery, 1000 as a row), G6 unchanged.
- Rows: commits/s durable/non-durable by P; sqlite twin: WAL single writer (its ceiling) for context.

## 7. Gates

```
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp             # GREEN incl. smw, scrash --mw
BEBOP_TMP=$OUT bash bench/vs_rust/sbench.sh                       # commits/s: P=3 >= 2 x P=1 (non-durable); durable row reported
TRIALS=1000 bash bench/vs_rust/scrash.sh --mw                     # 0 failures; every reopen: all partitions at the same global gen
```
RED: a lost update (fold below oracle), a reopen with partitions at different gens (2PC broke), P=3 < 1.5x P=1 (lock contention: the global toggle is doing too much work).

## 8. Risks and probes

| risk | probe |
|---|---|
| `sys_atomic_add` CAS emulation for the lock | use the G6 pattern (fetch-add ticket lock) rather than CAS; probe with 3 threads x 10^5 |
| clone-spanning fn keeps > 8 live symbols (pitfall) | writer bodies as separate small fns |
| bpref parity for threads (sequential emulation) | folds are order-independent (sums/xor) by design |
| region exhaustion in one partition while others are empty | exit 80 per region; sizing = arena/P; report |

## 9. VERDICT format

```
VERDICT: GREEN|RED
smw: writers <P> x <n> updates, fold <equal|MISMATCH>, lost <0>
commits_per_s: P1 <v> P2 <v> P3 <v> (durable P1 <v> P3 <v>)
crash_mw: trials <n> failures <k>
existing_gates: G5 G6 G7 G8 unchanged
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; store.bp tx API anchors; pool.bp clone/futex idiom (:1-72); G6/G5 harnesses; $OUT; rules; `<constraints>` single-partition behaviour unchanged (gates), no in-place writes, <= 8 live symbols in clone fns; `<output_format>` §9; `<task>` steps 1-3.
