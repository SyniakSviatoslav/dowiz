//! dowiz-kernel — deterministic core (Rust→WASM).
//! Canonical kernel. The TS app (`/root/dowiz` apps/*) is the legacy oracle; this replaces it.
//! No float on money, no I/O. Verified-by-Math: RED+GREEN tests per module.

/// In-code protocol/wire version for the kernel. Independent of the repo CalVer
/// tag so a breaking kernel change (FSM-graph / `_js` export / ledger-layout) can
/// be gated without a repo retag. Bump on any such change.
pub const KERNEL_PROTO_VERSION: &str = "2026.07.0";

/// OPT-IN post-quantum crypto core (ML-DSA-65 / ML-KEM-768 / X25519 / AES-GCM).
/// KAT-gated byte-exact vs NIST ACVP vectors. Behind `pq` feature so the
/// canonical order/money core stays serde-free. Mesh/transport identity seam.
#[cfg(feature = "pq")]
pub mod pq;

/// Reverse-engineering loop #R3 — absorbing Markov chain closed forms: fundamental matrix
/// N=(I−Q)⁻¹ (exact finite sum for the DAG lifecycle), expected steps-to-terminal, absorption probs.
pub mod absorbing;
pub mod analytics;
/// P11 §7 — CorePinning trait seam (Trait-as-Port): pluggable CPU-core-affinity
/// port with a zero-cost `NoOpCorePinning` default (NUMA crate DECART-deferred).
pub mod arena;
/// C-tier "attention lens": scaled dot-product attention as one learned-affinity
/// diffusion step — same f(L) family as markov PPR / heat-kernel.
pub mod attention;
/// B4 — native content-addressed backup organ: chunk (via `chunker`) → store
/// unique blocks by sha3_256 id → restore byte-identically from a manifest.
/// Dedups across small edits; fail-closed restore. Pure-Rust, no new deps.
pub mod backup;
pub mod bounded_drainer;
/// P11 §1 — compute budget accumulator (degrade-closed, zero-dep) + §4 Modal
/// `JobPort` / `BudgetedJobPort` seam (offline-err default; real adapter deferred).
pub mod budget;
/// RW-07 — cart state machine (consolidate 2 JS cart impls → kernel authority). Totals via money.
pub mod cart;
/// M1/M2 — trusted price catalog: the single kernel authority on line-item prices.
/// `place_order` re-derives `unit_price` from this, ignoring client-supplied prices.
pub mod catalog;
/// P9 growth-substrate: causal inference — back-door adjustment / do-operator
/// (Pearl). Provable causal effect from observational tables; fail-closed.
pub mod causal;
/// P9 growth-substrate: semi-Markovian **causal graph** primitives (directed +
/// bidirected arcs) — the structural backbone of the ID / IDC identification
/// algorithms: ancestors, descendants, c-components, bidirected-aware
/// d-separation, and the `G\X` / `G[V]` subgraph algebra.
pub mod cgraph;
/// B4 — deterministic content-defined chunker (Buzhash) for the native Rust
/// backup organ: content-addressed blocks that dedup across small edits.
pub mod chunker;
pub mod core_pinning;
/// Deterministic CSR graph + synchronous Jacobi personalized-PageRank
/// (retrieval-blueprint v2 diffusion/recall primitive).
pub mod csr;
pub mod domain;
/// P04 product-math: Disjoint-Set Union (union-find) + Kruskal MST — the single
/// canonical DSU/MST primitive. `cgraph::c_components` delegates here; Phase 9
/// mesh-heal + Phase 13 partition-tolerant delivery consume it directly.
pub mod dsu;
/// MESH-06 — per-node content-addressed event-log (local-first + sync).
pub mod event_log;
/// P34 — cross-repo mesh kernel wiring: append-only signed-append log
/// (`MeshLog`) + caller-supplied `HubTransport` trait (config-driven, NO
/// hardcoded endpoint, NO real cross-repo push). Gated behind `pq` because it
/// reuses the kernel's ML-DSA-65 primitive for signing/verification (see
/// `mesh.rs` header for the KAT-gated crypto rationale).
#[cfg(feature = "pq")]
pub mod mesh;
/// RW-06 — geo / route kinematics (pure-logic port from geo-anim.ts + delivery-zone.ts). Kernel authority.
pub mod geo;
/// Harmonic centrality H(v)=Σ 1/d(u,v) — the shared graph-ranking primitive the
/// agent-kernel (HK-05/HK-06) uses for model routing + memory ranking. Ported
/// here so the CANONICAL kernel and the agent-kernel share ONE graph-math
/// vocabulary (unify-the-kernels directive). Parity-gated vs the agent-kernel
/// reference in `harmonic::tests::parity_with_agent_kernel_reference`.
pub mod harmonic;
/// Householder QR + shifted-QR eigensolver (the dense-`n×n` "Ferrari"): all
/// eigenvalues, real + complex, stack-only for n ≤ 32 (no heap; FMA inner
/// product). Replaces the O(n⁴) Faddeev-LeVerrier path as the default for the
/// dense operators this kernel diagonalizes.
pub mod householder;
/// Воля АНУ — the hidden source of the self-evolving living organism. Single
/// kernel-internal entry point for closed-loop self-evolution (G7 source-hiding).
pub mod hydra;
/// C-tier "impedance lens": circuit/impedance as a resource framework — flow
/// reflection coefficient + backpressure gate (ρ<1 with margin, not power-match).
pub mod impedance;
/// BLUEPRINT-E1 — discrete gradient/divergence (oriented-edge incidence) and the
/// CANONICAL reference Laplacian `L = BᵀWB` (+(D−A) convention). The small
/// hand-oracle-tested reference every other Laplacian (dense/CSR/grid-stencil) is
/// parity-bound against; retires the last unpinned mirror at the kernel↔engine seam.
pub mod incidence;
pub mod intake;
pub mod isolation;
pub mod kalman;
/// §3.3 Layer-B (semantic) leakage gate — cosine-0.9 near-duplicate rejection over an injected
/// `&dyn LlmBackend` embedding model. Native, zero-dep; the live bridge lives in `llm-adapters`.
pub mod leak_gate;
/// P08 typed local-observability core — the pure-std, no-network, no-signing
/// HALF: typed-metrics schema + closed `LogEvent` enum (§2/§3) and the
/// claim-latency anomaly detector (§4). F40 ML-DSA signed envelope DEFERRED
/// pending bebop2 C4b — see `metrics.rs` header. Fail-closed local sink.
pub mod metrics;
/// P9 wave: deterministic seedable PRNG (SplitMix64 → PCG64), zero-dep,
/// reproducible Monte-Carlo for the empirical causal joint.
pub mod rng;
/// P11 §6 — `f64x4` struct-of-arrays (SoA) SIMD batch lane: vectorises softmax
/// ACROSS the batch (4 independent rows per step), each lane replaying the exact
/// scalar op order → bit-identical to `softmax_scalar` / `attention::softmax`.
/// AVX2 fast path with a scalar fallback (mirrors `householder.rs` runtime gate).
pub mod simd;
/// W2-7 — event-sourced, tamper-evident hash-chain knowledge spine
/// (Memory/Identity/Intent). Append-only record log; `verify_chain()` re-walks
/// the chain to detect any mutation. Pure-std (reuses `event_log::sha3_256`).
pub mod spine;
/// Spool — pure crash-safe async work-queue state machine (append / claim /
/// ack / reclaim). The I/O + drainer adapter lives outside the kernel
/// (pure-std firewall); this owns the Verified-by-Math transitions. Reused by
/// every async subsystem (reporting, governance, mesh sync).
pub mod spool;
/// E2 — the kernel's single uncertainty primitive: mean SE / normal & Wilson
/// intervals / the relocated CLT convergence envelope / a seeded bootstrap.
/// Zero-dep leaf (sibling of `rng`/`money`/`noether`); every layer depends on it
/// downward so a reported scalar can carry the check that would refute it.
pub mod stats;
// `loops` (BP-20 orchestration card parsing) depends on serde / serde_yaml →
// compiled only under the `wasm` feature so a native rlib build stays serde-free.
// NOT part of the canonical order/money core (decide/order_machine/domain/money).
/// WAVE P40 — bounded, fail-closed AgentLoop executor. Wires through the existing
/// `ports::tool` (`ToolPort`/`SkillRegistry`) + `ports::mcp` (`McpPort`) capability
/// firewall and the `token_bucket` degrade-closed budget. No tool runs without a
/// verified capability; unknown tools rejected; budget exhaustion terminates the loop.
pub mod agent;
/// Deterministic, zero-dependency fault-injection harness (P-H W-H1). The whole
/// module is `#[cfg(any(test, feature = "chaos"))]`; in a release build it
/// compiles to `()`, so no chaos symbol reaches a production artifact. This
/// `mod` line is the structural grep-guard (P24-grep-guard style): its presence
/// asserts the harness is reachably compiled under `cargo test` / `--features chaos`.
#[cfg(any(test, feature = "chaos"))]
pub mod chaos;
/// External capability ports (the seams where the kernel meets the outside world without importing
/// it) — currently the `LlmBackend` pluggable LLM backend trait (zero HTTP/serde; the concrete
/// `llm-adapters` crate implements it).
/// BLUEPRINT-P-F (Layer F) — MoE mesh DecisionUnit family: closed `DomainTag` capability routing
/// (NO-COURIER-SCORING), `DecisionUnit` family type (pure `decide()`, Escalate first-class),
/// FraudAuth escalate-only output, and the Pricing operator-activation money-gate. Kernel-only,
/// zero network/serde. See `decision/mod.rs` header for the firewall + red-line rationale.
pub mod decision;
/// A2 (BLUEPRINT-P-A §3.1) — generated kernel "organs" committed from eqc-rs.
/// Each fn is emitted by `tools/eqc-rs/src/bin/gen_kernel_organs.rs`; verify
/// against the hand-written law with a bit-parity `#[test]`.
pub mod eqc_gen;
/// E1 — verifiable-cognition benchmark generator: metamorphic MR items with
/// kernel-primitive oracles, deterministic mint-log leakage gate, and
/// calibration metrics (ECE/Brier/AURC). Pure-offline, zero-dep.
/// Uses `serde_json` for the `analyze.mjs` JSONL bridge → gated behind `wasm`
/// so the native rlib build stays serde-free. NOT part of the order/money core
/// (decide/order_machine/domain/money).
#[cfg(feature = "wasm")]
pub mod evals;
/// Living-knowledge retrieval — ADAPTER to the (separately-branched) JS engine.
/// serde-dependent (JSON bridge protocol) → gated behind `wasm` to keep the
/// native rlib build serde-free. Not part of the order/money core.
#[cfg(feature = "wasm")]
pub mod living_knowledge;
#[cfg(feature = "wasm")]
pub mod loops;
/// Reverse-engineering loop #R1 — Markov attractor detector (ASCENDed from markov_attractor.py);
/// reuses `spectral` as its eigen-core, killing the dual-authority hazard.
pub mod markov;
/// Contiguous row-major matrix helper — the single backing store / matmul impl
/// the spectral + absorbing subsystems route through (DOD/SIMD prep).
pub mod mat;
/// RW-08 — messenger deep-link builders (pure string logic → kernel authority).
pub mod messenger;
/// Reverse-mode automatic differentiation (scalar tape engine) — the
/// kernel-side fitting primitive (Tier B2: capture-field SIREN/splat fits).
pub mod micrograd;
pub mod money;
/// P9 / C-tier "invariance note": executable Noether check — verify a conserved
/// quantity survives a deterministic update (catches self-improvement drift).
pub mod noether;
/// B3 — deterministic offline-on-node online learner (LinearSGD ridge +
/// ScalarAdam), the self-adaptation substrate (E3). Local-first: no network.
pub mod online;
pub mod order_machine;
pub mod ports;
/// P38 O18a — graphics unlock. Feature-gated GPU render backend (presentation
/// only; the kernel remains the bit-deterministic state authority). Compiles to
/// NOTHING without the `gpu` feature; behind it, a REAL headless wgpu bring-up.
pub mod render;
/// M1 / L0 exact byte+regex search (vectorless) — deterministic trigram
/// inverted index + exact verify. NEW module; does not touch kernel authority.
pub mod retrieval;
/// P04 product-math: CSR-native Dijkstra / A* shortest path + Contraction-
/// Hierarchy shortcuts + OSM road-graph ingestion. Ported from bebop
/// `cost_estimate.rs`, zero-dep. Consumed by Phase 9/13/16/17 route work.
pub mod router;
/// Reverse-engineering loop #1 — general (non-symmetric) spectral engine: eigenvalues
/// (Faddeev-LeVerrier + Durand-Kerner), spectral gap γ, Laplacian Fiedler λ₂, DMD drift class.
pub mod spectral;
/// P11 §2 — content-addressed spectral eigensolve cache (zero-dep); routes
/// `spectral::eigenvalues` through a `&mut` cache with a recomputes-counter
/// falsifier (no thrashing, no stale-cache).
pub mod spectral_cache;
/// WAVE LAP — graph-Laplacian eigenmodes consumer of `spectral::eigh`: the
/// `k` smallest-eigenvalue eigenpairs (Fourier modes / field-UI basis) of a CSR
/// graph. New module; does not touch `spectral_cache`, `csr`, `householder`, or
/// `spectral`'s existing code.
pub mod spectral_laplacian;
/// Self-improvement loop: recurring-pattern surface over the tool-outcome
/// token stream (W19 — consumes `trigram` into the loop's telemetry path).
pub mod telemetry;
/// F33 — deterministic compute-budget token bucket (monotonic-clock, atomic, degrade-closed).
/// The `llm-adapters` `Dispatcher` reuses this to bound LLM-call concurrency.
pub mod token_bucket;
/// Deterministic n-gram (bigram + trigram) frequency extraction over a token
/// stream — the self-improvement loop's pattern-surface primitive (P9 / T2-β).
pub mod trigram;
/// P08 — typed metrics pure core: `/proc/self` sampling (CPU/mem) + a
/// deterministic, serde-free, parse-or-reject text schema for typed metric
/// records. NO egress / signing change; GPU is typed-absent (`Option`) until
/// hardware exists. Pure-std (default build has no serde).
pub mod typed_metrics;
/// C1 — verify-failure → retrieval-trigger: a claim check that, on failure,
/// emits a bounded structured re-verify request (the "verify then learn" loop).
pub mod verify_retrieval;
/// WASM/JS bindings — the only place the kernel touches the browser boundary.
/// Compiled ONLY under the `wasm` feature (see `#![cfg(feature = "wasm")]` in
/// wasm.rs); native rlib builds exclude it and pull no wasm-bindgen/serde.
#[cfg(feature = "wasm")]
pub mod wasm;

/// `json-api` — the JSON string boundary shared by the wasm JS surface AND the
/// native HTTP adapter (P37 `native-spa-server`). Compiled ONLY under the
/// `json-api` feature (and therefore under `wasm`, which enables it); the
/// DEFAULT kernel build stays serde-free. This is the single order JSON
/// authority for both surfaces (BLUEPRINT-P37 W37-1).
#[cfg(feature = "json-api")]
pub mod json_api;

// Re-export the headline types so wasm-bindgen consumers and tests share one surface.
// `evals` (benchmark/JSONL bridge) re-exported only when the `wasm` feature is on.
/// P9 growth-substrate: causal inference — back-door + front-door + instrumental-variable
/// + counterfactual (twin-network) + d-separation oracle + back-door/front-door
/// criterion verifiers (do-operator / Pearl / Wald).
pub use causal::{
    backdoor_adjust, backdoor_criterion, confounded, counterfactual_linear, d_separated,
    empirical_identify, frontdoor_adjust, frontdoor_criterion, identify_causal_effect,
    instrumental_adjust, sample_backdoor, CausalEffect, HedgeWitness, IdFormula, IdResult,
};
pub use csr::{precision_at_k, recall_at_k, Csr};
pub use domain::{apply_event, compute_order_total, place_order, Order, OrderItem};
#[cfg(feature = "wasm")]
pub use evals::{
    aurc, brier, ece, EmaTracker, EvalCheck, EvalRow, MetamorphicGenerator, MintLog, MrItem,
    RegressionGate, SelfAdaptator,
};
pub use money::{
    apply_tax, assert_non_negative, compute_line_total, convert_all_to_eur_cents, to_minor_unit,
};
pub use order_machine::{
    assert_transition, cyclomatic_number, fold_transitions, fsm_graph_report, has_cycle, reachable,
    spectral_radius, topological_order, verify_fsm_signature, verify_fsm_signature_against,
    FsmGraphReport, FsmSignatureDrift, OrderStatus, TransitionError,
};
// The wasm JS entry points are exposed only when the `wasm` feature is on. Re-export
// EVERY `#[wasm_bindgen]` item from `wasm` (not a hand-picked 5) so the cdylib and
// the generated `pkg-web` carry the full surface — a hand-list silently drops exports
// (the bug that left pkg-web missing fieldsim_*/knowledge_map/geo_*/spectral_*).
#[cfg(feature = "wasm")]
pub use wasm::*;

/// **Boot-time FSM drift gate (fail-closed).** Call this once before the event bus accepts
/// traffic — at kernel init, before `apply_event` is ever invoked. It compares the *live*
/// lifecycle graph against `FSM_GOLDEN_SIGNATURE`; `Err(drift)` means the committed lifecycle
/// no longer matches the 2026-07-14 recorded fingerprint. A mismatch ⇒ refuse to start (fail-closed):
/// a bad merge or a silent `allowed_next` edit is caught at the earliest possible point, before
/// any order can be folded through a drifted topology. (Blueprint `spectral-graph-fsm` §4.)
pub fn kernel_boot_verify_fsm() -> Result<(), FsmSignatureDrift> {
    verify_fsm_signature()
}

/// Authoritative fixed-timestep for the field-sim/animation integrator.
///
/// `dowiz-engine` (`engine/src/loop_.rs`) hardcodes the SAME value and its
/// `FixedTimestep` integrator MUST only ever see this dt. This constant is the
/// single source of truth: if you change it here, you MUST change
/// `engine/src/loop_.rs::DT_STABLE` to match, or the integrator and the kernel's
/// stability authority diverge silently. Pinned by [`dt_stable_is_authoritative`].
///
/// 0.02 s == 50 Hz — the cadence at which route-ping kinematics (geo) are sampled.
pub const DT_STABLE: f32 = 0.02;

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE (MESH-01 feature-gate invariant): the DEFAULT kernel build is
    // `std`-only (no `wasm`), so a native rlib consumer pulls NONE of
    // wasm-bindgen / serde / serde_json / serde_yaml. This is enforced by the
    // `[features]` table (the `wasm` feature is OPT-IN) and verified out-of-band
    // via `cargo build --no-default-features --features std` + `cargo tree -p
    // bebop-delivery-domain -e no-dev` (no wasm-bindgen / serde in the graph).
    // We deliberately do NOT assert `!cfg!(feature = "wasm")` here: that would
    // false-fail when the suite is legitimately run with `--features wasm`
    // (the wasm JS surface is then correctly compiled in). The gate's
    // correctness is the *absence* of these crates in the DEFAULT dependency graph.

    #[test]
    fn dt_stable_is_authoritative() {
        // Fail-closed pin: the engine's FixedTimestep and any kernel-side
        // stability math depend on this exact value. Never "round" it or the
        // integrator desyncs from the kernel's sampling cadence.
        assert_eq!(DT_STABLE, 0.02);
        // 50 Hz cadence — the contract the field-sim hook relies on.
        assert_eq!((1.0 / DT_STABLE as f64).round() as u32, 50);
    }
}

/// Install a `tracing-subscriber` with `RUST_LOG` env-filter.
/// Dev/CLI only — never called from the wasm cdylib (no stdio there).
#[cfg(not(target_arch = "wasm32"))]
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
