
## B3 step 4 (2026-09-07, e9159318): PreJIT pool + tier dispatch -- fold equal

| row | value | target | verdict |
|---|---|---|---|
| PreJIT build: compile gbpool.bp (ms) | 256 | - | - |
| PreJIT build: run (generate+compile+append 12 kernels, ms) | 1363 | - | - |
| pool: kernel count | 12 | 12 | MET |
| pool: total bytes | 355336 | - | - |
| tier 0 (first query, one gb_mxv_generic call, ms) | 1 | <=1 | MET |
| specialised compile (one kernel, in-process, ms) | 95 | <=50 | UNMET |
| pool hit (mmap-cached dispatch, ms) | 8 | <=0.1 | UNMET |
| tier0/specialised ratio | 0.01 | <=5x | MET |
- 1M/5M sgraph2 LCG graph row: not measured (BOX SAFETY time budget -- the 1k-graph rows above
  are the ones the blueprint's section 7 gate names; a 1M-scale gb_mxv specialised kernel is a
  step-4(f)-adjacent item, not gated here).
- fold: pool-hit == tier-0 == the gb_gen golden (bench/oracles/gb_pool.py), rolling-combined the
  same way as gb_gen_tier0/gb_gen_specialised.

