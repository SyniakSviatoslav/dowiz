# DEV-LOOP — inner development-loop timings

Produced by `bench/dev_loop.sh <bin> <label>`; one section per run, appended.
Caveats: box shared with another agent's compile jobs (cpus 5-6 busy); pinned
steps use `taskset -c <cpu>` on one A78 core; std_golden runs are unpinned
(the sharded one pins its own shards to the A78 cores). Numbers are single
runs, not medians. Logs stay under `$BEBOP_TMP/devloop`.

## now — bin aa41e8a1, 2026-09-06, pinned cpu4

| metric | now | unit | note |
|---|---|---|---|
| selfcompile | 15.13 | s | rc=0, cpu4 |
| std_compile_sgraph2 | 0.70 | s | rc=0, cpu4 |
| std_golden_seq | 24.99 | s | std_golden: 99 pass, 0 fail, unpinned |
| std_golden_par3 | 8.83 | s | std_golden: 99 pass, 0 fail (J=3 shards), unpinned |
| fuzz_rate | 0.56 | seeds/s | wall=54.26s, N=30 START=41000 OK=29 DIVERGE=0 COMPILEFAIL=0 CRASH=0 TIMEOUT=0 BPREF-ERROR=0 BPREF-DEPTH=0 BPREF-TIMEOUT=0 GENFAIL=0 STRAY=0 TRAP-OK=0 TRAP-UNPREDICTED=1, cpu4 |
| construct_parity | 9.66 | s | construct parity: pass=51 fail=0, cpu4 |

## before (519903c) — bin 0af854a9, 2026-09-06, pinned cpu4

| metric | before (519903c) | unit | note |
|---|---|---|---|
| std_golden_seq | 19.19 | s | std_golden: 99 pass, 0 fail, unpinned |
| std_golden_par3 | 27.38 | s | std_golden: 99 pass, 0 fail (J=3 shards), unpinned |
| fuzz_rate | 0.56 | seeds/s | wall=53.56s, N=30 START=41000 OK=29 DIVERGE=0 COMPILEFAIL=0 CRASH=0 TIMEOUT=0 BPREF-ERROR=0 BPREF-DEPTH=0 BPREF-TIMEOUT=0 GENFAIL=0 STRAY=0 TRAP-OK=0 TRAP-UNPREDICTED=1, cpu4 |
| construct_parity | 10.39 | s | construct parity: pass=43 fail=8, cpu4 |

## now-fuzzbatch — bin c3f58e8e, 2026-09-05, pinned cpu4

| metric | now-fuzzbatch | unit | note |
|---|---|---|---|
| selfcompile | 14.34 | s | rc=0, cpu4 |
| std_compile_sgraph2 | 0.52 | s | rc=0, cpu4 |
| fuzz_rate | 1.13 | seeds/s | wall=26.87s, N=30 START=41000 OK=29 DIVERGE=0 COMPILEFAIL=0 CRASH=0 TIMEOUT=0 BPREF-ERROR=0 BPREF-DEPTH=0 BPREF-TIMEOUT=0 GENFAIL=0 STRAY=0 TRAP-OK=0 TRAP-UNPREDICTED=1, cpu4 |

## Analysis — where the inner loop's time went and what changed (2026-09-06)

Numbers from docs/exp.journal + the tables above (single runs, this box, pinned cpu4 where marked).

| step of the loop | before (session 9/10 start) | now | factor | what changed |
|---|---|---|---|---|
| self-compile (one generation) | 292 s (T126 era), 187 s (2026-09-06 morning) | 14-17 s | 12-20x | 43 per-token `str_len(s)` NUL scans over the whole source -> length carried in pos[1] (slen); third compile pass (emit_offsets) deleted |
| one std gate cold compile (sgraph2) | 19.9 s | 0.5-0.7 s | 30x | same fix; warm .becache hit 113 ms (T108) |
| 4000-statement fn compile | 25 s (81 s with a call per statement) | 0.22-0.25 s | 100-300x | per-statement cost was O(source length), now O(1) |
| three-generation chain + full battery | ~8 hand-orchestrated runs, 439 s when scripted | one command, 94 s (non-codegen) / 123 s (codegen) | 4.7x, and no orchestration | tools/chain.sh: gen3->gen4 in parallel with the battery; tools/battery.sh: every gate script on its own core |
| std_golden (99 gates) | 25 s sequential (memoized) / 36 s cold | 8.8 s (J=3 shards) | 2.8x | tools/std_par.sh, failed gate retried standalone (timing flags) |
| fuzz throughput | 0.05/s (49 % oracle timeouts) | 0.56/s (generator fixed) -> 1.13/s per core, 3.5/s on 3 cores | 70x end to end | gen.py loop-name bug (non-terminating programs in BOTH engines); then 3 python interpreter starts per seed (~70 % of a seed under proot) -> one process per shard, bpref forked |
| 10^5-program fuzz target (TG-DONE 8) | ~23 days | ~8 h | | batches of 5000 = 25 min each |
| locating a miscompile | shrinker 90 min, no result (5 KB) | 4-6 cut-and-return probes, ~10 min | | the probe ladder (top-down cuts on the original, then a 12-line hypothesis probe) |
| compile error | bare exit code | `line:col: message` (T90 step 1) | | eight diag gates in the battery |
| crash triage | SIGSEGV / SIGBUS with no exit code | exit 82 (T118b stub), 87 (undefined call), 80/81 capacity, 88 cas digest | | traps replace gdb sessions under proot |

What still costs: the per-call n^2 term (4000 calls 1.8 s), the 36 s cold std_golden when
bebop.bp changes (every gate recompiles: 99 x 0.36 s on 3 cores), the 40 s bpref budget on
heavy nested-loop seeds (1 in 300), and any battery run alongside another battery (timing
gates flake: lcjit). Rule that paid off most: differential timing over gdb sampling under
proot, and one line in the emitter over any tooling when the cause is O(n) x O(n).
