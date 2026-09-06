# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)

Status: 2026-09-06 CURRENT (last 4 runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)

| metric | unit | 5ec3152/e14dd55e | f86bee7/e14dd55e | 01185e7/e14dd55e | e97d301/e14dd55e | last delta |
|---|---|---|---|---|---|---|
| battery_flakes | count | 0 | 0 | 0 | 0 | first |
| gate_run_ms | ms | 0 | 0 | 0 | 0 | first |
| chain_wall | s | 18 | 16 | 24 | 38 | first |
| chain_cpu | s | 27.63 | 26.4 | 28.79 | 50.14 | first |
| bin_words | words | 74804 | 74804 | 74804 | 74804 | 74804 -> 74804 |
| stub_words | words | 131 | 131 | 131 | 131 | 131 -> 131 |
| bin_fns | fns | 199 | 199 | 199 | 199 | first |
| cw:c01_lit | words | 42 |  |  |  | first |
| cw:c02_arith | words | 35 |  |  |  | first |
| cw:c03_precedence | words | 24 |  |  |  | first |
| cw:c04_cmp | words | 92 |  |  |  | first |
| cw:c05_if | words | 37 |  |  |  | first |
| cw:c06_let | words | 30 |  |  |  | first |
| cw:c07_while | words | 34 |  |  |  | first |
| cw:c08_call | words | 59 |  |  |  | first |
| cw:c09_recursion | words | 63 |  |  |  | first |
| cw:c10_struct | words | 92 |  |  |  | first |
| cw:c11_enum | words | 19 |  |  |  | first |
| cw:c12_match | words | 26 |  |  |  | first |
| cw:c13_array | words | 78 |  |  |  | first |
| cw:c14_string | words | 27 |  |  |  | first |
| cw:c15_bitwise | words | 42 |  |  |  | first |
| cw:c16_compound | words | 40 |  |  |  | first |
| cw:c17_neg | words | 45 |  |  |  | first |
| cw:c18_bigconst | words | 22 |  |  |  | first |
| cw:c19_multi | words | 105 |  |  |  | first |
| cw:c20_deep | words | 122 |  |  |  | first |
| cw:c21_param13 | words | 139 |  |  |  | first |
| cw:c22_matchbind | words | 25 |  |  |  | first |
| cw:c23_spillcall | words | 246 |  |  |  | first |
| cw:c24_ifspill | words | 169 |  |  |  | first |
| cw:c25_matchtail | words | 70 |  |  |  | first |
| cw:c26_selfrec | words | 239 |  |  |  | first |
| cw:c27_zeroarg | words | 42 |  |  |  | first |
| cw:c30_unary | words | 69 |  |  |  | first |
| cw:c31_nested_lit | words | 167 |  |  |  | first |
| cw:c32_asr | words | 75 |  |  |  | first |
| cw:c33_loopalloc | words | 127 |  |  |  | first |
| cw:c34_loopescape | words | 113 |  |  |  | first |
| cw:c35_return | words | 95 |  |  |  | first |
| cw:c36_break | words | 141 |  |  |  | first |
| cw:c40_struct | words | 206 |  |  |  | first |
| cw:c41_clz | words | 73 |  |  |  | first |
| cw:c42_crc32 | words | 196 |  |  |  | first |
| cw:c43_arena_persist | words | 285 |  |  |  | first |
| cw:c44_use24 | words | 586 |  |  |  | first |
| cw:c45_crc32x | words | 214 |  |  |  | first |
| cw:c46_andor | words | 173 |  |  |  | first |
| cw:c47_usenest | words | 91 |  |  |  | first |
| cw:c50_cas | words | 57 |  |  |  | first |
| cw:c53_param9 | words | 112 |  |  |  | first |
| fuzz_seeds_on_bin | seeds | 1000 | 3000 | 4000 | 8500 | first |
| fuzz_rate | prog/s | 1.6 | 1.58 | 1.58 | 1.7 | first |
| fuzz_trap_unpredicted | count | 7 | 15 | 24 | 30 | first |
| selfcompile_wall | ms | 1569 ? | 1567 ? | 1479 ? | 1723 ? | invalid window |
| selfcompile_utime | s | 1.529 ? | 1.522 ? | 1.432 ? | 1.648 ? | invalid window |
| selfcompile_stime | s | 0.02 ? | 0.0278 ? | 0.0199 ? | 0.0357 ? | invalid window |
| selfcompile_maxrss | kB | 23000 ? | 22988 ? | 23000 ? | 22900 ? | invalid window |
| selfcompile_energy | core-s@fmax (proxy) | 1.633 ? | 1.626 ? | 1.691 ? | 1.653 ? | invalid window |
| k1h_ms | ms/rep | 1.64 ? | 1.59 | 1.54 ? | 1.9 ? | invalid window |
| k1h_loopwords | words | 11 | 11 | 11 | 11 | 11 -> 11 |
| k2h_ms | ms/rep | 1.18 ? | 1.09 | 1.08 ? | 1.38 ? | invalid window |
| k2h_loopwords | words | 65 | 65 | 65 | 65 | 65 -> 65 |
| k3h_ms | ms/rep | 0.54 ? | 0.52 | 0.53 ? | 0.73 ? | invalid window |
| k3h_loopwords | words | 25 | 25 | 25 | 25 | 25 -> 25 |
| k4_ms | ms/rep | 4.04 ? | 3.75 | 3.77 ? | 4.41 ? | invalid window |
| k4_loopwords | words | 15 | 15 | 15 | 15 | 15 -> 15 |
| proc_floor_ms | ms | 23 |  |  |  | first |
| becache_warm_ms | ms | 36 |  |  |  | first |
| becache_cold_ms | ms | 48 |  |  |  | first |
| fuzz_trap82 | count |  |  | 0 | 0 | 0 TRAP-82 (SIGSEGV/SIGBUS) on e14dd55e, 0 tolerated |

Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).
Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).
