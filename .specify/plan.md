# Plan — KU03

## Architecture
- Kernel (`dowiz/kernel`) owns money authority. Add `PriceCatalog` (trusted source) +
  `Currency` typed enum with cross-currency guard. `place_order` accepts an optional catalog;
  when present, line `unit_price` is RE-DERIVED from catalog (caller value ignored/untrusted);
  when absent (legacy call path), caller value kept but flagged untrusted. This closes M1/M2/M5
  without breaking existing callers.
- Engine (`dowiz/engine`): add `feature = "gpu"` pulling `wgpu`. `VertexBridge::new` allocates a
  real `wgpu::Buffer` when gpu; `upload_once` calls `queue.write_buffer`. Default build keeps the
  CPU-only staging + a `HeadlessGpu` mock so the GREEN gate (1 upload, 0 json) is falsifiable
  headless. No `wgpu` in default deps (offline-clean mandate preserved).
- TS purge: scan for TS that re-implements kernel/engine math (haversine/geo, spectral, money,
  paginate, dedup). Delete those files; emit MANIFEST. Keep Svelte UI shell.

## API contract
- `PriceCatalog::unit_price(&self, product_id, modifier_ids) -> Option<i64>` (trusted).
- `place_order(..., catalog: Option<&PriceCatalog>) -> Result<Order, TransitionError>` —
  uses catalog when Some.
- `Currency` enum; `Money { amount: i64 minor, currency: Currency }`; cross-currency op returns Err.
- `VertexBridge::new_gpu(device, queue, count, stride) -> Self` (gpu feature only).

## File list (new/changed)
- kernel/src/domain.rs — catalog-aware place_order; currency typed.
- kernel/src/money.rs — Currency enum + cross-currency guard (M5).
- kernel/src/catalog.rs — NEW trusted PriceCatalog (M1/M2).
- engine/src/bridge.rs — gpu feature + HeadlessGpu mock.
- engine/Cargo.toml — [features] gpu = ["wgpu"].
- TS purge: delete dupe files; write docs/kernel-upgrade/TS-PURGE-MANIFEST.md.

## Risks
- wgpu GPU path untestable headless → mitigate with HeadlessGpu mock + feature-gate.
- TS deletion may break web build → mitigate by deleting ONLY verified-duplicate compute,
  documenting remaining 11k as UI shell (kept).
- Money = red-line-ish → changes are additive (catalog Option), no live pricing change;
  flagged in commit, not pushed (operator gate).

## RED→GREEN test plan per task
- T1 (catalog): RED fail — place_order ignores tampered price without catalog; GREEN — with
  catalog, tampered caller price overridden.
- T2 (currency): RED — mixed-currency add panics/Err; GREEN — same-currency ok, mixed Err.
- T3 (vertexbridge gpu): GREEN — 1 write_buffer on gpu; 0 on headless (mock).
- T4 (ts-purge): manifest written; kernel+engine suites green post-delete.
