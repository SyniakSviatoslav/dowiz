# BLUEPRINT — dowiz workspace split (user-space crates for parallel compilation)

Date: 2026-08-17
Author: Sviatoslav Syniak
Status: PROPOSED — ready for swarm execution
Goal: split the monolithic `dowiz-core` (~290 modules) into ~18 leaf no_std crates
and add a root Cargo workspace, so `cargo build`/`cargo test` compile in parallel
across cores and incremental builds only recompile the changed leaf.

## 1. Problem (measured)

- `dowiz-core` is ONE crate (`crates/dowiz-core/`, ~290 `pub mod` in `lib.rs`).
- There is NO root `Cargo.toml` → no workspace → cargo does not parallelize across
  crates; every `dowiz-core` source change recompiles the whole crate.
- Measured: cold `cargo build` ~90s; cold `cargo test` ~2m47s (incl. dev-deps
  serde_json/proptest/paste/eqc-rs); warm incremental still ~30-60s per dowiz-core
  edit (whole-crate recompile).
- Downstream crates (kernel, engine, node, agent-loop, …) each depend on the whole
  `dowiz-core`, so they also recompile/relink after any core change.

## 2. Target architecture

Root `Cargo.toml` with `[workspace] members = [...]` listing every crate, and a
split of `dowiz-core` into ~18 dependency-ordered leaf crates. `dowiz-core` becomes
a thin re-export facade (`pub use dowiz_math::*; …`) so `use dowiz_core::…` keeps
working unchanged while the real code lives in the leaves (progressive, low-risk).

Leaf crates (each no_std + alloc, zero external deps, own Cargo.toml + lib.rs):

| # | crate | modules (domains) | depends on |
|---|-------|-------------------|------------|
| 0 | dowiz-constants | constants, lut, rng, splitmix, sanitize, hex_util, tri_state, dsu | — |
| 1 | dowiz-math | math, complex, trig, spherical, modular, fft, ntt, harmonic, householder, eigen, mat | constants |
| 2 | dowiz-squash | squash, delta, csr, span, span_metrics | constants |
| 3 | dowiz-hyper | hypervector, hypervector_index, spectral, spectral_graph, spectral_laplacian, parametric_spectral, resonance | math, squash |
| 4 | dowiz-retrieval | retrieval/*, bm25, trigram, needle2, stem, semantic, memory_search, verify_retrieval, readability | hyper, squash |
| 5 | dowiz-memory | living_memory, living_knowledge, code_graph, context_pruner, reconstruction_memory, academy_store, snapshot, event_log | hyper, retrieval |
| 6 | dowiz-quantum | quantum, qstate, oracle, inference/*, predict, causal, trinary | math |
| 7 | dowiz-crypto | pq/*, crypto_signer, sha256_hw, checksum, ct_gate | math, squash |
| 8 | dowiz-agent | agent/*, agent_orchestrator, agent_browser, dynamic_actions, dynamic_spawner, skill_extractor, prompt_enrich, academia*, catalog, domain | memory, retrieval |
| 9 | dowiz-ml | micrograd, gboost, tensor, tensor_parser, predictor, kalman, markov, crystal, attention, fractal | math |
| 10 | dowiz-mesh | mesh, mesh_oracle, mesh_replication, p2p_delivery, cooperation_protocol, gossip, channel | hyper, memory |
| 11 | dowiz-os | pid, scheduler, spinlock, kthread, core_pinning, clock, time_stabilizer, clock_stabilizer, autonomic, autonomic_pmu, temporal_tmr | constants |
| 12 | dowiz-io | json, json_api, json_bridge, vfs, spool, channel, telemetry, telemetry_aggregator, telemetry_harvest, event_log, metrics | constants |
| 13 | dowiz-pq-extras | laplacian_eqc_parity, eqc_gen, self_harness, determinism, self_heal, self_reproduce, breaker, resilience, chaos | crypto, os |
| 14 | dowiz-finance | money, wallet, budget, entropy_budget, token_bucket, trading_*, power_forecast, foodcourt, storefront | io |
| 15 | dowiz-graph | cgraph, code_oracle, visual_index, glyph_dashboard, spine, ktg2, landmark, optical, incidence | hyper, ml |
| 16 | dowiz-geo | geo, oil_motion, field_eigenmodes, noether, impedance, numbat, absorbing, physics, simd, neon | math |
| 17 | dowiz-core (facade) | re-exports all leaves | 0..16 |

(Groupings are a starting proposal; the audit in Phase 0 finalizes boundaries so
each leaf's internal `crate::` graph is a DAG and no cycle spans two leaves.)

## 3. Execution plan (swarm)

Phase 0 — Audit + workspace (0.5 day)
- Generate the module dependency DAG (script: scan `use crate::X` / `crate::X::`
  per file → adjacency). Emit `docs/design/WORKSPACE-SPLIT-DAG-2026-08-17.md`.
- Add root `Cargo.toml` `[workspace]` with all members; verify `cargo build` still
  green before any split (workspace is a no-op superset first).

Phase 1 — Split leaves (1-2 days, 12 parallel agents, disjoint-file rule)
- Each leaf = one agent: create `crates/dowiz-<leaf>/Cargo.toml` + `lib.rs`,
  `git mv` its `.rs` files, rewrite `crate::X` → `dowiz_<leaf>::X` for cross-leaf
  imports only (intra-leaf `crate::` stays), fix `use super::`/`mod` declarations.
- Agents touch ONLY their own leaf dir + its Cargo.toml; the facade and the root
  workspace are edited by the orchestrator only (avoid merge conflicts).

Phase 2 — Re-wire downstream (0.5-1 day)
- Make `dowiz-core` facade re-export leaves so `use dowiz_core::…` is unchanged.
- Switch kernel/engine/node/etc. to depend on leaves directly (optional, phase 2b).

Phase 3 — Invariants + tests (1 day)
- Per leaf: `#![no_std]` + zero-dep proof (`cargo tree -e no-dev`), run its tests
  (`cargo test -p dowiz_<leaf>`), fix `crate::math::`-style cross-crate calls.
- Preserve the binding rule: every new fn/file reuses established patterns, no_std.

Phase 4 — CI replacement (0.5 day)
- Rewrite `.github/workflows/ci.yml` to build/test the workspace in parallel
  (matrix per leaf, `cargo build --workspace`, `cargo test --workspace`).

## 4. Time estimate

| phase | single-threaded | 12-agent swarm |
|-------|-----------------|----------------|
| 0 audit+workspace | 1 day | 0.5 day |
| 1 split (~290 files) | 4-6 days | 1-2 days |
| 2 re-wire downstream | 2 days | 0.5-1 day |
| 3 invariants+tests | 2-3 days | 1 day |
| 4 CI | 0.5 day | 0.5 day |
| **total** | **~10-12 days** | **~3.5-5 days** |

## 5. Expected speedup (honest)

- Cold full build: ~90s → ~15-30s. **~3-5x** (bounded by the deepest dependency
  chain — leaves at the same depth compile in parallel across cores).
- Incremental (edit 1 module): ~30-60s (whole core recompiles + downstream relink)
  → ~5-15s (recompile 1 small leaf + relink). **~4-10x**.
- `cargo test` targeted: ~90s cold → ~20-40s cold (only the leaf + its deps).
- NOT 100x. Compilation "100x" is not achievable for a ~300-module Rust codebase;
  the parallelism ceiling is the dependency DAG's critical path + core count.
  The 100x target is already met for the *runtime query* path (in-memory `serve`),
  not for compilation.

## 6. Risks / mitigations

- Cross-leaf `crate::` cycles → Phase 0 DAG audit rejects any cycle before moving.
- no_std/zero-dep breakage → per-leaf `#![no_std]` + `cargo tree` gate in Phase 3.
- Merge conflicts → disjoint-file rule (agents own their leaf dir; orchestrator owns
  facade + workspace + CI).
- Facade ambiguity (name collisions across leaves) → Phase 0 flags collisions;
  facade re-exports are explicit.
