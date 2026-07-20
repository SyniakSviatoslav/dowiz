//! telemetry/instrument.rs — P83 Layer 1 span wrappers for the 8 verified functions.
//!
//! SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 names exactly eight functions to
//! instrument with production span metrics. Three already carry spans in the kernel source
//! (`place_order`, `place_order_priced`, `fold_transitions`); this module adds the
//! remaining five as thin wrappers so the `SpanMetricsObserver` (obs.rs) can measure them
//! without touching the functions' bodies or their red-line logic:
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
//! Cutover note (roadmap items 4+29): the `#[tracing::instrument]` attribute is retired —
//! each wrapper now opens the span EXPLICITLY as its first body line
//! (`let _g = crate::fdr::info_span!("<name>").entered();`). This deletes the last consumer
//! of `tracing-attributes` (and with it the `proc-macro2`/`quote`/`syn` toolchain that only
//! ever served these 6 one-line wrappers). The `mldsa_verify` wrapper that used to be
//! DUPLICATED here and in `mldsa.rs` is now the single surviving copy (`mldsa.rs` deleted).
//!
//! The wrappers are `#[cfg(feature = "telemetry")]` so the shipping binary pays zero cost:
//! with the feature OFF, callers use the real functions directly; with it ON, `obs`'s
//! `init` installs the observer that records these spans to `metric.jsonl`. The wrappers
//! forward 1:1 to the underlying function (zero logic change, zero money/FSM surface).

/// Wrapper: `router::route` — CSR Dijkstra/A* shortest path.
#[cfg(feature = "telemetry")]
pub fn route(
    g: &crate::router::RoadGraph,
    src: usize,
    dst: usize,
    heuristic: bool,
    shortcuts: &[crate::router::Shortcut],
) -> Option<(Vec<usize>, f64)> {
    let _g = crate::fdr::info_span!("route").entered();
    crate::router::route(g, src, dst, heuristic, shortcuts)
}

/// Wrapper: `EventLog::commit_after_decide` — decide-before-commit law pole.
#[cfg(feature = "telemetry")]
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
    let _g = crate::fdr::info_span!("commit_after_decide").entered();
    log.commit_after_decide(ev, decide)
}

/// Wrapper: `ports::payment::decide_settlement` — courier settlement decision.
#[cfg(feature = "telemetry")]
pub fn decide_settlement<V: crate::ports::agent::SignatureVerifier>(
    state: &crate::ports::payment::SettlementState,
    att: &crate::ports::payment::CashAttestation,
    auth: &crate::ports::payment::SettlementAuth<'_, V>,
) -> crate::ports::payment::SettlementOutcome {
    let _g = crate::fdr::info_span!("decide_settlement").entered();
    crate::ports::payment::decide_settlement(state, att, auth)
}

/// Wrapper: `ports::agent::cap::verify_chain` — capability chain verification.
#[cfg(feature = "telemetry")]
pub fn verify_chain<V: crate::ports::agent::SignatureVerifier>(
    verifier: &V,
    roster: &crate::ports::agent::AnchorRoster,
    chain: &[crate::ports::agent::Delegation],
    cap: &crate::ports::agent::Capability,
    now: u64,
) -> Result<(), crate::ports::agent::ChainError> {
    let _g = crate::fdr::info_span!("cap_verify_chain").entered();
    crate::ports::agent::cap::verify_chain(verifier, roster, chain, cap, now)
}

/// Wrapper: `pq::dsa::verify` (ML-DSA-65 signature verify).
///
/// Item 61 (blueprint §3(d), resolution (i)): this wrapper previously carried a
/// `#[cfg(all(feature = "telemetry", feature = "pq"))]` double-gate, so a `pq`-only
/// production build (crypto ON, telemetry OFF) emitted ZERO crypto-latency telemetry —
/// a silent dark zone on the signature-verify path (gap G8). Fix: compile under `pq`
/// ALONE (the span handle is zero-cost to construct; `info_span!` takes no clock and
/// never allocates). The ring-record *emission* stays behind the cheap runtime
/// `SINK_ACTIVE` load (`fdr::emit_span_close` early-returns when no sink is installed),
/// so a `pq`-only build still pays only that one relaxed atomic load — closing the dark
/// zone with near-zero cost.
///
/// The span brackets the whole verify and does NOT branch on the verify result mid-call
/// (it forwards 1:1 to `pq::dsa::verify`), so it adds no timing side-channel (item 61
/// §4.2 / §7 accepted risk). This is the SINGLE surviving `mldsa_verify` wrapper (the
/// duplicate `mldsa.rs` was deleted in the items-4+29 cutover).
#[cfg(feature = "pq")]
pub fn mldsa_verify(
    pk: &crate::pq::dsa::MlDsa65Pk,
    msg: &[u8],
    sig: &crate::pq::dsa::MlDsa65Sig,
) -> bool {
    let _g = crate::fdr::info_span!("mldsa_verify").entered();
    crate::pq::dsa::verify(pk, msg, sig)
}
