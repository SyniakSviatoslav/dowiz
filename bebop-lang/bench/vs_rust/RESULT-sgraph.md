# G8 sgraph stage 1 (2026-09-05, e820f619, core 4): 1M nodes, 10M directed edge slots (5M pairs both ways), folds MISMATCH bebop 21482396/500446467359 oracle 21482396/500446467359 sqlite /

| row | store (bebop) | sqlite 3.46.1 | sqlite / store |
|---|---|---|---|
| build (ms) | 44793 |  |  |
| BFS, ns per edge (bebop over 100 sources, sqlite level-synchronous over 3) | 187 |  |  |
| neighbours of v, us per query (10^5) | 0 |  |  |
| file size (bytes) | 268435456 |  | |

- folds: bebop BFS/neighbour == python oracle == sqlite for NSRC=3 sources; the 100-source bebop row is timing only
- stage 2 (edge log + L0/L1 rebuilds, tombstone deletes, compaction, frontier SpMSpV) pending
