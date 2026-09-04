# T55 spike — one straight-line fn (12 ops, DAG depth 7) : linear vs cell substrate
- N=300000 evaluations, pinned core 4 (A78), R=5 medians, in-process clock_ms; bebop.bin md5 364009e9cb7e106dc91cdddcbf981f09
- fold identical across all three bebop modes: YES; sweeps == 7*N: YES; bebop fold 80999836923180048 == Rust twin fold YES
- linear inlined loop: backward-branch spans in spike_fold.bin (words/iteration incl. driver): [84, 105, 215, 265, 291, 364]

| mode | median ms | ns per op | vs linear-inlined | vs Rust |
|---|---|---|---|---|
| bebop linear, inlined | 18.0 | 5.0 | 1.00x | 18.05x |
| bebop linear, fn call per eval | 21.0 | 5.8 | 1.17x | 21.06x |
| bebop substrate (sweeps to quiescence) | 738.0 | 205.0 | 41.00x | 740.22x |
| Rust -O twin (inlined, black_box) | 1.0 | 0.3 | 0.06x | 1.00x |
| Rust -O twin of the SAME substrate engine (model floor) | 39.2 | 10.9 | 2.18x | 39.27x |

- substrate per sweep: 351 ns; per fired cell: 205 ns (each sweep = tzcnt drain + branch-free 6-way op select + candidate/readiness scan)
- Rust substrate twin: fold == YES, sweeps {2100000}; per sweep 18.6 ns, per cell 10.9 ns -> the MODEL alone costs 39x over linear Rust on this ISA; bebop codegen adds another 19x on top
- linear inlined per op: 5.0 ns; Rust per op: 0.28 ns
