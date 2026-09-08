# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)

Status: 2026-09-08 CURRENT (last 12 runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)

| metric | unit | 072ad81/3a827a3f | f1542f1/3a827a3f | 3005040/0ae01bd3 | 3005040/3a827a3f | 257e71b/ca69273e | 257e71b/3a827a3f | 63626eb/ca69273e | e19ac9b/ca69273e | /2e50a72f | /01e90e25 | /15e272fc | 06fd210/15e272fc | last delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| battery_flakes | count | 0 |  |  | 0 | 0 |  | 0 | 0 | 0 | 0 | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| gate_run_ms | ms | 5288 |  |  | 5553 | 21005 |  | 8079 | 8119 | 8938 | 28372 | 0 | 8931 | 2.837e+04 -> 8931 (-68.5 %, MAD 5.24e+03) |
| chain_wall | s | 104 |  |  |  |  |  |  |  |  |  | 139 | 107 | 104 -> 107 (+2.9 %, MAD 10.5) |
| chain_cpu | s | 54.36 |  |  |  |  |  |  |  |  |  | 82.38 | 56.98 | 54.36 -> 56.98 (+4.8 %, MAD 6.64) |
| bin_words | words | 39907 | 39907 |  | 39907 | 39678 | 39907 | 39678 | 39678 | 41161 | 41161 | 41217 | 41217 | 41161 -> 41217 |
| stub_words | words | 172 | 172 |  | 172 | 172 | 172 | 172 | 172 | 172 | 172 | 172 | 172 | 172 -> 172 |
| bin_fns | fns | 284 | 284 |  | 284 | 283 | 284 | 283 | 283 | 285 | 285 | 285 | 285 | 285 -> 285 (+0.0 %, MAD 0) |
| cw:c01_lit |  |  |  |  |  |  |  |  |  |  |  |  |  | 30 -> 9 (-70.0 %, MAD 0) |
| cw:c02_arith |  |  |  |  |  |  |  |  |  |  |  |  |  | 23 -> 7 (-69.6 %, MAD 0) |
| cw:c03_precedence |  |  |  |  |  |  |  |  |  |  |  |  |  | 12 -> 7 (-41.7 %, MAD 0) |
| cw:c04_cmp |  |  |  |  |  |  |  |  |  |  |  |  |  | 51 -> 41 (-19.6 %, MAD 17.5) |
| cw:c05_if |  |  |  |  |  |  |  |  |  |  |  |  |  | 18 -> 17 (-5.6 %, MAD 4) |
| cw:c06_let |  |  |  |  |  |  |  |  |  |  |  |  |  | 20 -> 13 (-35.0 %, MAD 0) |
| cw:c07_while |  |  |  |  |  |  |  |  |  |  |  |  |  | 24 -> 17 (-29.2 %, MAD 0) |
| cw:c08_call |  |  |  |  |  |  |  |  |  |  |  |  |  | 37 -> 25 (-32.4 %, MAD 0) |
| cw:c09_recursion |  |  |  |  |  |  |  |  |  |  |  |  |  | 31 -> 26 (-16.1 %, MAD 6) |
| cw:c10_struct |  |  |  |  |  |  |  |  |  |  |  |  |  | 54 -> 40 (-25.9 %, MAD 0) |
| cw:c11_enum |  |  |  |  |  |  |  |  |  |  |  |  |  | 19 -> 7 (-63.2 %, MAD 0) |
| cw:c12_match |  |  |  |  |  |  |  |  |  |  |  |  |  | 16 -> 10 (-37.5 %, MAD 0) |
| cw:c13_array | words |  |  |  |  |  |  |  |  |  |  | 32 |  | 29 -> 32 (+10.3 %, MAD 6) |
| cw:c14_string | words | 11 |  |  |  |  |  |  |  |  |  |  |  | 13 -> 11 (-15.4 %, MAD 4) |
| cw:c15_bitwise |  |  |  |  |  |  |  |  |  |  |  |  |  | 30 -> 7 (-76.7 %, MAD 0) |
| cw:c16_compound | words | 23 |  |  |  |  |  |  |  |  |  |  |  | 18 -> 23 (+27.8 %, MAD 10) |
| cw:c17_neg |  |  |  |  |  |  |  |  |  |  |  |  |  | 33 -> 10 (-69.7 %, MAD 0) |
| cw:c18_bigconst |  |  |  |  |  |  |  |  |  |  |  |  |  | 22 -> 10 (-54.5 %, MAD 0) |
| cw:c19_multi | words | 50 |  |  |  |  |  |  |  |  |  |  |  | 53 -> 50 (-5.7 %, MAD 10) |
| cw:c20_deep | words |  |  |  |  |  |  |  |  |  |  | 46 |  | 45 -> 46 (+2.2 %, MAD 27) |
| cw:c21_param13 |  |  |  |  |  |  |  |  |  |  |  |  |  | 122 -> 65 (-46.7 %, MAD 0) |
| cw:c22_matchbind |  |  |  |  |  |  |  |  |  |  |  |  |  | 15 -> 10 (-33.3 %, MAD 0) |
| cw:c23_spillcall | words |  |  |  |  |  |  |  |  |  |  | 116 |  | 115 -> 116 (+0.9 %, MAD 21) |
| cw:c24_ifspill | words |  |  |  |  |  |  |  |  |  |  | 82 |  | 81 -> 82 (+1.2 %, MAD 14.5) |
| cw:c25_matchtail | words |  |  |  |  |  |  |  |  |  |  | 29 |  | 28 -> 29 (+3.6 %, MAD 8) |
| cw:c26_selfrec | words | 113 |  |  |  |  |  |  |  |  |  |  |  | 108 -> 113 (+4.6 %, MAD 37.5) |
| cw:c27_zeroarg |  |  |  |  |  |  |  |  |  |  |  |  |  | 32 -> 29 (-9.4 %, MAD 0) |
| cw:c30_unary |  |  |  |  |  |  |  |  |  |  |  |  |  | 61 -> 31 (-49.2 %, MAD 0) |
| cw:c31_nested_lit | words |  |  |  |  |  |  |  |  |  |  | 71 |  | 67 -> 71 (+6.0 %, MAD 18) |
| cw:c32_asr |  |  |  |  |  |  |  |  |  |  |  |  |  | 69 -> 34 (-50.7 %, MAD 0) |
| cw:c33_loopalloc | words |  |  |  |  |  |  |  |  |  |  | 58 |  | 56 -> 58 (+3.6 %, MAD 6) |
| cw:c34_loopescape | words |  |  |  |  |  |  |  |  |  |  | 59 |  | 58 -> 59 (+1.7 %, MAD 6) |
| cw:c35_return |  |  |  |  |  |  |  |  |  |  |  |  |  | 59 -> 48 (-18.6 %, MAD 0) |
| cw:c36_break | words |  |  |  |  |  |  |  |  |  |  | 72 |  | 70 -> 72 (+2.9 %, MAD 4) |
| cw:c40_struct | words |  |  |  |  |  |  |  |  |  |  | 120 |  | 115 -> 120 (+4.3 %, MAD 14) |
| cw:c41_clz |  |  |  |  |  |  |  |  |  |  |  |  |  | 67 -> 42 (-37.3 %, MAD 0) |
| cw:c42_crc32 | words |  |  |  |  |  |  |  |  |  |  | 124 |  | 115 -> 124 (+7.8 %, MAD 4) |
| cw:c43_arena_persist | words | 126 |  |  |  |  |  |  |  |  |  | 135 |  | 126 -> 135 (+7.1 %, MAD 68.5) |
| cw:c44_use24 |  |  |  |  |  |  |  |  |  |  |  |  |  | 328 -> 297 (-9.5 %, MAD 0) |
| cw:c45_crc32x | words |  |  |  |  |  |  |  |  |  |  | 108 |  | 105 -> 108 (+2.9 %, MAD 6) |
| cw:c46_andor | words | 81 |  |  |  |  |  |  |  |  |  |  |  | 89 -> 81 (-9.0 %, MAD 28) |
| cw:c47_usenest |  |  |  |  |  |  |  |  |  |  |  |  |  | 51 -> 44 (-13.7 %, MAD 0) |
| cw:c50_cas |  |  |  |  |  |  |  |  |  |  |  |  |  | 31 -> 28 (-9.7 %, MAD 0) |
| cw:c53_param9 |  |  |  |  |  |  |  |  |  |  |  |  |  | 106 -> 64 (-39.6 %, MAD 0) |
| fuzz_seeds_on_bin | seeds | 0 | 0 |  |  |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_rate | prog/s | 0 | 0 |  |  |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| fuzz_trap_unpredicted | count | 0 | 0 |  |  |  |  |  |  |  |  | 0 | 0 | 0 -> 0 (+0.0 %, MAD 0) |
| selfcompile_wall | ms | 19.41 ? | 628.8 ? |  |  |  |  |  |  |  |  | 667.2 ? | 642.1 ? | invalid window |
| selfcompile_utime | s | 0.0045 ? | 0.5763 ? |  |  |  |  |  |  |  |  | 0.5863 ? | 0.5586 ? | invalid window |
| selfcompile_stime | s | 0 ? | 0.0309 ? |  |  |  |  |  |  |  |  | 0.0626 ? | 0.0591 ? | invalid window |
| selfcompile_maxrss | kB | 21316 ? | 22928 ? |  |  |  |  |  |  |  |  | 62024 ? | 62020 ? | invalid window |
| selfcompile_energy | core-s@fmax (proxy) | 0 ? | 0.5955 ? |  |  |  |  |  |  |  |  | 0.7029 ? | 0.7361 ? | invalid window |
| k1h_ms | ms/rep |  | 1.14 ? |  |  |  |  |  |  |  |  | 1.11 ? | 0.91 ? | invalid window |
| k1h_loopwords | words |  | 5 |  |  |  |  |  |  |  |  | 5 | 5 | 5 -> 5 |
| k2h_ms | ms/rep |  | 0.688 ? |  |  |  |  |  |  |  |  | 0.65 ? | 0.566 ? | invalid window |
| k2h_loopwords | words |  | 7 |  |  |  |  |  |  |  |  | 7 | 7 | 7 -> 7 |
| k3h_ms | ms/rep |  | 0.195 ? |  |  |  |  |  |  |  |  | 0.185 ? | 0.161 ? | invalid window |
| k3h_loopwords | words |  | 7 |  |  |  |  |  |  |  |  | 7 | 7 | 7 -> 7 |
| k4_ms | ms/rep |  | 4.33 ? |  |  |  |  |  |  |  |  | 4.04 ? | 3.57 ? | invalid window |
| k4_loopwords | words |  | 7 |  |  |  |  |  |  |  |  | 7 | 7 | 7 -> 7 |
| proc_floor_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_warm_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| becache_cold_ms |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| fuzz_trap82 | count | 0 | 0 |  |  |  |  |  |  |  |  | 0 | 0 | 0 TRAP-82 (SIGSEGV/SIGBUS) on 15e272fc, 0 tolerated |
| k8h_ms | ms/rep |  | 0.029 ? |  |  |  |  |  |  |  |  | 0.029 ? | 0.024 ? | invalid window |
| k8h_loopwords | words |  | 7 |  |  |  |  |  |  |  |  | 7 | 7 | 7 -> 7 |
| push_words | words | 0 | 0 |  | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 -> 0 |
| cw:c55_vswindow |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c56_nest |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c57_flags |  |  |  |  |  |  |  |  |  |  |  |  |  | 26 -> 25 (-3.8 %, MAD 0) |
| cw:c58_callmix |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c59_evict |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c60_nestctor |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c61_arrcall | words |  |  |  |  |  |  |  |  |  |  | 54 |  | 53 -> 54 (+1.9 %, MAD 0) |
| cw:c66_fncap |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c70_csel | words | 30 |  |  |  |  |  |  |  |  |  |  |  | 32 -> 30 (-6.2 %, MAD 0) |
| cw:c71_csel_impure | words | 49 |  | 49 |  |  |  |  |  |  |  |  |  | 49 -> 49 (+0.0 %, MAD 0) |
| cw:c72_hoist | words | 35 |  |  |  |  |  |  |  |  |  |  |  | 40 -> 35 (-12.5 %, MAD 0) |
| sgraph2_build_ms | ms |  |  | 1266 |  |  |  |  |  |  |  |  |  | 1.648e+04 -> 1266 (-92.3 %, MAD 420) |
| sgraph2_bfs_ns | ns |  |  | 111 |  |  |  |  |  |  |  |  |  | 119 -> 111 (-6.7 %, MAD 1.5) |
| sgraph2_log_ns | ns |  |  | 8019 |  |  |  |  |  |  |  |  |  | 1.393e+04 -> 8019 (-42.5 %, MAD 52.5) |
| sgraph2_compact_ms | ms |  |  | 424 |  |  |  |  |  |  |  |  |  | 521 -> 424 (-18.6 %, MAD 19) |
| sbench_insert |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_lookup |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_scan |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_update |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_reopen |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_size1 |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_compact |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_size2 |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_durable |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_dbatch10 |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_dbatch100 |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| sbench_recover |  |  |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c110_fence | words | 39 |  |  |  |  |  |  |  |  |  | 45 |  | ! 39 -> 45 (+15.4 %, MAD 0) |
| cw:c111_kernelfn | words | 30 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c73_hoistnest | words | 39 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c74_madd | words | 36 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c75_andimm | words | 36 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c76_ubfx | words | 34 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c78_scan | words | 720 |  |  |  |  |  |  |  |  |  | 735 |  | 720 -> 735 (+2.1 %, MAD 0) |
| cw:c84_run | words | 258 |  |  |  |  |  |  |  |  |  | 284 |  | ! 258 -> 284 (+10.1 %, MAD 0) |
| cw:c86_selfassign | words | 13 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c87_ifselfassign | words | 31 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c88_arrflags | words | 47 |  |  |  |  |  |  |  |  |  | 49 |  | 47 -> 49 (+4.3 %, MAD 0) |
| cw:c89_heaptrap | words | 31 |  |  |  |  |  |  |  |  |  | 32 |  | 31 -> 32 (+3.2 %, MAD 0) |
| cw:c90_symalias | words | 41 |  |  |  |  |  |  |  |  |  | 42 |  | 41 -> 42 (+2.4 %, MAD 0) |
| cw:c91_letlive | words | 55 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c94_fsync | words | 107 |  |  |  |  |  |  |  |  |  | 111 |  | 107 -> 111 (+3.7 %, MAD 0) |
| cw:c95_symspan | words | 90 |  |  |  |  |  |  |  |  |  |  |  | first |
| cw:c38_frameheap | words |  |  |  |  |  |  |  |  |  |  | 5461 |  | first |
| cw:c69_index_roundtrip | words |  |  |  |  |  |  |  |  |  |  | 97 |  | first |
| cw:c92_ptrfree | words |  |  |  |  |  |  |  |  |  |  | 108 |  | first |

Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).
Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).

## ALERTS

- cw:c110_fence: 39 -> 45 (+15.4 %, MAD 0)
- cw:c84_run: 258 -> 284 (+10.1 %, MAD 0)
