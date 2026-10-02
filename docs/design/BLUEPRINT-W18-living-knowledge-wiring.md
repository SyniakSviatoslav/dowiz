# BLUEPRINT W18 — living-knowledge Rust wiring

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


## WHY
`kernel/src/living_knowledge.rs` exists (4 pub items: eval/search/recall adapter) but
ORGANISM-STATUS 07-15: the JS living-knowledge engine was purged; the Rust adapter is NOT
consumed by the self-improvement loop. Recall@k must run through the Rust path (no JS).

## WHAT (acceptance)
- `LivingKnowledge` trait/adapter wired into `retrieval/mod.rs` as a PRIMARY recall source
  (mirrors `wire living-memory as PRIMARY retrieval` from 07-13 commits).
- `recall_at_k(query, k) -> Vec<(doc_id, score)>` deterministic, no float nondeterminism.
- Surface a `kalman`/`trigram`-fed pattern signal back into the loop (see W19).

## RED→GREEN
- RED: `living_knowledge` module compiles but no caller invokes recall (grep: 0 usages outside tests).
- GREEN: a `retrieval/mod.rs` integration test calls `recall_at_k` and asserts top-k matches a
  known fixture (recall@5=1.000 on the 324-file corpus, deterministic). 0 JS.

## FILES (Owns — disjoint)
- Modify: `kernel/src/retrieval/mod.rs` (register adapter), `kernel/src/living_knowledge.rs` (expose recall API)
- Test: `kernel/src/retrieval/tests.rs` (new recall integration test)

## RISKS
- Corpus path: Rust adapter needs a corpus source. Reuse `retrieval/fixtures.rs` or an in-repo
  indexed corpus; do NOT shell out to deleted JS. If no corpus on disk, build from `retrieval/index.rs`.
- Keep it std-only (M4: native store default, pgrust opt-in).

## NON-GOALS
- No ONNX/JS spike (that was the purged A2 branch). Pure Rust deterministic recall.
