Status: 2026-09-08, owner main session, grounded at 0b823d0 / bebop.bin 7939ad7e. From docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §5.2 (proposed there as B8a). Its mechanism is MEASURED, not inferred -- see §1. PROPOSAL pending operator decision.

# D1 build in RAM, publish sequentially

## 0. Goal

A scatter-heavy structure is built in anonymous memory and copied into store
objects in ONE sequential pass, instead of being scattered directly into the
store's shared file mapping.

Gate: **store arm <= 1.3x the plain arm** on the 1M-node / 10M-slot CSR build
(<= ~1 s where today is 18 s), fold identical.

## 1. Why this is not a guess

B2 measured that a CSR build spends 96 % of its time in "the store's page path"
(18,051 ms store fill vs 747 ms on plain arrays, identical fold). R1 then
isolated the mechanism by single-variable elimination, same box state, identical
fold in every arm (docs/exp.journal, 2026-09-08):

| eliminated | how | result |
|---|---|---|
| first-touch faulting | arithmetic: `ci` is 80 MB = 19,531 pages against LANG-DB §0.4's own 0.3 / 3.5 / 7.0 us per page | 6 / 68 / **137 ms** against 18,051 -- refuted by 132x |
| the filesystem | same program, store file on tmpfs vs f2fs | tmpfs 23,074 ms (36.5x), f2fs 8,048 ms (12.8x) -- the RAM-backed store is **3x slower**, so it is not writeback to flash |
| the scatter itself | plain `zeros()` arrays, identical access pattern | ~630 ms |
| **the mapping mode** | `store.bp:146` `sys_mmap(...,3,1,...)` -> `(...,3,2,...)`, nothing else, back to back | **MAP_PRIVATE 1,239 ms vs MAP_SHARED 27,404 ms = 22x** |

So the cost is the dirty-page tracking a shared writable file mapping performs
on every store. MAP_PRIVATE lands within 2x of plain arrays, which is the
headroom this row is claiming.

## 1b. A stronger form of the same fix, found 2026-09-08

The wild-ideas study proposes going further, and it is a better shape: **never map
the store file writable at all.**

The arena is append-only and objects are immutable once committed, so the dirty
set of any transaction is **contiguous by construction** -- exactly
`[old_tail, new_tail)`. The kernel's per-page bookkeeping is therefore tracking
something the store already knows precisely. The alternative is LMDB's default
design: a MAP_PRIVATE (or read-only) view for reading, one `pwrite` of the
appended tail, then the superblock toggle.

That turns D1 from a discipline every bulk builder must remember into a
**property of the store**, and removes the only writable shared mapping the
process ever holds -- which also retires the C1 hazard where a forked kernel
inherits a writable mapping of the file.

**SCOPED 2026-09-08, and the scoping is load-bearing.** The MAP_SHARED tax is a BULK-SCATTER tax: about 10 us of a
229 us durable commit and none at all of the 0.73 us update row. So `pwrite`-the-tail would ADD a syscall to every
commit to remove a cost single-row commits never pay -- it would help bulk builds and HURT the OLTP row this project
is separately trying to win. **D1 as written (§3) is the right change; the stronger form applies to bulk builds only**
and must never become the store's default commit path.

Two further things it does not cover: writes that are NOT appends (tombstone bitmaps, flipped in place) and the
reader's view of a concurrent writer.

## 2. Scope

**In.** The build path only: `rp`/`ci` allocated as plain arrays, filled, then
`st_alloc`'d and copied in one sequential pass, then sealed and committed as
today.

**Out.** Any change to the read path. The thesis sentence -- "a persisted object
IS an in-memory object, same layout, object-relative offsets" -- is preserved
for every read and narrowed for exactly one write shape: bulk construction. That
narrowing is the design of LMDB, SQLite's WAL, every LSM and Datomic, all of
which `docs/LANG-DB-DESIGN.md §1` lists among the SURVIVORS, against the
orthogonal-persistence family that died.

**Also out.** userfaultfd paging (a runtime), huge pages (THP is `[never]` here
and unprivileged), `MAP_POPULATE` of the 512 MB mapping (the file is sparse, so
that allocates blocks for pages never written), and a WAL (refuted; the
append-only arena IS an after-image log).

## 3. Design

    build:    rp = zeros(n+1); ci = zeros(m)      // anonymous, the 747 ms arm
              <counting pass, prefix sum, fill>    // unchanged
    publish:  o_rp = st_alloc(tx, n+1, d); copy   // one sequential pass
              o_ci = st_alloc(tx, m,   d); copy   // 88 MB at DRAM copy speed
              st_seal both; st_commit as today

Two open questions the implementation must answer, not assume:
1. **Does the copy itself pay the same tax?** It is sequential, so dirty pages
   are produced in address order and the kernel can batch them -- but that is a
   prediction. Measure the copy phase separately in the profile table.
2. **Does the plain arm want write-combining too?** It sits ~22x above its own
   ~33 ms bandwidth floor. 1008.2849 reaches 88 % of peak bandwidth on exactly
   this shape (a counting sort) using per-bucket buffers so each destination
   line is written once, whole. ~40 lines, and it is a SEPARATE step: land D1
   first, measure, then decide.

## 4. Files touched

`bench/vs_rust/std_tests/csr_build_profile.bp` (the kill experiment),
then `selfhost/std/sgraph2.bp:csr_build` and any other bulk builder;
`bench/vs_rust/csr_profile_b2.sh` gains a copy-phase row.

## 5. Steps

1. The kill experiment (§7). ~15 lines, no compiler change.
2. If it passes, move `csr_build` in `sgraph2.bp` and re-run the G8 row.
3. Only then consider write-combining for the plain arm.

## 6. Cost against the invariants

Zero dependencies: none. One-pass: none (no compiler change). D0: none. Memory:
the plain arrays are ~88 MB, already allocated in the arm that takes 747 ms, but
they now coexist with the store objects for the duration of the copy -- on a box
with ~2.3 GB available that is fine at this scale and must be checked at W's.

## 7. Cheapest experiment that kills it

Change `csr_build_profile.bp` so `rp`/`ci` are `zeros()` during the build and
are copied into `st_alloc`'d objects at the end (~15 lines), one slot run.
**If the store arm does not fall below 2 s, the diagnosis in §1 is wrong** --
then sample `/proc/meminfo` `Dirty`/`Writeback` every 100 ms during the original
build to test whether the kernel is writing pages back mid-build.
