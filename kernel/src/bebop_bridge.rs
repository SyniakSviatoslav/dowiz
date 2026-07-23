//! `kernel::bebop_bridge` — trinary/eigen/wave/chronos/hybrid → bebop protocol.
//!
//! Wires the new paradigm (3VL, eigen decomposition, wave propagation,
//! chrono-topological navigation, hybrid signing) into the bebop delivery protocol.
//!
//! # Integration points
//! 1. Capability chain: Tri(Allow/Deny/Pending) replaces bool may_delegate
//! 2. Envelope integrity: EigenDecomp of Bundle state for tamper detection
//! 3. Mesh sync: Wave propagation of state changes through the network
//! 4. DTN store-forward: Chronos time-indexed queuing with drift tracking
//! 5. Hybrid signing: RequireBoth (Ed25519 ⊕ ML-DSA-65) capability-signer bridge
//!    to the canonical `capability_cert::HybridSig` via `RefSigner` seam
//!
//! ZERO deps. Uses trinary, eigen, wave, chronos_topology, capability_cert.

use crate::trinary::Tri;
use crate::eigen::{EigenDecomp, decompose};
use crate::wave::InterferenceField;
use crate::chronos_topology::ChronoTopology;
use crate::delta::DeltaTracker;
use crate::capability_cert::{AlgSuite, HybridSig};
use crate::ports::agent::cap::{HybridPolicy, RefSigner, SignatureVerifier, Capability, NodeId};
use crate::ports::agent::scope::{Action, Resource, Scope};

/// Trinary capability decision — replaces binary allow/deny.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriCap {
    pub allow: Tri,      // Allow = True, Deny = False, Pending = Unknown
    pub delegate: Tri,   // may re-delegate? True/False/Unknown
    pub revoke: Tri,     // is this capability revoked?
}

impl TriCap {
    pub fn new() -> Self { TriCap { allow: Tri::Unknown, delegate: Tri::Unknown, revoke: Tri::Unknown } }

    pub fn effective(&self) -> Tri {
        let not_revoked = self.revoke.not();
        self.allow.and(not_revoked)
    }

    pub fn effective_lukasiewicz(&self) -> Tri {
        if self.allow == Tri::False { return Tri::False; }
        if self.revoke == Tri::True { return Tri::False; }
        if self.allow == Tri::True && self.revoke == Tri::False { return Tri::True; }
        Tri::Unknown
    }

    pub fn may_delegate(&self) -> Tri { self.delegate }
}

/// Eigen envelope — Bundle integrity via spectral decomposition.
#[derive(Debug, Clone)]
pub struct EigenEnvelope {
    pub bundle_id: [u8; 32],
    pub state_decomp: EigenDecomp,
    pub signature: Vec<u8>,
}

impl EigenEnvelope {
    pub fn new(bundle_id: [u8; 32], state_values: &[f64]) -> Self {
        EigenEnvelope { bundle_id, state_decomp: decompose(state_values, 8), signature: Vec::new() }
    }

    pub fn verify(&self, current_values: &[f64], tolerance: f64) -> bool {
        let current = decompose(current_values, 8);
        (self.state_decomp.spectral_radius() - current.spectral_radius()).abs() <= tolerance
    }

    pub fn is_stable(&self) -> bool {
        self.state_decomp.unstable_count() == 0
    }
}

/// Wave mesh sync — state changes propagate as spectral waves.
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

    pub fn propagate(&mut self, source_id: &str, values: &[f64]) {
        use crate::wave::spectral_fingerprint;
        let intensity = values.iter().map(|v| v.abs()).sum::<f64>() / values.len().max(1) as f64;
        let fingerprint = spectral_fingerprint(source_id, intensity, crate::now_ms());
        self.field.add_wave(fingerprint);
    }

    pub fn mesh_state(&self) -> crate::trig::Xyz {
        self.field.xyz_state()
    }

    pub fn cleanup(&mut self) -> usize {
        self.field.prune_decayed(0.001)
    }

    pub fn dashboard(&self) -> String {
        format!("═══ WAVE MESH ═══\n  active waves: {}\n  drift: {:.3}\n  state: {:?}",
            self.field.active_count(), self.drift.cumulative_drift, self.mesh_state())
    }
}

/// Chronos DTN — time-indexed store-and-forward queuing.
#[derive(Debug, Clone)]
pub struct ChronosDtn {
    pub topology: ChronoTopology,
    pub queue: Vec<(u64, Vec<u8>)>,
    pub max_queue: usize,
}

impl ChronosDtn {
    pub fn new(max_queue: usize) -> Self {
        ChronosDtn { topology: ChronoTopology::new(), queue: Vec::new(), max_queue }
    }

    pub fn store(&mut self, payload: Vec<u8>) -> u64 {
        let ts = crate::now_ms();
        self.queue.push((ts, payload));
        if self.queue.len() > self.max_queue { self.queue.remove(0); }
        ts
    }

    pub fn forward_before(&mut self, deadline_ms: u64) -> Vec<(u64, Vec<u8>)> {
        let (ready, waiting): (Vec<_>, Vec<_>) = self.queue.drain(..)
            .partition(|(ts, _)| *ts <= deadline_ms);
        self.queue = waiting;
        ready
    }

    pub fn pending(&self) -> usize { self.queue.len() }

    pub fn dashboard(&self) -> String {
        format!("═══ DTN QUEUE ═══\n  pending: {}/{}\n  oldest: {:?}\n  newest: {:?}",
            self.pending(), self.max_queue,
            self.queue.first().map(|(ts,_)| ts),
            self.queue.last().map(|(ts,_)| ts))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Hybrid sign bridge — RequireBoth hybrid signing wired through the canonical
// `capability_cert::HybridSig` + `RefSigner` seam.
// ══════════════════════════════════════════════════════════════════════════════

/// A signed capability frame bridged into the bebop protocol.
#[derive(Debug, Clone)]
pub struct HybridSignedFrame {
    pub capability: Capability,
    pub sig: HybridSig,
    pub issuer: NodeId,
    pub policy: HybridPolicy,
}

impl HybridSignedFrame {
    pub fn sign(
        capability: Capability,
        issuer: NodeId,
        classical_secret: &[u8; 32],
        pq_secret: &[u8; 32],
    ) -> Self {
        let verifier = RefSigner;
        let canonical = capability.canonical_bytes_tlv();
        let sig = HybridSig::sign(
            &verifier,
            AlgSuite::MlDsa65Ed25519,
            classical_secret,
            pq_secret,
            &canonical,
        );
        HybridSignedFrame { capability, sig, issuer, policy: HybridPolicy::RequireBoth }
    }

    pub fn verify(&self) -> Result<(), &'static str> {
        let verifier = RefSigner;
        let canonical = self.capability.canonical_bytes_tlv();
        let classical_pub = self.capability.subject_key;
        let pq_pub = self.capability.subject_key_pq.as_deref().unwrap_or(&[]);
        if self.sig.verify(&verifier, &classical_pub, pq_pub, &canonical) {
            Ok(())
        } else {
            Err("hybrid-verify-failed")
        }
    }
}

/// The bebop hybrid-sign bridge: canonical `HybridSig` signer + trinary policy gate.
#[derive(Debug, Clone)]
pub struct HybridSignBridge {
    pub suite: AlgSuite,
    pub policy: HybridPolicy,
    pub verifier: RefSigner,
}

impl Default for HybridSignBridge {
    fn default() -> Self {
        HybridSignBridge {
            suite: AlgSuite::MlDsa65Ed25519,
            policy: HybridPolicy::RequireBoth,
            verifier: RefSigner,
        }
    }
}

impl HybridSignBridge {
    pub fn new() -> Self { Self::default() }

    pub fn sign_msg(
        &self,
        classical_secret: &[u8; 32],
        pq_secret: &[u8; 32],
        msg: &[u8],
    ) -> HybridSig {
        HybridSig::sign(&self.verifier, self.suite, classical_secret, pq_secret, msg)
    }

    pub fn verify_msg(
        &self,
        classical_pub: &[u8; 32],
        pq_pub: &[u8],
        msg: &[u8],
        sig: &HybridSig,
    ) -> bool {
        sig.verify(&self.verifier, classical_pub, pq_pub, msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── original bebop bridge tests ──────────────────────────────────────────

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
        let _cleaned = mesh.cleanup();
        assert!(mesh.field.active_count() < 2);
    }

    #[test]
    fn chronos_dtn_store_and_forward() {
        let mut dtn = ChronosDtn::new(100);
        let _ts1 = dtn.store(vec![1, 2, 3]);
        let ts2 = dtn.store(vec![4, 5, 6]);
        let forwarded = dtn.forward_before(ts2 + 1000);
        assert_eq!(forwarded.len(), 2);
        assert_eq!(dtn.pending(), 0);
    }

    #[test]
    fn chronos_dtn_max_queue_pruning() {
        let mut dtn = ChronosDtn::new(3);
        dtn.store(vec![1]); dtn.store(vec![2]); dtn.store(vec![3]); dtn.store(vec![4]);
        assert_eq!(dtn.pending(), 3);
    }

    // ── NEW: hybrid sign bridge tests ──────────────────────────────────────

    #[test]
    fn hybrid_bridge_sign_verify_roundtrip() {
        let bridge = HybridSignBridge::new();
        let cls_secret = [7u8; 32];
        let pq_secret = [8u8; 32];
        let verifier = RefSigner;
        let cls_pub = verifier.classical_public(&cls_secret);
        let pq_pub = verifier.pq_public(&pq_secret);
        let msg = b"bebop2 hybrid bridge roundtrip test";

        let sig = bridge.sign_msg(&cls_secret, &pq_secret, msg);
        assert!(bridge.verify_msg(&cls_pub, &pq_pub, msg, &sig));
    }

    #[test]
    fn hybrid_bridge_tampered_msg_rejected() {
        let bridge = HybridSignBridge::new();
        let cls_secret = [10u8; 32];
        let pq_secret = [11u8; 32];
        let verifier = RefSigner;
        let cls_pub = verifier.classical_public(&cls_secret);
        let pq_pub = verifier.pq_public(&pq_secret);

        let sig = bridge.sign_msg(&cls_secret, &pq_secret, b"original");
        assert!(!bridge.verify_msg(&cls_pub, &pq_pub, b"tampered", &sig));
    }

    #[test]
    fn hybrid_bridge_wrong_key_rejected() {
        let bridge = HybridSignBridge::new();
        let verifier = RefSigner;
        let cls_a = verifier.classical_public(&[20u8; 32]);
        let cls_b = verifier.classical_public(&[30u8; 32]);
        let pq_a = verifier.pq_public(&[21u8; 32]);
        let pq_b = verifier.pq_public(&[31u8; 32]);
        let msg = b"wrong key attack";

        let sig = bridge.sign_msg(&[20u8; 32], &[21u8; 32], msg);
        assert!(!bridge.verify_msg(&cls_b, &pq_a, msg, &sig));
        assert!(!bridge.verify_msg(&cls_a, &pq_b, msg, &sig));
    }

    #[test]
    fn hybrid_signed_frame_sign_verify_roundtrip() {
        let verifier = RefSigner;
        let cls_secret = [40u8; 32];
        let pq_secret = [41u8; 32];
        let cls_pub = verifier.classical_public(&cls_secret);
        let pq_pub = verifier.pq_public(&pq_secret);
        let node_id = NodeId::from_keys(&pq_pub, &cls_pub);

        let cap = Capability::new_hybrid(
            cls_pub,
            pq_pub.clone(),
            Scope::single(Resource::Route, Action::Send),
            [9u8; 8],
            9999,
        );

        let frame = HybridSignedFrame::sign(cap, node_id, &cls_secret, &pq_secret);
        assert_eq!(frame.policy, HybridPolicy::RequireBoth);
        assert!(frame.verify().is_ok());
    }

    #[test]
    fn hybrid_signed_frame_tampered_capability_rejected() {
        let cls_secret = [50u8; 32];
        let pq_secret = [51u8; 32];
        let verifier = RefSigner;
        let cls_pub = verifier.classical_public(&cls_secret);
        let pq_pub = verifier.pq_public(&pq_secret);
        let node_id = NodeId::from_keys(&pq_pub, &cls_pub);

        let cap = Capability::new_hybrid(
            cls_pub,
            pq_pub.clone(),
            Scope::single(Resource::Route, Action::Send),
            [9u8; 8],
            9999,
        );

        let mut frame = HybridSignedFrame::sign(cap, node_id, &cls_secret, &pq_secret);
        frame.capability.nonce[0] ^= 0xFF;
        assert!(frame.verify().is_err());
    }

    #[test]
    fn hybrid_bridge_policy_is_always_require_both() {
        let bridge = HybridSignBridge::new();
        assert_eq!(bridge.policy, HybridPolicy::RequireBoth);
    }

    #[test]
    fn hybrid_bridge_deterministic_signing() {
        let bridge = HybridSignBridge::new();
        let cls = [60u8; 32];
        let pq = [61u8; 32];
        let msg = b"deterministic hybrid bridge";
        let a = bridge.sign_msg(&cls, &pq, msg);
        let b = bridge.sign_msg(&cls, &pq, msg);
        assert_eq!(a, b);
    }

    #[test]
    fn hybrid_bridge_distinct_keys_distinct_sigs() {
        let bridge = HybridSignBridge::new();
        let msg = b"same message";
        let sig_a = bridge.sign_msg(&[70u8; 32], &[71u8; 32], msg);
        let sig_b = bridge.sign_msg(&[72u8; 32], &[73u8; 32], msg);
        assert_ne!(sig_a, sig_b);
    }
}
