# PREP-b1-b3-b7 — B1/B3/B7/B4 preparation deliverables (python/shell/data only)

Status: 2026-09-06, prep session (parallel to the bebop.bp rewrite and the B2-twins prep
session). Nothing here touches the compiler, `selfhost/`, `tools/`, or `docs/` — every file
is new, under `bench/`, or scratch under `$OUT`. Grounded against the committed
`./bebop.bin` and `selfhost/prelude/store.bp` at the HEAD this session started from.

## 1. B1 — G5b torn-write harness

Files:
- `bench/vs_rust/std_tests/scrash_small.bp` — byte-for-byte the writer/reader/digest/LCG
  of `bench/vs_rust/std_tests/scrash.bp` (T113/G5), with a smaller generation count
  (1000, not 10^4) and arena size (4 MiB, not 64 MiB) so every trial's crashed-image file
  stays small (the box's filesystem-heaviness rule). `bench/oracles/scrash.py` is already
  generic in the generation number (no scrash.bp-specific constant), so it is reused
  UNMODIFIED against this writer's store — verified byte-exact (see below).
- `bench/vs_rust/scrash_torn.sh` — the harness. `scrash_torn.sh` runs the bebop-store
  trials; `scrash_torn.sh --sqlite` runs the sqlite-WAL trials. Both take `TRIALS`
  (default 1000) and `BEBOP_TMP=$OUT`.

**How it works (store side)**: run the writer once to build a real, complete store
(1000 generations). The store is append-only + superblock-toggle-only (verified against
store.bp:54 `st_sb_write_m`, :77 `st_pick`, :112 `st_alloc`), so a torn-image for an
arbitrary past commit k can be built directly from the ONE real final file: the payload
prefix up to commit k's cursor is byte-identical to the final file (never overwritten);
the two superblock pages are reconstructed analytically (gen/root/cursor/live are closed-
form in k, magic/version are constants, crc32 is `zlib.crc32` — this was verified against
the real file's actual last two generations' bytes, exact match including crc, before
trusting the formula for arbitrary k). For each trial: pick k uniform in [1,1000]; the
"changed pages" between commit k-1 and k are the ONE superblock page commit k toggles,
plus the (1-2) payload pages spanning the 400 new cells commit k appended; each such page
independently gets old | new | torn (first-or-last 512 B sector from `new`, rest `old` —
the SQLite atomiccommit sector model) | zeroed; the untouched superblock slot is always
set to its correct, valid gen-(k-1) state (NOT the final file's bytes there, which belong
to a much later generation). Reopen with the REAL bebop reader (`seed/build/seed`, 5 s
timeout) AND `bench/oracles/scrash.py --parse` (5 s timeout); invalid = parse fail,
reader non-zero exit or timeout, picked generation not in {k-1,k}, or a fold mismatch
among {reader, `--parse`, `scrash.py <g>`}.

**How it works (sqlite side)**: one real run of 300 single-row `BEGIN;INSERT;COMMIT`s in
WAL mode (`journal_mode=WAL`, `synchronous=NORMAL`); the WAL file only grows (no
checkpoint at this scale) so `os.path.getsize(wal)` right after each commit gives an
append-only length sequence, and (same reasoning as the store) a torn image for commit k
is `wal_bytes[:wal_len[k-1]] + variant(wal_bytes[wal_len[k-1]:wal_len[k]])` with
old=truncate, new=full tail, torn=truncate at a random point inside the tail (a partial
last frame — the case SQLite's WAL format is explicitly built to survive), zeroed=full
length with garbage bytes. The main `.db` file never changes after the schema commit (no
checkpoint), so it is copied byte-for-byte into every trial.

**GAP recorded (not closed here, per the task)**: `scrash.bp`/`scrash_small.bp` commit
with plain `st_commit` — no `st_sync`/`st_commit_sync` call, and `sys_fsync` does not
exist yet (`grep -n fsync bebop.bp` = 0 hits, confirmed this session). Without that
ordering barrier, nothing in the current writer guarantees a generation's payload pages
are durable before its superblock page is toggled. The harness's own "gen k, torn object"
failures (a valid-crc superblock for k whose root chain hits a torn/zeroed payload page)
are exactly this — a real gap, not a harness bug — see `REPORT-g5b.md` for the measured
rate. Closing it is B1 steps 1-2 (`sys_fsync`, `st_commit_batch`, `st_verify`,
`st_commit_sync` wired into the writer) — compiler work, out of scope for this prep task.

**Run it**: `TRIALS=1000 BEBOP_TMP=$OUT BEBOP_BIN=./bebop.bin nice -n10 taskset -c0-3
bash bench/vs_rust/scrash_torn.sh` (store) and `... scrash_torn.sh --sqlite` (WAL). Numbers:
`REPORT-g5b.md`.

## 2. B3 — python LAGraph-style oracles (pending gates)

Files: `bench/oracles/lag_common.py` (shared graph generators + kernels + fold helper),
`bench/oracles/gb_bfs.py`, `gb_pr.py`, `gb_tc.py`, `gb_cc.py`, `gb_sssp.py`.

Three deterministic graphs (the blueprint's own graphs are 1M/10M-scale future-work; this
task's fallback list is used instead): a 64-node ring with 32 diametrical chords, a
1000-node/4000-edge graph from the repo's standard LCG (A=6364136223846793005,
C=1442695040888963407, seed 42; a few duplicate/self-loop draws are dropped, leaving
3880 edges and — usefully for the CC/BFS kernels — more than one connected component),
and a 10x10 4-connected grid. All three undirected.

Kernels: BFS levels sum, PageRank in Q32 fixed point (10 iterations, d=85/100 built by
floor integer division — no float anywhere, including the damping constant), exact
triangle count, connected-components label sum (label = min id in the component), SSSP
via the min-plus semiring (Dijkstra — exact for the same fixed point a GraphBLAS
mxv-min-plus iteration converges to; edge weights are a deterministic pure function of
the edge, `1 + ((lo*1000003 + hi*7919) % 9)`, so SSSP is not degenerately equal to BFS).
Each `gb_*.py` prints one diagnostic line per graph, then a single combined fold
(`lag_common.combine`, a rolling `acc*1000003+v` multiply-fold, matching the repo's
`acc*31+val`-style fold convention elsewhere) as its last line — the shape `run_all.sh`
expects (`tail -1`).

Measured today (seed 42/20260906, this HEAD):
```
gb_bfs:  ring_chords 543 ; random_lcg -3 ; grid10x10 900   -> combined 543003255005778
gb_cc:   ring_chords 0   ; random_lcg 1500 ; grid10x10 0    -> combined 1500004500
gb_tc:   ring_chords 0   ; random_lcg 0    ; grid10x10 0    -> combined 0
gb_sssp: ring_chords 2906; random_lcg 1281 ; grid10x10 5499 -> combined 2906018717035496
gb_pr:   ring_chords -5113871573283360256 ; random_lcg 2510021250460919878 ; grid10x10 -7828416469603635568 -> combined -8699535297439791390
```
(Triangle count is 0 on all three by construction: a ring+chords graph, a bipartite-ish
random sparse graph at this density, and a grid are all triangle-free — this is expected,
not a bug, and gives the eventual `gb_tc` gate an easy first checkpoint.)

**Registration**: NO gate line was added to `bench/vs_rust/std_golden.sh` — there is no
frozen bebop value to compare against yet (no `gb_*` bebop program exists; B3 is
unimplemented past this prep). `bench/oracles/run_all.sh`'s `one()` loop iterates over
gate NAMES parsed out of `std_golden.sh` (`grep -E '^gate ... '`), not over
`ls bench/oracles/*.py` — an oracle file with no matching gate name is simply never
visited, so these 5 files (and `lag_common.py`, `tpch.py` below) cannot print `MISSING`
(that only fires the other way: a gate name in `std_golden.sh` with no oracle file) and
cannot turn any existing lane RED. Verified by reading `run_all.sh` in full this session;
not re-run live here (it would contend for the same cores as the 1000-trial G5b run and
adds no information — the mechanism is a straight grep, not a runtime behaviour). No
`.self-frozen` marker was created either: that convention exists in `run_all.sh` for the
opposite case (a gate name without an oracle) and has zero live examples in the tree today
(`grep -rn self-frozen` finds only the mechanism itself and journal prose, `self-frozen=0`
every time it has been measured) — inventing a new marker file for a case the tooling
doesn't check would be unrequested scaffolding.

## 3. B7 — TPC-H-like lineitem twin (pending gates)

Files: `bench/oracles/tpch.py` (canonical generator + Q6/Q1 fold definitions — the oracle
per docs/blueprints/B7-dsl-planner.md section 6), `bench/tq_sqlite/gen_lineitem.py` (CSV
writer, reuses `tpch.gen_rows` — no duplicated row logic), `bench/tq_sqlite/
tpch_sqlite.py` (sqlite twin: `load`, `q6`, `q1`, `--check`).

600,000 rows (SF-0.1-like cardinality; the real TPC-H SF 0.1 lineitem is ~600,572 rows —
600,000 flat is close enough for a prep twin and stays a round, documented number), one
LCG stream (seed 20260906), columns exactly as named in the blueprint: `shipdate` (a
proleptic-Gregorian ordinal day number, span 1992-01-01..1998-12-31 — matches real
TPC-H's date range and needs no date parsing downstream, just integer comparison),
`discount` (integer 0..10, i.e. TPC-H's own internal percentage-point representation
before it divides by 100 — no float anywhere), `quantity` (1..50), `extendedprice`
(integer CENTS = quantity x a per-row unit price in [90000,210100) cents — TPC-H-*like*,
not the exact TPC-H part-price formula), `returnflag`/`linestatus` (small-int codes for
TPC-H's char enums), `tax` (integer 0..8, same reasoning as discount). Written as plain
CSV to `$OUT/lineitem.csv` (15.3 MB) — loadable by sqlite today and, later, by the store
(a CSV row parses into store cells with no format translation needed).

Fold units are documented in `tpch.py`'s docstring, not TPC-H dollars: **Q6** fold =
`SUM(extendedprice_cents * discount_points)` filtered on `shipdate` in 1994,
`discount in [5,7]`, `quantity < 24` (exactly the real Q6 predicate, expressed in these
integer units). **Q1** fold = groups by `(returnflag, linestatus)` on
`shipdate <= 1998-12-01 - 90d` (exact real Q1 predicate), each group contributing
`(count, sum_qty, sum_extendedprice, sum_disc_price=sum(extendedprice*(100-discount)))`,
folded together in ascending group order via `lag_common.combine`.

**Verified this session** (`python3 bench/tq_sqlite/tpch_sqlite.py --check $OUT/
lineitem.csv $OUT/tpch.sqlite`, ctypes/prepared-statement sqlite twin vs the pure-python
oracle, same HEAD):
```
rows 600000
q6_fold  114672059591      (oracle == sqlite twin: OK)
q1_fold  6105941479581644684  (oracle == sqlite twin: OK)
```
The sqlite twin uses `sqlite3_stmt_status(.., SQLITE_STMTSTATUS_VM_STEP, 1)` per the
LANG-DB section 8 floor methodology; VM_STEP counts and wall-clock rows were captured
under contention with the concurrent G5b 1000-trial run on the same 4 cores, so they are
NOT reported as a clean benchmark here — only the fold match, which is the load-bearing
result for a gate with no bebop side yet. A clean timing pass is one invocation away once
the compiler side exists to compare against: `python3 bench/tq_sqlite/tpch_sqlite.py load
$OUT/lineitem.csv $OUT/tpch.sqlite && python3 bench/tq_sqlite/tpch_sqlite.py q6
$OUT/tpch.sqlite && ... q1 ...`.

No gate lines added to `std_golden.sh` (same reasoning as B3 — no bebop `tpch_q6`/
`tpch_q1`/`c70_qdsl`/`rank3` counterpart exists).

## 4. B4 — sqlite WAL update twin (functional only, not timed)

File: `bench/tq_sqlite/sgraph_update_sqlite.py` — per docs/blueprints/
B4-functional-tensor-updates.md section 6 ("Twin: sqlite WAL UPDATE per row"). Table
`cells(id INTEGER PRIMARY KEY, val INTEGER)`; each of R repetitions runs N single-row
`BEGIN;UPDATE cells SET val=val+1 WHERE id=?;COMMIT` (WAL, `synchronous=NORMAL` — the same
durability class `st_commit_sync` will eventually be compared against), row id chosen by
the repo's standard LCG; reports the MEDIAN us/row across R repetitions
(`bench/tq_sqlite/run.sh`'s `statistics.median` convention). Defaults N=1,000,000, R=11
(the gate's real scale, per the task); **not run at that scale here** — 1e6 fsync-bound
WAL commits x 11 repetitions is tens of minutes of pure I/O with no bebop counterpart yet
to compare against, and the box rules ask for minimal filesystem load during this prep.

**Verified functionally** (`python3 bench/tq_sqlite/sgraph_update_sqlite.py --check`,
N=50, R=3): every single-row update lands exactly (final `val` per row == the number of
times an LCG draw picked that row across all repetitions, asserted row-for-row) and
`statistics.median` is sane. Output: `check OK: 50 rows, 3 reps, single-row UPDATE lands
exactly, median() sane`.

**To run the real gate** once the store side of B4 exists: `N=1000000 R=11 BEBOP_TMP=$OUT
python3 bench/tq_sqlite/sgraph_update_sqlite.py` -> prints `update_wal <median_us_per_row>
1000000 11` plus all 11 per-repetition numbers, for direct comparison against the
store's `us_per_row` (docs/blueprints/B4-functional-tensor-updates.md section 9 VERDICT
line `updates: us_per_row <v> ... sqlite_us_per_row <v>`).

## 5. What awaits the compiler work (all four blueprints)

- B1: `sys_fsync`, `st_commit_batch`, `st_verify`, wiring `st_commit_sync` (or an
  equivalent barrier) into the writer -- closes the GAP in section 1 above; then the
  real `scrash_torn.sh` gate can run against the FULL 10^4-generation `scrash.bp`
  writer, not the 1000-generation `scrash_small.bp` stand-in.
- B3: `selfhost/prelude/gb.bp`, `selfhost/std/gen_gb.bp`, the `umulh` builtin (plus-times
  Q32 without schoolbook) -- then `bench/vs_rust/std_tests/gb_*.bp` programs can be
  written and gated against `gb_bfs.py` etc. in `std_golden.sh`, and the 5 oracle files
  stop being orphans.
- B7: `selfhost/std/qdsl.bp`, `selfhost/std/qplan.bp`, the join/order-by templates --
  then `bench/vs_rust/std_tests/tpch_lineitem.bp` (the blueprint's own compiler-side
  generator/query program) can be gated against `tpch.py`'s folds.
- B4: the tiered GbMatrix (tail/L0/L1), `assign`/`promote_tail`/`promote_L0` -- then
  `sgraph_update_sqlite.py`'s N=1e6/R=11 run becomes the twin row in the B4 VERDICT.

## Files added

```
bench/vs_rust/std_tests/scrash_small.bp
bench/vs_rust/scrash_torn.sh
bench/vs_rust/REPORT-g5b.md
bench/oracles/lag_common.py
bench/oracles/gb_bfs.py
bench/oracles/gb_pr.py
bench/oracles/gb_tc.py
bench/oracles/gb_cc.py
bench/oracles/gb_sssp.py
bench/oracles/tpch.py
bench/tq_sqlite/gen_lineitem.py
bench/tq_sqlite/tpch_sqlite.py
bench/tq_sqlite/sgraph_update_sqlite.py
bench/vs_rust/PREP-b1-b3-b7.md   (this file)
```
Nothing under `selfhost/`, `tools/`, `docs/`, or `bebop.bp` was touched. No commits were
made (per the task's constraints) — the files sit in the working tree for the operator to
review and commit.
