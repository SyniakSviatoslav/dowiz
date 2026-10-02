# BLUEPRINT W20 — VertexBridge CPU-complete + gpu feature-gate (honest stub)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


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
