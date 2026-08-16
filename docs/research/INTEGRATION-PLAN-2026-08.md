# Integration Plan — AI Tools Compendium → dowiz (2026-08-16)

Source: `docs/research/AI-TOOLS-COMPENDIUM-2026-08.md` (1006 lines, pasted prompt).
Goal: analyze/research everything in the compendium, improve existing systems, add new
ones, reverse-engineer & integrate into dowiz kernel (Rust, maximally `#![no_std]` core).

## Priority (fixed by user)
1. **Token-usage optimization — FIRST Hermes, THEN the rest.**
2. Then: quantum state + Krylov algorithms (user-named examples), then all other integrations.

## Phase 0 — Hermes token optimization (IN PROGRESS)
Reduce per-turn token cost of the Hermes agent itself before touching dowiz.
- [x] Compact MEMORY.md / USER.md (injected every turn).
- [ ] Consolidate/prune skills (fewer, leaner descriptions).
- [ ] Tune hermes config (context window / compaction aggressiveness).

## Phase 1 — New `dowiz-core` modules (no_std, reuse existing math/patterns)
| Module | Source concept (compendium) | Reuses existing |
|--------|------------------------------|-----------------|
| `quantum.rs` | Qubit state \|ψ⟩=α\|0⟩+β\|1⟩, Bloch sphere, Pauli/Hadamard/CNOT, superposition, measurement collapse (IV.6–8) | `crate::math` (complex, sqrt/exp), `crate::math::vec` |
| `krylov.rs` | Krylov subspaces K_m(A,r0), Arnoldi→Hessenberg, Lanczos→tridiag, CG/GMRES/MINRES/BiCGSTAB, preconditioning (IV.4) | `csr` (sparse mat-vec), `spectral` (power/rayleigh), `eigen` |
| `quantize.rs` | turbovec-style product quantization (31GB→4GB), k-means, codebook | `hypervector`, `math::vec`, `squash` |
| `graph_engine.rs` | typed-edge knowledge graph (KAG), hypergraph already exists | `hypergraph`, `csr` |

Integration seam rule (binding): every new file/function reuses established best
patterns/algorithms, is `no_std` wherever possible, and KAT-gates any crypto.

## Phase 2 — Reverse-engineer & integrate the ~90 repos/tools
Grouped: retrieval (KAG/RAG/hybrid), agents (multi-agent orchestration), evals (benchmarks),
quantum (Cirq/Qiskit→native), graph (typed edges), memory (temporal/long-term).
Each repo → map to an existing dowiz module or a new `crates/dowiz-core/src/*.rs`.

## Done in this session
- [x] Lint cleanup: dowiz-core dead-code removal + `#[cfg(test)]` gating + check-cfg (commit `9c6a52b`).
- [x] Persisted compendium + this plan to repo (`1e55259`).
- [x] `quantum.rs` — Qubit/Bloch/Pauli/Hadamard/S/T/RX/RY/RZ, measurement, fidelity, CNOT+Bell (`b5544be`).
- [x] `krylov.rs` — CG, GMRES (Arnoldi+Givens), Arnoldi, Lanczos (`f82d1f2`).
- [x] Hermes token-optimization phase 0: memory compacted 96%→78%, `proactive_prune_tokens` 0→32000, memory limits 2200/1375 → 1800/1200.
- [x] `QTri` — quantum tri-state (qutrit) generalizing TriState (`0a7f8a9`); re-exported (`ed08c1b`).
- [x] `QState` — N-level superposition + Grover oracle prediction (`cc0e481`).
- [x] `context_pruner.rs` — native headroom port (line-importance pruning) (`6bab945`).
- [x] `.agents/rules/ponytail.md` — lazy-senior-dev ladder, always_on (`6bab945`).
- [x] `code_graph.rs` — native graphify port (queryable knowledge graph) (`1a1dacb`).
- [x] Test speedup: `[profile.test] opt-level=3` → full suite 25s→2.77s; release-only `debug_assert!` contract fix (`3ffdde1`).

## Living-memory graph (priority #1, in progress)
Unified no_std layer: `code_graph` (nav) + `context_pruner` (token saving) +
`hypervector` (vector nav) + `pixel_snapshot` (compact viz) + `QState`/Grover
(prediction of consequences/resources/time) + command registry w/ alternatives
+ PID parallel concurrency. Add mempalace best practices + all memory types
(episodic/semantic/long/short/working). Replaces grep; used by Hermes + all agents.
