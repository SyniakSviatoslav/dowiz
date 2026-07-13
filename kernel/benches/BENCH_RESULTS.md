# dowiz-pq-kernel — criterion baseline (FIRST CAPTURE)

Machine: Linux 6.8.0-124-generic. Captured 2026-07-13.
Run: `cargo bench --bench criterion -- --warm-up-time 1 --measurement-time 2 --sample-size 10`

| benchmark            | mean (ns) | low (ns) | high (ns) |
|----------------------|-----------|----------|-----------|
| place_order/5_items  | 124.8     | 124.5    | 125.2     |
| fold_transitions/5_hops | 10.82  | 10.51    | 11.08     |

Notes:
- Same hot paths as dowiz-kernel; ~1.4x slower on place_order / ~2x on fold
  (PQ crypto deps linked, though not on this path — likely codegen/lto diff).
- Re-run after any hot-path change to detect regressions.
