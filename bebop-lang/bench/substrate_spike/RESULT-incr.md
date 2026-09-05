# T107 incremental-substrate curve (2026-09-05, 362c0e02, core 4, R=3 medians, us per rep of k changes)

| k | bebop sweep | bebop full | sweep/full | Rust sweep | Rust full | sweep/full | folds |
|---|---|---|---|---|---|---|---|
| 1 | 15 | 1031 | 0.01x | 4 | 132 | 0.03x | equal |
| 16 | 234 | 984 | 0.24x | 50 | 127 | 0.39x | equal |
| 256 | 1828 | 1078 | 1.70x | 525 | 129 | 4.07x | equal |
| 4096 | 5281 | 1109 | 4.76x | 1446 | 135 | 10.71x | equal |

- crossover (first k where sweep >= full): bebop 256, Rust 256; k/N = 0.39% (bebop)
- N = 65536 cells (16 layers x 4096), 64 reps per measurement, same LCG change set in both modes and engines
