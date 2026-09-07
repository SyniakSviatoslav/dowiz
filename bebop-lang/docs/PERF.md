# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)

Status: 2026-09-07 CURRENT (last 12 runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)

| metric | unit | 2a9f7e9/53d13800 | 2a9f7e9/831a357c | a77adc6/831a357c | 44d0e06/aebd2f49 | eb73c29/f20535be | 3b58f34/d3097e90 | 3b58f34/e1df4dcc | 3b58f34/4c53832d | 71eaeed/33ce624c | 71eaeed/a81f8798 | 71eaeed/4c53832d | 71eaeed/9694b780 | last delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| battery_flakes | count | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| gate_run_ms | ms | 0 | 0 | 0 | 0 | 0 | 19104 | 0 | 0 | 2863 | 0 |  | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| chain_wall | s |  |  |  | 93 | 103 |  | 202 | 130 |  | 108 |  | 108 | 108 -> 108 (+0.0 %, MAD 46) |
| chain_cpu | s |  |  |  | 54.49 | 59.34 |  | 156.6 | 86.94 |  | 63.39 |  | 62.8 | 63.39 -> 62.8 (-0.9 %, MAD 15) |
| bin_words | words | 36218 | 36059 | 36059 | 36195 | 36195 | 36910 | 37687 | 37693 | 38806 | 38842 | 37693 | 38975 | 38842 -> 38975 |
| stub_words | words | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 | 131 -> 131 |
| bin_fns | fns | 255 | 255 | 255 | 255 | 255 | 257 | 262 | 262 | 268 | 268 | 262 | 268 | 268 -> 268 (+0.0 %, MAD 0) |
| cw:c01_lit | words |  |  |  | 9 |  |  |  |  |  |  |  |  | 30 -> 9 (-70.0 %, MAD 0) |
| cw:c02_arith | words |  |  |  | 7 |  |  |  |  |  |  |  |  | 23 -> 7 (-69.6 %, MAD 0) |
| cw:c03_precedence | words |  |  |  | 7 |  |  |  |  |  |  |  |  | 12 -> 7 (-41.7 %, MAD 0) |
| cw:c04_cmp | words |  |  |  | 51 |  |  | 41 |  |  |  |  |  | 51 -> 41 (-19.6 %, MAD 17.5) |
| cw:c05_if | words |  |  |  | 18 |  |  | 17 |  |  |  |  |  | 18 -> 17 (-5.6 %, MAD 4) |
| cw:c06_let | words |  |  |  | 13 |  |  |  |  |  |  |  |  | 20 -> 13 (-35.0 %, MAD 0) |
| cw:c07_while | words |  |  |  | 17 |  |  |  |  |  |  |  |  | 24 -> 17 (-29.2 %, MAD 0) |
| cw:c08_call | words |  |  |  | 25 |  |  |  |  |  |  |  |  | 37 -> 25 (-32.4 %, MAD 0) |
| cw:c09_recursion | words |  |  |  | 26 |  |  |  |  |  |  |  |  | 31 -> 26 (-16.1 %, MAD 6) |
| cw:c10_struct | words |  |  |  | 40 |  |  |  |  |  |  |  |  | 54 -> 40 (-25.9 %, MAD 0) |
| cw:c11_enum |  |  |  |  |  |  |  |  |  |  |  |  |  | 19 -> 7 (-63.2 %, MAD 0) |
| cw:c12_match | words |  |  |  | 10 |  |  |  |  |  |  |  |  | 16 -> 10 (-37.5 %, MAD 0) |
| cw:c13_array | words |  |  |  | 29 |  |  |  |  |  |  |  |  | 72 -> 29 (-59.7 %, MAD 0) |
| cw:c14_string | words |  |  |  | 13 |  |  |  |  |  |  |  |  | 17 -> 13 (-23.5 %, MAD 0) |
| cw:c15_bitwise | words |  |  |  | 7 |  |  |  |  |  |  |  |  | 30 -> 7 (-76.7 %, MAD 0) |
| cw:c16_compound | words |  |  |  | 18 |  |  |  |  |  |  |  |  | 30 -> 18 (-40.0 %, MAD 0) |
| cw:c17_neg | words |  |  |  | 10 |  |  |  |  |  |  |  |  | 33 -> 10 (-69.7 %, MAD 0) |
| cw:c18_bigconst |  |  |  |  |  |  |  |  |  |  |  |  |  | 22 -> 10 (-54.5 %, MAD 0) |
| cw:c19_multi | words |  |  |  | 53 |  |  |  |  |  |  |  |  | 67 -> 53 (-20.9 %, MAD 6) |
| cw:c20_deep | words |  |  |  | 46 |  |  | 45 |  |  |  |  |  | 46 -> 45 (-2.2 %, MAD 13.5) |
| cw:c21_param13 | words |  |  |  | 65 |  |  |  |  |  |  |  |  | 122 -> 65 (-46.7 %, MAD 0) |
| cw:c22_matchbind | words |  |  |  | 10 |  |  |  |  |  |  |  |  | 15 -> 10 (-33.3 %, MAD 0) |
| cw:c23_spillcall | words |  |  |  | 115 |  |  |  |  |  |  |  |  | 225 -> 115 (-48.9 %, MAD 0) |
| cw:c24_ifspill | words |  |  |  | 81 |  |  |  |  |  |  |  |  | 140 -> 81 (-42.1 %, MAD 6) |
| cw:c25_matchtail | words |  |  |  | 28 |  |  |  |  |  |  |  |  | 62 -> 28 (-54.8 %, MAD 0) |
| cw:c26_selfrec | words |  |  |  | 108 |  |  |  |  |  |  |  |  | 163 -> 108 (-33.7 %, MAD 20) |
| cw:c27_zeroarg | words |  |  |  | 29 |  |  |  |  |  |  |  |  | 32 -> 29 (-9.4 %, MAD 0) |
| cw:c30_unary | words |  |  |  | 31 |  |  |  |  |  |  |  |  | 61 -> 31 (-49.2 %, MAD 0) |
| cw:c31_nested_lit | words |  |  |  | 67 |  |  |  |  |  |  |  |  | 149 -> 67 (-55.0 %, MAD 0) |
| cw:c32_asr | words |  |  |  | 34 |  |  |  |  |  |  |  |  | 69 -> 34 (-50.7 %, MAD 0) |
| cw:c33_loopalloc | words |  |  |  | 56 |  |  |  |  |  |  |  |  | 121 -> 56 (-53.7 %, MAD 0) |
| cw:c34_loopescape | words |  |  |  | 58 |  |  |  |  |  |  |  |  | 107 -> 58 (-45.8 %, MAD 0) |
| cw:c35_return | words |  |  |  | 48 |  |  |  |  |  |  |  |  | 59 -> 48 (-18.6 %, MAD 0) |
| cw:c36_break | words |  |  |  | 70 |  |  |  |  |  |  |  |  | 137 -> 70 (-48.9 %, MAD 0) |
| cw:c40_struct | words |  |  |  | 115 |  |  |  |  |  |  |  |  | 192 -> 115 (-40.1 %, MAD 0) |
| cw:c41_clz | words |  |  |  | 42 |  |  |  |  |  |  |  |  | 67 -> 42 (-37.3 %, MAD 0) |
| cw:c42_crc32 | words |  |  |  | 115 |  |  |  |  |  |  |  |  | 192 -> 115 (-40.1 %, MAD 0) |
| cw:c43_arena_persist | words |  |  |  | 130 |  |  | 131 |  |  |  |  |  | 130 -> 131 (+0.8 %, MAD 11.5) |
| cw:c44_use24 | words |  |  |  | 297 |  |  |  |  |  |  |  |  | 328 -> 297 (-9.5 %, MAD 0) |
| cw:c45_crc32x | words |  |  |  | 105 |  |  |  |  |  |  |  |  | 208 -> 105 (-49.5 %, MAD 0) |
| cw:c46_andor | words |  |  |  | 93 |  |  | 89 |  |  |  |  |  | 93 -> 89 (-4.3 %, MAD 14) |
| cw:c47_usenest | words |  |  |  | 44 |  |  |  |  |  |  |  |  | 51 -> 44 (-13.7 %, MAD 0) |
| cw:c50_cas | words |  |  |  | 28 |  |  |  |  |  |  |  |  | 31 -> 28 (-9.7 %, MAD 0) |
| cw:c53_param9 | words |  |  |  | 64 |  |  |  |  |  |  |  |  | 106 -> 64 (-39.6 %, MAD 0) |
| fuzz_seeds_on_bin | seeds |  |  |  | 0 | 0 |  | 0 | 0 |  | 0 |  | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_rate | prog/s |  |  |  | 0 | 0 |  | 0 | 0 |  | 0 |  | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_trap_unpredicted | count |  |  |  | 0 | 0 |  | 0 | 0 |  | 0 |  | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| selfcompile_wall | ms |  |  |  | 1511 ? | 1475 ? |  | 3.848e+04 ? | 1695 ? |  | 1935 ? |  | 1936 ? | invalid window |
| selfcompile_utime | s |  |  |  | 1.435 ? | 1.428 ? |  | 25.28 ? | 1.615 ? |  | 1.86 ? |  | 1.86 ? | invalid window |
| selfcompile_stime | s |  |  |  | 0.0355 ? | 0.0278 ? |  | 0.0316 ? | 0.0394 ? |  | 0.0357 ? |  | 0.0356 ? | invalid window |
| selfcompile_maxrss | kB |  |  |  | 21424 ? | 21544 ? |  | 21812 ? | 21728 ? |  | 21952 ? |  | 21952 ? | invalid window |
| selfcompile_energy | core-s@fmax (proxy) |  |  |  | 1.422 ? | 1.424 ? |  | 31.89 ? | 1.598 ? |  | 1.824 ? |  | 1.825 ? | invalid window |
| k1h_ms | ms/rep | 1.09 ? |  |  | 1.25 ? | 1.27 ? |  | 1.26 ? | 1.19 ? |  |  |  | 1.24 ? | invalid window |
| k1h_loopwords | words | 5 |  |  | 5 | 5 |  | 5 | 5 |  |  |  | 5 | 5 -> 5 |
| k2h_ms | ms/rep | 0.76 ? |  |  | 0.86 ? | 0.84 ? |  | 0.85 ? | 0.82 ? |  |  |  | 0.81 ? | invalid window |
| k2h_loopwords | words | 51 |  |  | 51 | 51 |  | 51 | 51 |  |  |  | 51 | 51 -> 51 |
| k3h_ms | ms/rep | 0.29 ? |  |  | 0.36 ? | 0.35 ? |  | 0.33 ? | 0.34 ? |  |  |  | 0.29 ? | invalid window |
| k3h_loopwords | words | 8 |  |  | 8 | 8 |  | 8 | 8 |  |  |  | 8 | 8 -> 8 |
| k4_ms | ms/rep | 4.03 ? |  |  | 4.35 ? | 4.77 ? |  | 4.42 ? | 4.33 ? |  |  |  | 4.4 ? | invalid window |
| k4_loopwords | words | 7 |  |  | 7 | 7 |  | 7 | 7 |  |  |  | 7 | 7 -> 7 |
| proc_floor_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_warm_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_cold_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| fuzz_trap82 | count |  |  |  | 0 | 0 |  | 0 | 0 |  | 0 |  | 0 | 0 TRAP-82 (SIGSEGV/SIGBUS) on 9694b780, 0 tolerated |
| k8h_ms | ms/rep | 0.3 ? |  |  | 0.27 ? | 0.19 ? |  | 0.16 ? | 0.16 ? |  |  |  | 0.11 ? | invalid window |
| k8h_loopwords | words | 25 |  |  | 25 | 25 |  | 22 | 22 |  |  |  | 14 | 22 -> 14 |
| push_words | words | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 -> 0 |
| cw:c55_vswindow | words |  |  |  | 51 |  |  |  |  |  |  |  |  | first |
| cw:c56_nest | words |  |  |  | 62 |  |  |  |  |  |  |  |  | first |
| cw:c57_flags | words |  |  |  | 26 |  |  | 25 |  |  |  |  |  | 26 -> 25 (-3.8 %, MAD 0) |
| cw:c58_callmix | words |  |  |  | 65 |  |  |  |  |  |  |  |  | first |
| cw:c59_evict | words |  |  |  | 37 |  |  |  |  |  |  |  |  | first |
| cw:c60_nestctor | words |  |  |  | 31 |  |  |  |  |  |  |  |  | first |
| cw:c61_arrcall | words |  |  |  | 53 |  |  |  |  |  |  |  |  | first |
| cw:c66_fncap | words |  |  |  |  | 2582 |  |  |  |  |  |  |  | first |
| cw:c70_csel | words |  |  |  |  |  |  | 32 |  |  |  |  |  | first |
| cw:c71_csel_impure | words |  |  |  |  |  |  | 44 |  |  |  |  |  | first |
| cw:c72_hoist | words |  |  |  |  |  |  |  |  |  |  |  | 40 | first |

Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).
Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).
