//! `breaker/audit.rs` — hash-chained, tamper-evident audit over the **existing**
//! FDR ring (Blueprint A §2.4 / item-9 §3 row 5).
//!
//! The breaker does NOT build a second ring: it shares `fdr::ring::FdrRing` (the
//! Tier-1 flight recorder — synthesis §5: "the logger *is* the flight recorder").
//! Each breaker audit event is an FDR `Kind::Alarm` record carrying a
//! `prev_hash`/`seq` hash chain computed via `event_log::sha3_256`, so a deleted
//! or reordered entry is detectable. **Backpressure = stall, never drop** (§9): if
//! the durable ring cannot accept the entry (the drain sink has fallen behind and
//! the segment is at capacity), `tick` blocks rather than silently dropping the
//! audit row — losing an entry defeats the ring's tamper-evidence purpose.
//!
//! Pure `core` + `alloc` (no_std). The audit chain is in-memory too (for
//! the testkit `audit_drain` seam and for fast in-process verification), AND
//! mirrored to the durable FDR ring when one is installed.

use crate::breaker::state::BreakerState;
use crate::event_log::sha3_256;
use alloc::boxed::Box;
use alloc::vec::Vec;

/// The audit event kind (carried as an FDR field string, not via `fdr::Kind` —
/// `Kind::Alarm` already exists; these are the breaker *sub*-kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    /// A signal-only audit (calm tick).
    Signal,
    /// A state transition.
    Transition,
    /// A replay-probe result (HalfOpen).
    ProbeResult,
    /// A kill (terminal).
    Kill,
    /// A red-line gate assertion.
    RedLineGate,
    /// Truthfulness probe disarmed (detreduce deferred — item-9 non-goal).
    Disarm,
}

impl AuditKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditKind::Signal => "signal",
            AuditKind::Transition => "transition",
            AuditKind::ProbeResult => "probe_result",
            AuditKind::Kill => "kill",
            AuditKind::RedLineGate => "red_line_gate",
            AuditKind::Disarm => "disarm",
        }
    }
}

/// One hash-chained audit event. `prev_hash` links to the prior event; `self_hash`
/// binds `prev_hash ‖ seq ‖ body`, so deletion or reordering breaks the chain.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub seq: u64,
    pub prev_hash: [u8; 32],
    pub ts_millis: u64,
    pub agent_id: [u8; 16],
    pub kind: AuditKind,
    pub state_from: BreakerState,
    pub state_to: BreakerState,
    pub score: f32,
    pub self_hash: [u8; 32],
}

/// Compute `self_hash = sha3(prev_hash ‖ seq ‖ kind ‖ from ‖ to ‖ score_bytes)`.
fn compute_self_hash(
    prev_hash: &[u8; 32],
    seq: u64,
    kind: AuditKind,
    state_from: BreakerState,
    state_to: BreakerState,
    score: f32,
) -> [u8; 32] {
    let mut buf = Vec::with_capacity(64 + 16);
    buf.extend_from_slice(prev_hash);
    buf.extend_from_slice(&seq.to_le_bytes());
    buf.extend_from_slice(kind.as_str().as_bytes());
    buf.push(state_from as u8);
    buf.push(state_to as u8);
    buf.extend_from_slice(&score.to_le_bytes());
    sha3_256(&buf)
}

/// A durable audit mirror (the kernel's FDR ring impls this). The core
/// [`AuditChain`] is pure in-memory; when a mirror is installed, every appended
/// event is mirrored to it BEFORE being committed (backpressure = stall, never drop).
pub trait AuditMirror {
    /// Mirror one audit event durably. `Err` = the durable write failed; the
    /// caller must treat it as a hard stop (never proceed without the row).
    fn mirror(&mut self, ev: &AuditEvent) -> Result<(), AuditError>;
}

/// The in-memory hash-chained audit ledger. Holds the last event's `self_hash` as
/// the chaining tip. Mirrors each event to an optional [`AuditMirror`].
pub struct AuditChain {
    events: Vec<AuditEvent>,
    tip_hash: [u8; 32],
    /// Optional durable mirror (the kernel's FDR ring impls [`AuditMirror`]).
    /// `None` ⇒ audit-only in memory.
    mirror: Option<Box<dyn AuditMirror>>,
    agent_id: [u8; 16],
}

impl AuditChain {
    /// Create a fresh chain (genesis tip_hash = zero), audit-only in memory.
    pub fn new(agent_id: [u8; 16]) -> Self {
        AuditChain {
            events: Vec::new(),
            tip_hash: [0u8; 32],
            mirror: None,
            agent_id,
        }
    }

    /// Create a chain that also mirrors every event to a durable mirror
    /// (the kernel's FDR ring impls [`AuditMirror`]). Backpressure = stall.
    pub fn with_mirror(agent_id: [u8; 16], mirror: Box<dyn AuditMirror>) -> Self {
        AuditChain {
            events: Vec::new(),
            tip_hash: [0u8; 32],
            mirror: Some(mirror),
            agent_id,
        }
    }

    /// Number of events recorded.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Append one audit event, chaining `prev_hash` from the current tip and
    /// computing `self_hash`. Mirrors to the durable FDR ring if installed.
    ///
    /// **Backpressure (§9):** if a durable ring is installed and its `append`
    /// returns `Err`, this call **stalls** (returns `Err`) rather than dropping
    /// the entry silently. The caller's `tick` must treat a returned `Err` as a
    /// hard stop — never an opportunity to proceed without the audit row.
    pub fn append(
        &mut self,
        kind: AuditKind,
        state_from: BreakerState,
        state_to: BreakerState,
        score: f32,
        ts_millis: u64,
    ) -> Result<(), AuditError> {
        let seq = self.events.len() as u64;
        let prev = self.tip_hash;
        let self_hash = compute_self_hash(&prev, seq, kind, state_from, state_to, score);
        let ev = AuditEvent {
            seq,
            prev_hash: prev,
            ts_millis,
            agent_id: self.agent_id,
            kind,
            state_from,
            state_to,
            score,
            self_hash,
        };

        // STALL, never drop: mirror the event durably BEFORE committing it, so a
        // durable-write failure surfaces instead of proceeding without the row.
        if let Some(mirror) = self.mirror.as_mut() {
            mirror.mirror(&ev)?;
        }

        self.tip_hash = self_hash;
        self.events.push(ev);
        Ok(())
    }

    /// Drain + verify the in-memory chain: returns `Ok` only if `prev_hash` of
    /// every event equals the `self_hash` of its predecessor (genesis tip zero).
    /// A tampered `prev_hash` or `seq` gap breaks verification.
    pub fn verify_chain(&self) -> Result<(), ChainDefect> {
        let mut expected_prev: [u8; 32] = [0u8; 32];
        for (i, ev) in self.events.iter().enumerate() {
            if ev.seq != i as u64 {
                return Err(ChainDefect::SeqGap {
                    at: ev.seq,
                    expected: i as u64,
                });
            }
            if ev.prev_hash != expected_prev {
                return Err(ChainDefect::HashBreak {
                    at: ev.seq,
                    expected: expected_prev,
                    got: ev.prev_hash,
                });
            }
            // Recompute self_hash independently and compare.
            let recomputed = compute_self_hash(
                &ev.prev_hash,
                ev.seq,
                ev.kind,
                ev.state_from,
                ev.state_to,
                ev.score,
            );
            if recomputed != ev.self_hash {
                return Err(ChainDefect::SelfHashMismatch { at: ev.seq });
            }
            expected_prev = ev.self_hash;
        }
        // Tip must chain forward from the last event's self_hash.
        if let Some(last) = self.events.last() {
            if expected_prev != last.self_hash {
                return Err(ChainDefect::TipMismatch);
            }
        }
        Ok(())
    }

    /// Drain all in-memory events (consumed by the testkit `audit_drain` seam).
    pub fn drain(&mut self) -> Vec<AuditEvent> {
        core::mem::take(&mut self.events)
    }
}

/// Durable-write failure (backpressure sentinel): the caller must stall, not drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    RingPoisoned,
    RingWrite,
}

/// A tamper / loss detected while verifying the audit chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainDefect {
    SeqGap {
        at: u64,
        expected: u64,
    },
    HashBreak {
        at: u64,
        expected: [u8; 32],
        got: [u8; 32],
    },
    SelfHashMismatch {
        at: u64,
    },
    TipMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_verifies_when_intact() {
        let mut c = AuditChain::new([1u8; 16]);
        c.append(
            AuditKind::Signal,
            BreakerState::Closed,
            BreakerState::Closed,
            0.0,
            0,
        )
        .unwrap();
        c.append(
            AuditKind::Transition,
            BreakerState::Closed,
            BreakerState::Open,
            1.0,
            1,
        )
        .unwrap();
        c.append(
            AuditKind::Kill,
            BreakerState::Open,
            BreakerState::Killed,
            2.0,
            2,
        )
        .unwrap();
        assert_eq!(c.len(), 3);
        assert!(c.verify_chain().is_ok(), "intact chain must verify");
    }

    #[test]
    fn chain_breaks_on_tampered_prev_hash() {
        let mut c = AuditChain::new([1u8; 16]);
        c.append(
            AuditKind::Signal,
            BreakerState::Closed,
            BreakerState::Closed,
            0.0,
            0,
        )
        .unwrap();
        c.append(
            AuditKind::Transition,
            BreakerState::Closed,
            BreakerState::Open,
            1.0,
            1,
        )
        .unwrap();
        // Tamper: smash event[1].prev_hash so it no longer equals event[0].self_hash.
        c.events[1].prev_hash = [9u8; 32];
        assert!(matches!(
            c.verify_chain(),
            Err(ChainDefect::HashBreak { .. })
        ));
    }

    #[test]
    fn chain_breaks_on_seq_gap() {
        let mut c = AuditChain::new([1u8; 16]);
        c.append(
            AuditKind::Signal,
            BreakerState::Closed,
            BreakerState::Closed,
            0.0,
            0,
        )
        .unwrap();
        c.append(
            AuditKind::Transition,
            BreakerState::Closed,
            BreakerState::Open,
            1.0,
            1,
        )
        .unwrap();
        // Tamper: reorder / gap by bumping event[1].seq.
        c.events[1].seq = 5;
        assert!(matches!(c.verify_chain(), Err(ChainDefect::SeqGap { .. })));
    }

}
