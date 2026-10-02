# BLUEPRINT W19 — Kalman/trigram integration into decide/loop

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


## WHY
`kernel/src/kalman.rs` (456L, full n-D predict+update, T2-α) and `kernel/src/trigram.rs`
(138L, bigram+trigram, T2-β) exist but are STRANDED (ORGANISM-STATUS: "11/11 organs
stranded" — math present, not consumed). The decide/loop must actually use them: Kalman for
state estimation in the order/trust fold; trigram for recurring-pattern surfacing in the
self-improvement loop.

## WHAT (acceptance)
- `decide`/`fold` Law path calls `kalman::KalmanFilter::predict`+`update` for a courier/trust
  state estimate (extends `geo::ema_next` scalar steady-state — documented in kalman.rs header).
- Self-improvement loop calls `trigram::count` over its tool-outcome token stream and surfaces
  top-k recurring triples (deterministic ranking, lex tie-break).
- Both wired fail-closed: missing observation → Kalman holds prior; empty token stream → 0 trigrams.

## RED→GREEN
- RED: `decide`/`loop` never references `kalman`/`trigram` (grep: 0 usages in engine/telemetry).
- GREEN:
  (a) kernel test: a `decide` step with a noisy observation yields a Kalman-filtered estimate
      closer to truth than the raw observation (variance-reduction gate, mirrors bebop2 BP-21).
  (b) loop test: a known token sequence returns the expected top-1 trigram (deterministic).

## FILES (Owns — disjoint from W17/W18)
- Modify: `kernel/src/lib.rs` (wire modules into a `cortex` facade if absent),
  `kernel/src/decide.rs` or `domain.rs` (Kalman call), `telemetry/*.rs` (trigram call)
- Test: `kernel/src/tests.rs` (Kalman-in-decide) + `kernel/src/trigram.rs` existing 4 tests extended

## RISKS
- Don't re-implement geo/ema in the loop — consume kernel `kalman` (it generalises ema_next).
- Trigram must stay zero-dep (std HashMap only, per trigram.rs header).

## NON-GOALS
- No ML/N-gram model training. Deterministic counting only.
