# BLUEPRINT W21 — Field-UI GPU render loop (FE-04/05/08–17)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion). **The interface engine line (canvas text, shell platform, intent engine, field UI) has no successor row; the surfaces are plain JS + CSS under `workers/api/public/` with the `design`, `ui-adoption` and `sw-shell` gates.**


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
