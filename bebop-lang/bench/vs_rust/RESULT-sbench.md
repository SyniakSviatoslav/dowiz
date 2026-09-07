# G7 sbench (2026-09-07, e9159318, core 4): store vs sqlite 3.46.1 C API, folds equal

| phase | store | sqlite | sqlite / store |
|---|---|---|---|
| insert 1M + index + commit (ms) | 400 | 9754 | 24.4x |
| PK lookup, ns each (10^5) | 190 | 6531.0 | 34.4x |
| 3x3 cell-window scan, ns each (10^4) | 1200 | 51397.8 | 42.8x |
| update 10^5 in one transaction (ms) | 73 | 615 | 8.4x |
| reopen + first record, us (100x) | 24340 | 2266.3 | 0.1x |
| logical size after update, arena_used*8 (bytes) | 85197016 | 34070528 | 0.4x |
| compaction / VACUUM (ms) | 397 | 329 | 0.8x |
| logical size after compaction (bytes; file blocks allocated 72474624) | 72396960 | 34070528 | 0.5x |
| durable commit, us each (1000 x one record version; store = msync of the appended pages + the superblock pages; sqlite = WAL synchronous=NORMAL, no fsync per commit) | 229 | 62.6 | 0.3x |
| durable commit vs sqlite WAL synchronous=FULL (fsync per commit) | 229 | 277.5 | 1.2x |
| durable batch 10, us per commit (B1: st_commit_batch, one msync per 10 commits; sqlite = 10 UPDATEs per COMMIT, synchronous=FULL) | 30 | 44.2 | 1.5x |
| durable batch 100, us per commit (same, 100 commits per msync/COMMIT) | 7 | 10.9 | 1.6x |
| recover, us (median of 11; reopen after a torn image -- store = st_open/st_reopen_verify self-heal past a zeroed live-generation tail; sqlite = open with a non-empty -wal after no checkpoint) | 44713.2 | 3298.8 | 0.1x |

- pass rule (docs/LANG-DB-DESIGN.md §5 G7): PK lookup >= 3x sqlite native AND scan >= 5x; the file size is reported whatever it is (expected ~2.2x loss before compaction)
- store rows are in-process clock_ms deltas (insert/update/compact exclude the ~100 ms process floor and the open); sqlite rows go through ctypes (4 calls per lookup, ~8 us of ctypes per op, T100 measured ~19 us for the window query) inside one transaction with locking_mode=EXCLUSIVE, so its per-op rows are an upper bound on native sqlite by roughly that floor
- the store file is preallocated (256 MB ftruncate); the logical size is what a size-aware open would map
