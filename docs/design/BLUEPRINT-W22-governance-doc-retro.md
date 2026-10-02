# BLUEPRINT W22 — Governance/doc finalize + DOD retro

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


## WHY
After W17-21 verify, close the loop: document the governance state truthfully and produce a
DOD retro with literal 0-failed proof for every wave.

## WHAT (acceptance)
- `docs/design/` note: governance hooks are DELIBERATELY suspended per operator directive
  2026-07-15 (CLAUDE.md: "Mandatory Proof Rule / Ship Discipline / Self-improvement loop —
  SUSPENDED"). NOT a regression; do NOT restore without explicit operator word.
- DOD retro: append to SWARM-MANIFEST a "VERIFIED" section citing the literal `cargo test`
  count per wave (kernel/engine/web), each RED→GREEN gate name, and the commit SHA.
- Update `.specify/tasks.md` KU03-T* to DONE/STALE where covered by W17-20.

## RED→GREEN
- RED: no DOD retro; governance state ambiguous (looks like a bug).
- GREEN: doc states suspended-by-directive; retro has 0-failed per wave; tasks.md updated.

## FILES (Owns — docs only, disjoint)
- Modify: `docs/design/SWARM-MANIFEST-2026-07-16.md` (VERIFIED section), `.specify/tasks.md`
- Create: `docs/design/GOVERNANCE-SUSPENDED-2026-07-15.md` (truth note)

## RISKS
- Do NOT re-enable hooks (that would contradict the operator directive). Docs only.
