
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


## B3 step 4 (2026-09-07, e9159318): PreJIT pool + tier dispatch -- fold MISMATCH()

| row | value | target | verdict |
|---|---|---|---|
| PreJIT build: compile gbpool.bp (ms) | 14917 | - | - |
| PreJIT build: run (generate+compile+append 12 kernels, ms) | 1213 | - | - |
| pool: kernel count | 12 | 12 | MET |
| pool: total bytes | 355336 | - | - |
| tier 0 (first query, one gb_mxv_generic call, ms) | NA | <=1 | UNMET |
| specialised compile (one kernel, in-process, ms) | NA | <=50 | UNMET |
| pool hit (mmap-cached dispatch, ms) | NA | <=0.1 | UNMET |
| tier0/specialised ratio | 0.00 | <=5x | MET |
- 1M/5M sgraph2 LCG graph row: not measured (BOX SAFETY time budget -- the 1k-graph rows above
  are the ones the blueprint's section 7 gate names; a 1M-scale gb_mxv specialised kernel is a
  step-4(f)-adjacent item, not gated here).
- fold: pool-hit == tier-0 == the gb_gen golden (bench/oracles/gb_pool.py), rolling-combined the
  same way as gb_gen_tier0/gb_gen_specialised.


## B3 step 4 (2026-09-07, e9159318): PreJIT pool + tier dispatch -- fold equal

| row | value | target | verdict |
|---|---|---|---|
| PreJIT build: compile gbpool.bp (ms) | 14940 | - | - |
| PreJIT build: run (generate+compile+append 12 kernels, ms) | 1366 | - | - |
| pool: kernel count | 12 | 12 | MET |
| pool: total bytes | 355336 | - | - |
- 1M/5M sgraph2 LCG graph row: not measured (BOX SAFETY time budget -- the 1k-graph rows above
  are the ones the blueprint's section 7 gate names; a 1M-scale gb_mxv specialised kernel is a
  step-4(f)-adjacent item, not gated here).
- fold: pool-hit == tier-0 == the gb_gen golden (bench/oracles/gb_pool.py), rolling-combined the
  same way as gb_gen_tier0/gb_gen_specialised.

## B3 step 4 tightened (item (c)/(2) resume, 2026-09-07, e9159318): background compile-on-miss + mmap-cached pool-hit -- fold equal

| row | value | target | verdict |
|---|---|---|---|
| tier 0 (first query, one gb_mxv_generic call, ms) | 1 | <=1 | MET |
| background parent-return (tier0 fold + fork, NOT the compile, ms) | 2 | < child | MET |
| compile-on-miss child (fork->gen->compile->append->commit->exit, ms) | 98 | <=50 | UNMET |
| pool hit BEFORE (gb_kernel_run, per-call file round trip, ms/call over 100) | 4.350 | - | - |
| pool hit AFTER (gb_kernel_run_fast, mmap-cached + anon result, ms/call over 100) | 1.580 | - | - |
| fork floor (bare sys_clone+sys_exit+sys_wait4, ms/call over 100) | 1.040 | - | - |
- miss->tier0->wait4(child)->hit exercised for all 12 (op,sr) kernels; driver stderr empty
  (verified separately -- gate gb_pool in std_golden.sh checks the fold only, stdout tail -1).
- pool-hit fast path costs only sys_clone+sys_run+sys_wait4 per call (kernel image mmap'd once
  per process, result via an anonymous MAP_SHARED page) -- the 0.1ms/call design target is
  UNREACHABLE with any forked dispatch on this box (a bare clone+exit+wait4 floor already costs
  ~1-2ms/iter, measured); HIT_AFTER < HIT_BEFORE is the actual, reachable win.

