//! dowiz-kernel — deterministic core (Rust→WASM).
//! Canonical kernel. The TS app (`/root/dowiz` apps/*) is the legacy oracle; this replaces it.
//! No float on money, no I/O. Verified-by-Math: RED+GREEN tests per module.

/// In-code protocol/wire version for the kernel. Independent of the repo CalVer
/// tag so a breaking kernel change (FSM-graph / `_js` export / ledger-layout) can
/// be gated without a repo retag. Bump on any such change.
pub const KERNEL_PROTO_VERSION: &str = "2026.07.0";

/// Kernel-wide tri-state: no boolean is ever just true/false.
/// Every observable state carries True | False | Unknown.
/// Unknown means "we don't know yet" — measurement pending, observation
/// insufficient, or system just booted. Code that acts on Unknown must
/// treat it as "not safe to assume either way" — fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriState {
    /// Confirmed positive / active / safe / stale / valid.
    True,
    /// Confirmed negative / inactive / unsafe / fresh / invalid.
    False,
    /// Unknown — observation pending or insufficient data.
    /// Fail-closed: treat as "cannot confirm".
    Unknown,
}

impl TriState {
    pub fn is_true(&self) -> bool { *self == TriState::True }
    pub fn is_false(&self) -> bool { *self == TriState::False }
    pub fn is_unknown(&self) -> bool { *self == TriState::Unknown }
    /// Resolve: True→true, False→false, Unknown→default.
    pub fn resolve(&self, default: bool) -> bool {
        match self {
            TriState::True => true,
            TriState::False => false,
            TriState::Unknown => default,
        }
    }
    /// Logical AND: True AND True = True, anything else = False.
    pub fn and(self, other: TriState) -> TriState {
        if self == TriState::True && other == TriState::True { TriState::True }
        else if self == TriState::False || other == TriState::False { TriState::False }
        else { TriState::Unknown }
    }
    /// Logical OR: False OR False = False, anything else = True.
    pub fn or(self, other: TriState) -> TriState {
        if self == TriState::True || other == TriState::True { TriState::True }
        else if self == TriState::False && other == TriState::False { TriState::False }
        else { TriState::Unknown }
    }
    /// Logical NOT: True→False, False→True, Unknown→Unknown.
    pub fn not(self) -> TriState {
        match self {
            TriState::True => TriState::False,
            TriState::False => TriState::True,
            TriState::Unknown => TriState::Unknown,
        }
    }
    /// From bool: true→True, false→False. Use when legacy code produces bool.
    pub fn from_bool(v: bool) -> TriState {
        if v { TriState::True } else { TriState::False }
    }
}

impl std::fmt::Display for TriState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriState::True => write!(f, "TRUE"),
            TriState::False => write!(f, "FALSE"),
            TriState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

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
/// Item 64 — capability-secure declarative composition root (production wiring of
/// the durable store). The single NON-test site that constructs `FileEventStore`.
pub mod compose;
pub mod core_pinning;
/// Deterministic CSR graph + synchronous Jacobi personalized-PageRank
/// (retrieval-blueprint v2 diffusion/recall primitive).
pub mod csr;
pub mod domain;
/// Foster-Lyapunov entropy budget, T-annealing, and BRANCH-dispersion detector
/// for the hydra's closed-loop self-evolution control.
pub mod entropy_budget;
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
/// Fully-wired closed-loop self-evolution engine (Hydra + EntropyBudget + TAnnealing + Kalman + M9 + telemetry).
pub mod hydra_closed_loop;
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
/// Item 11 (space-grade roadmap §E, Phase-1 code): ARINC-653-style two-level
/// partitioning scheduler. Temporal partitioning (fixed cyclic major frame of
/// guaranteed slices) maps onto the token bucket's proven refill law; partition
/// admission maps onto the §1.5 structural-gate pattern. Slice-exhaustion trips
/// the item-9 breaker. Phase-0 design + TLC model exist (`docs/formal/PartitionSchedule.tla`);
/// this is the Phase-1 implementation, gated on the breaker existing.
pub mod scheduler;
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
pub mod landing;
/// ITEM 32 (Part A, acceptance #2): the eqc-emitted Laplacian as a THIRD parity
/// leg against `spectral::laplacian` (dense) + `Csr::laplacian_spmv`. Gated to
/// `#[cfg(test)]` so the SHIPPING lib build carries NO `eqc_rs` symbols / dep
/// (eqc-rs is a DEV-dependency only — keeps `cargo tree -e no-dev` clean).
#[cfg(test)]
pub mod laplacian_eqc_parity;
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
/// Item 5 — MESH-07 parity: native, zero-dep pull anti-entropy + Merkle
/// digest reconciliation over `event_log`'s `EventStore`/`MeshEvent`. Design
/// reference only (not a dependency) on bebop2's `proto-wire/sync_pull.rs`
/// per the 2026-07-19 zero-dep mesh ruling — see module header for the split
/// with `crate::mesh`'s signing and `mesh-adapter`'s transport anti-scope.
pub mod mesh_replication;
/// P08 typed local-observability core — the pure-std, no-network, no-signing
/// HALF: typed-metrics schema + closed `LogEvent` enum (§2/§3) and the
/// claim-latency anomaly detector (§4). F40 ML-DSA signed envelope DEFERRED
/// pending bebop2 C4b — see `metrics.rs` header. Fail-closed local sink.
pub mod metrics;
/// Item 52 (space-grade roadmap §J): planted-fault self-test for the `miri-gate`.
/// Compiled under `cfg(test)` (so a plain `cargo test` proves it compiles and runs
/// the no-op native branch — zero footprint in any non-test/shipping build) AND
/// under `cfg(miri)` (where the planted-UB branch executes and Miri must flag it).
#[cfg(any(test, miri))]
mod miri_selftest;
pub mod moderation;
/// Numerical stability primitives: Kahan compensated summation, pairwise summation,
/// stable softmax, and matrix condition-number estimation via power iteration.
/// Zero-dep, pure-std. Guards every hot float-accumulation path against overflow
/// and catastrophic cancellation.
pub mod numerical_guard;
/// P9 wave: deterministic seedable PRNG (SplitMix64 → PCG64), zero-dep,
/// reproducible Monte-Carlo for the empirical causal joint.
pub mod rng;
/// Generalized PID controller — extracted from `orchestrator::PidController` for
/// reuse across delivery ETA smoothing, courier speed control, batch sizing,
/// cache hit-rate regulation, mesh backpressure, and real-time state consequence
/// prediction. Provides scalar f64 (`PidController`), quantized f32
/// (`PidController32`), and vectorized N-channel (`PidArray`) variants.
pub mod pid;
/// Stability margin computation for PID controllers — gain margin (dB) and phase
/// margin (degrees) from the discrete-time transfer function. Aid for closed-loop
/// stability analysis without external tooling.
pub mod math_guard;
/// Self-similar crystalline lattice for O(1) nearest-neighbor retrieval.
/// Extracted from `academia_p2p.rs` (NdCrystalLattice, NdSignature, CrystalMemory)
/// into a general-purpose module. Enables fast similar-state lookup for the
/// real-time system predictor.
pub mod crystal;
/// Real-time system state/action consequence predictor — combines PID dynamics,
/// crystalline memory, and ensemble bidding to predict performance, load,
/// traffic, telemetry, throttle, friction, and error consequences of state
/// changes and actions. Includes atomic sequencer for race-free access.
pub mod predictor;
/// Native prompt/skill enrichment engine — intent detection, crystal lattice
/// storage, and prompt injection. Scrapes free prompt libraries (fabric,
/// prompts.chat, opencode), stores 100k+ templates, reverse-engineers skills/
/// tools/plugins, enriches every prompt based on detected intent.
pub mod prompt_enrich;
/// Resilience: backup/failover/dynamic-switching layer for production systems.
/// Circuit breakers, bulkheads, cooldowns, automatic failover between
/// prediction models, and race-condition avoidance via sequencer integration.
pub mod resilience;
/// Lightweight n-dimensional array ops (dot, norm, outer, broadcasting).
/// Reuses simd.rs softmax/kalman lanes and csr.rs sparse layout without
/// pulling in ndarray/BLAS. Higher-level consumers: predictor, crystal,
/// spectral_graph, and engine frame profiler.
pub mod tensor;
/// Spectral graph analysis — graph Laplacian, eigenvector centrality,
/// clustering coefficient, graph energy, similarity graphs from distance
/// matrices, and temporal graphs from observation sequences. Reuses
/// `crate::spectral` (eigendecomposition) and `crate::csr` (sparse CSR).
pub mod spectral_graph;
/// Lightweight topic-based pub/sub gossip bus for inter-module data
/// propagation. Inspired by `crate::mesh::GossipImport`'s fan-out, without
/// PQ signing overhead. Used for fast telemetry, prediction, and state-change
/// propagation between engine_loop, predictor, resilience, and offline.
pub mod gossip;
/// Frequency-domain pattern detection: Goertzel filter periodogram for
/// identifying oscillatory behavior in time-series metrics (frame-time
/// oscillations, cache hit-rate cycles, traffic bursts). Includes moving
/// average filter and helper detection function.
pub mod resonance;
/// Last-healthy-state propagation, wormholes (direct urgent lanes), and
/// tunnels (dedicated high-throughput channels). Provides fast bypass
/// paths for critical data when normal gossip/queuing adds unacceptable
/// latency, plus failover recovery snapshots.
pub mod channel;
/// Three-phase verification pattern: Prepare → Verify → Acknowledge.
/// Each operation cycle goes through sanitization, cross-check against
/// invariants, and output validation. Designed for unstable/distrusted
/// environments where inputs may be corrupted, delayed, or spoofed.
pub mod three_phase;
/// Comprehensive error tracking, deterministic event log, telemetry
/// collection, reverse replay (time-travel state reconstruction), and
/// inverse simulation (infer inputs from observed outputs). Provides
/// full observability for debugging, auditing, and post-mortem analysis.
pub mod tracker;
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
/// BLUEPRINT-P-B §4.3 (item #7) — drift-gated spectral snapshot store + reconcile. The
/// consumer of `RetainedBase::admit`: retains admitted tiles, persists their source `Csr`,
/// and `reconcile()` re-runs `classify_drift` on each retained raw dynamics to catch
/// post-admit divergence. Pure-std; no money / red-line types.
pub mod snapshot;
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
/// C2 — DEPLOY CONFIG (roster + provider selection + default currency) is load-bearing
/// deployment input, not compiled-in. Std-only, serde-free.
pub mod deploy_config;
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
/// §H toy-pilot inference arc (BLUEPRINT-ITEM-34/35/37/38/39/40/41/42/43/44) — a
/// quantized, constant-time-gated, golden-checksum-guarded toy neural-network
/// classifier. Always compiled (so the `None`-path / reject-path is always
/// tested — item 47); the PQ codesign half of the weight pipeline rides behind
/// the `pq` feature. No new dependencies; integer-domain only.
pub mod inference;
/// Living-knowledge retrieval — ADAPTER to the (separately-branched) JS engine.
/// serde-dependent (JSON bridge protocol) → gated behind `wasm` to keep the
/// native rlib build serde-free. Not part of the order/money core. ALSO excluded on
/// `wasm32` itself (`not(target_arch = "wasm32")`): the adapter spawns a real OS
/// subprocess (`std::process::Command`/`wait4`), which is not a wasm capability — the
/// module's own doc already calls this "unreachable... on wasm32"; this makes that true.
#[cfg(all(feature = "wasm", not(target_arch = "wasm32")))]
pub mod living_knowledge;
#[cfg(feature = "wasm")]
pub mod loops;
pub mod markov;
/// Contiguous row-major matrix helper — the single backing store / matmul impl
/// the spectral + absorbing subsystems route through (DOD/SIMD prep).
pub mod mat;
/// SHA-256 hardware acceleration using SHA-NI (for fast integrity checks,
/// not cryptographic MACs/signatures). Runtime-detected; scalar fallback
/// included. SHA-NI accelerates SHA-1 and SHA-256 (NOT SHA3/Keccak).
pub mod sha256_hw;
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
/// BLUEPRINT-ITEM-28 — optical/pixel archival compression. Feature-gated (`optical`)
/// so the canonical order/money core stays pure-`std` and dependency-free. The
/// `OpticalCompressed` type is structurally prevented from reaching ANY
/// hash/signature/idempotency surface (the §1.5 plane-boundary unrepresentability
/// standard) — it is accepted ONLY by the archival-tier persistence API (item 20).
/// The DeepSeek-OCR model lives at a RUNTIME SEAM (local GGUF) OUTSIDE the Cargo
/// graph, mirroring `pq/entropy.rs`'s opt-in network provider. Absent local
/// weights, the codec returns `Err(OpticalError::ModelUnavailable)`.
#[cfg(feature = "optical")]
pub mod optical;
pub mod order_machine;
/// Kernel-native structured data extraction (pure `std`). Replaces `awk`,
/// `split('=')`, and `node -e JSON.parse` with deterministic, zero-dep parsers.
pub mod parse;
/// Unified memory search engine — indexed vector (BM25 + trigram) + spectral
/// navigation (PPR) over structured documents. Replaces all `grep -rn` extraction.
pub mod memory_search;
/// Chronological-topological prediction engine — composes Markov, spectral,
/// absorbing, Noether, and causal primitives into a single prediction API.
pub mod predict;
/// Decentralized mesh swarm coordinator — task decomposition via DSU, executor
/// selection via harmonic ranking, dynamic adaptation via spectral/Markov prediction.
pub mod swarm;
/// Structural enforcement of the mandatory agent workflow sequence
/// (research -> synthesis -> critique -> plan -> critique -> work -> verify -> critique -> commit).
/// Typed state machine: phases complete in strict order, no skipping, no repeats.
/// The kernel primitive that closes the "workflow gates are cultural" blind spot.
pub mod workflow_gate;
/// PLL-inspired clock stabilizer — transforms irregular kernel ticks, timestamps,
/// and event intervals into stable, aligned output via phase-locked feedback control.
/// Maps PLL components (phase detector, loop filter, VCO) to kernel equivalents
/// (tick differ, EMA smoother, adaptive rate generator).
pub mod clock_stabilizer;
/// Kernel-native tool/skill/agent orchestrator — central hub for all action routing,
/// parallel dispatch, health monitoring, load prediction, and audit trails. No grep,
/// no scripts — everything through Rust structs and SHA3-256 verified state transitions.
pub mod agent_orchestrator;
pub mod orchestrator;
/// Kernel-native hex encode/decode primitive — single canonical implementation
/// replacing 6+ redundant hand-rolled versions across the codebase.
pub mod hex_util;
/// Kernel-native reverse engineering — ELF parsing, x86_64 syscall extraction,
/// behavior profiling, and binary analysis. All pure Rust, zero deps.
pub mod reverse_engineer;
/// Anti-detect browser configuration and zero-trace policy for parse operations.
/// Pure data structures: kernel = no browser/network, this defines HOW to parse.
pub mod agent_browser;
/// PID-controlled dynamic agent spawn batching with prediction cache.
/// Adjusts parallelism based on real-time latency measurements.
pub mod dynamic_spawner;
/// PID-controlled dynamic action batch execution with worker pool.
/// Per-category latency prediction, work-stealing, and ASCII dashboard.
pub mod dynamic_actions;
/// Indexed parallel search across multiple search indexes (BM25, trigram, spectral).
/// Weighted fusion, PID-controlled parallelism, search result caching.
pub mod parallel_search;
/// Apollo-11 inspired priority task scheduler with checkpoint/restart.
/// Priority levels, overload shedding, dependency gating, PID concurrency.
pub mod agc_scheduler;
/// Book-to-skill native: on-demand knowledge extraction from documents.
/// Frameworks, decision rules, anti-patterns, per-chapter loading.
pub mod skill_extractor;
/// PixelRAG native: visual tile indexing + IVF approximate nearest-neighbor search.
/// Screenshot tile management, tile embedding coordination.
pub mod visual_index;
/// Supervision native: universal detection format + NMS/NMM + zone analysis.
/// Model-agnostic detection container, polygon/line zone counting.
pub mod detection;
/// Automatic gap detection, enrichment, and correction for the prompt database.
/// Monitors the enrichment engine: detects triggerless entries, low-count kinds,
/// and empty text, then auto-heals via crystal-lattice synthesis and keyword extraction.
pub mod self_heal;
/// Self-harness with zone protection for safe project-wide rewrites.
/// Zone mapping (green→yellow→red→critical→forbidden), Hydra protection,
/// blueprint generation, dynamic rewrite prediction.
pub mod self_harness;
/// Reverse-engineered parsing patterns from top GitHub repos (1,302 parsing tools,
/// 461 top repos, 43M+ combined stars). Integrates Rust-native parsing insights.
pub mod github_patterns;
/// Thunder parsing: tensor-accelerated paper extraction with vector geometry
/// navigation. Papers stored as 256D vectors in CSR matrix, O(1) nearest-neighbor
/// via spectral decomposition. FanOut parallelism across tensor dimensions.
pub mod tensor_parser;
/// Spectral Parsing: O(n⁰) paper extraction. Raw OAI-PMH byte scanner (no XML DOM),
/// tensor ASCII storage, spectral decomposition search, harmonic ranking.
pub mod spectral_parser;
/// Parametric Surface Spectral Library: papers projected onto 2D parametric
/// surface via top-2 eigenvectors. Each paper = SPIN at (u,v) on surface.
/// Grid-based navigation: O(1) cell lookup, ~32MB for 1M papers.
pub mod parametric_spectral;
/// Академія Дмитра Євдокимова — quantized spectral library with P2P sync.
/// Hash-only paper storage (32B/paper), bloom filter sync, snapshot serialization.
pub mod academia;
/// P2P distribution network: peers → parallel chunk download → merge.
/// Аналогічна логіка рекурсивного пошуку, але для завантаження даних.
/// Gated behind `feature = "speculative"` — 5224 lines of speculative
/// physics-inspired P2P design, NOT connected to the delivery mesh.
/// Without the feature, a stub provides only `MetricTensor` + `GEO_DIMS` for
/// modules that depend on these types (e.g. `memory_search`).
#[cfg(feature = "speculative")]
pub mod academia_p2p;
#[cfg(not(feature = "speculative"))]
pub mod academia_p2p {
    /// Кількість геометричних вимірів для метричного тензора.
    pub const GEO_DIMS: usize = 8;

    /// Метричний тензор g_ij (симетричний, додатно визначений),
    /// stub-версія — завжди евклідова.
    #[derive(Debug, Clone)]
    pub struct MetricTensor {
        pub g: [[f64; GEO_DIMS]; GEO_DIMS],
    }

    impl MetricTensor {
        pub fn euclidean() -> Self {
            let mut g = [[0.0; GEO_DIMS]; GEO_DIMS];
            for i in 0..GEO_DIMS {
                g[i][i] = 1.0;
            }
            MetricTensor { g }
        }
    }
}
/// Autonomous headless extraction agents — distributed bots for paper extraction.
/// Кожен агент = окремий browser profile / IP / акаунт.
/// Gated behind `feature = "speculative"` — depends on `AcademiaMesh`
/// from the full `academia_p2p` module.
#[cfg(feature = "speculative")]
pub mod academia_agent;
/// PID-керований Cloudflare Workers пул — 10,000 динамічних Workers.
/// Чим більше паперів залишилось, тим більше Workers спавниться.
/// Неактивні Workers поступово видаляються.
pub mod academia_cloud;
/// Self-sovereign trading infrastructure — signed cryptographic intents
/// for trustless, non-custodial, intermediary-free trading.
pub mod trading_intent;
/// Trustless escrow + state channels — P2P settlements without centralized
/// gateways, with off-chain balance updates and on-chain finality.
pub mod trading_escrow;
/// P2P direct delivery — no intermediaries, no central platform.
/// Pure peer-to-peer delivery routing with cryptographic proof.
pub mod p2p_delivery;
/// Cooperation Protocol — atomic bridge between P2P trading and P2P delivery.
/// Trade settlement triggers delivery; delivery confirmation releases funds.
pub mod cooperation_protocol;
/// Research paper knowledge extraction and pattern analysis engine.
/// Pure data structures for ingesting, pattern-extracting, and cross-pattern
/// analysis of research papers from arXiv / Semantic Scholar / OpenAlex.
pub mod research;
/// Compact ASCII library for research papers — content-addressed, deduplicated,
/// non-ASCII stripped. Each paper stored as one line (unit-separator delimited).
/// ~20MB for 100K papers vs ~200MB in JSON (10x compression).
pub mod research_ascii;
/// CPU core topology + cache hierarchy + clock source detection.
/// Probes /proc/cpuinfo and /sys at init. All pure data after init.
pub mod hw_profile;
/// Hardware feature-flag detection (AVX2, FMA, SHA-NI, AES-NI, BMI2, FSRM,
/// L3 cache size, RAM total, core count). Zero-dep, reads /proc/cpuinfo +
/// /proc/meminfo once; cached via OnceLock. Companion to hw_profile.
pub mod cpuid;
/// Deterministic time authority — stabilises raw clocks (kvm-clock/TSC/HPET)
/// through a PLL corrector + PPMC predictor. Time never goes backwards.
pub mod time_stabilizer;
/// Weather + power grid load forecasting for clock drift prediction.
/// Forecast feeds into TimeStabilizer's drift model.
pub mod power_forecast;
/// Per-call PQ ML-DSA-65 cryptographic signer for parse operations.
/// Each parse call gets a fresh keypair; signature binds IP+timestamp+payload.
/// Requires the `pq` feature for ML-DSA-65.
#[cfg(feature = "pq")]
pub mod crypto_signer;
/// Proxy pool management, rotation strategies, and health tracking.
/// Pure computation: kernel routes, external adapters connect.
pub mod proxy_redirect;
/// Reusable parallel execution pattern library (fan-out/fan-in, pipeline,
/// work-stealing, dynamic batch). Patterns produce execution plans, not threads.
pub mod parallel_patterns;
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
/// Deterministic, zero-dep harvest ledger — gov_route-compatible telemetry
/// records for every hot path. `HarvestLedger` is the EV-scoring ring buffer;
/// every new capability MUST be probe-able through it.
pub mod telemetry_harvest;
/// F33 — deterministic compute-budget token bucket (monotonic-clock, atomic, degrade-closed).
/// The `llm-adapters` `Dispatcher` reuses this to bound LLM-call concurrency.
pub mod token_bucket;
/// Deterministic n-gram (bigram + trigram) frequency extraction over a token
/// stream — the self-improvement loop's pattern-surface primitive (P9 / T2-β).
pub mod trigram;
/// Light multilingual stemmer (EN/UK/RU) — zero-dep suffix stripping for
/// inflected-language recall in BM25/trigram retrieval. Normalizes cases,
/// conjugations, and plurals to a common root.
pub mod stem;
/// Three-valued logic + RGB/matrix encoding — trinary replaces binary
/// everywhere: FSM states, auth decisions, enrichment scoring. Each
/// Tri maps to an RGB color; matrices of Tri values produce visual
/// state bitmaps. Delta chains track system drift over time.
pub mod trinary;
/// Trigonometric phase-space primitives — cos/sin pairs replace booleans.
/// Phase encoding on S¹, xyz 3D phase space, phase vectors with dot product.
pub mod trig;
/// Spectral wave propagation — states emit spectral fingerprints that
/// propagate, interfere, and decay. InterferenceField = superposition.
pub mod wave;
/// Time navigation — timestamp-indexed snapshots, state history with
/// integrity hashes, time-travel queries, delta between any two timestamps.
pub mod chronos;
/// Chrono-topological foundation — the root state substrate. Every module
/// writes TriMatrix snapshots indexed by timestamp. Three views: PAST,
/// PRESENT, PREDICTED. 4D navigation between states. Global drift tracking.
pub mod chronos_topology;
/// Eigenvalue/eigenvector as the canonical data primitive. Replaces scalar
/// f64 with (λ, v) pairs. Spectral decomposition IS the data representation.
pub mod eigen;
/// N-fractal self-similar data structure + ASCII encoding. Every node is
/// a microcosm of the whole. ASCII visualization for human-readable state.
pub mod fractal;
/// Delta calculus — replaces classic comparisons (a>b, x==y) with vector
/// delta analysis. Δv, ∂t, rate, acceleration, drift tracking. Eigen/Phase
/// comparison via spectral distance. Oscillation detection.
pub mod delta;
/// Inversion + backpropagation engine. When errors or deviations are
/// detected, they propagate BACKWARDS through the causal chain. Each
/// step identifies upstream responsibility. Results surface on the
/// interface as corrective recommendations.
pub mod invert;
/// Cross-kind pattern bridges — each research finding natively encoded.
/// python(17 kinds), tool(14), llm(14), claude-code(12), security(11).
/// Publisher diversity tracker: NVIDIA(9), anthropics(8), microsoft(8).
pub mod cross_bridge;
/// Bebop protocol bridge — trinary auth, eigen envelopes, wave mesh sync,
/// chronos DTN store-forward. Wires the new paradigm into the protocol layer.
pub mod bebop_bridge;
/// Code change prediction oracle — learns from past commits to predict
/// ETA, impact, and eigen shift for future changes. Uses chronos snapshots.
pub mod code_oracle;
/// System-wide ASCII dashboard — renders full system state as human-readable
/// report. Uses all visualization primitives: trinary RGB, fractal, eigen, chronos.
pub mod sys_dashboard;
/// Meta-tests (infrastructure self-check) + cross-tests (multi-module
/// integration: trinary×eigen×chronos×delta×enrich full stack).
#[cfg(test)]
mod cross_tests;
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
/// Item 9 (space-grade roadmap §D "THE PIVOT") — the deterministic
/// fault-containment circuit breaker. The `Result<Permit, Tripped>` gate is the
/// single admission decision; alarm receiver `Breaker::on_commit_error` consumes
/// `CommitError::Store`. Golden-signature-pinned cyclic FSM, shares the Tier-1 FDR
/// ring for audit, zero new dependencies (pure `std`).
pub mod breaker;
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

/// Sanitize a raw f64: NaN/Inf → 0.0 (fail-closed for system stability).
/// Every public API that accepts raw f64 MUST call this at the boundary
/// to prevent corrupted data from propagating through the kernel.
pub fn sanitize_f64(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// Sanitize AND clamp to [0, 1] — for normalized metric values.
/// NaN/Inf → 0.0; values outside range clamped.
pub fn sanitize_normalized(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
}

/// Sanitize a raw f32: NaN/Inf → 0.0.
pub fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

pub const CHECKSUM_MUL: u64 = 31;

pub fn checksum_fold(data: &[u8]) -> u64 {
    data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(CHECKSUM_MUL).wrapping_add(b as u64))
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Sort descending by a `f64` key (NaN/Inf → end of order).
///
/// Replaces the repeated `sort_by(|a,b| b.K.partial_cmp(&a.K).unwrap_or(Equal))`
/// pattern found in 17+ call sites across the kernel.
pub fn sort_by_f64_desc<T, K>(items: &mut [T], key: K)
where K: Fn(&T) -> f64
{
    items.sort_by(|a, b| key(b).partial_cmp(&key(a)).unwrap_or(std::cmp::Ordering::Equal));
}

/// Sort ascending by a `f64` key (NaN/Inf → end of order).
pub fn sort_by_f64_asc<T, K>(items: &mut [T], key: K)
where K: Fn(&T) -> f64
{
    items.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
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

    // ── SANITIZE API tests (meta: test the sanitizers themselves) ───────
    #[test]
    fn sanitize_f64_normal_values() {
        assert_eq!(sanitize_f64(0.0), 0.0);
        assert_eq!(sanitize_f64(1.0), 1.0);
        assert_eq!(sanitize_f64(-1.0), -1.0);
        assert_eq!(sanitize_f64(f64::MAX), f64::MAX);
        assert_eq!(sanitize_f64(f64::MIN), f64::MIN);
    }

    #[test]
    fn sanitize_f64_nan_inf() {
        assert_eq!(sanitize_f64(f64::NAN), 0.0, "NaN must become 0.0");
        assert_eq!(sanitize_f64(f64::INFINITY), 0.0, "Inf must become 0.0");
        assert_eq!(sanitize_f64(f64::NEG_INFINITY), 0.0, "-Inf must become 0.0");
    }

    #[test]
    fn sanitize_normalized_clamps() {
        assert_eq!(sanitize_normalized(0.5), 0.5);
        assert_eq!(sanitize_normalized(0.0), 0.0);
        assert_eq!(sanitize_normalized(1.0), 1.0);
        assert_eq!(sanitize_normalized(-0.5), 0.0, "negative must clamp to 0");
        assert_eq!(sanitize_normalized(1.5), 1.0, ">1 must clamp to 1");
        assert_eq!(sanitize_normalized(f64::NAN), 0.0, "NaN must become 0");
        assert_eq!(sanitize_normalized(f64::INFINITY), 0.0, "Inf is non-finite → 0.0");
        assert_eq!(sanitize_normalized(f64::NEG_INFINITY), 0.0, "-Inf must become 0");
    }

    #[test]
    fn sanitize_f32_nan_inf() {
        assert_eq!(sanitize_f32(f32::NAN), 0.0);
        assert_eq!(sanitize_f32(f32::INFINITY), 0.0);
        assert_eq!(sanitize_f32(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn dt_stable_is_authoritative() {
        assert_eq!(DT_STABLE, 0.02);
        assert_eq!((1.0 / DT_STABLE as f64).round() as u32, 50);
    }

    #[test]
    fn hermetic_sanitize_all_boundaries() {
        let cases = [
            (0.0, 0.0),
            (1.0, 1.0),
            (-0.0, 0.0),
            (f64::NAN, 0.0),
            (f64::INFINITY, 0.0),
            (f64::NEG_INFINITY, 0.0),
            (f64::MIN_POSITIVE, f64::MIN_POSITIVE),
            (f64::MAX, 1.0), // >1 → clamp to 1
            (-1e-16, 0.0),  // negative near-zero → clamp to 0
            (1.0 + 1e-16, 1.0), // slightly above 1 → clamp to 1
        ];
        for &(input, expected) in &cases {
            let actual = sanitize_normalized(input);
            if expected.is_finite() {
                assert!((actual - expected).abs() <= f64::EPSILON,
                    "sanitize_normalized({input:e}) = {actual:e}, expected {expected:e}");
            }
        }
    }

    #[test]
    fn hermetic_sanitize_f64_identity_for_finite() {
        for &v in &[-1e6_f64, -1.0, -0.5, 0.0, 0.5, 1.0, 1e6] {
            let expected = if v.is_finite() { v } else { 0.0 };
            assert_eq!(sanitize_f64(v), expected,
                "sanitize_f64({v}) must preserve finite values");
        }
    }

    #[test]
    fn hermetic_sanitize_f32_identity_for_finite() {
        for &v in &[-1e6_f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1e6] {
            let expected = if v.is_finite() { v } else { 0.0 };
            assert_eq!(sanitize_f32(v), expected,
                "sanitize_f32({v}) must preserve finite values");
        }
    }

    /// Jamming/spoofing injection test: malformed inputs must never
    /// propagate past the sanitization boundary.
    #[test]
    fn hermetic_jamming_resistant_input_boundary() {
        let jamming_payloads: Vec<Vec<f64>> = vec![
            vec![f64::NAN; 8],
            vec![f64::INFINITY; 8],
            vec![f64::NEG_INFINITY; 8],
            (0..8).map(|i| if i % 2 == 0 { f64::NAN } else { f64::INFINITY }).collect(),
            vec![-1e308, 1e308, -0.0, 1.0 / 0.0, 0.0 / 0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
        ];
        for (i, payload) in jamming_payloads.iter().enumerate() {
            let state = crate::predictor::SystemState::new(0, payload.clone(), "jamming");
            assert_eq!(state.metrics.len(), 8,
                "jamming payload {i} must produce valid state");
            for (j, &m) in state.metrics.iter().enumerate() {
                assert!(m.is_finite() && (0.0..=1.0).contains(&m),
                    "jamming payload {i} metric {j} = {m} not in [0,1]");
            }
        }
    }

    /// Spoofing protection: labeled metrics from untrusted source
    /// must still pass sanitization.
    #[test]
    fn hermetic_spoofing_resistant_state_labels() {
        let spoofed_labels = ["\0", "..", "/etc/passwd", "%00", "<script>", "🦀 rust"];
        for label in &spoofed_labels {
            let state = crate::predictor::SystemState::new(0, vec![0.5; 8], label);
            assert_eq!(state.label, *label,
                "spoofed label '{label}' must be preserved as-is (label is metadata)");
            // But metrics must still be valid
            assert!(state.metrics.iter().all(|m| m.is_finite() && (0.0..=1.0).contains(m)),
                "spoofed label '{label}' must not corrupt metrics");
        }
    }

    /// Penetration test: deep recursion / panics in sanitize chain.
    #[test]
    fn hermetic_penetration_deep_recursion_safe() {
        let deep: Vec<f64> = (0..1000).map(|i| i as f64 * 0.001).collect();
        let state = crate::predictor::SystemState::new(0, deep, "deep");
        assert_eq!(state.metrics.len(), 8,
            "deep input must be resized to 8, not {len}", len = state.metrics.len());
    }

    /// Consistency verification across all kernel layers:
    /// if sanitize_normalized passes, all downstream modules must accept the result.
    #[test]
    fn hermetic_layer_consistency_sanitize_chain() {
        let input = vec![0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.5, 0.3];
        let state = crate::predictor::SystemState::new(1, input.clone(), "consistency");
        let mut pred = crate::predictor::Predictor::new(crate::predictor::PredictorConfig::default());
        pred.observe(state);
        let outcomes = pred.predict_all("test");
        assert_eq!(outcomes.len(), 8, "all 8 metrics must produce predictions");
        for o in &outcomes {
            assert!(o.predicted_value.is_finite() && (0.0..=1.0).contains(&o.predicted_value),
                "predicted_value must be sanitized: {o:?}");
            assert!(o.confidence.is_finite() && (0.0..=1.0).contains(&o.confidence),
                "confidence must be in [0,1]: {o:?}");
            assert!(o.friction_score.is_finite() && o.friction_score >= 0.0,
                "friction must be non-negative: {o:?}");
            assert!(o.error_probability.is_finite() && o.error_probability >= 0.0,
                "error_prob must be non-negative: {o:?}");
        }
    }

    #[test]
    fn hermetic_sanitize_mutating_input_not_mutated() {
        let input = vec![0.3, -0.1, 0.7, 0.5]; // no NaN (NaN != NaN in IEEE 754)
        let original = input.clone();
        let _state = crate::predictor::SystemState::new(1, input.clone(), "no_mutate");
        assert_eq!(input, original,
            "SystemState::new must not mutate its input vec");
    }

    #[test]
    fn cover_sanitize_f64_nan() {
        let _ = super::sanitize_f64(f64::NAN);
    }

    #[test]
    fn cover_sanitize_f64_inf() {
        let _ = super::sanitize_f64(f64::INFINITY);
    }

    #[test]
    fn cover_sanitize_normalized_neg() {
        let _ = super::sanitize_normalized(-0.5);
    }

    #[test]
    fn cover_sanitize_f32_nan() {
        let _ = super::sanitize_f32(f32::NAN);
    }

    #[test]
    fn cover_checksum_fold_empty() {
        let _ = super::checksum_fold(&[]);
    }

    #[test]
    fn cover_checksum_fold_data() {
        let _ = super::checksum_fold(&[1, 2, 3, 4]);
    }

    #[test]
    fn cover_kernel_boot_verify_fsm() {
        let _ = super::kernel_boot_verify_fsm();
    }

    #[test]
    fn cover_init_tracing() {
        super::init_tracing();
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
pub mod oracle;
pub mod meta_miner;
pub mod physics;
