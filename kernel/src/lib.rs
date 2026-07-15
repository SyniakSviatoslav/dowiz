//! dowiz-kernel — deterministic core (Rust→WASM).
//! Canonical kernel. The TS app (`/root/dowiz` apps/*) is the legacy oracle; this replaces it.
//! No float on money, no I/O. Verified-by-Math: RED+GREEN tests per module.

/// Reverse-engineering loop #R3 — absorbing Markov chain closed forms: fundamental matrix
/// N=(I−Q)⁻¹ (exact finite sum for the DAG lifecycle), expected steps-to-terminal, absorption probs.
pub mod absorbing;
/// C-tier "impedance lens": circuit/impedance as a resource framework — flow
/// reflection coefficient + backpressure gate (ρ<1 with margin, not power-match).
pub mod impedance;
pub mod analytics;
/// C-tier "attention lens": scaled dot-product attention as one learned-affinity
/// diffusion step — same f(L) family as markov PPR / heat-kernel.
pub mod attention;
/// RW-07 — cart state machine (consolidate 2 JS cart impls → kernel authority). Totals via money.
pub mod cart;
/// P9 growth-substrate: causal inference — back-door adjustment / do-operator
/// (Pearl). Provable causal effect from observational tables; fail-closed.
pub mod causal;
/// P9 wave: deterministic seedable PRNG (SplitMix64 → PCG64), zero-dep,
/// reproducible Monte-Carlo for the empirical causal joint.
pub mod rng;
/// P9 growth-substrate: semi-Markovian **causal graph** primitives (directed +
/// bidirected arcs) — the structural backbone of the ID / IDC identification
/// algorithms: ancestors, descendants, c-components, bidirected-aware
/// d-separation, and the `G\X` / `G[V]` subgraph algebra.
pub mod cgraph;
/// Deterministic CSR graph + synchronous Jacobi personalized-PageRank
/// (retrieval-blueprint v2 diffusion/recall primitive).
pub mod csr;
/// Spool — pure crash-safe async work-queue state machine (append / claim /
/// ack / reclaim). The I/O + drainer adapter lives outside the kernel
/// (pure-std firewall); this owns the Verified-by-Math transitions. Reused by
/// every async subsystem (reporting, governance, mesh sync).
pub mod spool;
/// B4 — deterministic content-defined chunker (Buzhash) for the native Rust
/// backup organ: content-addressed blocks that dedup across small edits.
pub mod chunker;
pub mod domain;
/// MESH-06 — per-node content-addressed event-log (local-first + sync).
pub mod event_log;
/// RW-06 — geo / route kinematics (pure-logic port from geo-anim.ts + delivery-zone.ts). Kernel authority.
pub mod geo;
/// Householder QR + shifted-QR eigensolver (the dense-`n×n` "Ferrari"): all
/// eigenvalues, real + complex, stack-only for n ≤ 32 (no heap; FMA inner
/// product). Replaces the O(n⁴) Faddeev-LeVerrier path as the default for the
/// dense operators this kernel diagonalizes.
pub mod householder;
pub mod kalman;
pub mod intake;
pub mod isolation;
pub mod loops;
/// Reverse-engineering loop #R1 — Markov attractor detector (ASCENDed from markov_attractor.py);
/// reuses `spectral` as its eigen-core, killing the dual-authority hazard.
pub mod markov;
/// Contiguous row-major matrix helper — the single backing store / matmul impl
/// the spectral + absorbing subsystems route through (DOD/SIMD prep).
pub mod mat;
/// RW-08 — messenger deep-link builders (pure string logic → kernel authority).
pub mod messenger;
pub mod money;
pub mod order_machine;
/// Reverse-engineering loop #1 — general (non-symmetric) spectral engine: eigenvalues
/// (Faddeev-LeVerrier + Durand-Kerner), spectral gap γ, Laplacian Fiedler λ₂, DMD drift class.
pub mod spectral;
/// Deterministic n-gram (bigram + trigram) frequency extraction over a token
/// stream — the self-improvement loop's pattern-surface primitive (P9 / T2-β).
pub mod trigram;
/// Reverse-mode automatic differentiation (scalar tape engine) — the
/// kernel-side fitting primitive (Tier B2: capture-field SIREN/splat fits).
pub mod micrograd;
/// P9 / C-tier "invariance note": executable Noether check — verify a conserved
/// quantity survives a deterministic update (catches self-improvement drift).
pub mod noether;
/// B3 — deterministic offline-on-node online learner (LinearSGD ridge +
/// ScalarAdam), the self-adaptation substrate (E3). Local-first: no network.
pub mod online;
/// E1 — verifiable-cognition benchmark generator: metamorphic MR items with
/// kernel-primitive oracles, deterministic mint-log leakage gate, and
/// calibration metrics (ECE/Brier/AURC). Pure-offline, zero-dep.
pub mod evals;
/// C1 — verify-failure → retrieval-trigger: a claim check that, on failure,
/// emits a bounded structured re-verify request (the "verify then learn" loop).
pub mod verify_retrieval;
/// Living-knowledge retrieval — ADAPTER to the (separately-branched) JS engine.
pub mod living_knowledge;
/// WASM/JS bindings — the only place the kernel touches the boundary.
pub mod wasm;

// Re-export the headline types so wasm-bindgen consumers and tests share one surface.
pub use evals::{
    aurc, brier, ece, EmaTracker, EvalCheck, EvalRow, MetamorphicGenerator, MintLog, MrItem,
    RegressionGate, SelfAdaptator,
};
pub use csr::{
    precision_at_k, recall_at_k, Csr,
};
/// P9 growth-substrate: causal inference — back-door + front-door + instrumental-variable
/// + counterfactual (twin-network) + d-separation oracle + back-door/front-door
/// criterion verifiers (do-operator / Pearl / Wald).
pub use causal::{
    backdoor_adjust, backdoor_criterion, confounded, counterfactual_linear, d_separated,
    empirical_identify, frontdoor_adjust, frontdoor_criterion, identify_causal_effect,
    instrumental_adjust, sample_backdoor, CausalEffect, HedgeWitness, IdFormula, IdResult,
};
pub use domain::{apply_event, compute_order_total, place_order, Order, OrderItem};
pub use money::{
    apply_tax, assert_non_negative, compute_line_total, convert_all_to_eur_cents, to_minor_unit,
};
pub use order_machine::{
    assert_transition, cyclomatic_number, fold_transitions, fsm_graph_report, has_cycle, reachable,
    spectral_radius, topological_order, verify_fsm_signature, verify_fsm_signature_against,
    FsmGraphReport, FsmSignatureDrift, OrderStatus, TransitionError,
};
pub use wasm::{apply_event_js, boot_verify_fsm_js, channel_ledger_js, place_order_js, reduce_anomalies_js};

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
