# G8 sgraph stage 1 (2026-09-05, e820f619, core 4): 1M nodes, 10M directed edge slots (5M pairs both ways), folds MISMATCH bebop 21482396/500446467359 oracle 21482396/500446467359 sqlite /

| row | store (bebop) | sqlite 3.46.1 | sqlite / store |
|---|---|---|---|
| build (ms) | 44793 |  |  |
| BFS, ns per edge (bebop over 100 sources, sqlite level-synchronous over 3) | 187 |  |  |
| neighbours of v, us per query (10^5) | 0 |  |  |
| file size (bytes) | 268435456 |  | |

- folds: bebop BFS/neighbour == python oracle == sqlite for NSRC=3 sources; the 100-source bebop row is timing only
- stage 2 (edge log + L0/L1 rebuilds, tombstone deletes, compaction, frontier SpMSpV) pending

## stage 2 (2026-09-05, b43fe630, core 4): edge log, tombstones, compaction — folds MISMATCH nbr0 500446467359/500446467359 nbrlog 516007685908/550529568420 nbr 468402521302,468402521302/502924403814 bfs 22027072,22027072/21550931

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 39982 |
| insert 1M edges through the log, 100 batches with an L0 rebuild each: amortized ns per edge / max batch stall ms |  / 0 |
| neighbours of v after the log, us per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 154 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 103 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 288 |
| logical size before / after compaction (bytes) | 383180048 / 104699984 |
| compaction (ms) | 788 |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- not yet: the frontier SpMSpV variant with the push/pull switch, the 1%-hub skew variant

## stage 2 (2026-09-05, b43fe630, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 90081 |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 30033 / 747 |
| neighbours of v after the log, us per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 172 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 131 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 240 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 795 |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- not yet: the frontier SpMSpV variant with the push/pull switch, the 1%-hub skew variant

## stage 2 (2026-09-07, 9694b780, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 17542 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 116 vs 23 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 13445 / 231 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 820 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 33 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 135 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 452 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 13069 / 106 vs 11 / 570 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-07, e9159318, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 16766 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 116 vs 23 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 13408 / 230 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 800 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 33 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 136 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 414 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 10842 / 104 vs 11 / 530 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-07, e9159318, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 15927 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 112 vs 26 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 13340 / 233 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 830 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 33 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 137 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 415 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 15249 / 106 vs 12 / 510 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-07, e9159318, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 16478 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 119 vs 26 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 13934 / 289 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 840 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 36 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 154 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 521 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 6472 / 107 vs 13 / 520 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-08, 0ae01bd3, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 1266 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 111 vs 26 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 8019 / 138 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 800 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 32 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 131 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 424 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 1243 / 105 vs 12 / 490 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-08, ca69273e, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 1256 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 115 vs 25 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 8243 / 143 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 820 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 36 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 135 |
| logical size before / after compaction (bytes) | 122511752 / 121261640 |
| compaction (ms) | 458 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 1233 / 104 vs 12 / 510 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-08, ca69273e, core 4): edge log, tombstones, compaction — folds equal

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) | 1289 |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot | 117 vs 25 (folds equal) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 985 / 74 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 830 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) | 31 |
| BFS with tombstones + log, 3 sources, ns per edge slot | 137 |
| logical size before / after compaction (bytes) | 108109392 / 106859272 |
| compaction (ms) | 422 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query | 1329 / 106 vs 11 / 530 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)

## stage 2 (2026-09-11, 24b98f90, core 4): edge log, tombstones, compaction — folds MISMATCH nbr0 820644230268/500446467359 nbrlog 0/550529568420 nbr ,0/502924403814 bfs ,3/21550931

| row | value |
|---|---|
| build L1 + empty log/L0/bitmap (ms) |  |
| BFS on L1 before the log, 3 sources: queue vs frontier SpMSpV (push/pull, alpha 14), ns per edge slot |  vs 0 (folds DIFFER) |
| insert 1M edges through the log, 100 batches with an L0 rebuild each (compaction every 20): amortized ns per edge / max batch stall ms | 0 / 0 |
| neighbours of v after the log, ns per query (L1 slice + L0 slice + tombstone bits, 3 slices) | 400 |
| tombstone 10% of the L1 slots (one bitmap version) + commit (ms) |  |
| BFS with tombstones + log, 3 sources, ns per edge slot |  |
| logical size before / after compaction (bytes) | 8192 / 8192 |
| compaction (ms) | 87 |
| 1%-hub skewed graph (sgraph2h.store): build ms / BFS queue vs frontier ns per slot / neighbours ns per query |  /  vs 0 / 600 (folds DIFFER) |
- folds: neighbours before/after the log and before/after compaction, BFS before/after compaction == python oracle
- the hub variant's oracle is the same array BFS with the skewed generator (the log/tombstone phases run on the uniform graph only)
