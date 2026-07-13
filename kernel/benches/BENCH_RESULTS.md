# dowiz-kernel — criterion baseline (FIRST CAPTURE)

Machine: Linux 6.8.0-124-generic. Captured 2026-07-13.
Run: `cargo bench --bench criterion -- --warm-up-time 1 --measurement-time 2 --sample-size 10`

| benchmark            | mean (ns) | low (ns) | high (ns) |
|----------------------|-----------|----------|-----------|
| place_order/5_items  | 90.4      | 88.8     | 92.9      |
| fold_transitions/5_hops | 5.59   | 5.44     | 5.80      |

Notes:
- `place_order` exercises OrderItem subtotal (checked i64 arithmetic) + struct build.
- `fold_transitions` exercises the legal Pending→Confirmed→Preparing→Ready→InDelivery→Delivered path.
- Re-run after any hot-path change to detect regressions.
