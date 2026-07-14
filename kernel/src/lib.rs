//! dowiz-kernel — deterministic core (Rust→WASM).
//! Canonical kernel. The TS app (`/root/dowiz` apps/*) is the legacy oracle; this replaces it.
//! No float on money, no I/O. Verified-by-Math: RED+GREEN tests per module.

pub mod analytics;
/// RW-07 — cart state machine (consolidate 2 JS cart impls → kernel authority). Totals via money.
pub mod cart;
pub mod domain;
/// MESH-06 — per-node content-addressed event-log (local-first + sync).
pub mod event_log;
/// RW-06 — geo / route kinematics (pure-logic port from geo-anim.ts + delivery-zone.ts). Kernel authority.
pub mod geo;
pub mod intake;
pub mod isolation;
pub mod loops;
/// Reverse-engineering loop #R1 — Markov attractor detector (ASCENDed from markov_attractor.py);
/// reuses `spectral` as its eigen-core, killing the dual-authority hazard.
pub mod markov;
/// RW-08 — messenger deep-link builders (pure string logic → kernel authority).
pub mod messenger;
pub mod money;
pub mod order_machine;
/// Reverse-engineering loop #1 — general (non-symmetric) spectral engine: eigenvalues
/// (Faddeev-LeVerrier + Durand-Kerner), spectral gap γ, Laplacian Fiedler λ₂, DMD drift class.
pub mod spectral;
pub mod wasm;

// Re-export the headline types so wasm-bindgen consumers and tests share one surface.
pub use analytics::{reduce_anomalies, ChannelEvent, ChannelLedger};
pub use domain::{apply_event, compute_order_total, place_order, Order, OrderItem};
pub use money::{
    apply_tax, assert_non_negative, compute_line_total, convert_all_to_eur_cents, to_minor_unit,
};
pub use order_machine::{
    assert_transition, cyclomatic_number, fold_transitions, fsm_graph_report, has_cycle, reachable,
    spectral_radius, topological_order, verify_fsm_signature, FsmGraphReport, FsmSignatureDrift,
    OrderStatus, TransitionError,
};
pub use wasm::{apply_event_js, channel_ledger_js, place_order_js, reduce_anomalies_js};

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
