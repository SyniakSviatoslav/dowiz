# BLUEPRINT W21 — Field-UI GPU render loop (FE-04/05/08–17)

## STATUS: BLOCKED OFFLINE (wgpu uncached — verified 2026-07-16)

This is the single hard blocker to "finish all". ORGANISM-STATUS 07-15: "an actual wgpu
render loop — still 0% code across FE-04/05 and FE-08 through FE-17. Nothing renders to a
GPU anywhere." The `wgpu` crate is NOT in the cargo cache and absent from every Cargo.lock,
so it cannot be built air-gapped (decart W15 + SWARM-MANIFEST §3: reject offline).

## WHAT (when unblocked — network cargo-add granted)
- `engine/src/gpu.rs` — real `wgpu::Device`/`Surface` + render pipeline for the field-frame
  vertex buffer (VertexBridge feeds it, see W20).
- FE-04 (render loop), FE-05 (composition shader), FE-08..FE-17 (operator M, UI shell on GPU).
- `cargo add wgpu` under `feature="gpu"`; run GPU-raster smoke in CI.

## RED→GREEN (target)
- RED: zero `wgpu::Device`/`Surface` references in engine.
- GREEN: `cargo build --features gpu` links wgpu + a headless/CI GPU-raster smoke renders
  one field-frame frame (pixel hash deterministic).

## DECISION REQUIRED (operator gate)
Until `cargo add wgpu` is possible (network), this wave = DOCUMENTED CEILING:
- `feature="gpu"` stays EMPTY (W20).
- No fake-green: we do NOT claim GPU render works. The CPU field-frame (W10) remains the
  only demonstrable render path.
- Trigger to unblock: operator grants network `cargo add wgpu` → re-dispatch W21 for real.

## NON-GOALS (while blocked)
- No software-raster GPU emulation (impossible without a GPU binding crate).
