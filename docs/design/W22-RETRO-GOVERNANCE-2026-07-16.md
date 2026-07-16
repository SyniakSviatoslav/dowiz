# W22 — Governance/doc finalize + DOD retro

## Governance truth (operator decision, NOT a regression)
`.claude/CLAUDE.md` + `settings.json`: hooks SUSPENDED (operator directive 2026-07-15).
All 10 `.claude/hooks/` scripts are present but NO-OP (321B each, valid JSON, zero side effects).
This is the operator's FULL-SELF-MGMT choice (AUTHORITY memory 2026-07-16: autopilot incl. push).
**Decision: do NOT restore hooks.** They were consciously disabled, not broken.
If the operator later wants guard-rail hooks back, that is a separate decision — not part of "finish all 6".

## DOD retro — every wave verified with literal 0-failed proof

| Wave | Unit | Gate | Literal evidence | Status |
|------|------|------|------------------|--------|
| W17 | web-UI wasm bridges (Rust-native, NO TS) | node test + grep no JS re-impl | `node web/src/lib/kernel/kernel.test.mjs` → 4 ok (spectral=1, malformed→{ok:false}, fsm ok, geo ok), EXIT=0; grep haversine/eigen/fsm/acos/atan2 in web/src = 0 matches | GREEN |
| W18 | living-knowledge Rust PRIMARY recall | cargo test retrieval:: + recall@5=1.0 | `cargo test -p dowiz-kernel --lib` → 333 passed; `w18_primary_recall_at_5_is_one_point_zero_on_deterministic_fixture` ok (recall@5=1.000) | GREEN |
| W19 | Kalman/trigram into decide/loop | variance-reduction + deterministic trigram | Kalman fold MSE 85.2% lower than raw (0.034151→0.005060), fail-closed prior-hold Δ<1e-12; trigram top-1 `abc` count2, empty→0; kernel 333/0 | GREEN |
| W20 | VertexBridge CPU + gpu feature-gate | default engine test + gpu build | `cargo test -p dowiz-engine` 47 passed (1 logical upload, 0 gpu, 0 json); `cargo build --features gpu` Finished; new_gpu→honest Err; grep wgpu = only inside 2 `#[cfg(feature="gpu")]` blocks | GREEN |
| W21 | Field-UI GPU render loop | — | BLOCKED: wgpu NOT in cargo cache (verified 2026-07-16), absent from all Cargo.lock. Decart W15 = reject offline. Left as documented ceiling; trigger = network cargo-add. | CEILING (not failed) |
| W22 | governance doc + retro | this file | governance truth recorded; retro above | DONE |

**Consolidated suite-green (post-merge):** dowiz-kernel `--lib` = 333 passed/0 failed; fsm_boot = 4/0;
dowiz-engine = 47/0 (default) + 48/0 (`--features gpu`); web node bridge = 0 failures.

## Net
5 of 6 tracks FINISHED + GREEN (W17-W20, W22). W21 is an honest offline ceiling (wgpu uncached),
not a failure — documented with a falsifiable trigger. Zero fake-green: every gate has literal output.

## Disk (parallel analysis, operator decision pending)
Root fs at 94% (4.8G free). Deep analysis delivered (see conversation). Recoverable:
~18-19G safe-delete (scratch/caches) + ~8.6G external-backup candidates. Operator approval needed
for any destructive/archive action (red-line). Proposed phased plan: (a) /tmp scratch now (~5G),
(b) cargo clean + caches after this push (~15G), (c) Bucket-C off-box + Bucket-B zstd.
