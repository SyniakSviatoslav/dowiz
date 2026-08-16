//! P97 / P101 — locked local-model registry + CPU-only serving topology.
//!
//! Operator ruling (2026-07-20, P101 correction): the local-model topology is locked
//! to EXACTLY two named models, run concurrently/crosswired, one pair for both mobile
//! and server, replacing the earlier abstract tier system:
//!   * LFM2.5-VL-450M        (vision-language, mobile+server)
//!   * SmolVLM-256M-Instruct (vision-language, mobile+server)
//! O-1 (LFM license) RULED "clear to ship"; O-2 (small-model bake-off) superseded by
//! the same correction. O-3 (optional CPU-LoRA probe) remains open (per-change-gated).
//!
//! This module is the SINGLE declarative source of the locked pair + its serving
//! topology. It reuses the existing `LlmBackend` port (zero new HTTP/serde; the
//! `llm-adapters` crate implements it against llama.cpp/Ollama CPU). It does NOT
//! embed weights or spawn runtimes — it declares the contract the serving layer must
//! satisfy, and is fully testable offline (no model download).
//!
//! L1 opacity: every model is named + version-pinned + provenance-tagged; no opaque
//! "the model decided" path exists. The supervisor in `model_pair.rs` is the only
//! place model outputs become evidence, and only via 2-of-2 dual-witness.

/// A locked model's identity + serving contract.
#[derive(Debug, Clone, PartialEq)]
pub struct LockedModel {
    /// Stable id (matches `model_pair::MODEL_A` / `MODEL_B`).
    pub id: usize,
    /// Canonical model name (operator-locked).
    pub name: &'static str,
    /// Parameter count (millions) — for telemetry/resource budgeting only.
    pub params_m: u32,
    /// Serving backend: CPU-only llama.cpp (OD-18b approved) via the `LlmBackend` port.
    pub backend: ServingBackend,
    /// Provenance: where the weights come from + that they are version-pinned.
    pub provenance: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServingBackend {
    /// llama.cpp CPU tier (GGUF), no GPU required. OD-18b GO.
    CpuLlamaCpp,
}

/// The locked pair, in `model_pair` node-index order (0 = A, 1 = B).
pub fn locked_pair() -> [LockedModel; 2] {
    [
        LockedModel {
            id: 0,
            name: "LFM2.5-VL-450M",
            params_m: 450,
            backend: ServingBackend::CpuLlamaCpp,
            provenance: "operator-locked 2026-07-20; version-pinned GGUF; no finetune",
        },
        LockedModel {
            id: 1,
            name: "SmolVLM-256M-Instruct",
            params_m: 256,
            backend: ServingBackend::CpuLlamaCpp,
            provenance: "operator-locked 2026-07-20; version-pinned GGUF; no finetune",
        },
    ]
}

/// Serving topology: the two models run concurrently and crosswire (P101 ruling) on a
/// single CPU host; both are reachable through the existing `LlmBackend` port. This is
/// a declarative descriptor consumed by the serving bootstrap — it carries no runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct ServingTopology {
    pub models: [LockedModel; 2],
    /// Crosswire: each model's output is a witness for the other (feeds `model_pair::arbitrate`).
    pub crosswired: bool,
    /// CPU-only, no GPU path (OD-18b).
    pub cpu_only: bool,
}

pub fn serving_topology() -> ServingTopology {
    ServingTopology {
        models: locked_pair(),
        crosswired: true,
        cpu_only: true,
    }
}

/// Validate that the running serving layer matches the locked contract. Returns the
/// first violation, or `None` if the topology is exactly the locked pair (L1: no
/// silent model swap / no black-box addition).
pub fn validate_topology(actual: &[LockedModel]) -> Option<&'static str> {
    let locked = locked_pair();
    if actual.len() != locked.len() {
        return Some("model count differs from locked pair");
    }
    for (a, l) in actual.iter().zip(locked.iter()) {
        if a.id != l.id || a.name != l.name {
            return Some("model identity differs from locked pair");
        }
        if a.backend != l.backend {
            return Some("serving backend differs from locked CPU-only contract");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_pair_is_exactly_two_named_models() {
        let p = locked_pair();
        assert_eq!(p[0].name, "LFM2.5-VL-450M");
        assert_eq!(p[1].name, "SmolVLM-256M-Instruct");
        assert!(p.iter().all(|m| m.backend == ServingBackend::CpuLlamaCpp));
    }

    #[test]
    fn topology_is_crosswired_cpu_only() {
        let t = serving_topology();
        assert!(t.crosswired);
        assert!(t.cpu_only);
        assert_eq!(t.models.len(), 2);
    }

    #[test]
    fn validate_topology_accepts_locked_pair() {
        assert!(validate_topology(&locked_pair()).is_none());
    }

    #[test]
    fn validate_topology_rejects_silent_swap() {
        // An extra / different model is rejected — no black-box addition.
        let mut bad = locked_pair().to_vec();
        bad.push(LockedModel {
            id: 2,
            name: "Some-Other-Model-7B",
            params_m: 7000,
            backend: ServingBackend::CpuLlamaCpp,
            provenance: "unlocked",
        });
        assert!(validate_topology(&bad).is_some());
        // A renamed model is rejected too.
        let mut renamed = locked_pair();
        renamed[0].name = "LFM2.5-VL-2B";
        assert!(validate_topology(&renamed).is_some());
    }
}
