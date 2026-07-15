# Tasks — KU03 (ordered, RED→GREEN)

- T1 [LANE:1] — kernel: trusted PriceCatalog + catalog-aware place_order (M1/M2). RED: place_order honors tampered caller price; GREEN: with catalog, caller price overridden by trusted value. → kernel/src/catalog.rs, domain.rs
- T2 [LANE:1] — kernel: Currency typed enum + cross-currency guard in money.rs (M5). RED: mixed-currency add returns Ok; GREEN: same-currency Ok, mixed Err.
- T3 [LANE:2] — engine: VertexBridge gpu feature + HeadlessGpu mock (real write_buffer on gpu, 0 on headless). RED: upload_once no-op; GREEN: gpu=1 real upload, headless=mock(0 json).
- T4 [LANE:3] — TS purge: scan+delete TS duplicating kernel/engine compute; write MANIFEST. RED: dupe TS present; GREEN: deleted + kernel/engine green.
- T5 [LANE:1] — kernel: parity re-check harmonic+eigen + full lib green (regression guard).
- T6 [LANE:2] — engine: full crate green with/without gpu feature.
- T7 — DoD: telemetry step ticks per T; retro at end; commit all; report DONE with 0 failed.

Lanes: 1=kernel, 2=engine, 3=ts-purge (collision-free files → could parallelize, but MAIN re-verifies each with literal cargo test before commit).
