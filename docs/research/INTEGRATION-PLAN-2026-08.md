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
- [x] `quantum.rs` — Qubit/Bloch/Pauli/Hadamard/S/T/RX/RY/RZ, measurement, fidelity, CNOT+Bell (11 tests, `b5544be`).
- [x] `krylov.rs` — CG, GMRES (Arnoldi+Givens), Arnoldi, Lanczos (7 tests, `f82d1f2`).
- [x] Hermes token-optimization phase 0: memory compacted 96%→78%, `proactive_prune_tokens` 0→32000, memory limits 2200/1375 → 1800/1200.
- [x] `QTri` — quantum tri-state (qutrit |ψ⟩=a|T⟩+b|F⟩+c|U⟩) generalizing TriState, 5 tests (`0a7f8a9`); re-exported at crate roots (`ed08c1b`).

## Quantum-state-everywhere migration (user directive)
Replace classical `TriState` (273 usages) with `QTri` superposition where partial
information adds value. **Not a mechanical find-replace**: `QTri` is f64-based
(`PartialEq` only — no `Eq`/`Hash`), so hash-map keys / Eq-comparisons must stay
`TriState`. Strategy: QTri = storage/uncertainty representation; `TriState` =
collapsed boundary + hash key. Migrate module-by-module (swarm, disjoint files).

## Next concrete step
Swarm-migrate high-value TriState → QTri sites (measurement/confidence/drift),
keeping fail-closed boundaries on TriState; then `quantize.rs` (turbovec).
