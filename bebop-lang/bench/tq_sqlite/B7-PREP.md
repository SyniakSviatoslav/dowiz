# B7-PREP — store-side twins for the DSL/planner blueprint (section 6)

Status: 2026-09-06, store-side prep session (parallel to the bebop.bp/compiler session
and the store.bp/durability session — neither touched here; `selfhost/`, `tools/`,
`docs/`, `bebop.bp` untouched, everything below is new under `bench/`). Grounded on the
already-committed `bench/oracles/tpch.py` / `bench/tq_sqlite/gen_lineitem.py` /
`bench/tq_sqlite/tpch_sqlite.py` (bench/vs_rust/PREP-b1-b3-b7.md) and the current
`./bebop.bin` (md5 `f7a25d38`, HEAD `fc43a9f`). Reads: docs/blueprints/B7-dsl-planner.md
section 6 (the gates: Q6 >= 10x, Q1 >= 5x sqlite native, on lineitem SF 0.1 in the
store), docs/RESEARCH-GRAPHBLAS-2026-09-06.md section 1.2 (Q6 = zone-map skip + masks +
fused madd/sum; Q1 = 6 groups -> dense accumulators, one pass), `selfhost/prelude/
store.bp` (used read-only, as its committed API — nothing in it changed or needed
changing for this task).

## What exists now

| file | what |
|---|---|
| `bench/tq_sqlite/tpch_load.bp` | loads the 600,000-row lineitem CSV into the store as 7 `arr i64` SoA columns (shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax — CSV/oracle column order) + a root object, one transaction, one commit. |
| `bench/tq_sqlite/tpch_q6.bp` | hand-written Q6 kernel: filter shipdate in [1994,1995) / discount in [5,7] / quantity < 24, `sum(extendedprice*discount)`, one fused loop, mask-multiply-accumulate (no data-dependent branch), constants baked in. |
| `bench/tq_sqlite/tpch_q1.bp` | hand-written Q1 kernel: group by (returnflag,linestatus) (dense 6x4 accumulator array, key = flag*2+status, no sort), 4 aggregates per group (count, sum_qty, sum_extendedprice, sum_disc_price), one fused loop; fold = `bench/oracles/lag_common.combine` over the 24 cells in ascending group order. |
| `bench/tq_sqlite/tpch_twin.sh` | twin harness: loads the store, runs both bebop kernels and the sqlite twin (`tpch_sqlite.py`, ctypes/prepared, VM_STEP per LANG-DB-DESIGN.md section 8) R=11 times pinned to core 4, cross-checks folds, writes `bench/tq_sqlite/REPORT-tpch.md` with the section-7 gate verdicts and first-query/repeat latency placeholder columns (real numbers need B7's tier-0/pool, not built yet). |
| `bench/tq_sqlite/REPORT-tpch.md` | output of one R=1 smoke run of the harness (folds verified GREEN; timing numbers explicitly marked as noise/placeholder — box was contended, R=1 is not the real gate; banner at the top says so). |
| `bench/tq_sqlite/B7-PREP.md` | this file. |

Both kernels are written the way `gen_gb`'s generated scan-filter-agg / scan-group-agg
templates should look (docs/blueprints/B7-dsl-planner.md section 3): one `while` loop per
kernel (verified below by disassembly — a single backward branch each, not unrolled or
multi-pass), predicate/group constants as literals in the loop body (the digest-stability
requirement in section 8 doesn't apply yet since there is no digest/pool for hand-written
kernels — that's a gen_gb-only mechanism), dense accumulators for Q1 (no `csr_build`/sort:
6 groups fit in registers/cells directly, per the research doc).

## How to run

```
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT nice -n10 taskset -c0-3 bash bench/tq_sqlite/tpch_twin.sh
```
This regenerates the CSV, compiles all three `.bp` files against `$BEBOP_BIN`, loads a
fresh store, runs the folds + timing, and rewrites `REPORT-tpch.md`. For just the store
side (no sqlite, no timing):
```
./seed/build/seed ./bebop.bin compile bench/tq_sqlite/tpch_load.bp $OUT/tpch_load.bin
./seed/build/seed ./bebop.bin compile bench/tq_sqlite/tpch_q6.bp   $OUT/tpch_q6.bin
./seed/build/seed ./bebop.bin compile bench/tq_sqlite/tpch_q1.bp   $OUT/tpch_q1.bin
python3 bench/tq_sqlite/gen_lineitem.py $OUT/lineitem.csv
./seed/build/seed $OUT/tpch_load.bin $OUT/lineitem.csv $OUT/tpch.store
./seed/build/seed $OUT/tpch_q6.bin $OUT/tpch.store    # -> 114672059591
./seed/build/seed $OUT/tpch_q1.bin $OUT/tpch.store    # -> 6105941479581644684
```

## Folds verified this session

600,000 rows, same LCG-generated data as `bench/oracles/tpch.py` (seed 20260906) and the
sqlite twin (`bench/tq_sqlite/tpch_sqlite.py --check`, reconfirmed this session:
`CHECK q6 OK` / `CHECK q1 OK`):
```
tpch_q6.bp -> 114672059591       (== oracle == sqlite twin)
tpch_q1.bp -> 6105941479581644684 (== oracle == sqlite twin)
```
Verified row-for-row against the CSV too (a small debug dump program, not kept — read
rows 0,1,2,90000,300000,599999 back out of the store and diffed byte-for-byte against
`lineitem.csv`) after finding and fixing the parsing bugs below.

**Two real bugs found and fixed while building the loader** (documented since they are
exactly the kind of trap the task warned about, but weren't in the given list):
1. Python's `csv.writer` (opened with `newline=''`, the correct usage) emits `\r\n` line
   endings, not `\n`. The first cut of `read_int`'s field scanner treated the field
   terminator (comma or the first non-digit byte) as fully consumed and stopped — for the
   last field of every row that byte is `\r`, leaving the following `\n` to be consumed as
   a bogus zero-valued "field 0" of the next row, which shifts every column of every row
   after row 0 by one field forever (Q6 fold came out as 2262538934 instead of
   114672059591; Q1 SIGSEGV'd — the shifted flag/linestatus values sometimes produced a
   group index outside the dense 6-group array). Fixed by having `read_int` peek one byte
   past its terminator and additionally consume it if it's `\n`.
2. `sys_slurp(fd, len)`'s `len` is a raw BYTE count for the underlying `read()` — NOT
   `len` cells (i.e. not `len*8` bytes), even though the arena allocation for the buffer
   *is* `round16(len*8)` bytes (verified empirically: passing `len=2_200_000` against a
   16,068,481-byte CSV silently truncated the read at byte ~2,200,000, i.e. row ~86,000 of
   600,000, leaving the rest of the store's columns at their zero-initialized default —
   every prior use of `sys_slurp` in the tree stays under ~1.6M, comfortably inside a
   ~200 KB-3.2 MB source file, so this units mismatch had never been hit before). Fixed by
   passing `len=16_100_000` (>= the CSV's actual byte size, confirmed with `wc -c`), which
   allocates ~129 MB in the 256 MB program arena — comfortably inside budget since nothing
   else in the loader uses more than a few hundred cells of scratch (columns are written
   straight into the store's own mmap, not staged in the arena).

## Loader

`bench/tq_sqlite/tpch_load.bp`: 600,000 rows, store size 64 MiB (~34 MB actually used: 7
columns x 600,001 cells + root + superblocks), one `st_begin`/`st_commit`. Load time this
session (uncontended-ish, informational only, not gated per the task): **~250-580 ms**
across runs (varies with box contention from the other two parallel workers) for the full
parse-into-store pass, single-threaded, no SIMD, one `st_put` per cell. Row count and
commit generation returned in the packed print value; not a benchmark number.

## Kernel shape verified by disassembly (informational — the "single fused loop" claim)

`objdump -D -b binary -m aarch64` on the compiled `.bin`s, located each kernel's loop by
its baked-in predicate constant (Q6: 0xb1b79/0xb1ce6 = 727929/728294; Q1: 0xb2222 =
729634) and confirmed exactly one backward branch (`b.lt`) closing each loop — i.e. both
kernels really are the single fused scan the blueprint's generator is supposed to emit,
not an unrolled or multi-pass shape:
- Q6: loop body 0x5234-0x5420, 124 words/iteration.
- Q1: loop body 0x5450-0x56c4, 158 words/iteration.
Both counts are dominated by call overhead (`st_get` and, for Q1, the per-row `upd`
accumulator update are compiled as real `bl` calls with stack-spilled arguments, not
inlined — the current compiler doesn't inline across `use`-imported store.bp functions)
rather than by useful work; this is exactly the gap the register-model work and B7's own
`gen_gb` (which can afford to inline a fixed 2-4 field access pattern per template, unlike
a general-purpose compiler) are expected to close. Recorded here as a concrete "before"
number for that later comparison, not a gate.

## What awaits the compiler/planner work (from docs/blueprints/B7-dsl-planner.md)

- `selfhost/std/qdsl.bp` (parser+AST+`explain`), `selfhost/std/qplan.bp` (planner),
  `selfhost/std/gen_gb.bp` (scan-filter-agg / scan-group-agg / join / order-by templates)
  — none of these exist yet; `tpch_q6.bp`/`tpch_q1.bp` above are the hand-written stand-ins
  gen_gb's generated code should structurally resemble (fused loop, baked constants, dense
  accumulators).
- The pool/tier-0 timing infra (B3) that would let `tpch_twin.sh` report a real
  first-query-vs-repeat split; right now those two REPORT-tpch.md columns are literal
  `n/a` placeholders because nothing produces those numbers yet.
- A real (non-contended, R=11, dedicated-core) run of `tpch_twin.sh` for the actual
  Q6 >= 10x / Q1 >= 5x sqlite-native gate — not done here per the task ("functional parity
  now; no timing claims"); the one run captured in `REPORT-tpch.md` is explicitly labeled
  a smoke test, not the gate.
- The join pipeline, order-by/limit (radix sort), and rank-3 construct (B7 blueprint
  section 3/6 steps 2-3) have no store-side twin at all yet — out of scope for this prep
  task (Q6/Q1 only, per the task).
- `selfhost/prelude/store.bp`'s API was sufficient as committed for this task (`st_open`,
  `st_begin`, `st_alloc`, `st_put`, `st_link`, `st_seal`, `st_commit`, `st_map_ro`,
  `st_root`, `st_ref`, `st_get`, `st_digest`) — no blocker, no change requested of the
  worker owning that file.
