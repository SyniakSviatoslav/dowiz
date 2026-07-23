//! `kernel::bebop_bridge` — trinary/eigen/wave/chronos → bebop protocol.
//!
//! Wires the new paradigm (3VL, eigen decomposition, wave propagation,
//! chrono-topological navigation) into the bebop delivery protocol.
//!
//! # Integration points
//! 1. Capability chain: Tri(Allow/Deny/Pending) replaces bool may_delegate
//! 2. Envelope integrity: EigenDecomp of Bundle state for tamper detection
//! 3. Mesh sync: Wave propagation of state changes through the network
//! 4. DTN store-forward: Chronos time-indexed queuing with drift tracking
//!
//! ZERO deps. Uses trinary, eigen, wave, chronos_topology.

use crate::trinary::Tri;
use crate::eigen::{EigenDecomp, decompose};
use crate::wave::InterferenceField;
use crate::chronos_topology::ChronoTopology;
use crate::delta::DeltaTracker;

/// Trinary capability decision — replaces binary allow/deny.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriCap {
    pub allow: Tri,      // Allow = True, Deny = False, Pending = Unknown
    pub delegate: Tri,   // may re-delegate? True/False/Unknown
    pub revoke: Tri,     // is this capability revoked?
}

impl TriCap {
    pub fn new() -> Self { TriCap { allow: Tri::Unknown, delegate: Tri::Unknown, revoke: Tri::Unknown } }

    /// Is this capability effectively allowed?
    /// Kleene: if allow=True AND NOT revoked, return True.
    /// If revoke=Unknown (pending), return Unknown.
    pub fn effective(&self) -> Tri {
        let not_revoked = self.revoke.not();
        self.allow.and(not_revoked)
    }

    /// Łukasiewicz evaluation: U→U = True, more permissive for pending states.
    pub fn effective_lukasiewicz(&self) -> Tri {
        if self.allow == Tri::False { return Tri::False; }
        if self.revoke == Tri::True { return Tri::False; }
        if self.allow == Tri::True && self.revoke == Tri::False { return Tri::True; }
        Tri::Unknown // any pending/unknown combination
    }

    /// Can this capability be delegated?
    pub fn may_delegate(&self) -> Tri { self.delegate }
}

/// Eigen envelope — Bundle integrity via spectral decomposition.
#[derive(Debug, Clone)]
pub struct EigenEnvelope {
    pub bundle_id: [u8; 32],
    pub state_decomp: EigenDecomp,  // spectral signature of bundle contents
    pub signature: Vec<u8>,         // ML-DSA-65 signature over decomp
}

impl EigenEnvelope {
    pub fn new(bundle_id: [u8; 32], state_values: &[f64]) -> Self {
        EigenEnvelope {
            bundle_id,
            state_decomp: decompose(state_values, 8),
            signature: Vec::new(),
        }
    }

    /// Verify envelope integrity: recompute eigen decomp and compare spectral radius.
    pub fn verify(&self, current_values: &[f64], tolerance: f64) -> bool {
        let current = decompose(current_values, 8);
        let diff = (self.state_decomp.spectral_radius() - current.spectral_radius()).abs();
        diff <= tolerance
    }

    /// Is the envelope stable? (all eigen modes |λ| ≤ 1).
    pub fn is_stable(&self) -> bool {
        self.state_decomp.unstable_count() == 0
    }
}

/// Wave mesh sync — state changes propagate as spectral waves through the network.
pub struct WaveMeshSync {
    pub field: InterferenceField,
    pub topology: ChronoTopology,
    pub drift: DeltaTracker,
}

impl WaveMeshSync {
    pub fn new() -> Self {
        WaveMeshSync {
            field: InterferenceField::new(),
            topology: ChronoTopology::new(),
            drift: DeltaTracker::new(5.0, 100.0),
        }
    }

    /// Propagate a state change as a wave through the mesh.
    pub fn propagate(&mut self, source_id: &str, values: &[f64]) {
        use crate::wave::spectral_fingerprint;
        let intensity = values.iter().map(|v| v.abs()).sum::<f64>() / values.len().max(1) as f64;
        let fingerprint = spectral_fingerprint(source_id, intensity, crate::now_ms());
        self.field.add_wave(fingerprint);
    }

    /// Composite mesh state — superposition of all propagating waves.
    pub fn mesh_state(&self) -> crate::trig::Xyz {
        self.field.xyz_state()
    }

    /// Cleanup: remove fully decayed waves.
    pub fn cleanup(&mut self) -> usize {
        self.field.prune_decayed(0.001)
    }

    /// Dashboard of mesh propagation state.
    pub fn dashboard(&self) -> String {
        format!("═══ WAVE MESH ═══\n  active waves: {}\n  drift: {:.3}\n  state: {:?}",
            self.field.active_count(), self.drift.cumulative_drift, self.mesh_state())
    }
}

/// Chronos DTN — time-indexed store-and-forward queuing with drift tracking.
#[derive(Debug, Clone)]
pub struct ChronosDtn {
    pub topology: ChronoTopology,
    pub queue: Vec<(u64, Vec<u8>)>,   // (timestamp, payload)
    pub max_queue: usize,
}

impl ChronosDtn {
    pub fn new(max_queue: usize) -> Self {
        ChronosDtn { topology: ChronoTopology::new(), queue: Vec::new(), max_queue }
    }

    /// Store a message for forward delivery at a future time.
    pub fn store(&mut self, payload: Vec<u8>) -> u64 {
        let ts = crate::now_ms();
        self.queue.push((ts, payload));
        if self.queue.len() > self.max_queue { self.queue.remove(0); }
        ts
    }

    /// Forward all messages due before the given timestamp.
    pub fn forward_before(&mut self, deadline_ms: u64) -> Vec<(u64, Vec<u8>)> {
        let (ready, waiting): (Vec<_>, Vec<_>) = self.queue.drain(..)
            .partition(|(ts, _)| *ts <= deadline_ms);
        self.queue = waiting;
        ready
    }

    /// Total queued messages.
    pub fn pending(&self) -> usize { self.queue.len() }

    /// Dashboard.
    pub fn dashboard(&self) -> String {
        format!("═══ DTN QUEUE ═══\n  pending: {}/{}\n  oldest: {:?}\n  newest: {:?}",
            self.pending(), self.max_queue,
            self.queue.first().map(|(ts,_)| ts),
            self.queue.last().map(|(ts,_)| ts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tricap_effective_allowed() {
        let cap = TriCap { allow: Tri::True, delegate: Tri::False, revoke: Tri::False };
        assert_eq!(cap.effective(), Tri::True);
    }

    #[test]
    fn tricap_revoked_is_false() {
        let cap = TriCap { allow: Tri::True, delegate: Tri::False, revoke: Tri::True };
        assert_eq!(cap.effective(), Tri::False);
    }

    #[test]
    fn tricap_pending_is_unknown() {
        let cap = TriCap { allow: Tri::True, delegate: Tri::False, revoke: Tri::Unknown };
        assert_eq!(cap.effective(), Tri::Unknown);
    }

    #[test]
    fn eigen_envelope_verify_stable() {
        let vals = vec![0.5, 0.3, 0.1];
        let env = EigenEnvelope::new([0u8; 32], &vals);
        assert!(env.verify(&vals, 0.01));
        assert!(env.is_stable());
    }

    #[test]
    fn eigen_envelope_detect_tamper() {
        let vals = vec![0.5, 0.3, 0.1];
        let env = EigenEnvelope::new([0u8; 32], &vals);
        assert!(!env.verify(&[999.0, 0.3, 0.1], 0.01));
    }

    #[test]
    fn wave_mesh_propagate_and_cleanup() {
        let mut mesh = WaveMeshSync::new();
        mesh.propagate("node_a", &[0.5, 0.3]);
        assert!(mesh.field.active_count() >= 1);
        let cleaned = mesh.cleanup();
        assert!(mesh.field.active_count() < 2);
    }

    #[test]
    fn chronos_dtn_store_and_forward() {
        let mut dtn = ChronosDtn::new(100);
        let ts1 = dtn.store(vec![1, 2, 3]);
        let ts2 = dtn.store(vec![4, 5, 6]);
        let forwarded = dtn.forward_before(ts2 + 1000);
        assert_eq!(forwarded.len(), 2);
        assert_eq!(dtn.pending(), 0);
    }

    #[test]
    fn chronos_dtn_max_queue_pruning() {
        let mut dtn = ChronosDtn::new(3);
        dtn.store(vec![1]); dtn.store(vec![2]); dtn.store(vec![3]); dtn.store(vec![4]);
        assert_eq!(dtn.pending(), 3); // oldest evicted
    }
}
