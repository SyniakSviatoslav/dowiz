Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on nothing in Phase A (python part any time; the `sys_fsync` builtin needs the A-series builtin discipline only). Feeds B5 (cross-partition commit reuses the same harness) and B8.

# B1 durability -- torn-write proof, fsync of the directory, group commit, verify, recovery row

## 0. Goal

Prove the store survives a POWER LOSS model (torn sector writes), not only `kill -9`: G5b = 1000 torn-write trials, 0 invalid reopens, the same harness run over sqlite WAL; add `sys_fsync`, group commit (`st_commit_batch`), `st_verify`, and a recovery-time row. Numbers today (verified bench/vs_rust/RESULT-sbench.md rows, HISTORY STORE PULL): G5 SIGKILL 100/100, durable commit 506 us (sqlite WAL NORMAL 78 us / FULL 567 us), reopen 590 us.

## 1. Scope

In: `bench/vs_rust/scrash_torn.sh` (python harness, page-level corruption model), `sys_fsync(fd)` builtin, `st_compact` fsync of the directory after `sys_rename`, `st_commit_batch`, `st_verify`, two new sbench rows (commits/s in batches of 1/10/100; recovery time). Out: WAL/redo (not needed: append-only + root swap is the after-image log, LANG-DB §4c verified docs/LANG-DB-DESIGN.md:190), multi-writer (B5), any change to the superblock layout (16 cells, crc in cell 15, two copies at cell 0 and 512 -- verified store.bp:150-168 `st_snapshot`). Fixed points: `st_commit_sync` order (msync appended range, toggle superblock, msync pages 0-1 -- verified store.bp:176-181) is the durable order and stays.

## 2. Preconditions

- store.bp at HEAD: `st_begin`:100, `st_alloc`:112, `st_seal`:121, `st_check`:126, `st_commit`:134 -> `st_sb_write_m`:54 writes the OTHER superblock (`512 - sb`), `st_snapshot`:150 picks the higher-gen valid copy, `st_sync`:171 = `sys_msync(addr, n, MS_SYNC=4)`, `st_compact`:247 copies into `<path>.tmp` and `sys_rename`s at :305.
- No `sys_fsync` builtin exists (verified: `grep -n fsync bebop.bp` = 0 hits); `emit_sys_rename` exists at bebop.bp:5659, `emit_sys_msync` at :1412 -- the fsync builtin follows the msync pattern.
- G5 harness bench/vs_rust/scrash.sh (verified: TRIALS default 100, SIGKILL at a random moment of the writer run, oracle bench/oracles/scrash.py `--parse` + fold), gate `scrash` in std_golden.sh:634-637.
- Box fact: f2fs `fsync_mode=nobarrier` -- msync/fsync return without a device flush (LANG-DB §3, verified docs/LANG-DB-DESIGN.md:102 ff.); a real power cut cannot be produced here, so the sector model is the proof.

## 3. Design

Corruption model (SQLite atomiccommit, fetched in RESEARCH-NOPOINTERS-SQL §2.1): a sector write is linear and non-atomic; after a crash a sector is either old, new, or torn with the first or last bytes changed. Harness per trial:

```
run writer normally for k commits (k random in [1, 10^4]); snapshot file S_k after commit k
    (the writer prints the byte range [lo, hi) it msync'ed for commit k and the superblock index)
build the crashed image C from S_{k-1} and S_k: for every 4 KiB page changed between them,
    choose uniformly: old page | new page | torn (first or last 512 B from new, rest old) | zeroed
    -- the superblock page 0 (cells 0..1023 = both superblocks) is subject to the same rule
reopen C with the bebop reader and with bench/oracles/scrash.py --parse:
    both must pick gen k or k-1, a crc-clean chain for that gen, and equal folds
invalid = parse fail, gen not in {k-1, k}, fold mismatch, or the reader trapping
```
Why it passes by design: the superblock for commit k is written after its data is msync'ed (store.bp:176-181), each object is crc'd (`st_seal`:121, `st_check`:126), a torn superblock fails its crc and the other copy (gen k-1) is taken (`st_snapshot`:150-168). What can fail: the rename in `st_compact` without fsync of the directory (the new name may not survive), and the batch path below if it ever toggles the superblock before its range is synced.

`sys_fsync(fd)` -- syscall 82 (fsync), one word block after the `emit_sys_msync` pattern (bebop.bp:1412): deliver fd in x0, `mov x8,#82 ; svc #0`, result in x0; add to `bpref.py` as a no-op returning 0, to check_abi's sys allowlist (its words are harvested from `em(` literals by `sys_allow`, verified tools/check_abi.py:102-108), and to the RESERVED builtin list. `st_compact` after `sys_rename` (store.bp:305): `sys_open(dir, O_RDONLY)`, `sys_fsync(dfd)`, `sys_close` -- the directory path = `path` up to the last `/` (or `.` when none).

`st_commit_batch(base, tx, root, tmp)`: identical to `st_commit_sync` but the caller commits N transactions with `st_commit` (no sync) and calls `st_commit_batch` once: msync of the union range `[first mark, cursor)` + the superblock pages. Group commit = one msync per N; correctness unchanged because nothing is published to another process until the superblock page is synced, and in-process readers already see the root swap (that is the G6 model).

`st_verify(base, root, lay, nlay, tmp)`: the Cheney walk of `st_compact` (store.bp:226-246 `st_copy_obj`/`st_forward`) without copying: visit every reachable object from root, `st_check` each, count objects/cells; returns (bad_count << 32 | visited). Used by the harness after each reopen and as a background health row.

Recovery time row: `reopen` phase of sbench already measures open + first record (590 us, verified RESULT-sbench.md); add `recover` = reopen after a torn image (the harness's median) vs sqlite WAL recovery (replay frames: `sqlite3_open` on a db with a non-empty `-wal` after kill; python twin).

## 4. Files and functions touched

| file | function / place | anchor | change |
|---|---|---|---|
| bebop.bp | new `emit_sys_fsync` next to `emit_sys_msync` | 1412 | ~20 lines, 6-8 words; dispatch line in `emit_call_or_ctor` by hash (grep `is_msy` for the pattern) |
| tools/bpref.py | builtin table | grep `sys_msync` | `sys_fsync` -> 0 |
| tools/check_abi.py | `sys_allow` harvests `em(` literals | 102-108 | no edit if the words are `em(` literals |
| selfhost/prelude/store.bp | `st_compact` after `sys_rename` | 305 | fsync dir (~10 lines) |
| selfhost/prelude/store.bp | new `st_commit_batch`, `st_verify` | after 181, after 246 | ~15 + ~40 lines |
| bench/vs_rust/std_tests/scrash.bp | writer prints msync ranges per commit | grep `st_commit_sync` in it | ~10 lines |
| bench/vs_rust/scrash_torn.sh | new harness (python inside, like scrash.sh) | new | ~150 lines |
| bench/tq_sqlite/sbench_sqlite.py | `durable_batch`, `recover` phases | 20-24 (`opendb`, `exe`) | ~40 lines |
| bench/vs_rust/sbench.sh | rows `durable batch 10/100`, `recover` | 22-40 | ~8 lines |
| docs/LANG-DB-DESIGN.md §4g | the torn-write model paragraph | 270 | doc only |

## 5. Steps (one chain-gated commit each)

1. `sys_fsync` builtin + words.objdump + bpref + construct `c69_fsync` (opens a file, writes, fsync, returns 0) + `st_compact` directory fsync. Chain `--codegen` (the words lane needs the listing).
2. `st_commit_batch` + `st_verify` + scrash.bp msync-range print + sbench rows (`durable batch`, `recover`). Chain without `--codegen`.
3. `scrash_torn.sh` + the sqlite WAL torn twin (truncate/tear the `-wal` file at page granularity) + `docs/LANG-DB-DESIGN.md` §4g paragraph; register `scrash_torn` as a std_golden gate with TRIALS=50 (the 1000-trial run is a report row, ~10 min). The worker leaves all three uncommitted per step; the main session commits.

## 6. Constructs, oracles, twins

- `c69_fsync` (bench/parity_constructs): EXPECT derived by bpref (0).
- `scrash_torn` gate: `bench/vs_rust/scrash_torn.sh` exit 0 with `0 failures`; oracle = bench/oracles/scrash.py (`--parse` and fold, unchanged).
- sbench rows: `durable batch 10`, `durable batch 100` (us per commit), `recover` (us) -- twin phases in sbench_sqlite.py: `PRAGMA synchronous=FULL` + N inserts per `COMMIT`; recovery = open after the writer was killed with a non-empty WAL.

## 7. Gates

```
TRIALS=1000 BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/scrash_torn.sh   # 0 failures
TRIALS=1000 ... scrash_torn.sh --sqlite                                              # sqlite WAL under the same tearing: report, both must survive
BEBOP_TMP=$OUT bash bench/vs_rust/sbench.sh                                           # rows: durable 506 -> batch100 <= 60 us/commit; recover <= 2x reopen
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp                                     # GREEN (std_golden incl. scrash + scrash_torn)
```
RED looks like: a reopen at gen k with a torn data page passing `st_snapshot` but failing `st_verify` (means a commit published before its range was synced -- order bug), or a rename lost after a torn directory page (missing fsync).

## 8. Risks and probes

| risk | probe |
|---|---|
| the tearing model is too weak (only whole pages) | add the 512 B first/last-bytes variant (in the design) -- both must be in the trial mix |
| msync under proot returns before the page cache write (nobarrier) -- cannot be measured here | state it in the row note; forward-port on a `barrier`/`strict` box (LANG-DB §6) |
| `st_verify` walking a superseded object graph | walk from the picked root only; superseded objects are unreachable by construction |
| group commit hides a torn middle transaction | the batch's superblock is one write: either all N or none are visible; the harness tears inside the batch range too |

## 9. VERDICT format

```
VERDICT: GREEN|RED
torn_trials: <n> failures <k> (bebop) ; sqlite_wal: <n>/<k>
durable_us: single <v> ; batch10 <v> ; batch100 <v>   (sqlite NORMAL/FULL <v>/<v>)
recover_us: bebop <v> ; sqlite <v>
verify: objects <n> bad <k>
battery: <GREEN line or failing lane>
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

```
<context> repo /root/dowiz/bebop-lang; this blueprint docs/blueprints/B1-durability-torn-write.md;
store.bp anchors (§4); scrash.sh as the harness template; $OUT scratch; chain/battery commands (§7);
traps: never cp over bebop.bin, reap after every run, proc cap 30, no pkill -f literal </context>
<constraints> three commits' worth, left uncommitted one at a time; no store layout change; zero deps;
python only in the harness and the sqlite twin </constraints>
<output_format> the §9 block </output_format>
<task> implement steps 1-3 of B1 and report the VERDICT block after each step </task>
```
