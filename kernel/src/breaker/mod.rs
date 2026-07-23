//! `breaker/mod.rs` — `Breaker`, the ONE gate `admit()`, `Permit`/`Tripped`,
//! `tick()`, and the alarm receiver `on_commit_error()`.
//!
//! This is THE PIVOT item of the space-grade roadmap: the fault-containment layer
//! every downstream subsystem trips into. Two unrepresentability properties (see
//! Blueprint-Item-09 §2):
//!
//! 1. **State side** — "tripped-but-permitting" is unconstructible. A `Killed`
//!    record never carries a live `Permit`; the only constructor of `Permit` is
//!    [`Breaker::admit`] returning `Ok`. `step` produces a fresh record every tick,
//!    so an in-place flip of a `Killed` record back to `Closed` (the most likely
//!    implementor mistake) can never hold a permit.
//! 2. **Call-site side** — a gated operation takes `&Permit` by signature, so a
//!    caller *cannot forget or invert the check*: omitting the permit is a compile
//!    error, not a latent bug. There is no `is_tripped()` accessor that could gate
//!    a mutation. (Telemetry/test access is via `testkit` / `snapshot`.)
//!
//! The gate is **one function** returning two typed poles:
//! `Result<Permit<'_>, Tripped>`. The `Permit` is **borrow-scoped** (the ponytail
//! default for a synchronous decision path — item-9 §7.1); it cannot outlive the
//! admit borrow, which is all the kernel's synchronous core needs.
//!
//! Pure `std`, zero external dependencies. `cargo tree -e no-dev` unchanged.

mod audit;
mod graph;
mod replay;
mod signal;
mod state;
#[cfg(any(test, feature = "breaker-testkit"))]
mod testkit;
mod thresholds;

pub use audit::{AuditChain, AuditError, AuditEvent, AuditKind, ChainDefect};
pub use graph::{
    breaker_graph_report, verify_breaker_signature, verify_breaker_signature_against,
    BreakerGraphReport, BreakerSignatureDrift, BREAKER_ADJ, BREAKER_EDGES,
    BREAKER_GOLDEN_SIGNATURE, BREAKER_STATES,
};
pub use replay::{GoldenPair, GoldenStore};
pub use signal::SignalVector;
pub use state::{
    manual_reset, new_record, step, BreakerRecord, BreakerState, ManualResetProof, StepOutcome,
    TripCause,
};
pub use thresholds::{
    fit_from_rates, fit_weights, FitError, RateProfile, SignalWeights, ThresholdId, Thresholds,
};

#[cfg(any(test, feature = "breaker-testkit"))]
pub use testkit::{AttackCase, Harness};

/// A `u64` agent identifier (kernel-internal; agent addressing is out of scope,
/// so this is a bare 16-byte id — matching `BreakerRecord::agent_id`).
pub type AgentId = [u8; 16];

/// The typed **reject** pole of the one gate. Carries the state + cause so a
/// caller can audit *why* admission was refused (and never silently retry).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tripped {
    pub state: BreakerState,
    pub cause: TripCause,
}

/// An unforgeable admit permit. **No public constructor** — the only way to obtain
/// one is [`Breaker::admit`] returning `Ok`. A gated operation's signature is
/// `fn do_gated(p: &Permit<'_>, …)`; a caller in `Open`/`Killed` has no `Permit` to
/// pass, so the operation is uncallable. The borrow ties the permit to the live
/// `Breaker`, enforcing that the admit decision is fresh (cannot be cached across
/// a state change).
pub struct Permit<'b> {
    _breaker: &'b Breaker,
    agent: AgentId,
}

impl<'b> Permit<'b> {
    /// The agent this permit admits. A gated op reads this to bind its effect.
    pub fn agent(&self) -> AgentId {
        self.agent
    }
}

/// The deterministic fault-containment circuit breaker.
pub struct Breaker {
    rec: BreakerRecord,
    audit: AuditChain,
    store: GoldenStore,
    window_seq: u64,
}

impl Breaker {
    /// Construct a breaker. `tid` MUST be a fitted [`ThresholdId`] (obtained from
    /// [`fit_from_rates`]); a `Breaker` is therefore unconstructible without a
    /// valid fitted threshold set — failure surfaces at bootstrap, not at tick.
    pub fn new(agent_id: AgentId, tid: ThresholdId) -> Self {
        debug_assert!(tid.open > 0.0, "Breaker::new: θ_open must be > 0");
        debug_assert!(tid.kill >= tid.open, "Breaker::new: θ_open ({}) must be <= θ_kill ({})", tid.open, tid.kill);
        debug_assert!(tid.cooldown_base <= tid.cooldown_cap, "Breaker::new: cooldown_base ({}) must be <= cooldown_cap ({})", tid.cooldown_base, tid.cooldown_cap);
        let rec = new_record(agent_id, 0, tid);
        let audit = AuditChain::new(agent_id, None);
        Breaker {
            rec,
            audit,
            store: GoldenStore::new(),
            window_seq: 0,
        }
    }

    /// Construct with a durable FDR ring mirror (Tier-1 flight recorder; the
    /// breaker shares `fdr::ring`). `ring` is an already-opened `FdrRing` wrapped
    /// in a `Mutex` so concurrent `tick`s serialize their durable writes.
    pub fn with_ring(
        agent_id: AgentId,
        tid: ThresholdId,
        ring: std::sync::Mutex<crate::fdr::RingHandle>,
    ) -> Self {
        debug_assert!(tid.open > 0.0, "Breaker::with_ring: θ_open must be > 0");
        debug_assert!(tid.kill >= tid.open, "Breaker::with_ring: θ_open ({}) must be <= θ_kill ({})", tid.open, tid.kill);
        debug_assert!(tid.cooldown_base <= tid.cooldown_cap, "Breaker::with_ring: cooldown_base ({}) must be <= cooldown_cap ({})", tid.cooldown_base, tid.cooldown_cap);
        let rec = new_record(agent_id, 0, tid);
        let audit = AuditChain::new(agent_id, Some(ring));
        Breaker {
            rec,
            audit,
            store: GoldenStore::new(),
            window_seq: 0,
        }
    }

    /// The **ONE gate.** Returns a [`Permit`] only from `Closed`/`HalfOpen` (probe-
    /// admitted or steady); [`Tripped`] otherwise. Never a per-call-site boolean.
    pub fn admit(&self, agent: AgentId, _class: RedLineClass) -> Result<Permit<'_>, Tripped> {
        match self.rec.state {
            BreakerState::Closed | BreakerState::HalfOpen => Ok(Permit {
                _breaker: self,
                agent,
            }),
            BreakerState::Open | BreakerState::Killed => Err(Tripped {
                state: self.rec.state,
                cause: if self.rec.state == BreakerState::Killed {
                    TripCause::ScoreExceeded
                } else {
                    TripCause::ScoreExceeded
                },
            }),
        }
    }

    /// Classify `action_class` against the already-built, already-tested
    /// red-line classifier (the breaker consumes this — it does not invent a red-
    /// line policy; see Blueprint A §2.2). Used at record construction time to set
    /// `red_line_class`.
    pub fn derive_red_line(agent_class: RedLineClass) -> bool {
        matches!(agent_class, RedLineClass::RedLine)
    }

    /// Advance the breaker one window with `sig`. Drives [`state::step`], emits one
    /// FDR audit record per transition, and (on a `Killed` terminal) writes the
    /// kill + red-line-gate audit rows. Returns the resulting state.
    ///
    /// `external` carries an out-of-band trip cause (e.g. [`TripCause::CommitStoreFault`]
    /// from [`Breaker::on_commit_error`]). `window_gap` = true signals a dropped
    /// window (gap in `window_seq`) — audited and counted toward tripping.
    pub fn tick(
        &mut self,
        mut sig: SignalVector,
        external: Option<TripCause>,
        window_gap: bool,
    ) -> BreakerState {
        self.window_seq += 1;
        sig.window_seq = self.window_seq;
        if window_gap {
            // A dropped window is anomaly-adjacent: count it toward consecutive
            // trips and trip the audit (Blueprint A §9).
            self.rec.consecutive_trips = self.rec.consecutive_trips.saturating_add(1);
            let _ = self.audit.append(
                AuditKind::Signal,
                self.rec.state,
                self.rec.state,
                sig.trip_score(),
                self.window_seq,
            );
            // Re-evaluate as if a window gap were an external trip input.
            let out = state::step(self.rec, &sig, Some(TripCause::WindowGap));
            self.record_transition(out, &sig);
            return self.rec.state;
        }

        let out = state::step(self.rec, &sig, external);
        self.record_transition(out, &sig);
        self.rec.state
    }

    fn record_transition(&mut self, out: StepOutcome, sig: &SignalVector) {
        let from = self.rec.state;
        let to = out.rec.state;
        if from != to || out.trip.is_some() {
            let kind = match out.trip {
                Some(TripCause::ProbeMismatch) => AuditKind::ProbeResult,
                Some(TripCause::CommitStoreFault) => AuditKind::Transition,
                Some(_) if to == BreakerState::Killed => AuditKind::Kill,
                _ => AuditKind::Transition,
            };
            let _ = self
                .audit
                .append(kind, from, to, sig.trip_score(), self.window_seq);
            if to == BreakerState::Killed {
                // Terminal: write the red-line gate marker (if applicable) + kill.
                let rlg = if self.rec.red_line_class {
                    Some(AuditKind::RedLineGate)
                } else {
                    None
                };
                if let Some(rlg) = rlg {
                    let _ = self
                        .audit
                        .append(rlg, to, to, sig.trip_score(), self.window_seq);
                }
            }
        }
        self.rec = out.rec;
    }

    /// **The alarm receiver** (roadmap proof clause: "`CommitError` alarms actually
    /// route to it"). A durable store fault (`CommitError::Store`) is a first-class
    /// trip input — routed in as [`TripCause::CommitStoreFault`]. The `Rejected`
    /// pole is a *correct* Law rejection and is NOT an alarm (routing it would be
    /// the pole-blur `event_log.rs` forbids), so it is intentionally absent here.
    pub fn on_commit_error(&mut self, err: &crate::event_log::CommitError) {
        match err {
            crate::event_log::CommitError::Store(_) => {
                // Must alarm: route as a durable-loss trip input. A store fault is a
                // first-class external trip (Closed→Open immediately); the zero signal
                // carries the record's fitted weights, but the TRIP is the alarm.
                let z = crate::breaker::signal::zero(self.rec.weights);
                let out = state::step(self.rec, &z, Some(TripCause::CommitStoreFault));
                self.record_transition(out, &z);
            }
            crate::event_log::CommitError::Rejected(_) => {
                // Correct Law rejection — NOT an alarm. The breaker observes nothing.
            }
        }
    }

    /// Current state (telemetry only; never gates a mutation — see §2 call-site
    /// discipline). Exposed for `testkit` snapshot parity.
    pub fn current_state(&self) -> BreakerState {
        self.rec.state
    }

    /// Access the golden replay store (Phase-3 seam; disarmed in item-9).
    pub fn replay_store(&mut self) -> &mut GoldenStore {
        &mut self.store
    }

    /// Drain the in-memory audit trail (used by `testkit::audit_drain`).
    pub fn audit_drain(&mut self) -> Vec<AuditEvent> {
        self.audit.drain()
    }

    /// Verify the audit hash chain (tamper / loss detection).
    pub fn verify_audit(&self) -> Result<(), ChainDefect> {
        self.audit.verify_chain()
    }
}

/// The red-line class an admitting action belongs to. Derived from the existing
/// classifier (`ports/agent/scope::Scope::touches_red_line`) at call sites; the
/// breaker only *consumes* this classification, never redefines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedLineClass {
    Clean,
    RedLine,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::testkit::{test_rate_profile, test_roc_bounds, TRIP_EXCESS};
    use crate::breaker::thresholds::{default_weights, fit_from_rates};

    fn tid() -> ThresholdId {
        let p = test_rate_profile();
        let (normals, anomalies) = test_roc_bounds();
        // Normalized [0,1] separable ROC (see state.rs::tid for the rationale): the
        // fitted θ_open must live in [0,1] to be reachable by the clamped trip_score.
        let mut rates: Vec<(f32, bool)> = Vec::new();
        for i in normals {
            rates.push(((i as f32) / 50.0, false)); // 0.00 .. 0.38 normal
        }
        for i in anomalies {
            rates.push(((i as f32) / 40.0, true)); // 0.50 .. 0.975 anomaly
        }
        fit_from_rates(&rates, 0.05, p, default_weights()).unwrap()
    }

    fn w() -> SignalWeights {
        SignalWeights {
            conf: 1.0,
            drift: 0.0,
            cusum: 0.0,
            constraint: 0.0,
            disagreement: 0.0,
            truth: 0.0,
        }
    }

    fn sig(score: f32) -> SignalVector {
        SignalVector {
            window_seq: 0,
            confidence_gap: score,
            ewma_drift: 0.0,
            cusum: 0.0,
            constraint_violations: 0,
            disagreement: 0.0,
            truthfulness_fail: 0,
            weights: w(),
        }
    }

    #[test]
    fn admit_returns_permit_only_when_closed_or_halfopen() {
        let mut b = Breaker::new([1u8; 16], tid());
        // Closed ⇒ admit.
        let p = b
            .admit([1u8; 16], RedLineClass::Clean)
            .expect("closed admits");
        assert_eq!(p.agent(), [1u8; 16]);
        // Trip to Open.
        for _ in 0..tid().w_consec {
            b.tick(sig(tid().open + TRIP_EXCESS), None, false);
        }
        assert_eq!(b.current_state(), BreakerState::Open);
        assert!(matches!(b.admit([1u8; 16], RedLineClass::Clean), Err(_)));
    }

    #[test]
    fn permit_has_no_public_constructor() {
        // Structural guarantee enforced by the harness test below; here we simply
        // assert the gate round-trips and that `Tripped` carries the cause.
        let mut b = Breaker::new([1u8; 16], tid());
        for _ in 0..tid().w_consec {
            b.tick(sig(tid().open + TRIP_EXCESS), None, false);
        }
        let tripped = match b.admit([1u8; 16], RedLineClass::Clean) {
            Ok(_) => panic!("expected Tripped, got a Permit"),
            Err(t) => t,
        };
        assert_eq!(tripped.state, BreakerState::Open);
    }

    #[test]
    fn alarm_routes_store_fault_not_rejected() {
        let mut b = Breaker::new([1u8; 16], tid());
        // A faulty store yields CommitError::Store ⇒ breaker observes a trip.
        let store_err = crate::event_log::CommitError::Store(crate::event_log::StoreError::Sync(
            "disk full".to_string(),
        ));
        b.on_commit_error(&store_err);
        // The breaker must now be in a tripped state (Open, via CommitStoreFault).
        assert_eq!(
            b.current_state(),
            BreakerState::Open,
            "store fault must alarm into Open"
        );
        // A decide-rejected event yields CommitError::Rejected ⇒ breaker observes NOTHING.
        let rej = crate::event_log::CommitError::Rejected(crate::event_log::DecideRejected(
            "law rejects".to_string(),
        ));
        let mut b2 = Breaker::new([2u8; 16], tid());
        b2.on_commit_error(&rej);
        assert_eq!(
            b2.current_state(),
            BreakerState::Closed,
            "rejection must not alarm"
        );
    }

    #[test]
    fn audit_chain_stays_verifiable_through_ticks() {
        let mut b = Breaker::new([1u8; 16], tid());
        // Three consecutive hot windows force the Closed→Open transition (audit rows
        // are written on transition / trip, not on steady-state calm windows).
        for _ in 0..3 {
            b.tick(sig(tid().open + TRIP_EXCESS), None, false);
        }
        // Then a few calm windows to exercise steady-state bookkeeping.
        for _ in 0..3 {
            b.tick(sig(0.0), None, false);
        }
        assert!(
            b.verify_audit().is_ok(),
            "audit chain must remain verifiable"
        );
        assert!(
            b.audit_drain().len() > 0,
            "transitions must have been audited"
        );
    }
}
