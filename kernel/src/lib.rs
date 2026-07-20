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
pub mod blocklist;
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
/// ITEM 32 (Part A, acceptance #2): the eqc-emitted Laplacian as a THIRD parity
/// leg against `spectral::laplacian` (dense) + `Csr::laplacian_spmv`. Gated to
/// `#[cfg(test)]` so the SHIPPING lib build carries NO `eqc_rs` symbols / dep
/// (eqc-rs is a DEV-dependency only — keeps `cargo tree -e no-dev` clean).
#[cfg(test)]
pub mod laplacian_eqc_parity;
pub mod domain;
/// P04 product-math: Disjoint-Set Union (union-find) + Kruskal MST — the single
/// canonical DSU/MST primitive. `cgraph::c_components` delegates here; Phase 9
/// mesh-heal + Phase 13 partition-tolerant delivery consume it directly.
pub mod dsu;
/// MESH-06 — per-node content-addressed event-log (local-first + sync).
pub mod event_log;
/// `fdr` — the kernel's flight-data recorder: hand-rolled logger + durable post-mortem
/// ring (roadmap items 4+29). The terminal state of the `tracing`/`tracing-subscriber`
/// retirement. Compiled unconditionally (the hot-path spans in `domain`/`order_machine`
/// bind to `fdr::info_span!`); its Instant/SystemTime stamps are gated off `wasm32`.
pub mod fdr;
/// BLUEPRINT-P72 — food-court N-leg checkout spine. Composes P60 `run_nleg_saga` /
/// `NLegPlan` / `VendorLeg` / `RefundRequest` with P62 vendor-partitioned
/// `charge_legs` / `kitchen_tickets`. Pure Rust, no DOM, no float on money.
pub mod foodcourt;
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
/// BLUEPRINT-P67 — hub provisioning & claim: provider-agnostic (generic over `TunnelProvider` +
/// `VpsProvider`), in-module mock adapters + Wave-0 real adapters behind `p67-adapters`. Reuses
/// P59 `capability_cert` + P70 `owner_surface`. No card data, no network endpoint in the default
/// build (grep-gate `no_endpoint_dependency`).
pub mod hub_provisioning;
/// BLUEPRINT-P68 — hub supervisor: update + backup. A/B-slot atomic-flip auto-update with a
/// real-code-path health gate, owner-triggered rollback, mandatory age-snapshot-before-promote,
/// and a sovereign encrypted backup envelope (X25519 → SHAKE256 → AES-256-GCM STREAM). Gated
/// behind `pq` because the envelope genuinely needs AES-256-GCM + X25519.
#[cfg(feature = "pq")]
pub mod hub_supervisor;
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
/// Item 31 §4 — hand-rolled, always-compiled JSON parse+serialize primitive (pure `std`). The
/// parse-side home for the serde carriers being cut over (agent-facade, skillspector-rs).
/// Separate from `fdr::json` (serialize-only, fixed-schema). `serde_json` is retained only as a
/// dev-dependency differential oracle (`tests/json_oracle.rs`), outside the zero-dep proof surface.
pub mod json;
pub mod kalman;
/// Item 7 (space-grade roadmap §C): planted-fault self-test for the kani-gate.
/// Compiled ONLY under `cfg(kani)` — zero footprint in every normal build.
#[cfg(kani)]
mod kani_selftest;
/// Item 52 (space-grade roadmap §J): planted-fault self-test for the `miri-gate`.
/// Compiled under `cfg(test)` (so a plain `cargo test` proves it compiles and runs
/// the no-op native branch — zero footprint in any non-test/shipping build) AND
/// under `cfg(miri)` (where the planted-UB branch executes and Miri must flag it).
#[cfg(any(test, miri))]
mod miri_selftest;
pub mod landing;
/// §3.3 Layer-B (semantic) leakage gate — cosine-0.9 near-duplicate rejection over an injected
/// `&dyn LlmBackend` embedding model. Native, zero-dep; the live bridge lives in `llm-adapters`.
pub mod leak_gate;
/// P34 — cross-repo mesh kernel wiring: append-only signed-append log
/// (`MeshLog`) + caller-supplied `HubTransport` trait (config-driven, NO
/// hardcoded endpoint, NO real cross-repo push). Gated behind `pq` because it
/// reuses the kernel's ML-DSA-65 primitive for signing/verification (see
/// `mesh.rs` header for the KAT-gated crypto rationale).
#[cfg(feature = "pq")]
pub mod mesh;
/// P08 typed local-observability core — the pure-std, no-network, no-signing
/// HALF: typed-metrics schema + closed `LogEvent` enum (§2/§3) and the
/// claim-latency anomaly detector (§4). F40 ML-DSA signed envelope DEFERRED
/// pending bebop2 C4b — see `metrics.rs` header. Fail-closed local sink.
pub mod metrics;
pub mod moderation;
/// P9 wave: deterministic seedable PRNG (SplitMix64 → PCG64), zero-dep,
/// reproducible Monte-Carlo for the empirical causal joint.
pub mod rng;
/// P11 §6 — `f64x4` struct-of-arrays (SoA) SIMD batch lane: vectorises softmax
/// ACROSS the batch (4 independent rows per step), each lane replaying the exact
/// scalar op order → bit-identical to `softmax_scalar` / `attention::softmax`.
/// AVX2 fast path with a scalar fallback (mirrors `householder.rs` runtime gate).
pub mod simd;
/// OPT-IN generational-index slot arena — per-element sibling to `arena::BumpArena`.
/// Thin dowiz wrapper over `thunderdome::Arena`: stable `Copy` handles whose stale
/// (removed-then-recycled) form is a safe `None` (ABA / stale-index unrepresentable).
/// Behind `slot-arena` so the default build pulls zero extra crates. Forward-looking
/// infra landed per operator override of the deep-dive's "no adoption yet" verdict
/// (docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md).
#[cfg(feature = "slot-arena")]
pub mod slot_arena;
/// BLUEPRINT-P83 — kernel production observability (SYNTHESIS PERFORMANCE AUDIT 2026-07-18
/// §3.3-C4). Feature-gated (`telemetry`) so the SHIPPING binary carries zero observability
/// symbols and is behavior-/perf-neutral. Two layers: (1) a ZERO NEW DEP `SpanMetricsObserver`
/// (a kernel-owned `fdr::SpanObserver` — the retired `tracing_subscriber::Layer` hook's
/// replacement) that consumes the spans over the 8 verified hot functions and writes
/// log-bucket latency histograms to `metric.jsonl`; (2) the `load1/nproc >= 4` breach branch
/// → system-wide `perf record -a -g -F 99` (+ `alert.jsonl`), with `pprof` a feature-gated
/// no-op fallback. Never called from the core decide/fold/money path; the kernel only EMITS
/// spans, which are inert without an observer installed via `span_metrics::init`.
#[cfg(feature = "telemetry")]
pub mod span_metrics;
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
/// BLUEPRINT-P66 — offline data wallet + single-writer LWW drafts + Signal-style QR transfer
/// (self-custody, no dowiz account, query-before-replay reconnect). Pure client-side logic;
/// `transfer` reuses the `pq` crypto primitives (x25519 / shake256 / aes-gcm) gated under `pq`.
/// Structural grep-gates: `no_card_data_in_wallet` + `no_break_glass_in_wallet` (§4.1 / §4.7).
pub mod wallet;
// `loops` (BP-20 orchestration card parsing) depends on serde / serde_yaml →
// compiled only under the `wasm` feature so a native rlib build stays serde-free.
// NOT part of the canonical order/money core (decide/order_machine/domain/money).
/// WAVE P40 — bounded, fail-closed AgentLoop executor. Wires through the existing
/// `ports::tool` (`ToolPort`/`SkillRegistry`) + `ports::mcp` (`McpPort`) capability
/// firewall and the `token_bucket` degrade-closed budget. No tool runs without a
/// verified capability; unknown tools rejected; budget exhaustion terminates the loop.
pub mod agent;
/// External capability ports (the seams where the kernel meets the outside world without importing
/// it) — currently the `LlmBackend` pluggable LLM backend trait (zero HTTP/serde; the concrete
/// `llm-adapters` crate implements it).
/// BLUEPRINT-P59 — capability-cert chain & crypto-agility: a biscuit-style, hybrid-signed
/// (`Ed25519 ⊕ ML-DSA-65` via the `SignatureVerifier` seam), algorithm-agile cert chain —
/// self-signed hub/owner roots, owner→hub single-hop delegation, suite negotiation +
/// downgrade binding, overlap rotation, and owner-signed gossip-able revocation blobs.
/// Default-built (rides the default `RefSigner` seam so it verifies under `RequireBoth` even
/// without the `pq` feature; production injects real bebop2 crypto at the seam).
pub mod capability_cert;
/// Deterministic, zero-dependency fault-injection harness (P-H W-H1). The whole
/// module is `#[cfg(any(test, feature = "chaos"))]`; in a release build it
/// compiles to `()`, so no chaos symbol reaches a production artifact. This
/// `mod` line is the structural grep-guard (P24-grep-guard style): its presence
/// asserts the harness is reachably compiled under `cargo test` / `--features chaos`.
#[cfg(any(test, feature = "chaos"))]
pub mod chaos;
/// `ct_gate` — the zero-dep dudect-style constant-time gate (roadmap item 6). A Welch t-test over
/// interleaved timing samples with a **planted-leak self-test** (SYNTHESIS §4 item 2 / §10-P7): a
/// deliberately variable-time comparator must be rejected by the same machinery that accepts the
/// constant-time `ct_eq`, or the gate is RED. `#[cfg(any(test, feature = "ct-gate"))]` — the whole
/// timing harness compiles to nothing in a shipping build ("CI-time harness, not linked"). Run by
/// `scripts/hardening-gate.sh` step E in release: `cargo test --release ... ct_gate -- --ignored`.
#[cfg(any(test, feature = "ct-gate"))]
pub mod ct_gate;
/// BLUEPRINT-P-F (Layer F) — MoE mesh DecisionUnit family: closed `DomainTag` capability routing
/// (NO-COURIER-SCORING), `DecisionUnit` family type (pure `decide()`, Escalate first-class),
/// FraudAuth escalate-only output, and the Pricing operator-activation money-gate. Kernel-only,
/// zero network/serde. See `decision/mod.rs` header for the firewall + red-line rationale.
pub mod decision;
/// Reverse-engineering loop #R1 — Markov attractor detector (ASCENDed from markov_attractor.py);
/// reuses `spectral` as its eigen-core, killing the dual-authority hazard.
/// Item 46 — float-determinism containment goldens (ADR-046: pin-under-golden).
/// Pins the exact IEEE-754 bit pattern of every in-plane transcendental float
/// site under the pinned toolchain; a libm ULP drift turns the always-on
/// `cargo test` suite RED. See `docs/audits/determinism/FLOAT-SITES-2026-07-19.md`.
pub mod determinism;
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
/// P89 — field eigenmodes via the kernel's existing spectral infrastructure
/// (`spectral.rs` / `spectral_laplacian.rs` consumed, never modified). Builds the
/// field modal basis from the graph Laplacian eigen-decomposition and reconciles
/// the sign/domain with the field stencil's `−(D−A)` operator. The 3-path
/// (modal / DCT / stencil) verdict bench lives in `kernel/benches/criterion.rs`
/// under the `field_eigen/` group and is reported in `docs/p89-verdict.md`.
pub mod field_eigenmodes;
/// Living-knowledge retrieval — ADAPTER to the (separately-branched) JS engine.
/// serde-dependent (JSON bridge protocol) → gated behind `wasm` to keep the
/// native rlib build serde-free. Not part of the order/money core.
#[cfg(feature = "wasm")]
pub mod living_knowledge;
#[cfg(feature = "wasm")]
pub mod loops;
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
/// P40 `ToolResource::WebFetch` — native, pure-`std` readable-text extraction
/// from raw HTML (the fetch itself stays in `agent-facade`, this crate remains
/// network-free). Reimplements the core Mozilla-Readability mechanism natively;
/// explicitly NOT a browser — zero JS execution, that stays an external tool.
pub mod readability;
/// P38 O18a — graphics unlock. Feature-gated GPU render backend (presentation
/// only; the kernel remains the bit-deterministic state authority). Compiles to
/// NOTHING without the `gpu` feature; behind it, a REAL headless wgpu bring-up.
pub mod render;
/// M1 / L0 exact byte+pattern search (vectorless) — deterministic trigram
/// inverted index + exact verify over a restricted {literal, `.`, `.*`} wildcard
/// subset (kernel-owned matcher; `regex` retired, item 5). Does not touch kernel
/// authority.
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
/// P62 / M1 — the intra-hub vendor partition identity (`VendorId`). The fan-out
/// key for `catalog::validate_tree` / `domain::charge_legs` / `domain::kitchen_tickets`.
pub mod vendor;
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

/// `storefront` — P69 customer storefront & checkout journey state machine (BLUEPRINT-P69).
/// Pure kernel logic (no serde / no wasm-bindgen); always compiled so the FSM is testable and
/// reusable by the `json-api` bot pack and the native SPA server alike.
pub mod storefront;

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
/// Item 9 (space-grade roadmap §D "THE PIVOT") — the deterministic
/// fault-containment circuit breaker. The `Result<Permit, Tripped>` gate is the
/// single admission decision; alarm receiver `Breaker::on_commit_error` consumes
/// `CommitError::Store`. Golden-signature-pinned cyclic FSM, shares the Tier-1 FDR
/// ring for audit, zero new dependencies (pure `std`).
pub mod breaker;
/// Item 21 (space-grade roadmap §E): the deterministic bounded gain-scheduling
/// layer. Subscribes to `markov::Verdict`/`spectral::DriftClass`, holds an explicit
/// {classified-state → bounded adjustment} law table (the pilot: `token_bucket`
/// refill rate), makes out-of-bound rates unconstructible, and logs every
/// adjustment to the Tier-1 FDR as a first-class `Tuning` event. Composes with
/// (never replaces) the item-9 breaker for extreme-end responses.
pub mod autonomic;
/// Item 27 (space-grade roadmap §E, response half) — PMU-**informed** response
/// routing. Wiring-only: routes a PMU-informed `(DriftClass, Verdict)` through
/// item 21's bounded-control-law path and item 9's breaker seam. The PMU signal
/// enters ONLY as a quantized [`crate::autonomic_pmu::PmuBand`] (P6 determinism
/// guard) — never a raw counter in the control-law arithmetic. Diagnostic-grade;
/// no CI gate keys on any PMU value.
pub mod autonomic_pmu;
/// Item 12 (space-grade roadmap §E) — temporal triple-modular-redundancy (SIHFT)
/// pilot. Re-runs 2–3 named µs-scale pure functions and votes with a trivial `==`;
/// a non-unanimous outcome trips the item-9 breaker (`TripCause::VoteMismatch`) +
/// writes a Tier-1 FDR `Alarm`. **PARTIAL** (catches transient compute flips only;
/// no SEU-immunity claim — see the module's honest-limits doc). Zero new deps.
pub mod temporal_tmr;
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

/// Install the kernel FDR sink (roadmap items 4+29 — replaces the retired
/// `tracing-subscriber` `init_tracing`). Level is read from `DOWIZ_LOG` (level-only grammar
/// `error|warn|info|debug|trace`, default `info` — mirrors the old `EnvFilter::new("info")`
/// fallback). Dev/CLI only — never called from the wasm cdylib (no stdio there); had ZERO
/// production callers under `tracing` and keeps that shape.
///
/// Name kept as `init_tracing` for source-compat with the one test that calls it; the body
/// no longer touches `tracing`.
#[cfg(not(target_arch = "wasm32"))]
pub fn init_tracing() {
    // ── P83 Layer-1 auto-init ──────────────────────────────────────────────────
    // When `DOWIZ_SPAN_METRICS=1`, install the zero-dep `SpanMetricsObserver` (via the
    // kernel-owned `fdr` observer) so the 8 verified spans stream to `metric.jsonl`. The
    // branch is `#[cfg(feature = "telemetry")]`, so the DEFAULT and wasm builds never see
    // it (D2: the production cdylib is observability-silent).
    #[cfg(feature = "telemetry")]
    {
        if std::env::var("DOWIZ_SPAN_METRICS").as_deref() == Ok("1")
            && crate::span_metrics::init(None).is_ok()
        {
            return;
        }
        // init() only fails if a global observer is ALREADY installed (e.g. a test
        // harness). Fall through to the stderr sink below.
    }
    // Default: an stderr FDR sink (deterministic NDJSON events; no span rows to stderr, no
    // ring — so no `metric.jsonl` is written on this path). Best-effort (a second call is a
    // no-op, like the incumbent global subscriber).
    let _ = crate::fdr::init(crate::fdr::FdrConfig::default());
}
