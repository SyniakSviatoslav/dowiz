Status: 2026-09-08, owner main session, grounded at 1ed3719. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §5 (proposal 4, "abolish the WAL for PMEM", verdict REFUTE-as-stated). CONDITIONAL -- do not start this row until its trigger fires. PROPOSAL pending operator decision.

# C6 a `persist` builtin, if a byte-addressable device ever exists

## 0. The proposal's premise is false and the correction matters more than the proposal

The proposal states that the 44.7 ms recovery row exists because disk-style safety is being
imitated in RAM through logging. **There is no WAL in this store.** Nothing logs. The row is
`st_reopen_verify` (`selfhost/prelude/store.bp:131-139`) crc-scanning the object arena from cell
1024 to the live cursor on **every** non-fresh open (`:103-117`).

The fix needs no new hardware and is already designed: a page anchor in a superblock cell so only
the last commit's pages are verified -- `docs/blueprints/B1-durability-torn-write.md:117-188`, with
three acceptance criteria including a negative test that tears a byte in the shared page.

**Do that row, not this one.** C6 exists only to record what survives the correction.

## 1. What survives

Exactly one builtin at two call sites. The format is already DAX-shaped: position-independent
offsets, self-describing objects, two superblocks, root swap. Nothing about the layout assumes a
block device.

- `dc cvap` (clean to point of persistence) + `dsb`, replacing `sys_msync` in `st_commit_sync`
  (`store.bp:235-241`) and in `st_commit_batch`.
- The instruction is available: `dcpop` is present on all 8 cores of this box (ARMv8.2), measured
  by `grep -o dcpop /proc/cpuinfo` = 8.

That is the whole port. It is small precisely because the store never had a WAL to remove.

## 2. Why it must not be started now

- **No device.** `/dev/pmem*` absent; no ACPI NFIT; `/sys/bus/nd` not enumerable from this uid.
  There is nothing to test against, and a durability change that cannot be tested is worse than
  no change.
- **No market.** Optane was cancelled in July 2022, and this project's target nodes are couriers'
  phones (D0: local-first, mesh). Optimising exclusively for hardware no target node has is the
  opposite of the invariant.
- **The headline claim is false even on real PMEM.** "Recovery = 0" does not hold: ADR platforms
  do not flush CPU caches, so a torn multi-word commit is exactly as possible as a torn page --
  PMDK's own store replays undo/redo logs at open. Byte-addressability moves the tear from a
  4 KiB page to a cache line; it does not abolish it.
- **Power-loss durability is not provable on this box in any case**: the f2fs mount hosting the
  tree is `fsync_mode=nobarrier` (`grep f2fs /proc/mounts`), so only process-crash consistency can
  be demonstrated here -- a fact `LANG-DB-DESIGN.md §0.4` already recorded.

## 3. Trigger to reopen

A target node with a DAX-mapped device, plus a harness that can cut power to it. Absent both, the
correct action is B1's follow-up card. If the trigger ever fires, the work is: one builtin, two
call sites, and the torn-write gate re-run at cache-line granularity instead of page granularity.
