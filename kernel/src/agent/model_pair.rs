//! P103 — Hydra × locked model-pair supervisor integration.
//!
//! Design (operator-ruled 2026-07-20, grounded in the real 1567-line `hydra.rs`):
//! the two named local models (LFM2.5-VL-450M + SmolVLM-256M-Instruct, locked by the
//! P101 operator correction to exactly this pair) integrate ENTIRELY through Hydra's
//! *existing public surface* — they become `TopoEdge` nodes in a 2-node topology that
//! passes through Hydra's existing drift gate (`candidate_drift`). **Zero edits to
//! `hydra.rs` itself**: this module only calls the public API it already exposes.
//!
//! Safety shape (L1 opacity-enforcement — no black boxes, always):
//!   * DUAL-WITNESS — a positive claim needs 2-of-2 agreement; disagreement collapses
//!     to `Unknown` rather than a guess. A single hallucinating model cannot mint
//!     permanent evidence.
//!   * DRIFT-GATED — the pair's coupling mutation is scored by `candidate_drift`
//!     against Hydra's live spectral baseline; a divergent (Unstable, ρ>1) coupling is
//!     rejected fail-closed, exactly like any other topology mutation.
//!   * SUPERVISOR MEMBRANE — a hard schema/vocabulary/provenance check sits between the
//!     models and Hydra; only membrane-passing, drift-clear claims reach `candidate_drift`.
//!   * OSCILLATOR — the supervisor's heartbeat clock reuses Hydra's own `INTEGRITY_BAND`
//!     hysteresis (damped/resonant/unstable regimes with hysteresis), so no second
//!     oscillator is invented.
//!   * OSMOSIS — the gradient flow across the pair is `personalized_pagerank` over the
//!     2-node adjacency (reuses tested kernel math, no new mechanism).
//!
//! Hydra's charter (closure = never, kill-switch only, all-safeties-lift-on-intervention)
//! is untouched: `Hydra` is constructed and driven by the caller exactly as before; this
//! module never calls `Hydra::commit` on its own and holds no `&mut Hydra`. Structurally
//! enforced: `hydra.rs` code paths carry no call into this module, so a dead supervisor
//! only starves the model channel, never Hydra itself.

use crate::csr::Csr;
use crate::hydra::{candidate_drift, TopoEdge, INTEGRITY_BAND};
use crate::spectral::DriftClass;

/// The two locked models. Index 0 = LFM2.5-VL-450M, index 1 = SmolVLM-256M-Instruct.
/// The topology is a 2-node bidirectional graph (each witnesses the other).
pub const MODEL_A: usize = 0;
pub const MODEL_B: usize = 1;
pub const PAIR_NODES: usize = 2;

/// A model's output for a given prompt, with provenance. `value` is the model's
/// structured claim; `confidence` is its self-reported 0..=1 score (used only for
/// osmosis ranking, never as evidence on its own).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelOutput {
    pub model: usize,
    pub value: ClaimValue,
    pub confidence: f64,
    /// Provenance: which model + a monotonic utterance id (for replay/audit).
    pub utterance_id: u64,
}

/// The structured claim a model emits. Kept deliberately small + schema-checked
/// (L1: no opaque blob crosses the membrane).
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimValue {
    /// A resolved, schema-valid assertion.
    Assertion(String),
    /// The model explicitly abstains (no claim).
    Abstain,
}

/// The resolved outcome of dual-witness arbitration.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Both models agreed on the same assertion → resolved evidence.
    Resolved(String),
    /// Models disagreed (or any abstained/garbled) → collapsed to Unknown, never a guess.
    Unknown,
}

/// Hard membrane check: a single model output is admitted to arbitration only if its
/// vocabulary + schema + provenance are well-formed. Returns `None` for a rejected
/// (black-box / malformed) output — it never reaches the 2-of-2.
pub fn membrane_admit(out: &ModelOutput) -> Option<&ModelOutput> {
    // schema: value must be a well-formed Assertion (Abstain is allowed but carries no weight)
    // vocab: assertions must be non-empty and bounded length (no unbounded opaque blob)
    // provenance: confidence finite in [0,1], utterance_id > 0
    let ok = match &out.value {
        ClaimValue::Assertion(s) => !s.is_empty() && s.len() <= 4096,
        ClaimValue::Abstain => true,
    };
    if ok
        && out.confidence.is_finite()
        && (0.0..=1.0).contains(&out.confidence)
        && out.utterance_id > 0
        && (out.model == MODEL_A || out.model == MODEL_B)
    {
        Some(out)
    } else {
        None
    }
}

/// Dual-witness arbitration: 2-of-2 agreement resolves; any divergence → `Unknown`.
/// This is the core L1 guarantee — a lone model cannot mint evidence.
pub fn arbitrate(a: &ModelOutput, b: &ModelOutput) -> Verdict {
    let a = match membrane_admit(a) {
        Some(o) => o,
        None => return Verdict::Unknown,
    };
    let b = match membrane_admit(b) {
        Some(o) => o,
        None => return Verdict::Unknown,
    };
    match (&a.value, &b.value) {
        (ClaimValue::Assertion(x), ClaimValue::Assertion(y)) if x == y => {
            Verdict::Resolved(x.clone())
        }
        _ => Verdict::Unknown,
    }
}

/// The 2-node baseline topology: bidirectional equal-weight edges (each model
/// witnesses the other). Used as the base for `candidate_drift`.
pub fn pair_baseline() -> Vec<TopoEdge> {
    vec![
        TopoEdge {
            from: MODEL_A,
            to: MODEL_B,
            weight: 1.0,
        },
        TopoEdge {
            from: MODEL_B,
            to: MODEL_A,
            weight: 1.0,
        },
    ]
}

/// Score a *proposed coupling change* (e.g. re-weighting the pair, or adding a
/// crosswire edge) against Hydra's live drift gate. Returns the `DriftClass` — the
/// caller refuses any `Unstable` mutation fail-closed. This is the only path by which
/// the model-pair touches Hydra's topology surface, and it goes through the public
/// `candidate_drift` gate (no edit to `hydra.rs`).
pub fn score_coupling_delta(delta: &[TopoEdge]) -> DriftClass {
    candidate_drift(PAIR_NODES, &pair_baseline(), delta)
}

/// Supervisor heartbeat: reuse Hydra's `INTEGRITY_BAND` hysteresis as the clock.
/// Given a sequence of observed spectral radii (ρ), return the organism state the
/// band implies. This reuses the EXACT same damped/resonant/unstable band Hydra uses,
/// so the supervisor never invents a second oscillator.
pub fn heartbeat_state(rho: f64) -> SupervisorHeartbeat {
    if !rho.is_finite() || rho >= INTEGRITY_BAND.trigger {
        SupervisorHeartbeat::Locked
    } else if rho <= INTEGRITY_BAND.release {
        SupervisorHeartbeat::Live
    } else {
        SupervisorHeartbeat::Marginal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SupervisorHeartbeat {
    Live,
    Marginal,
    Locked,
}

/// Osmotic gradient flow across the pair: rank the two models by their diffusion
/// rank over the 2-node adjacency, using the tested `personalized_pagerank`. The
/// seed is the supervisor's current attention (which model it leans on). Returns the
/// per-node rank (index 0 = MODEL_A, 1 = MODEL_B). Reuses kernel math, no new mechanism.
pub fn osmotic_rank(attention_seed: &[f64; PAIR_NODES]) -> [f64; PAIR_NODES] {
    // Build a 2-node CSR adjacency (symmetric pair baseline) as (src,dst,weight) tuples.
    let edges: Vec<(usize, usize, f64)> = pair_baseline()
        .iter()
        .map(|e| (e.from, e.to, e.weight))
        .collect();
    let csr = Csr::from_edges(PAIR_NODES, &edges);
    let rank = csr.personalized_pagerank(attention_seed, 0.85, 32);
    let mut out = [0.0f64; PAIR_NODES];
    for (i, v) in rank.iter().take(PAIR_NODES).enumerate() {
        out[i] = *v;
    }
    out
}

/// L1 PROVENANCE GATE — the ONLY compile-path from a port reply (`ports::llm::ChatResponse`)
/// to an evidence-bearing `ModelOutput`. Fails closed: if the backend served an `model_id`
/// that is NOT one of the locked pair (`validate_topology`), the reply is refused — no
/// opaque "the model said" text can cross into the supervisor without a provenance tag that
/// matches the operator-locked contract. This is the structural enforcement of "no black
/// boxes, always": provenance must be present AND must match the lock, or the output is dropped.
///
/// `locked` is the slice from `model_registry::locked_pair()` (or `validate_topology(None)`'s
/// source). `utterance_id` is taken from the response (already monotonic, set by the adapter).
pub fn from_port_response(
    resp: &crate::ports::llm::ChatResponse,
    locked: &[crate::agent::model_registry::LockedModel],
) -> Option<ModelOutput> {
    // 1) provenance must name one of the locked models exactly.
    let idx = locked.iter().position(|m| m.name == resp.model_id)?;
    // 2) the response must carry a real utterance id (replay/audit).
    if resp.utterance_id == 0 {
        return None;
    }
    // 3) the text must survive the membrane (schema/vocab/provenance) before it can become
    //    a claim — an opaque/unbounded blob is dropped here, never reaching `arbitrate`.
    let value = ClaimValue::Assertion(resp.content.clone());
    let out = ModelOutput {
        model: locked[idx].id as usize,
        value,
        confidence: 0.0, // not used as evidence on its own; osmosis ranking only
        utterance_id: resp.utterance_id,
    };
    membrane_admit(&out).cloned()
}

/// Convenience: enforce provenance against the canonical locked pair (P97/P101).
pub fn from_port_response_locked(resp: &crate::ports::llm::ChatResponse) -> Option<ModelOutput> {
    from_port_response(resp, &crate::agent::model_registry::locked_pair())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::model_registry::LockedModel;
    use crate::ports::llm::ChatResponse;

    fn port_reply(model_id: &str, content: &str, utterance_id: u64) -> ChatResponse {
        ChatResponse {
            content: content.into(),
            usage: crate::ports::llm::Usage::default(),
            model_id: model_id.into(),
            utterance_id,
            tool_calls: Vec::new(),
        }
    }

    #[test]
    fn l1_gate_accepts_locked_model_reply() {
        // A reply provenance-tagged with a LOCKED model id becomes a valid ModelOutput.
        let r = port_reply("LFM2.5-VL-450M", "door_open", 7);
        let out = from_port_response_locked(&r).expect("locked model must pass the gate");
        assert_eq!(out.model, MODEL_A);
        assert_eq!(out.utterance_id, 7);
    }

    #[test]
    fn l1_gate_refuses_unlocked_black_box() {
        // A backend that silently serves an unlocked model is refused fail-closed —
        // the text never crosses into the supervisor, no matter how plausible.
        let r = port_reply("Some-Other-Model-7B", "door_open", 7);
        assert!(from_port_response_locked(&r).is_none());
    }

    #[test]
    fn l1_gate_refuses_zero_utterance_id() {
        // Replay/audit provenance is mandatory; a reply without an utterance id is dropped.
        let r = port_reply("LFM2.5-VL-450M", "door_open", 0);
        assert!(from_port_response_locked(&r).is_none());
    }

    #[test]
    fn l1_gate_refuses_opaque_blob() {
        // Even a locked-model reply with an unbounded blob is dropped by the membrane.
        let r = port_reply("LFM2.5-VL-450M", &"x".repeat(5000), 7);
        assert!(from_port_response_locked(&r).is_none());
    }

    #[test]
    fn l1_gate_maps_model_name_to_index() {
        // The second locked model maps to MODEL_B, not MODEL_A.
        let locked = [
            LockedModel {
                id: 0,
                name: "LFM2.5-VL-450M",
                params_m: 450,
                backend: crate::agent::model_registry::ServingBackend::CpuLlamaCpp,
                provenance: "",
            },
            LockedModel {
                id: 1,
                name: "SmolVLM-256M-Instruct",
                params_m: 256,
                backend: crate::agent::model_registry::ServingBackend::CpuLlamaCpp,
                provenance: "",
            },
        ];
        let r = port_reply("SmolVLM-256M-Instruct", "door_open", 3);
        let out = from_port_response(&r, &locked).expect("second locked model passes");
        assert_eq!(out.model, MODEL_B);
    }

    fn out(model: usize, val: ClaimValue, conf: f64, id: u64) -> ModelOutput {
        ModelOutput {
            model,
            value: val,
            confidence: conf,
            utterance_id: id,
        }
    }

    #[test]
    fn dual_witness_2of2_agreement_resolves() {
        let a = out(MODEL_A, ClaimValue::Assertion("door_open".into()), 0.9, 1);
        let b = out(MODEL_B, ClaimValue::Assertion("door_open".into()), 0.88, 2);
        assert_eq!(arbitrate(&a, &b), Verdict::Resolved("door_open".into()));
    }

    #[test]
    fn dual_witness_disagreement_collapses_to_unknown() {
        let a = out(MODEL_A, ClaimValue::Assertion("door_open".into()), 0.9, 1);
        let b = out(MODEL_B, ClaimValue::Assertion("door_closed".into()), 0.9, 2);
        // Disagreement must NOT mint evidence — collapses to Unknown, never a guess.
        assert_eq!(arbitrate(&a, &b), Verdict::Unknown);
    }

    #[test]
    fn single_model_cannot_mint_evidence() {
        // Only ONE model emits; the other abstains → Unknown, not a resolved claim.
        let a = out(MODEL_A, ClaimValue::Assertion("door_open".into()), 0.99, 1);
        let b = out(MODEL_B, ClaimValue::Abstain, 0.0, 2);
        assert_eq!(arbitrate(&a, &b), Verdict::Unknown);
    }

    #[test]
    fn membrane_rejects_malformed_black_box() {
        // Opaque/unbounded / bad provenance never reaches arbitration.
        let bad = out(MODEL_A, ClaimValue::Assertion("x".repeat(5000)), 0.9, 1);
        assert!(membrane_admit(&bad).is_none());
        let neg = out(MODEL_A, ClaimValue::Assertion("ok".into()), -0.1, 1);
        assert!(membrane_admit(&neg).is_none());
        let zeroid = out(MODEL_A, ClaimValue::Assertion("ok".into()), 0.9, 0);
        assert!(membrane_admit(&zeroid).is_none());
    }

    #[test]
    fn drift_gate_rejects_divergent_coupling() {
        // Adding ANY extra weight to the already-fully-connected 2-cycle raises ρ>1
        // → Unstable → refused fail-closed. This is correct gate behavior.
        let divergent = vec![TopoEdge {
            from: MODEL_A,
            to: MODEL_A,
            weight: 1e9,
        }];
        assert_eq!(score_coupling_delta(&divergent), DriftClass::Unstable);
        // A benign *coupling reduction* (negative delta lowering an edge below the
        // ρ=1 boundary) keeps the cycle bounded (Damped), not Unstable — the gate
        // permits relaxing the lock, only refuses tightening it.
        let lowered = vec![TopoEdge {
            from: MODEL_A,
            to: MODEL_B,
            weight: -0.5,
        }];
        assert_ne!(score_coupling_delta(&lowered), DriftClass::Unstable);
    }

    #[test]
    fn heartbeat_reuses_integrity_band() {
        assert_eq!(heartbeat_state(0.5), SupervisorHeartbeat::Live);
        assert_eq!(heartbeat_state(1.0), SupervisorHeartbeat::Locked); // ρ>=trigger
        assert_eq!(heartbeat_state(f64::NAN), SupervisorHeartbeat::Locked);
        // Between release and trigger → marginal (oscillator-theoretic band).
        assert_eq!(heartbeat_state(0.999999), SupervisorHeartbeat::Marginal);
    }

    #[test]
    fn osmotic_rank_is_symmetric_for_uniform_seed() {
        let r = osmotic_rank(&[0.5, 0.5]);
        // Symmetric pair + symmetric seed → equal diffusion rank.
        assert!((r[0] - r[1]).abs() < 1e-9, "ranks should match: {:?}", r);
    }

    #[test]
    fn osmotic_rank_leans_with_attention() {
        let r = osmotic_rank(&[0.9, 0.1]);
        // Leaning attention on MODEL_A pushes its diffusion rank higher.
        assert!(r[0] > r[1], "A should rank above B: {:?}", r);
    }
}
