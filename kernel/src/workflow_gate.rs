//! workflow_gate.rs — Structural enforcement of the mandatory agent workflow sequence.
//!
//! # What this is
//! A kernel primitive that TRACKS and ENFORCES the research -> synthesis -> critique ->
//! plan -> critique -> work -> verify -> critique -> commit sequence. Unlike the cultural
//! discipline documented in MEMORY.md, this is a typed state machine that structurally
//! prevents skipping phases.
//!
//! # Why this exists (Ananke applied)
//! The 2-question doubt check (2026-07-21) found that workflow gates were purely
//! behavioral — no kernel code caught an agent skipping critique or verification.
//! This module closes that gap by making phase completion a typed, auditable, and
//! structurally enforced contract.
//!
//! # Design
//! - `WorkflowGate` tracks which phases have completed via a bitmask
//! - `GatePhase` is a closed enum of all 9 phases
//! - `advance()` returns `Err` if the requested phase isn't the valid next step
//! - `can_commit()` returns true only when ALL required phases are complete
//! - The gate is deterministic (no RNG, no network) — pure stdlib

use std::fmt;

/// The 9 mandatory workflow phases, in order.
/// This is a CLOSED enum — new phases require a conscious edit + gate update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GatePhase {
    /// Explore codebase, read docs, understand ground truth.
    Research = 0,
    /// Combine findings into coherent understanding.
    Synthesis = 1,
    /// Challenge assumptions, find gaps, stress-test logic.
    Critique1 = 2,
    /// Produce blueprint with explicit dependencies + falsifiable checks.
    Plan = 3,
    /// Verify plan against live repo, check dependency graph.
    Critique2 = 4,
    /// Implement per blueprint, TDD (RED -> GREEN).
    Work = 5,
    /// DIFFERENT MODEL/AGENT reviews (never self-verification).
    Verify = 6,
    /// Reviewer challenges implementation, finds edge cases.
    Critique3 = 7,
    /// Evidence in commit message, save to living memory.
    Commit = 8,
}

impl GatePhase {
    /// All phases in order.
    pub const ALL: &'static [GatePhase] = &[
        GatePhase::Research,
        GatePhase::Synthesis,
        GatePhase::Critique1,
        GatePhase::Plan,
        GatePhase::Critique2,
        GatePhase::Work,
        GatePhase::Verify,
        GatePhase::Critique3,
        GatePhase::Commit,
    ];

    /// Convert to a bit position (0-8).
    fn bit(self) -> u8 {
        self as u8
    }

    /// The valid next phase, or None if this is the final phase.
    pub fn next(self) -> Option<GatePhase> {
        let idx = self.bit() as usize;
        GatePhase::ALL.get(idx + 1).copied()
    }

    /// Human-readable name for telemetry and error messages.
    pub fn as_name(self) -> &'static str {
        match self {
            GatePhase::Research => "research",
            GatePhase::Synthesis => "synthesis",
            GatePhase::Critique1 => "critique-1",
            GatePhase::Plan => "plan",
            GatePhase::Critique2 => "critique-2",
            GatePhase::Work => "work",
            GatePhase::Verify => "verify",
            GatePhase::Critique3 => "critique-3",
            GatePhase::Commit => "commit",
        }
    }
}

impl fmt::Display for GatePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_name())
    }
}

/// Error when a phase transition is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    /// The requested phase hasn't had its prerequisite completed yet.
    SkippedPhase {
        expected: GatePhase,
        requested: GatePhase,
    },
    /// The phase was already completed (idempotent advance is not allowed —
    /// each phase must be completed exactly once).
    PhaseAlreadyComplete {
        phase: GatePhase,
    },
    /// The gate is already at commit — nothing advances past commit.
    GateAlreadyClosed,
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateError::SkippedPhase { expected, requested } => {
                write!(
                    f,
                    "workflow gate: cannot advance to '{}' — must complete '{}' first",
                    requested, expected
                )
            }
            GateError::PhaseAlreadyComplete { phase } => {
                write!(
                    f,
                    "workflow gate: phase '{}' already completed — each phase runs exactly once",
                    phase
                )
            }
            GateError::GateAlreadyClosed => {
                write!(f, "workflow gate: already at commit — no further advancement")
            }
        }
    }
}

/// The workflow gate — a typed, auditable state machine for agent task execution.
///
/// # Invariants
/// - Phases complete in strict order (no skipping)
/// - Each phase completes exactly once (no repeats)
/// - `can_commit()` is true only when all 9 phases are done
/// - The gate is deterministic and pure (no RNG, no I/O)
///
/// # Usage
/// ```
/// use dowiz_kernel::workflow_gate::{WorkflowGate, GatePhase};
///
/// let mut gate = WorkflowGate::new();
///
/// // Must start with Research
/// assert!(gate.advance(GatePhase::Research).is_ok());
/// assert!(gate.advance(GatePhase::Synthesis).is_ok());
///
/// // Skipping is rejected
/// assert!(gate.advance(GatePhase::Work).is_err()); // skipped critique-1, plan, critique-2
///
/// // Must complete all phases before commit
/// assert!(!gate.can_commit());
/// ```
pub struct WorkflowGate {
    /// Bitmask of completed phases (bit i = phase i completed).
    completed: u16,
    /// The phase that was last completed (for diagnostics).
    last_completed: Option<GatePhase>,
    /// Monotonic counter of completed phases.
    count: u8,
}

impl WorkflowGate {
    /// Create a new gate with no phases completed.
    pub fn new() -> Self {
        WorkflowGate {
            completed: 0,
            last_completed: None,
            count: 0,
        }
    }

    /// Advance the gate by completing a phase.
    ///
    /// Returns `Ok(())` if the phase is the valid next step and hasn't been done yet.
    /// Returns `Err(GateError)` if the transition is invalid.
    pub fn advance(&mut self, phase: GatePhase) -> Result<(), GateError> {
        // Gate already closed — nothing past commit.
        if self.is_complete() {
            return Err(GateError::GateAlreadyClosed);
        }

        // Phase already completed — no repeats.
        if self.is_done(phase) {
            return Err(GateError::PhaseAlreadyComplete { phase });
        }

        // Determine the expected next phase.
        let expected = match self.last_completed {
            None => GatePhase::Research, // First phase must be Research
            Some(last) => last.next().expect("checked is_complete above"),
        };

        // The requested phase must match the expected next phase.
        if phase != expected {
            return Err(GateError::SkippedPhase {
                expected,
                requested: phase,
            });
        }

        // Record the completion.
        self.completed |= 1 << phase.bit();
        self.last_completed = Some(phase);
        self.count += 1;

        Ok(())
    }

    /// Check if a specific phase has been completed.
    pub fn is_done(&self, phase: GatePhase) -> bool {
        (self.completed & (1 << phase.bit())) != 0
    }

    /// Check if ALL phases are complete (gate is closed).
    pub fn is_complete(&self) -> bool {
        self.count >= GatePhase::ALL.len() as u8
    }

    /// Check if commit is allowed (all 9 phases done).
    pub fn can_commit(&self) -> bool {
        self.is_complete()
    }

    /// How many phases have been completed.
    pub fn completed_count(&self) -> u8 {
        self.count
    }

    /// The last completed phase, or None if no phase has been done.
    pub fn last_phase(&self) -> Option<GatePhase> {
        self.last_completed
    }

    /// Return the completed phases as a bitmask (for telemetry/serialization).
    pub fn bitmask(&self) -> u16 {
        self.completed
    }

    /// SHA3-256 hash of the gate's current state (byte-by-byte canonical).
    /// Used for cryptographic verification of gate integrity.
    pub fn state_hash(&self) -> [u8; 32] {
        use crate::event_log::sha3_256;
        let bytes = self.state_bytes();
        sha3_256(&bytes)
    }

    /// Canonical byte representation of the gate state (4 bytes).
    /// Layout: bitmask (2 bytes LE) + count (1 byte) + last_completed (1 byte, 0xff = None).
    pub fn state_bytes(&self) -> [u8; 4] {
        let last = match self.last_completed {
            Some(p) => p as u8,
            None => 0xff,
        };
        [
            self.completed as u8,
            (self.completed >> 8) as u8,
            self.count,
            last,
        ]
    }

    /// Verify the gate's state matches an expected hash.
    /// Returns Ok(()) if byte-identical, Err(expected, actual) if mismatch.
    pub fn verify_state(&self, expected_hash: &[u8; 32]) -> Result<(), ([u8; 32], [u8; 32])> {
        let actual = self.state_hash();
        if actual == *expected_hash {
            Ok(())
        } else {
            Err((*expected_hash, actual))
        }
    }

    /// ASCII status display for diagnostics.
    ///
    /// ```text
    /// [x] research
    /// [x] synthesis
    /// [x] critique-1
    /// [ ] plan
    /// [ ] critique-2
    /// [ ] work
    /// [ ] verify
    /// [ ] critique-3
    /// [ ] commit
    /// ```
    pub fn ascii_status(&self) -> String {
        let mut out = String::with_capacity(200);
        for &phase in GatePhase::ALL {
            let mark = if self.is_done(phase) { "x" } else { " " };
            out.push_str(&format!("[{}] {}\n", mark, phase.as_name()));
        }
        out
    }
}

impl Default for WorkflowGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_gate_has_no_completed_phases() {
        let gate = WorkflowGate::new();
        assert_eq!(gate.completed_count(), 0);
        assert!(!gate.is_complete());
        assert!(!gate.can_commit());
        assert_eq!(gate.last_phase(), None);
    }

    #[test]
    fn research_is_valid_first_phase() {
        let mut gate = WorkflowGate::new();
        assert!(gate.advance(GatePhase::Research).is_ok());
        assert!(gate.is_done(GatePhase::Research));
        assert_eq!(gate.completed_count(), 1);
    }

    #[test]
    fn cannot_skip_phases() {
        let mut gate = WorkflowGate::new();
        assert!(gate.advance(GatePhase::Research).is_ok());
        // Skip synthesis, try to jump to critique-1
        assert_eq!(
            gate.advance(GatePhase::Critique1),
            Err(GateError::SkippedPhase {
                expected: GatePhase::Synthesis,
                requested: GatePhase::Critique1,
            })
        );
    }

    #[test]
    fn cannot_repeat_phases() {
        let mut gate = WorkflowGate::new();
        assert!(gate.advance(GatePhase::Research).is_ok());
        assert_eq!(
            gate.advance(GatePhase::Research),
            Err(GateError::PhaseAlreadyComplete {
                phase: GatePhase::Research
            })
        );
    }

    #[test]
    fn full_happy_path_all_9_phases() {
        let mut gate = WorkflowGate::new();
        for &phase in GatePhase::ALL {
            assert!(gate.advance(phase).is_ok(), "failed on {}", phase);
        }
        assert!(gate.is_complete());
        assert!(gate.can_commit());
        assert_eq!(gate.completed_count(), 9);
        assert_eq!(gate.last_phase(), Some(GatePhase::Commit));
    }

    #[test]
    fn cannot_advance_past_commit() {
        let mut gate = WorkflowGate::new();
        for &phase in GatePhase::ALL {
            gate.advance(phase).unwrap();
        }
        // Try to advance again — gate is closed
        assert_eq!(gate.advance(GatePhase::Research), Err(GateError::GateAlreadyClosed));
    }

    #[test]
    fn first_phase_must_be_research() {
        let mut gate = WorkflowGate::new();
        assert_eq!(
            gate.advance(GatePhase::Plan),
            Err(GateError::SkippedPhase {
                expected: GatePhase::Research,
                requested: GatePhase::Plan,
            })
        );
    }

    #[test]
    fn can_commit_only_after_all_phases() {
        let mut gate = WorkflowGate::new();
        for &phase in &GatePhase::ALL[..7] {
            gate.advance(phase).unwrap();
        }
        // 7 phases done (research through verify), not yet commit
        assert!(!gate.can_commit());
        gate.advance(GatePhase::Critique3).unwrap();
        assert!(!gate.can_commit());
        gate.advance(GatePhase::Commit).unwrap();
        assert!(gate.can_commit());
    }

    #[test]
    fn bitmask_tracks_completed_phases() {
        let mut gate = WorkflowGate::new();
        assert_eq!(gate.bitmask(), 0);
        gate.advance(GatePhase::Research).unwrap();
        assert_eq!(gate.bitmask(), 1); // bit 0
        gate.advance(GatePhase::Synthesis).unwrap();
        assert_eq!(gate.bitmask(), 3); // bits 0+1
    }

    #[test]
    fn ascii_status_shows_progress() {
        let mut gate = WorkflowGate::new();
        gate.advance(GatePhase::Research).unwrap();
        gate.advance(GatePhase::Synthesis).unwrap();
        let status = gate.ascii_status();
        assert!(status.contains("[x] research"));
        assert!(status.contains("[x] synthesis"));
        assert!(status.contains("[ ] critique-1"));
        assert!(status.contains("[ ] commit"));
    }

    #[test]
    fn phase_names_are_human_readable() {
        assert_eq!(GatePhase::Research.as_name(), "research");
        assert_eq!(GatePhase::Critique1.as_name(), "critique-1");
        assert_eq!(GatePhase::Commit.as_name(), "commit");
    }

    #[test]
    fn nine_phases_total() {
        assert_eq!(GatePhase::ALL.len(), 9);
    }
}
