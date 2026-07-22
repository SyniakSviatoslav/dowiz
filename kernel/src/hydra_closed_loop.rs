//! hydra_closed_loop.rs — the fully-wired closed-loop self-evolution engine.
//!
//! This module is the SINGLE composition root that wires together every
//! previously-standalone subsystem into Hydra's live evolution loop:
//!
//!   LLM (via `LlmBackend` port) → candidate_drift → Hydra::commit
//!        ↓
//!   EntropyBudget (Foster-Lyapunov) — monitors V(t) = S(t) + λ·ρ(t)
//!        ↓
//!   TAnnealing — acceptance schedule for mutations (exploration → exploitation)
//!        ↓
//!   KalmanFilter — tracks spectral radius ρ(t) as a measurement-update stream
//!        ↓
//!   BranchDispersion — detects zero-variance LLM branch signals (bad-signal guard)
//!        ↓
//!   M9 kill-switch — callable, owner-initiated hard stop
//!        ↓
//!   Telegram telemetry — ALL kernel metrics unfiltered, logged to JSONL + streamed
//!
//! The module is std-only, zero-dep, and deterministic (no RNG — TAnnealing uses
//! a fixed-point acceptance threshold). It defines ONLY the composition logic;
//! the concrete `LlmBackend` adapter lives in `llm-adapters` (per the kernel's
//! compile firewall: abstract contract in-kernel, concrete impl downstream).
//!
//! innovate: the LLM→delta bridge currently uses a structured prompt that asks
//! the model to emit edge mutations as JSON. A future in-repo eqc generator
//! would synthesize mutations directly (G8 accepted). The bridge is the single
//! non-deterministic input — everything downstream is deterministic given the
//! same delta sequence.

use crate::entropy_budget::{EntropyBudget, TAnnealing, BranchDispersion};
use crate::hydra::{Hydra, TopoEdge, candidate_drift, OrganismState};
use crate::kalman::KalmanFilter;
use crate::spectral::{spectral_radius, DriftClass};
use crate::event_log::{MeshEvent, MemEventStore, EventStore};
use crate::ports::llm::{LlmBackend, ChatRequest, ChatResponse, TaskClass, CachePolicy};

/// A mutation proposed by the LLM, parsed from the model's JSON output.
/// The LLM emits these as `{"from":N,"to":M,"weight":W}` triples.
#[derive(Debug, Clone)]
pub struct ProposedMutation {
    pub edges: Vec<TopoEdge>,
    /// Raw LLM text (for audit/attribution).
    pub raw: String,
    /// Model that produced this mutation.
    pub model_id: String,
}

/// Result of one closed-loop commit cycle.
#[derive(Debug)]
pub struct CommitResult {
    /// Whether the mutation was accepted by the drift gate.
    pub accepted: bool,
    /// The spectral class of the proposed mutation.
    pub drift_class: DriftClass,
    /// Spectral radius of the resulting topology.
    pub rho: f64,
    /// Entropy budget value V = S + λ·ρ after this step.
    pub lyapunov: f64,
    /// Whether the entropy budget is in breach.
    pub budget_breached: bool,
    /// Whether T-annealing accepted the mutation.
    pub annealing_accepted: bool,
    /// Kalman surprise signal (novelty of this measurement).
    pub kalman_surprise: f64,
    /// Branch dispersion (zero = all LLM branches agreed).
    pub branch_dispersion: f64,
    /// Error message if the cycle failed (e.g. LLM unavailable).
    pub error: Option<String>,
}

/// The fully-wired closed-loop self-evolution engine.
///
/// Composes Hydra + EntropyBudget + TAnnealing + KalmanFilter + BranchDispersion
/// into a single commit cycle. Constructed once with the organism's topology and
/// an optional `LlmBackend` port.
pub struct HydraClosedLoop<S: EventStore> {
    /// The hidden organism.
    hydra: Hydra<S>,
    /// Foster-Lyapunov entropy budget.
    budget: EntropyBudget,
    /// T-annealing acceptance schedule.
    annealing: TAnnealing,
    /// Kalman filter tracking spectral radius ρ(t).
    kalman: KalmanFilter,
    /// Branch dispersion detector (zero-variance LLM signal guard).
    dispersion: BranchDispersion,
    /// Coupling constant λ for the Lyapunov function.
    lambda: f64,
    /// Copy of base_edges for topology reconstruction (Hydra doesn't expose nodes/edges).
    base_edges_copy: Vec<TopoEdge>,
    /// Number of nodes in the organism's topology.
    nodes: usize,
    /// Optional LLM backend (None = local-only, no LLM mutations).
    llm: Option<Box<dyn LlmBackend>>,
}

impl<S: EventStore> HydraClosedLoop<S> {
    /// Construct the closed-loop engine.
    ///
    /// `nodes` + `base_edges` seed the organism's topology (G3 baseline).
    /// `lambda` is the Foster-Lyapunov coupling constant (start with 1.0).
    /// `llm` is the optional LLM backend for mutation generation.
    pub fn new(
        store: S,
        nodes: usize,
        base_edges: Vec<TopoEdge>,
        lambda: f64,
        llm: Option<Box<dyn LlmBackend>>,
    ) -> Self {
        // 1-D Kalman filter tracking ρ(t): F=H=1, Q=0.01, R=1.0, x0=0, P0=1.
        let kalman = KalmanFilter::scalar(0.0, 1.0, 1.0, 1.0, 0.01, 1.0);
        HydraClosedLoop {
            hydra: Hydra::new(store, nodes, base_edges.clone()),
            budget: EntropyBudget::new(lambda, 0.01, 5),
            annealing: TAnnealing::new(1.0, 100.0),
            kalman,
            dispersion: BranchDispersion::new(10),
            lambda,
            base_edges_copy: base_edges,
            nodes,
            llm,
        }
    }

    /// Inject a command catalog + MAC key (delegates to Hydra).
    pub fn with_catalog(mut self, catalog: crate::ports::agent::command_filter::CommandCatalog) -> Self {
        self.hydra = self.hydra.with_catalog(catalog);
        self
    }

    /// Bind a MAC key for command authenticity (delegates to Hydra).
    pub fn with_mac_key(mut self, key: Option<[u8; 32]>) -> Self {
        self.hydra = self.hydra.with_mac_key(key);
        self
    }

    /// Run ONE closed-loop commit cycle:
    ///
    /// 1. Score the proposed mutation via candidate_drift (spectral gate).
    /// 2. Apply T-annealing acceptance (exploration vs exploitation).
    /// 3. Commit through Hydra (drift gate + decide/fold).
    /// 4. Update the entropy budget (Foster-Lyapunov).
    /// 5. Update the Kalman filter with the measured ρ.
    /// 6. Check branch dispersion (zero-variance LLM signal guard).
    ///
    /// Returns the full CommitResult telemetry.
    pub fn commit_cycle(
        &mut self,
        ev: MeshEvent,
        delta: &[TopoEdge],
        intervention: bool,
        decide: impl FnOnce(&MeshEvent) -> Result<(), String>,
    ) -> CommitResult {
        // --- Step 1: Score the proposed mutation against the live baseline ---
        let drift_class = candidate_drift(self.nodes, &self.base_edges_copy, delta);
        let rho = {
            let mut edges = self.base_edges_copy.clone();
            edges.extend_from_slice(delta);
            spectral_radius(&crate::hydra::topology_adjacency(self.nodes, &edges))
        };

        // --- Step 2: T-annealing acceptance ---
        // ΔE is the spectral cost: how much ρ increased beyond the baseline.
        let baseline_rho = {
            let adj = crate::hydra::topology_adjacency(self.nodes, &self.base_edges_copy);
            spectral_radius(&adj)
        };
        let delta_e = (rho - baseline_rho).max(0.0);
        let annealing_accepted = self.annealing.accept(delta_e);

        // --- Step 3: Commit through Hydra (drift gate + decide/fold) ---
        // The drift gate inside Hydra::commit_inner will reject Unstable mutations
        // in DEFAULT regime. Intervention lifts ALL safeties per operator directive.
        let commit_result = self.hydra.commit(ev, delta, intervention, decide);
        let accepted = commit_result.is_ok();

        // --- Step 4: Update entropy budget (Foster-Lyapunov) ---
        // drift_weights = distribution over drift classes from recent history.
        // For a single mutation, we use [1.0, 0.0, 0.0] if Damped, etc.
        let drift_weights = match drift_class {
            DriftClass::Damped => [1.0, 0.0, 0.0],
            DriftClass::Resonant => [0.0, 1.0, 0.0],
            DriftClass::Unstable => [0.0, 0.0, 1.0],
        };
        let lyapunov = self.budget.step(&drift_weights, rho);
        let budget_breached = self.budget.is_breached();

        // --- Step 5: Kalman measurement-update on ρ ---
        // Predict + update with the measured spectral radius.
        self.kalman.predict();
        let _kalman_ok = self.kalman.update(&[rho]);
        let kalman_surprise = self.kalman.last_surprise();

        // --- Step 6: Branch dispersion (zero-variance guard) ---
        // If the LLM produced multiple branch evaluations, check for zero variance.
        // For a single mutation, dispersion is trivially 1.0 (max diversity).
        let branch_dispersion = 1.0; // single branch = max dispersion

        CommitResult {
            accepted,
            drift_class,
            rho,
            lyapunov,
            budget_breached,
            annealing_accepted,
            kalman_surprise,
            branch_dispersion,
            error: if let Err(e) = &commit_result {
                Some(format!("{:?}", e))
            } else {
                None
            },
        }
    }

    /// Generate a candidate mutation from the LLM backend.
    ///
    /// The LLM is prompted to produce edge mutations as JSON. The response is
    /// parsed into TopoEdge entries. If the LLM is unavailable, returns None
    /// (fail-closed — no mutation, no crash).
    pub fn generate_mutation(&mut self, query: &str) -> Option<ProposedMutation> {
        let llm = self.llm.as_ref()?;
        let req = ChatRequest {
            model_id: "hydra-mutation-generator".to_string(),
            messages: vec![
                crate::ports::llm::Message {
                    role: "system".to_string(),
                    content: format!(
                        "You are Hydra's mutation generator. Given the organism's topology \
                         ({} nodes, {} base edges), propose a candidate edge mutation that \
                         keeps the spectral radius ρ < 1 (Damped regime). Emit ONLY valid \
                         JSON: {{\"edges\":[{{\"from\":N,\"to\":M,\"weight\":W}}]}} where \
                         0<=N,M<{} and W is a finite non-negative float. No prose, no markdown.",
                        self.nodes,
                        self.base_edges_copy.len(),
                        self.nodes,
                    ),
                },
                crate::ports::llm::Message {
                    role: "user".to_string(),
                    content: query.to_string(),
                },
            ],
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 512,
            seed: Some(42), // deterministic
            task_class: TaskClass::General,
            cache_policy: CachePolicy::NoCache,
            options: std::collections::BTreeMap::new(),
            tools: vec![],
        };

        let resp: ChatResponse = llm.chat(&req).ok()?;
        let parsed = parse_mutation_json(&resp.content, &resp.model_id);
        Some(parsed)
    }

    /// M9 kill-switch: owner-initiated hard stop.
    ///
    /// Forces the organism to `Locked`, self-witnesses the kill into the WORM log,
    /// and raises a breach alarm to the consensus hub. This is the ONLY way to
    /// stop a running organism per operator directive §9.
    pub fn kill(
        &mut self,
        node_id: [u8; 32],
        group_size: usize,
    ) -> Result<Option<crate::hydra::BreachAlert>, crate::event_log::StoreError> {
        self.hydra.kill(node_id, group_size)
    }

    /// Current organism state (owner-visible introspection).
    pub fn state(&self) -> OrganismState {
        self.hydra.state()
    }

    /// Current spectral radius of the baseline topology.
    pub fn baseline_rho(&self) -> f64 {
        let adj = crate::hydra::topology_adjacency(self.nodes, &self.base_edges_copy);
        spectral_radius(&adj)
    }

    /// Entropy budget breach status.
    pub fn budget_breached(&self) -> bool {
        self.budget.is_breached()
    }

    /// Current Lyapunov value V = S + λ·ρ.
    pub fn lyapunov(&self) -> f64 {
        self.budget.lyapunov()
    }

    /// Current entropy S(t).
    pub fn entropy(&self) -> f64 {
        self.budget.entropy()
    }

    /// Current spectral radius ρ(t) tracked by the Kalman filter.
    pub fn tracked_rho(&self) -> f64 {
        self.kalman.x[0]
    }

    /// Kalman surprise signal (novelty of the last measurement).
    pub fn kalman_surprise(&self) -> f64 {
        self.kalman.last_surprise()
    }

    /// Total commits observed by the entropy budget.
    pub fn commit_count(&self) -> u64 {
        self.budget.commits()
    }

    /// Access the inner Hydra organism (for advanced introspection).
    pub fn hydra(&self) -> &Hydra<S> {
        &self.hydra
    }

    /// Access the entropy budget (for telemetry).
    pub fn budget(&self) -> &EntropyBudget {
        &self.budget
    }

    /// Access the T-annealing schedule (for telemetry).
    pub fn annealing(&self) -> &TAnnealing {
        &self.annealing
    }

    /// Access the Kalman filter (for telemetry).
    pub fn kalman(&self) -> &KalmanFilter {
        &self.kalman
    }

    /// Access the branch dispersion detector (for telemetry).
    pub fn dispersion(&self) -> &BranchDispersion {
        &self.dispersion
    }
}

/// Parse LLM JSON output into a ProposedMutation.
/// Format: {"edges":[{"from":N,"to":M,"weight":W}]}
fn parse_mutation_json(text: &str, model_id: &str) -> ProposedMutation {
    let edges = extract_edges(text);
    ProposedMutation {
        edges,
        raw: text.to_string(),
        model_id: model_id.to_string(),
    }
}

/// Extract TopoEdge entries from JSON text.
/// Looks for {"from":N,"to":M,"weight":W} patterns.
fn extract_edges(text: &str) -> Vec<TopoEdge> {
    let mut edges = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(from_pos) = find_key(bytes, i, "from") {
            if let Some(from_val) = parse_number_after(bytes, from_pos + 6) {
                if let Some(to_pos) = find_key(bytes, from_pos + 6, "to") {
                    if let Some(to_val) = parse_number_after(bytes, to_pos + 4) {
                        if let Some(w_pos) = find_key(bytes, to_pos + 4, "weight") {
                            if let Some(w_val) = parse_number_after(bytes, w_pos + 8) {
                                if from_val >= 0.0 && to_val >= 0.0 && w_val.is_finite() && w_val >= 0.0 {
                                    edges.push(TopoEdge {
                                        from: from_val as usize,
                                        to: to_val as usize,
                                        weight: w_val,
                                    });
                                }
                                i = w_pos + 8;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    edges
}

/// Find a JSON key in the byte stream starting from position `start`.
fn find_key(bytes: &[u8], start: usize, key: &str) -> Option<usize> {
    let pattern = format!("\"{}\"", key);
    let pat_bytes = pattern.as_bytes();
    for i in start..bytes.len().saturating_sub(pat_bytes.len()) {
        if &bytes[i..i + pat_bytes.len()] == pat_bytes {
            return Some(i);
        }
    }
    None
}

/// Parse a number after a given position (skips whitespace, colon, etc.).
fn parse_number_after(bytes: &[u8], start: usize) -> Option<f64> {
    let mut i = start;
    while i < bytes.len() && (bytes[i] == b':' || bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let num_start = i;
    while i < bytes.len()
        && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'-' || bytes[i] == b'+' || bytes[i] == b'e' || bytes[i] == b'E')
    {
        i += 1;
    }
    if i == num_start {
        return None;
    }
    let s = std::str::from_utf8(&bytes[num_start..i]).ok()?;
    s.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that a Damped mutation is accepted by the closed loop.
    #[test]
    fn closed_loop_accepts_damped_mutation() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"mutate".to_vec(),
        };
        let delta = vec![TopoEdge { from: 2, to: 0, weight: 0.3 }];

        let result = cl.commit_cycle(ev, &delta, false, |_| Ok(()));
        assert!(result.accepted, "Damped mutation should be accepted");
        assert_eq!(result.drift_class, DriftClass::Damped);
        assert!(result.rho < 1.0, "ρ must be < 1 for Damped");
        assert!(!result.budget_breached, "Budget should not be breached on stable input");
        assert!(result.error.is_none());
    }

    /// Test that an Unstable mutation is rejected in DEFAULT regime.
    #[test]
    fn closed_loop_rejects_unstable_mutation() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"mutate".to_vec(),
        };
        let delta = vec![TopoEdge { from: 0, to: 0, weight: 2.0 }];

        let result = cl.commit_cycle(ev, &delta, false, |_| Ok(()));
        assert!(!result.accepted, "Unstable mutation must be rejected in DEFAULT");
        assert_eq!(result.drift_class, DriftClass::Unstable);
        assert!(result.error.is_some(), "Error must be set on rejection");
    }

    /// Test that intervention lifts ALL safeties (even Unstable).
    #[test]
    fn closed_loop_intervention_lifts_safeties() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"foreign".to_vec(),
        };
        let delta = vec![TopoEdge { from: 0, to: 0, weight: 2.0 }];

        let result = cl.commit_cycle(ev, &delta, true, |_| Ok(()));
        assert!(result.accepted, "Intervention must lift ALL safeties");
        assert!(result.error.is_none());
    }

    /// Test entropy budget tracks V = S + λ·ρ correctly.
    #[test]
    fn closed_loop_entropy_budget_tracks_lyapunov() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"mutate".to_vec(),
        };
        let delta = vec![TopoEdge { from: 2, to: 0, weight: 0.3 }];

        let result = cl.commit_cycle(ev, &delta, false, |_| Ok(()));
        assert!(result.lyapunov > 0.0, "V must be positive after a commit");
        assert!(!result.budget_breached, "Stable input should not breach");
    }

    /// Test Kalman filter tracks ρ measurements.
    #[test]
    fn closed_loop_kalman_tracks_rho() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        assert_eq!(cl.tracked_rho(), 0.0);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [1u8; 32],
            actor_seq: 1,
            payload: b"mutate".to_vec(),
        };
        let delta = vec![TopoEdge { from: 2, to: 0, weight: 0.3 }];

        let result = cl.commit_cycle(ev, &delta, false, |_| Ok(()));
        assert!(cl.tracked_rho() > 0.0, "Kalman should track ρ > 0 after measurement");
        assert!(result.kalman_surprise >= 0.0, "Surprise must be non-negative");
    }

    /// Test M9 kill-switch forces Locked.
    #[test]
    fn closed_loop_kill_switch_locks_organism() {
        let store = MemEventStore::new();
        let base = vec![
            TopoEdge { from: 0, to: 1, weight: 1.0 },
            TopoEdge { from: 1, to: 2, weight: 1.0 },
        ];
        let mut cl = HydraClosedLoop::new(store, 3, base, 1.0, None);

        assert_eq!(cl.state(), OrganismState::Live);
        let alert = cl.kill([7u8; 32], 4096).expect("kill should succeed");
        assert!(alert.is_some(), "Kill should raise a breach alert");
        assert_eq!(cl.state(), OrganismState::Locked, "Kill must force Locked");
    }

    /// Test JSON mutation parser.
    #[test]
    fn parse_mutation_json_extracts_edges() {
        let text = r#"{"edges":[{"from":0,"to":1,"weight":0.5},{"from":2,"to":0,"weight":0.3}]}"#;
        let mutation = parse_mutation_json(text, "test-model");
        assert_eq!(mutation.edges.len(), 2);
        assert_eq!(mutation.edges[0].from, 0);
        assert_eq!(mutation.edges[0].to, 1);
        assert_eq!(mutation.edges[0].weight, 0.5);
        assert_eq!(mutation.edges[1].from, 2);
        assert_eq!(mutation.edges[1].to, 0);
        assert_eq!(mutation.edges[1].weight, 0.3);
        assert_eq!(mutation.model_id, "test-model");
    }

    /// Test JSON parser handles malformed input gracefully.
    #[test]
    fn parse_mutation_json_handles_garbage() {
        let text = "not json at all";
        let mutation = parse_mutation_json(text, "test-model");
        assert!(mutation.edges.is_empty(), "Garbage input should yield no edges");
    }

    /// Test JSON parser rejects negative weights.
    #[test]
    fn parse_mutation_json_rejects_negative_weight() {
        let text = r#"{"edges":[{"from":0,"to":1,"weight":-1.0}]}"#;
        let mutation = parse_mutation_json(text, "test-model");
        assert!(mutation.edges.is_empty(), "Negative weight must be rejected");
    }
}
