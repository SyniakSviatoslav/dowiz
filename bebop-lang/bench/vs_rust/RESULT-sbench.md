# G7 sbench (2026-09-05, e820f619, core 4): store vs sqlite 3.46.1 C API, folds equal

| phase | store | sqlite | sqlite / store |
|---|---|---|---|
| insert 1M + index + commit (ms) | 980000 | 15005 | 0.0x |
| PK lookup, us each (10^5) | 0 | 1043.99 | 1043990.0x |
| 3x3 cell-window scan, us each (10^4) | 3 | 1275.3 | 425.1x |
| update 10^5 in one transaction (ms) | 322000 | 1851 | 0.0x |
| reopen + first record, us (100x) | 1170 | 3757.7 | 3.2x |
| file size after update (bytes) | 268435456 | 34070528 | 0.1x |
| compaction / VACUUM (ms) | 1001000 | 600 | 0.0x |
| file size after compaction (bytes) | 268435456 | 34070528 | 0.1x |

- pass rule (docs/LANG-DB-DESIGN.md §5 G7): PK lookup >= 3x sqlite native AND scan >= 5x; the file size is reported whatever it is (expected ~2.2x loss before compaction)
- the store's bebop numbers include the ~100 ms process floor only in 'insert'/'update'/'compact' (one process each); per-op rows are in-process clock_ms deltas divided by the op count
