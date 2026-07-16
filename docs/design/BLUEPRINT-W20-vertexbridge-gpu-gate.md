# BLUEPRINT W20 — VertexBridge CPU-complete + gpu feature-gate (honest stub)

## WHY
KU03-T3: `engine/src/bridge.rs::VertexBridge` `upload_once()` only increments a counter —
never touches a GPU. The real "unwired organ". But `wgpu` is NOT in the cargo cache
(verified 2026-07-16) → real GPU path unbuildable air-gapped (decart W15 + SWARM-MANIFEST §3).
This blueprint completes the CPU staging path + a HONEST feature-gated gpu stub (no fake-green).

## WHAT (acceptance)
- CPU path: `upload_once` performs a real CPU staging copy (vertex buffer slice → host staging
  vec) so the "upload" is falsifiable headless (1 logical upload, 0 GPU json, 0 GPU calls).
- `feature = "gpu"`: gate exists, pulls `wgpu` ONLY when available. Until wgpu is cached, the
  gate is EMPTY (`gpu = []`) and `VertexBridge::new_gpu` is a `#[cfg(feature="gpu")]` stub
  returning `Err("gpu adapter not built — wgpu uncached")`. Headless `HeadlessGpu` mock satisfies
  the GREEN gate (1 mock upload, 0 json).
- NO `wgpu` added to default deps (offline-clean mandate preserved).

## RED→GREEN
- RED: `cargo build --features gpu` fails (unknown `wgpu` dep) OR `upload_once` is a no-op counter.
- GREEN:
  (a) default `cargo test -p dowiz-engine` → VertexBridge does 1 logical upload, 0 json, 0 GPU.
  (b) `cargo build --features gpu` → compiles (empty gate) + `new_gpu` returns the honest Err.

## FILES (Owns — disjoint, engine crate only)
- Modify: `engine/src/bridge.rs` (real CPU staging + cfg-gated gpu stub + HeadlessGpu mock),
  `engine/Cargo.toml` (`gpu = []` empty feature; comment: enable `wgpu` when cached)
- Test: `engine/src/bridge.rs` tests (upload count + headless 0-json + gpu-Err)

## RISKS
- Feature-flag trap: `new_gpu` + wgpu symbols MUST be `#[cfg(feature="gpu")]`. MAIN re-verifies
  `cargo build --features gpu` AND default build (no wgpu in graph).
- Do NOT add `wgpu` to Cargo.toml (uncached → breaks offline build). Ceiling documented.

## NON-GOALS
- Real GPU raster (that is W21, blocked on network). This is the honest boundary.
