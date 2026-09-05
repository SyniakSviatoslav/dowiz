# G7 sbench (2026-09-05, b43fe630, core 4): store vs sqlite 3.46.1 C API, folds equal

| phase | store | sqlite | sqlite / store |
|---|---|---|---|
| insert 1M + index + commit (ms) | 725 | 11933 | 16.5x |
| PK lookup, ns each (10^5) | 630 | 7783.6 | 12.4x |
| 3x3 cell-window scan, ns each (10^4) | 1800 | 63574.7 | 35.3x |
| update 10^5 in one transaction (ms) | 169 | 806 | 4.8x |
| reopen + first record, us (100x) | 1070 | 3627.7 | 3.4x |
| logical size after update, arena_used*8 (bytes) | 85197016 | 34070528 | 0.4x |
| compaction / VACUUM (ms) | 690 | 417 | 0.6x |
| logical size after compaction (bytes; file blocks allocated 72474624) | 72396960 | 34070528 | 0.5x |
| durable commit, us each (1000 x one record version; store = msync of the appended pages + the superblock pages; sqlite = WAL synchronous=NORMAL, no fsync per commit) | 506 | 78.0 | 0.2x |
| durable commit vs sqlite WAL synchronous=FULL (fsync per commit) | 506 | 567.1 | 1.1x |

- pass rule (docs/LANG-DB-DESIGN.md §5 G7): PK lookup >= 3x sqlite native AND scan >= 5x; the file size is reported whatever it is (expected ~2.2x loss before compaction)
- store rows are in-process clock_ms deltas (insert/update/compact exclude the ~100 ms process floor and the open); sqlite rows go through ctypes (4 calls per lookup, ~8 us of ctypes per op, T100 measured ~19 us for the window query) inside one transaction with locking_mode=EXCLUSIVE, so its per-op rows are an upper bound on native sqlite by roughly that floor
- the store file is preallocated (256 MB ftruncate); the logical size is what a size-aware open would map
