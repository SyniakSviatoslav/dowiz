# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)

Status: 2026-09-06 CURRENT (last 12 runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)

| metric | unit | dcaccb6/0a8bfe9f | dcaccb6/1a3b2cc2 | 8c4a336/a903d33b | 3d512dc/a903d33b | 3d512dc/df6044a8 | 69e0eb5/a903d33b | 69e0eb5/df6044a8 | 69e0eb5/f7a25d38 | 2a9f7e9/3bed1c48 | 2a9f7e9/4c8fc5a6 | 2a9f7e9/c74110f9 | 2a9f7e9/53d13800 | last delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| battery_flakes | count | 0 | 0 | 0 |  | 0 |  |  | 0 | 0 | 0 | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| gate_run_ms | ms | 24746 | 0 | 0 |  | 23487 |  |  | 0 | 21438 | 2916 | 0 | 0 | 0 -> 0 (+0.0 %, MAD 1.56e+03) |
| chain_wall | s |  |  | 18 |  |  |  |  |  |  |  |  |  | 38 -> 18 (-52.6 %, MAD 6) |
| chain_cpu | s |  |  | 20.41 |  |  |  |  |  |  |  |  |  | 50.14 -> 20.41 (-59.3 %, MAD 2.39) |
| bin_words | words |  | 74222 | 67775 | 67775 |  | 67775 | 68233 | 68229 |  |  |  | 36218 | 68229 -> 36218 |
| stub_words | words |  | 131 | 131 | 131 |  | 131 | 131 | 131 |  |  |  | 131 | 131 -> 131 |
| bin_fns | fns |  | 202 | 202 | 202 |  | 202 | 204 | 204 |  |  |  | 255 | ! 204 -> 255 (+25.0 %, MAD 1) |
| cw:c01_lit | words |  | 30 |  |  |  |  |  |  |  |  |  |  | 42 -> 30 (-28.6 %, MAD 0) |
| cw:c02_arith | words |  | 23 |  |  |  |  |  |  |  |  |  |  | 35 -> 23 (-34.3 %, MAD 0) |
| cw:c03_precedence | words |  | 12 |  |  |  |  |  |  |  |  |  |  | 24 -> 12 (-50.0 %, MAD 0) |
| cw:c04_cmp | words |  | 86 | 56 |  |  |  |  |  |  |  |  |  | 86 -> 56 (-34.9 %, MAD 0) |
| cw:c05_if | words |  | 25 | 17 |  |  |  |  |  |  |  |  |  | 25 -> 17 (-32.0 %, MAD 0) |
| cw:c06_let | words |  | 20 |  |  |  |  |  |  |  |  |  |  | 30 -> 20 (-33.3 %, MAD 0) |
| cw:c07_while | words |  | 24 |  |  |  |  |  |  |  |  |  |  | 34 -> 24 (-29.4 %, MAD 0) |
| cw:c08_call | words |  | 37 |  |  |  |  |  |  |  |  |  |  | 59 -> 37 (-37.3 %, MAD 0) |
| cw:c09_recursion | words |  | 37 | 31 |  |  |  |  |  |  |  |  |  | 37 -> 31 (-16.2 %, MAD 0) |
| cw:c10_struct | words |  | 54 |  |  |  |  |  |  |  |  |  |  | 92 -> 54 (-41.3 %, MAD 0) |
| cw:c11_enum | words |  | 7 |  |  |  |  |  |  |  |  |  |  | 19 -> 7 (-63.2 %, MAD 0) |
| cw:c12_match | words |  | 16 |  |  |  |  |  |  |  |  |  |  | 26 -> 16 (-38.5 %, MAD 0) |
| cw:c13_array | words |  | 72 |  |  |  |  |  |  |  |  |  |  | 78 -> 72 (-7.7 %, MAD 0) |
| cw:c14_string | words |  | 17 |  |  |  |  |  |  |  |  |  |  | 27 -> 17 (-37.0 %, MAD 0) |
| cw:c15_bitwise | words |  | 30 |  |  |  |  |  |  |  |  |  |  | 42 -> 30 (-28.6 %, MAD 0) |
| cw:c16_compound | words |  | 30 |  |  |  |  |  |  |  |  |  |  | 40 -> 30 (-25.0 %, MAD 0) |
| cw:c17_neg | words |  | 33 |  |  |  |  |  |  |  |  |  |  | 45 -> 33 (-26.7 %, MAD 0) |
| cw:c18_bigconst | words |  | 10 |  |  |  |  |  |  |  |  |  |  | 22 -> 10 (-54.5 %, MAD 0) |
| cw:c19_multi | words |  | 73 | 67 |  |  |  |  |  |  |  |  |  | 73 -> 67 (-8.2 %, MAD 0) |
| cw:c20_deep | words |  | 100 | 95 |  |  |  |  |  |  |  |  |  | 100 -> 95 (-5.0 %, MAD 0) |
| cw:c21_param13 | words |  | 122 |  |  |  |  |  |  |  |  |  |  | 139 -> 122 (-12.2 %, MAD 0) |
| cw:c22_matchbind | words |  | 15 |  |  |  |  |  |  |  |  |  |  | 25 -> 15 (-40.0 %, MAD 0) |
| cw:c23_spillcall | words |  | 225 |  |  |  |  |  |  |  |  |  |  | 246 -> 225 (-8.5 %, MAD 0) |
| cw:c24_ifspill | words |  | 146 | 140 |  |  |  |  |  |  |  |  |  | 146 -> 140 (-4.1 %, MAD 0) |
| cw:c25_matchtail | words |  | 62 |  |  |  |  |  |  |  |  |  |  | 70 -> 62 (-11.4 %, MAD 0) |
| cw:c26_selfrec | words |  | 183 | 163 |  |  |  |  |  |  |  |  |  | 183 -> 163 (-10.9 %, MAD 0) |
| cw:c27_zeroarg | words |  | 32 |  |  |  |  |  |  |  |  |  |  | 42 -> 32 (-23.8 %, MAD 0) |
| cw:c30_unary | words |  | 61 |  |  |  |  |  |  |  |  |  |  | 69 -> 61 (-11.6 %, MAD 0) |
| cw:c31_nested_lit | words |  | 149 |  |  |  |  |  |  |  |  |  |  | 167 -> 149 (-10.8 %, MAD 0) |
| cw:c32_asr | words |  | 69 |  |  |  |  |  |  |  |  |  |  | 75 -> 69 (-8.0 %, MAD 0) |
| cw:c33_loopalloc | words |  | 121 |  |  |  |  |  |  |  |  |  |  | 127 -> 121 (-4.7 %, MAD 0) |
| cw:c34_loopescape | words |  | 107 |  |  |  |  |  |  |  |  |  |  | 113 -> 107 (-5.3 %, MAD 0) |
| cw:c35_return | words |  | 59 |  |  |  |  |  |  |  |  |  |  | 95 -> 59 (-37.9 %, MAD 0) |
| cw:c36_break | words |  | 137 |  |  |  |  |  |  |  |  |  |  | 141 -> 137 (-2.8 %, MAD 0) |
| cw:c40_struct | words |  | 192 |  |  |  |  |  |  |  |  |  |  | 206 -> 192 (-6.8 %, MAD 0) |
| cw:c41_clz | words |  | 67 |  |  |  |  |  |  |  |  |  |  | 73 -> 67 (-8.2 %, MAD 0) |
| cw:c42_crc32 | words |  | 192 |  |  |  |  |  |  |  |  |  |  | 196 -> 192 (-2.0 %, MAD 0) |
| cw:c43_arena_persist | words |  | 272 | 262 |  |  |  |  |  |  |  |  |  | 272 -> 262 (-3.7 %, MAD 0) |
| cw:c44_use24 | words |  | 328 |  |  |  |  |  |  |  |  |  |  | 586 -> 328 (-44.0 %, MAD 0) |
| cw:c45_crc32x | words |  | 208 |  |  |  |  |  |  |  |  |  |  | 214 -> 208 (-2.8 %, MAD 0) |
| cw:c46_andor | words |  | 169 | 145 |  |  |  |  |  |  |  |  |  | 169 -> 145 (-14.2 %, MAD 0) |
| cw:c47_usenest | words |  | 51 |  |  |  |  |  |  |  |  |  |  | 91 -> 51 (-44.0 %, MAD 0) |
| cw:c50_cas | words |  | 31 |  |  |  |  |  |  |  |  |  |  | 57 -> 31 (-45.6 %, MAD 0) |
| cw:c53_param9 | words |  | 106 |  |  |  |  |  |  |  |  |  |  | 112 -> 106 (-5.4 %, MAD 0) |
| fuzz_seeds_on_bin | seeds |  | 0 | 0 | 0 |  |  |  | 0 |  |  |  |  | 0 -> 0 (+0.0 %, MAD 1e+03) |
| fuzz_rate | prog/s |  | 0 | 0 | 0 |  |  |  | 0 |  |  |  |  | 0 -> 0 (+0.0 %, MAD 0.17) |
| fuzz_trap_unpredicted | count |  | 0 | 0 | 0 |  |  |  | 0 |  |  |  |  | 0 -> 0 (+0.0 %, MAD 7) |
| selfcompile_wall | ms |  | 1809 ? | 1585 ? | 1582 ? |  |  |  | 1508 ? |  |  |  |  | invalid window |
| selfcompile_utime | s |  | 1.736 ? | 1.504 ? | 1.506 ? |  |  |  | 1.44 ? |  |  |  |  | invalid window |
| selfcompile_stime | s |  | 0.0237 ? | 0.0397 ? | 0.0356 ? |  |  |  | 0.0237 ? |  |  |  |  | invalid window |
| selfcompile_maxrss | kB |  | 22992 ? | 22520 ? | 22508 ? |  |  |  | 22628 ? |  |  |  |  | invalid window |
| selfcompile_energy | core-s@fmax (proxy) |  | 1.729 ? | 1.499 ? | 1.506 ? |  |  |  | 0 ? |  |  |  |  | invalid window |
| k1h_ms | ms/rep |  | 2.13 ? | 2.13 ? | 2.13 ? |  |  |  | 1.75 ? |  |  |  | 1.09 ? | invalid window |
| k1h_loopwords | words |  | 11 | 11 | 11 |  |  |  | 10 |  |  |  | 5 | 10 -> 5 |
| k2h_ms | ms/rep |  | 1.01 ? | 0.77 ? | 0.89 ? |  |  |  | 0.71 ? |  |  |  | 0.76 ? | invalid window |
| k2h_loopwords | words |  | 51 | 51 | 51 |  |  |  | 51 |  |  |  | 51 | 51 -> 51 |
| k3h_ms | ms/rep |  | 0.63 ? | 0.71 ? | 0.74 ? |  |  |  | 0.54 ? |  |  |  | 0.29 ? | invalid window |
| k3h_loopwords | words |  | 25 | 25 | 25 |  |  |  | 24 |  |  |  | 8 | 24 -> 8 |
| k4_ms | ms/rep |  | 4.66 ? | 4.62 ? | 4.63 ? |  |  |  | 4.04 ? |  |  |  | 4.03 ? | invalid window |
| k4_loopwords | words |  | 15 | 15 | 15 |  |  |  | 14 |  |  |  | 7 | 14 -> 7 |
| proc_floor_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_warm_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_cold_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| fuzz_trap82 | count |  | 0 | 0 | 0 |  |  |  | 0 |  |  |  |  | 0 TRAP-82 (SIGSEGV/SIGBUS) on f7a25d38, 0 tolerated |
| k8h_ms | ms/rep |  |  |  | 0.39 ? |  |  |  | 0.33 ? |  |  |  | 0.3 ? | invalid window |
| k8h_loopwords | words |  |  |  | 40 |  |  |  | 39 |  |  |  | 25 | 39 -> 25 |
| push_words | words |  |  |  |  |  |  |  |  |  |  |  | 0 | 0 -> 0 |

Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).
Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).

## ALERTS

- bin_fns: 204 -> 255 (+25.0 %, MAD 1)
