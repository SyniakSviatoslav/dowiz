# REPORT-g5b — B1 torn-write harness, measured

Status: 2026-09-06, prep session. Harness: `bench/vs_rust/scrash_torn.sh` (see
`bench/vs_rust/PREP-b1-b3-b7.md` section 1 for the full method). Store side runs against
`bench/vs_rust/std_tests/scrash_small.bp` (same writer/reader/digest/LCG as the committed
G5 `scrash.bp`, 1000 generations / 4 MiB arena instead of 10^4 / 64 MiB, purely to keep
each trial's file small -- the harness logic and the crash model are identical). sqlite
side runs a 300-commit single-row-insert WAL workload with the equivalent page-tear model
applied to the `-wal` tail. Command lines, seeds and box (`nice -n10 taskset -c0-3`) as in
the task; both runs used seed `20260906`.

## Gate

The G5b gate is **0 invalid reopens**. Neither run hit 0.

```
VERDICT: RED (store side; gate = 0 invalid reopens, gap documented below)
torn_trials: 1000 failures 475 (bebop store, scrash_small.bp NGEN=1000) ; sqlite_wal: 1000/0 (0 invalid, NGEN=300 commits sampled)
```

## Store (bebop) — 1000 trials, 475 invalid reopens (47.5%)

```
$ TRIALS=1000 BEBOP_TMP=$OUT BEBOP_BIN=./bebop.bin nice -n10 taskset -c0-3 bash bench/vs_rust/scrash_torn.sh
...
picked generations: 411 distinct, k-1/k pairs seen ok
scrash_torn: 1000 trials, 475 invalid reopens (bebop store, NGEN=1000)
```

Every one of the 475 failures is the SAME failure mode, cross-checked by grepping the
full trial log (`grep -c "torn object"` = 475 = the total invalid count; 0 "gen ... not
in {...}" mismatches, 0 reader traps/timeouts, 0 fold mismatches outside this category):
`bench/oracles/scrash.py --parse` raises `AssertionError: ('torn object', <offset>)` --
the picked superblock IS generation k (or k-1, always a valid member of `{k-1,k}` -- the
harness's own "gen not in {k-1,k}" counter never fired), but the chain walk from that
superblock's root hits an object whose crc fails, because the payload page holding that
object was torn or zeroed by the trial while, independently, the superblock page that
names it survived as a fully valid `new` write.

**Root cause, not a harness artifact**: `scrash.bp`/`scrash_small.bp` commit with plain
`st_commit` (store.bp:134) -- there is no `st_sync`/`st_commit_sync` call anywhere in the
writer, and `sys_fsync` does not exist yet (`grep -n fsync bebop.bp` = 0 hits, checked
this session). `st_commit_sync` (store.bp:176-181) is specifically designed to prevent
this: msync the appended payload range, THEN toggle the superblock, THEN msync the
superblock pages -- an order that guarantees a reader can never observe a valid-crc
superblock for generation k whose payload is not yet durable. Without it, the harness's
page-independent tear model (each of the 1-3 changed pages per commit gets old/new/torn/
zeroed independently, matching a real disk write-back with no ordering barrier) exposes
exactly the failure the blueprint's own risk table names (section 8: "reopen at gen k
with a torn data page passing `st_snapshot` but failing `st_verify` -- means a commit
published before its range was synced -- order bug"). This is the B1 blueprint's GAP,
explicitly deferred in this task ("the `sys_fsync` builtin ... is deferred until the
compiler work lands -- record that gap, do not add the builtin") -- **not closed here**.
Closing it is B1 steps 1-2 (`sys_fsync`, `st_commit_batch`, `st_verify`, and wiring
`st_commit_sync` -- or an equivalent barrier -- into the writer), which is compiler work
out of scope for this python/shell/data prep task.

Sanity checks performed before trusting this result:
- The superblock reconstruction formula (gen/root/cursor/live are closed-form in the
  generation number; magic/version are constants; crc32 = `zlib.crc32`) was verified
  byte-for-byte against the real store's actual last two generations (999, 1000) before
  being trusted for arbitrary k -- exact match including the crc word.
- An earlier version of the harness had a cell-index/byte-offset bug (superblock slot
  512 is CELL index 512 = BYTE offset 4096, not byte offset 512) that corrupted the wrong
  region of the image and produced "generation not in {k-1,k}" failures instead of the
  real "torn object" ones; that bug is fixed and confirmed absent from the 1000-trial run
  (0 occurrences of that failure string in the log, 411 distinct generations picked
  showing k-1/k pairs resolve correctly whenever the tear doesn't hit a payload page).
- The reader (real `seed/build/seed` binary) and `bench/oracles/scrash.py --parse` were
  both run per trial with a 5 s timeout; no trap/timeout occurred in 1000 trials.

## sqlite WAL — 1000 trials, 0 invalid reopens

```
$ TRIALS=1000 BEBOP_TMP=$OUT nice -n10 taskset -c0-3 bash bench/vs_rust/scrash_torn.sh --sqlite
scrash_torn --sqlite: 1000 trials, 0 invalid reopens (NGEN=300 commits sampled)
```

Same page-tear methodology (old / new / torn-at-a-random-point / zeroed applied to the
bytes appended to the `-wal` file since the previous commit; the main `.db` file never
changes after the initial schema commit, so it needed no tearing). 0/1000 invalid: every
reopen resolved to either generation k-1 or k with the correct fold, including all
"torn" trials (a randomly truncated partial last frame) and "zeroed" trials (a
full-length but garbage tail). This is the expected, designed-for outcome -- SQLite's WAL
format carries a per-frame checksum specifically so that a torn/garbage tail is detected
and the replay simply stops at the last good frame; it does not need (and was not given
here) an application-level `sys_fsync` equivalent for this property to hold, because the
ordering between "append a frame" and "the frame's own checksum bytes" is intrinsic to a
single sequential frame write, unlike the store's separate superblock-vs-payload commit
op.

## Honest comparison

| | trials | invalid | rate |
|---|---|---|---|
| bebop store (`scrash_small.bp`, no fsync yet) | 1000 | 475 | 47.5% |
| sqlite WAL (`journal_mode=WAL`, `synchronous=NORMAL`) | 1000 | 0 | 0% |

This is not "sqlite is better engineered" in general -- it is the SPECIFIC, documented
consequence of the store's durability barrier (`sys_fsync` + `st_commit_sync` ordering,
B1 steps 1-2) not existing yet. The blueprint predicts this exact gap (section 8's risk
table) and gates its closure behind the compiler work this prep task explicitly defers.
Once `sys_fsync`/`st_commit_batch`/`st_verify` land and the writer is changed to call
`st_commit_sync` (or equivalent), re-running this SAME harness (unchanged) against the
FULL `scrash.bp` (10^4 generations, not the small stand-in) is the G5b regression test.

## Reproduce

```
mkdir -p $OUT
TRIALS=1000 BEBOP_TMP=$OUT BEBOP_BIN=./bebop.bin nice -n10 taskset -c0-3 bash bench/vs_rust/scrash_torn.sh
TRIALS=1000 BEBOP_TMP=$OUT                        nice -n10 taskset -c0-3 bash bench/vs_rust/scrash_torn.sh --sqlite
```

## 2026-09-07 — GAP CLOSED: G5b is GREEN, 0/1000 invalid reopens

Status: B1 steps 1-3 non-codegen work (docs/blueprints/B1-durability-torn-write.md), HEAD
7d3e256, bin e9159318 (FROZEN, untouched). `sys_fsync` / `st_compact` directory-fsync
remain OUT of scope (deferred until the A4 fuzz window closes, per this task's own
instructions) -- not needed to close this gap: `st_commit_sync`'s existing `sys_msync`
ordering is what the torn-write proof exercises (a real device flush is unreachable
under proot/f2fs nobarrier regardless, LANG-DB section 3).

```
VERDICT: GREEN
torn_trials: 1000 failures 0 (bebop store, scrash_small.bp NGEN=1000) ; sqlite_wal: 1000/0 (0 invalid, NGEN=300 commits sampled)
```

### What changed since the 475/1000 RED run above

The prep session's 475/1000 result (and its "the sys_fsync builtin ... is deferred, not
closed here" framing) predates work already landed at this HEAD, found while root-causing
the gap this session:

1. **scrash_small.bp already commits via `st_commit_sync`** (store.bp:235: msync the
   appended range, THEN toggle the superblock, THEN msync the superblock pages) --
   confirmed NOT a leftover of the 475/1000 run; `git log`/diff shows this landed before
   this session. `scrash.bp` itself (the separate G5 SIGKILL gate) intentionally still
   uses plain `st_commit` -- SIGKILL has no torn-page model, so it has no ordering gap to
   close; out of scope, unchanged.
2. **`st_verify`/`st_reopen_verify` already exist and `st_open` already calls
   `st_reopen_verify`** (store.bp:103,131,149) -- a torn/zeroed PAYLOAD page behind an
   otherwise-valid-crc superblock is now self-healing: the real reader falls back to the
   other superblock and stamps the bad one's magic word to 0 in the mapping.

A fresh TRIALS=50 measurement at session start (this task's own dynamic-values line)
already showed only 10/50 invalid -- not the 475/1000-style "torn object" failures, but a
DIFFERENT pair of harness-model bugs and one oracle bug, root-caused and fixed this
session (see the inline comments at each fix for the full mechanism):

- **(b) harness bug #1 — durable-prefix tear.** `build_image`'s payload-page loop treated
  the FIRST (straddling) page's pre-commit-k bytes `[page0,lo)` as unwritten (zero-filled
  in `old_page`), so a `zeroed`/`torn` draw on that page could wipe an EARLIER,
  already-durable generation's own root/objects even though commit k never touched those
  bytes again. Empirically reproduced (seed 20260906, trial 0, k=910): page0's variant
  drew `zeroed`, wiping gen 909's root cell (cell 364620, byte 2916960 < lo=2916992);
  `bench/oracles/scrash.py --parse` raised `('torn object', 364620)` while walking gen
  909 -- a generation commit 910 never touched. Fixed in `bench/vs_rust/scrash_torn.sh`:
  only `[max(p,lo), pend)` (the bytes commit k actually appended) is subject to the
  old|new|torn|zeroed draw; `[p, max(p,lo))` keeps its real, durable bytes.
- **(b) harness bug #2 — superblock sector-tear/payload ordering not honoured.** The
  superblock's crc-checked region (cells 0-14 + the crc word = 128 bytes) sits entirely
  inside the first 512 B sector, so a `torn`+`first` draw is byte-indistinguishable from a
  full `new` write (fully valid gen k), and `torn`+`last` is indistinguishable from full
  `old` (gen k-2) -- confirmed empirically on the SAME trial 0 (sb draw was `torn`+`last`,
  landing slot 0 as a fully valid gen 908). The harness didn't correlate this outcome with
  the payload-page tear draws, so a durably-landed gen k superblock could coexist with an
  independently-torn payload page inside k's OWN synced range -- a state `st_commit_sync`'s
  ordering makes physically impossible (the superblock write is ordered strictly after the
  payload msync it depends on). Fixed: when the superblock lands durably `new` (by this
  crc-coincidence rule, not just `choice=='new'`), every payload page in `[page0,hi)` is
  forced to `new` too.
- **(c) oracle bug.** `bench/oracles/scrash.py --parse` picked the higher-gen crc-valid
  superblock and walked its root chain with no fallback -- a payload-verify failure (even
  one the real reader would self-heal via `st_reopen_verify`) was fatal
  (`AssertionError`). Fixed: try superblocks highest-generation-first, fall back to the
  other valid one on a walk failure, mirroring `st_reopen_verify`'s own fallback.

An audit trail was added per this task's instructions: `scrash_small.bp` now prints
`lo hi sbidx` per commit (the exact page-aligned msync range and superblock cell index
`st_commit_sync` used), and `scrash_torn.sh` cross-checks all 1000 printed lines against
its own closed-form `[lo,hi)`/superblock-index model after every writer run -- **0
mismatches** in both the TRIALS=50 and TRIALS=1000 runs below, i.e. the harness's crash
model is now verified byte-exact against the real writer, not merely assumed.

### Report runs (this session, both at the 2-run-per-session cap)

```
$ TRIALS=1000 BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT nice -n10 taskset -c4 bash bench/vs_rust/scrash_torn.sh
sync-range audit: 1000/1000 commits logged, 0 mismatches
picked generations: 627 distinct, k-1/k pairs seen ok
scrash_torn: 1000 trials, 0 invalid reopens (bebop store, NGEN=1000)

$ TRIALS=1000 BEBOP_TMP=$OUT nice -n10 taskset -c4 bash bench/vs_rust/scrash_torn.sh --sqlite
scrash_torn --sqlite: 1000 trials, 0 invalid reopens (NGEN=300 commits sampled)
```

| | trials | invalid | rate |
|---|---|---|---|
| bebop store (`scrash_small.bp`, st_commit_sync + st_reopen_verify) | 1000 | 0 | 0% |
| sqlite WAL (`journal_mode=WAL`, `synchronous=NORMAL`) | 1000 | 0 | 0% |

G5b's gate (0 invalid reopens) is now GREEN on both sides. `scrash_torn` is registered as
a `std_golden.sh` gate (TRIALS=50, golden = 0) with `bench/oracles/scrash_torn.py`;
`FORCE=1 J=1 bash bench/oracles/run_all.sh` -> `SUMMARY ok=111 self-frozen=0 mismatch=0
missing=0` (scrash_torn included). B1 step 2 (`st_commit_batch`, sbench `durable batch
10`/`durable batch 100`/`recover` rows) landed the same session -- see
`bench/vs_rust/RESULT-sbench.md` for the measured numbers (durable batch100 well under
the blueprint's 60 us/commit target).
