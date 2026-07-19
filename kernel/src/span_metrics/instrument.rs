//! telemetry/instrument.rs — P83 Layer 1 span wrappers for the 8 verified functions.
//!
//! SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 names exactly eight functions to
//! instrument with production span metrics. Three already carry `tracing` spans in
//! the kernel source (`place_order`, `place_order_priced`, `fold_transitions`); this
//! module adds the remaining five as thin `#[instrument(...)]`-style wrappers so the
//! `SpanMetricsLayer` (obs.rs) can measure them without touching the functions'
//! bodies or their red-line logic:
//!
//!   * `route`              — `crate::router::route`                          (always)
//!   * `commit_after_decide`— `crate::event_log::EventLog::commit_after_decide`(always)
//!   * `decide_settlement`  — `crate::ports::payment::decide_settlement`      (always)
//!   * `cap::verify_chain`  — `crate::ports::agent::cap::verify_chain`        (always)
//!   * `mldsa verify`       — `crate::pq::dsa::verify`                (behind `pq`)
//!
//! Explicitly EXCLUDED (SYNTHESIS §6-E18): `assert_transition`'s inner loop is NOT
//! instrumented — the `fold_transitions` span + Layer-2 sampler cover it.
//!
//! The wrappers are `#[cfg(feature = "telemetry")]` so the shipping binary pays zero
//! cost: with the feature OFF, callers use the real functions directly; with it ON,
//! the `obs` layer's `init` registers a subscriber that records these spans to
//! `metric.jsonl`. The wrappers exist so the spans are NAMED and discoverable even if
//! a caller does not otherwise enter them — they forward 1:1 to the underlying function
//! and add only a `tracing` span (zero logic change, zero money/FSM surface touched).

#[cfg(feature = "telemetry")]
use tracing::instrument;

/// Wrapper: `router::route` — CSR Dijkstra/A* shortest path.
#[cfg(feature = "telemetry")]
#[instrument(name = "route", skip_all, level = "info")]
pub fn route(
    g: &crate::router::RoadGraph,
    src: usize,
    dst: usize,
    heuristic: bool,
    shortcuts: &[crate::router::Shortcut],
) -> Option<(Vec<usize>, f64)> {
    crate::router::route(g, src, dst, heuristic, shortcuts)
}

/// Wrapper: `EventLog::commit_after_decide` — decide-before-commit law pole.
#[cfg(feature = "telemetry")]
#[instrument(name = "commit_after_decide", skip_all, level = "info")]
pub fn commit_after_decide<S, D, T, E>(
    log: &mut crate::event_log::EventLog<S>,
    ev: crate::event_log::MeshEvent,
    decide: D,
) -> Result<(crate::event_log::AppendOutcome, Option<T>), crate::event_log::CommitError>
where
    S: crate::event_log::EventStore,
    D: FnOnce(&crate::event_log::MeshEvent) -> Result<T, E>,
    E: std::fmt::Display,
{
    log.commit_after_decide(ev, decide)
}

/// Wrapper: `ports::payment::decide_settlement` — courier settlement decision.
#[cfg(feature = "telemetry")]
#[instrument(name = "decide_settlement", skip_all, level = "info")]
pub fn decide_settlement<V: crate::ports::agent::SignatureVerifier>(
    state: &crate::ports::payment::SettlementState,
    att: &crate::ports::payment::CashAttestation,
    auth: &crate::ports::payment::SettlementAuth<'_, V>,
) -> crate::ports::payment::SettlementOutcome {
    crate::ports::payment::decide_settlement(state, att, auth)
}

/// Wrapper: `ports::agent::cap::verify_chain` — capability chain verification.
#[cfg(feature = "telemetry")]
#[instrument(name = "cap_verify_chain", skip_all, level = "info")]
pub fn verify_chain<V: crate::ports::agent::SignatureVerifier>(
    verifier: &V,
    roster: &crate::ports::agent::AnchorRoster,
    chain: &[crate::ports::agent::Delegation],
    cap: &crate::ports::agent::Capability,
    now: u64,
) -> Result<(), crate::ports::agent::ChainError> {
    crate::ports::agent::cap::verify_chain(verifier, roster, chain, cap, now)
}

/// Wrapper: `pq::dsa::verify` (ML-DSA-65 signature verify) — behind `pq` only.
#[cfg(all(feature = "telemetry", feature = "pq"))]
#[instrument(name = "mldsa_verify", skip_all, level = "info")]
pub fn mldsa_verify(
    pk: &crate::pq::dsa::MlDsa65Pk,
    msg: &[u8],
    sig: &crate::pq::dsa::MlDsa65Sig,
) -> bool {
    crate::pq::dsa::verify(pk, msg, sig)
}
