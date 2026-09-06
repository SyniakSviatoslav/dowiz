# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)

Status: 2026-09-07 CURRENT (last 12 runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)

| metric | unit | 3d512dc/df6044a8 | 69e0eb5/a903d33b | 69e0eb5/df6044a8 | 69e0eb5/f7a25d38 | 2a9f7e9/3bed1c48 | 2a9f7e9/4c8fc5a6 | 2a9f7e9/c74110f9 | 2a9f7e9/53d13800 | 2a9f7e9/831a357c | a77adc6/831a357c | 44d0e06/aebd2f49 | eb73c29/f20535be | last delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| battery_flakes | count | 0 |  |  | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| gate_run_ms | ms | 23487 |  |  | 0 | 21438 | 2916 | 0 | 0 | 0 | 0 | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| chain_wall | s |  |  |  |  |  |  |  |  |  |  | 93 | 103 | 93 -> 103 (+10.8 %, MAD 8.5) |
| chain_cpu | s |  |  |  |  |  |  |  |  |  |  | 54.49 | 59.34 | 54.49 -> 59.34 (+8.9 %, MAD 7.65) |
| bin_words | words |  | 67775 | 68233 | 68229 |  |  |  | 36218 | 36059 | 36059 | 36195 | 36195 | 36195 -> 36195 |
| stub_words | words |  | 131 | 131 | 131 |  |  |  | 131 | 131 | 131 | 131 | 131 | 131 -> 131 |
| bin_fns | fns |  | 202 | 204 | 204 |  |  |  | 255 | 255 | 255 | 255 | 255 | 255 -> 255 (+0.0 %, MAD 0) |
| cw:c01_lit | words |  |  |  |  |  |  |  |  |  |  | 9 |  | 30 -> 9 (-70.0 %, MAD 0) |
| cw:c02_arith | words |  |  |  |  |  |  |  |  |  |  | 7 |  | 23 -> 7 (-69.6 %, MAD 0) |
| cw:c03_precedence | words |  |  |  |  |  |  |  |  |  |  | 7 |  | 12 -> 7 (-41.7 %, MAD 0) |
| cw:c04_cmp | words |  |  |  |  |  |  |  |  |  |  | 51 |  | 56 -> 51 (-8.9 %, MAD 6) |
| cw:c05_if | words |  |  |  |  |  |  |  |  |  |  | 18 |  | 17 -> 18 (+5.9 %, MAD 8) |
| cw:c06_let | words |  |  |  |  |  |  |  |  |  |  | 13 |  | 20 -> 13 (-35.0 %, MAD 0) |
| cw:c07_while | words |  |  |  |  |  |  |  |  |  |  | 17 |  | 24 -> 17 (-29.2 %, MAD 0) |
| cw:c08_call | words |  |  |  |  |  |  |  |  |  |  | 25 |  | 37 -> 25 (-32.4 %, MAD 0) |
| cw:c09_recursion | words |  |  |  |  |  |  |  |  |  |  | 26 |  | 31 -> 26 (-16.1 %, MAD 6) |
| cw:c10_struct | words |  |  |  |  |  |  |  |  |  |  | 40 |  | 54 -> 40 (-25.9 %, MAD 0) |
| cw:c11_enum |  |  |  |  |  |  |  |  |  |  |  |  |  | 19 -> 7 (-63.2 %, MAD 0) |
| cw:c12_match | words |  |  |  |  |  |  |  |  |  |  | 10 |  | 16 -> 10 (-37.5 %, MAD 0) |
| cw:c13_array | words |  |  |  |  |  |  |  |  |  |  | 29 |  | 72 -> 29 (-59.7 %, MAD 0) |
| cw:c14_string | words |  |  |  |  |  |  |  |  |  |  | 13 |  | 17 -> 13 (-23.5 %, MAD 0) |
| cw:c15_bitwise | words |  |  |  |  |  |  |  |  |  |  | 7 |  | 30 -> 7 (-76.7 %, MAD 0) |
| cw:c16_compound | words |  |  |  |  |  |  |  |  |  |  | 18 |  | 30 -> 18 (-40.0 %, MAD 0) |
| cw:c17_neg | words |  |  |  |  |  |  |  |  |  |  | 10 |  | 33 -> 10 (-69.7 %, MAD 0) |
| cw:c18_bigconst |  |  |  |  |  |  |  |  |  |  |  |  |  | 22 -> 10 (-54.5 %, MAD 0) |
| cw:c19_multi | words |  |  |  |  |  |  |  |  |  |  | 53 |  | 67 -> 53 (-20.9 %, MAD 6) |
| cw:c20_deep | words |  |  |  |  |  |  |  |  |  |  | 46 |  | 95 -> 46 (-51.6 %, MAD 5) |
| cw:c21_param13 | words |  |  |  |  |  |  |  |  |  |  | 65 |  | 122 -> 65 (-46.7 %, MAD 0) |
| cw:c22_matchbind | words |  |  |  |  |  |  |  |  |  |  | 10 |  | 15 -> 10 (-33.3 %, MAD 0) |
| cw:c23_spillcall | words |  |  |  |  |  |  |  |  |  |  | 115 |  | 225 -> 115 (-48.9 %, MAD 0) |
| cw:c24_ifspill | words |  |  |  |  |  |  |  |  |  |  | 81 |  | 140 -> 81 (-42.1 %, MAD 6) |
| cw:c25_matchtail | words |  |  |  |  |  |  |  |  |  |  | 28 |  | 62 -> 28 (-54.8 %, MAD 0) |
| cw:c26_selfrec | words |  |  |  |  |  |  |  |  |  |  | 108 |  | 163 -> 108 (-33.7 %, MAD 20) |
| cw:c27_zeroarg | words |  |  |  |  |  |  |  |  |  |  | 29 |  | 32 -> 29 (-9.4 %, MAD 0) |
| cw:c30_unary | words |  |  |  |  |  |  |  |  |  |  | 31 |  | 61 -> 31 (-49.2 %, MAD 0) |
| cw:c31_nested_lit | words |  |  |  |  |  |  |  |  |  |  | 67 |  | 149 -> 67 (-55.0 %, MAD 0) |
| cw:c32_asr | words |  |  |  |  |  |  |  |  |  |  | 34 |  | 69 -> 34 (-50.7 %, MAD 0) |
| cw:c33_loopalloc | words |  |  |  |  |  |  |  |  |  |  | 56 |  | 121 -> 56 (-53.7 %, MAD 0) |
| cw:c34_loopescape | words |  |  |  |  |  |  |  |  |  |  | 58 |  | 107 -> 58 (-45.8 %, MAD 0) |
| cw:c35_return | words |  |  |  |  |  |  |  |  |  |  | 48 |  | 59 -> 48 (-18.6 %, MAD 0) |
| cw:c36_break | words |  |  |  |  |  |  |  |  |  |  | 70 |  | 137 -> 70 (-48.9 %, MAD 0) |
| cw:c40_struct | words |  |  |  |  |  |  |  |  |  |  | 115 |  | 192 -> 115 (-40.1 %, MAD 0) |
| cw:c41_clz | words |  |  |  |  |  |  |  |  |  |  | 42 |  | 67 -> 42 (-37.3 %, MAD 0) |
| cw:c42_crc32 | words |  |  |  |  |  |  |  |  |  |  | 115 |  | 192 -> 115 (-40.1 %, MAD 0) |
| cw:c43_arena_persist | words |  |  |  |  |  |  |  |  |  |  | 130 |  | 262 -> 130 (-50.4 %, MAD 10) |
| cw:c44_use24 | words |  |  |  |  |  |  |  |  |  |  | 297 |  | 328 -> 297 (-9.5 %, MAD 0) |
| cw:c45_crc32x | words |  |  |  |  |  |  |  |  |  |  | 105 |  | 208 -> 105 (-49.5 %, MAD 0) |
| cw:c46_andor | words |  |  |  |  |  |  |  |  |  |  | 93 |  | 145 -> 93 (-35.9 %, MAD 4) |
| cw:c47_usenest | words |  |  |  |  |  |  |  |  |  |  | 44 |  | 51 -> 44 (-13.7 %, MAD 0) |
| cw:c50_cas | words |  |  |  |  |  |  |  |  |  |  | 28 |  | 31 -> 28 (-9.7 %, MAD 0) |
| cw:c53_param9 | words |  |  |  |  |  |  |  |  |  |  | 64 |  | 106 -> 64 (-39.6 %, MAD 0) |
| fuzz_seeds_on_bin | seeds |  |  |  | 0 |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_rate | prog/s |  |  |  | 0 |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_trap_unpredicted | count |  |  |  | 0 |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| selfcompile_wall | ms |  |  |  | 1508 ? |  |  |  |  |  |  | 1511 ? | 1475 ? | invalid window |
| selfcompile_utime | s |  |  |  | 1.44 ? |  |  |  |  |  |  | 1.435 ? | 1.428 ? | invalid window |
| selfcompile_stime | s |  |  |  | 0.0237 ? |  |  |  |  |  |  | 0.0355 ? | 0.0278 ? | invalid window |
| selfcompile_maxrss | kB |  |  |  | 22628 ? |  |  |  |  |  |  | 21424 ? | 21544 ? | invalid window |
| selfcompile_energy | core-s@fmax (proxy) |  |  |  | 0 ? |  |  |  |  |  |  | 1.422 ? | 1.424 ? | invalid window |
| k1h_ms | ms/rep |  |  |  | 1.75 ? |  |  |  | 1.09 ? |  |  | 1.25 ? | 1.27 ? | invalid window |
| k1h_loopwords | words |  |  |  | 10 |  |  |  | 5 |  |  | 5 | 5 | 5 -> 5 |
| k2h_ms | ms/rep |  |  |  | 0.71 ? |  |  |  | 0.76 ? |  |  | 0.86 ? | 0.84 ? | invalid window |
| k2h_loopwords | words |  |  |  | 51 |  |  |  | 51 |  |  | 51 | 51 | 51 -> 51 |
| k3h_ms | ms/rep |  |  |  | 0.54 ? |  |  |  | 0.29 ? |  |  | 0.36 ? | 0.35 ? | invalid window |
| k3h_loopwords | words |  |  |  | 24 |  |  |  | 8 |  |  | 8 | 8 | 8 -> 8 |
| k4_ms | ms/rep |  |  |  | 4.04 ? |  |  |  | 4.03 ? |  |  | 4.35 ? | 4.77 ? | invalid window |
| k4_loopwords | words |  |  |  | 14 |  |  |  | 7 |  |  | 7 | 7 | 7 -> 7 |
| proc_floor_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_warm_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_cold_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| fuzz_trap82 | count |  |  |  | 0 |  |  |  |  |  |  | 0 | 0 | 0 TRAP-82 (SIGSEGV/SIGBUS) on f20535be, 0 tolerated |
| k8h_ms | ms/rep |  |  |  | 0.33 ? |  |  |  | 0.3 ? |  |  | 0.27 ? | 0.19 ? | invalid window |
| k8h_loopwords | words |  |  |  | 39 |  |  |  | 25 |  |  | 25 | 25 | 25 -> 25 |
| push_words | words |  |  |  |  |  |  |  | 0 | 0 | 0 | 0 | 0 | 0 -> 0 |
| cw:c55_vswindow | words |  |  |  |  |  |  |  |  |  |  | 51 |  | first |
| cw:c56_nest | words |  |  |  |  |  |  |  |  |  |  | 62 |  | first |
| cw:c57_flags | words |  |  |  |  |  |  |  |  |  |  | 26 |  | first |
| cw:c58_callmix | words |  |  |  |  |  |  |  |  |  |  | 65 |  | first |
| cw:c59_evict | words |  |  |  |  |  |  |  |  |  |  | 37 |  | first |
| cw:c60_nestctor | words |  |  |  |  |  |  |  |  |  |  | 31 |  | first |
| cw:c61_arrcall | words |  |  |  |  |  |  |  |  |  |  | 53 |  | first |
| cw:c66_fncap | words |  |  |  |  |  |  |  |  |  |  |  | 2582 | first |

Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).
Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).
