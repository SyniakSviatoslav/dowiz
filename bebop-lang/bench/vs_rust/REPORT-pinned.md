Status: 2026-09-04 CURRENT (T63/T72/T97, decision D3: pinned in-process clock_ms is the primary column; regenerate with `R=11 bench/vs_rust/bench_pinned.sh`; supersedes REPORT-630.md for K1-K4 timing)

# REPORT-pinned — K1-K4 pinned vs unpinned, Bebop vs Rust once-twins (T63/T72/T97, D3)

- date: 2026-09-04 22:06; bebop.bin md5 104b629124d4e243642137ff73b37154; runs per cell R=11 (+1 warmup); rustc -O (no lto), twins bench/vs_rust/rust_once/k*.rs
- big cores (part 0xd41): [4 5 6 7], sched_getaffinity usable: [0 1 2 3 4 5 6 7], pinned core: 4; taskset honoured under proot: yes (Cpus_allowed_list=4)
- wall = whole process (seed load+mmap+run / rust start+run), perf_counter around spawn..wait4; RSS = ru_maxrss from wait4 (= VmHWM hiwater_rss, no /proc race); K0 = empty program = startup floor
- ratio = bebop pinned median / rust pinned median (process wall, includes both startup floors)

| kernel | bebop pinned med/p95 ms | bebop unpinned med/p95 ms | rust pinned med/p95 ms | rust unpinned med ms | ratio | RSS bebop KB | RSS rust KB | fold ok |
|---|---|---|---|---|---|---|---|---|
| K0 | 8.38 / 17.71 | 4.18 / 5.95 | 12.42 / 14.76 | 5.74 | 0.67x | 16896 | 17024 | ok |
| K1 | 10.68 / 13.13 | 5.94 / 6.62 | 17.95 / 23.60 | 9.77 | 0.60x | 17024 | 17024 | ok |
| K2 | 19.18 / 24.94 | 5.92 / 6.27 | 14.73 / 20.33 | 8.38 | 1.30x | 17152 | 17152 | ok |
| K3 | 9.94 / 15.47 | 3.74 / 4.22 | 12.98 / 17.75 | 7.31 | 0.77x | 17152 | 17152 | SAME-but-unexpected |
| K4 | 23.40 / 24.52 | 16.33 / 16.95 | 22.53 / 29.21 | 15.59 | 1.04x | 17152 | 17152 | ok |

## In-process clock_ms (bench630/k*t.bp, D3 primary column), ms

| kernel | pinned med / p95 | unpinned med / p95 | spread unpinned/pinned |
|---|---|---|---|
| K1 | 3.0 / 4.0 | 3.0 / 4.0 | 1.00x |
| K2 | 1.6 / 2.8 | 1.6 / 1.6 | 1.00x |
| K3 | 0.6 / 0.6 | 0.6 / 0.6 | 1.00x |
| K4 | 13.0 / 18.0 | 13.0 / 13.0 | 1.00x |

## Words per iteration (backward-branch spans in the compiled .bin, head..back-branch inclusive; smallest = innermost loop)

| kernel | loop spans (words) |
|---|---|
| K1 | [25] |
| K2 | [] |
| K3 | [46, 64] |
| K4 | [40] |

## Self-compile (`seed bebop.bin compile bebop.bp`), pinned, 3 runs

- wall median 108720 ms, peak RSS 79616 KB (77.8 MB)


## Before/after T96 step 1 (in-process pinned ms, same twins; "before" = docs/SPEEDUP-ANALYSIS.md §2 measured at 364009e9)

| kernel | before | after step 1 | Rust twin | after/Rust |
|---|---|---|---|---|
| K1 | 10.0 | 3.0 | 2.41 | 1.24x |
| K2 | 2.85 | 1.6 | 0.277 | 5.8x |
| K3 | 1.2-1.5 | 0.6 | 0.213 | 2.8x |
| K4 | 32 | 13.0 | 2.85 | 4.6x |
