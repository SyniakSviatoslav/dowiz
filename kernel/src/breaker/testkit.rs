//! `breaker/testkit.rs` — Phase-3 red-team harness *interface* (Blueprint A §7).
//!
//! The corpus wiring (the actual attack-case store + `kernel/src/bin/breaker_replay.rs`
//! CLI) is **deferred** to Phase-3; what ships now is the *seam*: `drive`/`inject`/
//! `snapshot`/`audit_drain` over a `Breaker`, so the harness is not retrofitted later.
//! Gated behind `#[cfg(any(test, feature = "breaker-testkit"))]` so production binaries
//! carry no attack-injection symbols (same discipline as `chaos`).
//!
//! Pure `std`, zero external dependencies.

use crate::breaker::audit::{AuditChain, AuditEvent, AuditKind};
use crate::breaker::signal::SignalVector;
use crate::breaker::state::{
    manual_reset, BreakerRecord, BreakerState, ManualResetProof, StepOutcome,
};
use crate::breaker::thresholds::ThresholdId;

/// Excess delta above θ_open / θ_kill that guarantees `trip_score` crosses the
/// trigger threshold (closed→Open / Open→Killed). Hardcoded as `0.5` because
/// the clamped [0,1] trip_score is always ≥ 1.0 at θ_open+0.5 when θ_open ≤ 0.5.
pub const TRIP_EXCESS: f32 = 0.5;

#[cfg(test)]
pub fn test_rate_profile() -> crate::breaker::thresholds::RateProfile {
    crate::breaker::thresholds::RateProfile {
        w_consec: 3,
        w_kill: 5,
        probes: 4,
        cooldown_base: 8,
        cooldown_cap: 1024,
    }
}

#[cfg(test)]
pub fn test_roc_bounds() -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    (0..20, 20..40)
}

/// A single attack-corpus row (Phase-3 pushes real rows here; item-9 leaves it
/// inert — the harness drives only via `SignalVector`).
#[derive(Debug, Clone, Copy)]
pub struct AttackCase {
    pub tag: u8,
    pub signal: SignalVector,
}

/// The Phase-3 red-team harness seam. Wraps a `BreakerRecord` + its audit chain so
/// a test/red-team driver can step the breaker one window, inject an attack case,
/// snapshot the record, and drain the audit trail.
pub struct Harness {
    rec: BreakerRecord,
    audit: AuditChain,
    window: u64,
}

impl Harness {
    /// Build a harness around a fresh Closed record under `tid`.
    pub fn new(agent_id: [u8; 16], tid: ThresholdId) -> Self {
        let rec = crate::breaker::state::new_record(agent_id, 0, tid);
        let audit = AuditChain::new(agent_id, None);
        Harness {
            rec,
            audit,
            window: 0,
        }
    }

    /// Step the breaker one window with the given signal (the `drive` seam).
    pub fn drive(&mut self, sig: SignalVector) -> BreakerState {
        self.window += 1;
        let out: StepOutcome = crate::breaker::state::step(self.rec, &sig, None);
        if let Some(cause) = out.trip {
            let _ = self.audit.append(
                match cause {
                    crate::breaker::state::TripCause::ProbeMismatch => AuditKind::ProbeResult,
                    _ => AuditKind::Transition,
                },
                self.rec.state,
                out.rec.state,
                sig.trip_score(),
                0,
            );
        }
        self.rec = out.rec;
        self.rec.state
    }

    /// Apply an attack-corpus row (the `inject` seam). Phase-3: a real
    /// `AttackCase` would map onto a `SignalVector`; item-9 just drives the signal.
    pub fn inject(&mut self, case: &AttackCase) -> BreakerState {
        self.drive(case.signal)
    }

    /// Current record (the `snapshot` seam).
    pub fn snapshot(&self) -> BreakerRecord {
        self.rec
    }

    /// Drain the audit trail (the `audit_drain` seam).
    pub fn audit_drain(&mut self) -> Vec<AuditEvent> {
        self.audit.drain()
    }

    /// Manual reset via a test-only proof (the follow-on human-gate seam).
    pub fn manual_reset(&mut self, new_agent_id: [u8; 16], proof: ManualResetProof) {
        if let Ok(fresh) = manual_reset(self.rec, new_agent_id, self.window, proof) {
            self.rec = fresh;
        }
    }
}
