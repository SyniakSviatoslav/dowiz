# T100 — tensor query vs sqlite, 1M points, pinned core 4 (A78), R=5 medians, bebop.bin 104b6291
- data: LCG seed 12345, (u,v) in [-2^30,2^30), 1024x1024 cells of 2^21; queries: next 1000 LCG pairs; folds mod 1e9+7 (bench/tq_sqlite/oracle.py)
- fold checks: bebop scan == truth_fold_Q20 (YES); bebop indexed == python window fold == sqlite C-API fold (YES); 3x3 window == true nearest on 998/1000 queries (both windowed engines share the miss)
- sqlite 3.46.1 in-memory, build+index 2457 ms (python executemany); bebop build = zeros + LCG fill + counting sort, inside the same process (not timed separately)

| engine / query | per query | vs bebop same class |
|---|---|---|
| sqlite scan, `ORDER BY d LIMIT 1` (python wrapper, Q=20) | 183.2 ms | 9.9x slower |
| bebop scan nn.bp (Q=20) | 18.4 ms | 1.0x |
| sqlite indexed 3x3 window, python wrapper (Q=1000) | 44.1 us | 11.0x slower |
| sqlite indexed 3x3 window, C API prepared statement (Q=1000) | 55.2 us | 13.8x slower |
| bebop indexed nnidx.bp: cell -> CSR bucket -> 3x3 window (Q=1000) | 4.0 us | 1.0x |

- pass rule (docs/SPEEDUP-ANALYSIS.md 4.3): indexed <= 10 us AND >= 3x sqlite C-API: PASS; scan >= 10x sqlite scan: FAIL (9.9x)

| nn4.bp bucketed scan, 1 A78 vs 3 A78 (sys_setaffinity, R=5) | seq 219 ms / par 99 ms | 2.21x | folds equal: 1 |
