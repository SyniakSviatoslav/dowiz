# T100 — tensor query vs sqlite, 1M points, pinned core 4 (A78), R=5 medians, bebop.bin 15e272fc
- data: LCG seed 12345, (u,v) in [-2^30,2^30), 1024x1024 cells of 2^21; queries: next 1000 LCG pairs; folds mod 1e9+7 (bench/tq_sqlite/oracle.py)
- fold checks: bebop scan == truth_fold_Q20 (YES); bebop indexed == python window fold == sqlite C-API fold (YES); 3x3 window == true nearest on 998/1000 queries (both windowed engines share the miss)
- sqlite 3.46.1 in-memory, build+index 2678 ms (python executemany); bebop build = zeros + LCG fill + counting sort, inside the same process (not timed separately)

| engine / query | per query | vs bebop same class |
|---|---|---|
| sqlite scan, `ORDER BY d LIMIT 1` (python wrapper, Q=20) | 197.6 ms | 41.6x slower |
| bebop scan nn.bp (Q=20) | 4.8 ms | 1.0x |
| sqlite indexed 3x3 window, python wrapper (Q=1000) | 46.0 us | 23.0x slower |
| sqlite indexed 3x3 window, C API prepared statement (Q=1000) | 57.2 us | 28.6x slower |
| bebop indexed nnidx.bp: cell -> CSR bucket -> 3x3 window (Q=1000) | 2.0 us | 1.0x |

- pass rule (docs/SPEEDUP-ANALYSIS.md 4.3): indexed <= 10 us AND >= 3x sqlite C-API: PASS; scan >= 10x sqlite scan: PASS
