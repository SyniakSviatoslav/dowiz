//! `kernel::self_harness` — Self-harness with zone protection.
//!
//! Analyzes the entire dowiz project, generates blueprints for safe
//! rewrites, predicts impact dynamically, and enforces zone protection.
//! Hydra (M9) is forbidden from modification. Red/critical zones require
//! human operator approval.
//!
//! # Zone Model
//! - **Green**: fully safe to rewrite (tests, docs, helpers)
//! - **Yellow**: safe with verification (kernel internals, parallel patterns)
//! - **Red**: requires operator approval (orchestrator, workflow gate, crypto)
//! - **Critical**: requires dual-witness (P103 supervisor, PQ primitives)
//! - **Forbidden**: NEVER modify (Hydra kill-switch, command filter, G9 breach alarm)

use std::collections::HashMap;
use crate::TriState;

/// Zone severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Zone {
    Green = 0,
    Yellow = 1,
    Red = 2,
    Critical = 3,
    Forbidden = 4,
}

impl Zone {
    pub fn can_rewrite(&self) -> TriState { TriState::from_bool(*self < Zone::Red) }
    pub fn needs_operator(&self) -> TriState { TriState::from_bool(*self >= Zone::Red) }
    pub fn is_forbidden(&self) -> TriState { TriState::from_bool(*self == Zone::Forbidden) }
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::Green => write!(f, "GREEN"),
            Zone::Yellow => write!(f, "YELLOW"),
            Zone::Red => write!(f, "RED"),
            Zone::Critical => write!(f, "CRITICAL"),
            Zone::Forbidden => write!(f, "FORBIDDEN"),
        }
    }
}

// ─── File Entry ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub zone: Zone,
    pub lines: usize,
    pub test_count: usize,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub hash: [u8; 32],
}

// ─── Rewrite Blueprint ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RewriteBlueprint {
    pub target: String,
    pub zone: Zone,
    pub description: String,
    pub risk_score: f64,
    pub affected_files: Vec<String>,
    pub verification_steps: Vec<String>,
    pub rollback_hash: [u8; 32],
    pub predicted_latency_ms: f64,
    pub predicted_test_delta: i64,
}

impl RewriteBlueprint {
    pub fn is_safe(&self) -> TriState {
        TriState::from_bool(self.zone < Zone::Red)
    }
    pub fn needs_approval(&self) -> TriState {
        TriState::from_bool(self.zone >= Zone::Red)
    }
}

// ─── Impact Analysis ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    pub file: String,
    pub zone: Zone,
    pub direct_deps: Vec<String>,
    pub transitive_deps: Vec<String>,
    pub risk_score: f64,
    pub rewrite_estimate_ms: f64,
}

// ─── Zone Map ────────────────────────────────────────────────────────────

/// Known zone assignments for the dowiz kernel.
pub fn zone_for_path(path: &str) -> Zone {
    // Forbidden: Hydra kill-switch, command filter, G9 breach alarm.
    if path.contains("hydra") && (path.contains("kill") || path.contains("breach")) {
        return Zone::Forbidden;
    }
    // Critical: PQ primitives, P103 supervisor, crypto signer.
    if path.contains("pq/") || path.contains("p103") || path.contains("crypto_signer") {
        return Zone::Critical;
    }
    // Red: orchestrator, workflow gate, memory, intake firewall.
    if path.contains("orchestrator") || path.contains("workflow_gate")
        || path.contains("memory") || path.contains("intake")
    {
        return Zone::Red;
    }
    // Yellow: parallel patterns, dynamic spawner/actions, search, AGC, parse.
    if path.contains("parallel_") || path.contains("dynamic_")
        || path.contains("agc_") || path.contains("search")
        || path.contains("parse/") || path.contains("predict")
        || path.contains("swarm") || path.contains("detection")
        || path.contains("skill_extract") || path.contains("visual_index")
    {
        return Zone::Yellow;
    }
    // Green: everything else (tests, docs, helpers, CLI, hex_util).
    Zone::Green
}

// ─── Self Harness ────────────────────────────────────────────────────────

/// The self-harness for analyzing and predicting rewrites.
pub struct SelfHarness {
    pub files: Vec<FileEntry>,
    pub blueprints: Vec<RewriteBlueprint>,
    pub impacts: Vec<ImpactAnalysis>,
    pub zone_counts: HashMap<Zone, usize>,
    pub total_risk: f64,
}

impl SelfHarness {
    pub fn new() -> Self {
        SelfHarness {
            files: Vec::new(),
            blueprints: Vec::new(),
            impacts: Vec::new(),
            zone_counts: HashMap::new(),
            total_risk: 0.0,
        }
    }

    /// Register a file entry and compute its zone.
    pub fn register_file(&mut self, path: &str, lines: usize, test_count: usize,
                         imports: Vec<String>, exports: Vec<String>) {
        let zone = zone_for_path(path);
        let hash = crate::event_log::sha3_256(path.as_bytes());
        *self.zone_counts.entry(zone).or_insert(0) += 1;
        self.files.push(FileEntry {
            path: path.to_string(), zone, lines, test_count,
            imports, exports, hash,
        });
    }

    /// Analyze impact of rewriting a specific file.
    pub fn analyze_impact(&mut self, target: &str) -> ImpactAnalysis {
        let zone = zone_for_path(target);
        let direct_deps: Vec<String> = self.files.iter()
            .filter(|f| f.imports.iter().any(|i| i.contains(target)))
            .map(|f| f.path.clone())
            .collect();

        let transitive: Vec<String> = self.files.iter()
            .filter(|f| direct_deps.iter().any(|d| f.imports.iter().any(|i| i.contains(d.as_str()))))
            .map(|f| f.path.clone())
            .collect();

        let risk = match zone {
            Zone::Green => 0.1,
            Zone::Yellow => 0.3,
            Zone::Red => 0.7,
            Zone::Critical => 0.9,
            Zone::Forbidden => 1.0,
        };
        let estimate_ms = (direct_deps.len() as f64 * 50.0) + (transitive.len() as f64 * 20.0);

        ImpactAnalysis {
            file: target.to_string(), zone, direct_deps, transitive_deps: transitive,
            risk_score: risk, rewrite_estimate_ms: estimate_ms,
        }
    }

    /// Generate a rewrite blueprint.
    pub fn blueprint(&mut self, target: &str, description: &str) -> Option<RewriteBlueprint> {
        let zone = zone_for_path(target);
        if zone.is_forbidden().is_true() {
            return None; // Cannot create blueprint for forbidden zone.
        }
        let impact = self.analyze_impact(target);
        let rollback_hash = self.files.iter()
            .find(|f| f.path == target)
            .map(|f| f.hash)
            .unwrap_or([0u8; 32]);

        let bp = RewriteBlueprint {
            target: target.to_string(),
            zone,
            description: description.to_string(),
            risk_score: impact.risk_score,
            affected_files: impact.direct_deps.clone(),
            verification_steps: vec![
                format!("cargo test --features pq"),
                format!("cargo clippy --features pq -- -D warnings"),
                format!("git diff --stat HEAD"),
            ],
            rollback_hash,
            predicted_latency_ms: impact.rewrite_estimate_ms,
            predicted_test_delta: impact.direct_deps.len() as i64 * -2,
        };
        self.total_risk += bp.risk_score;
        self.blueprints.push(bp.clone());
        Some(bp)
    }

    /// Predict whether a rewrite is safe (no zone violations).
    pub fn predict_safe(&self, target: &str) -> TriState {
        zone_for_path(target).can_rewrite()
    }

    /// ASCII dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("Self-Harness Dashboard\n");
        out.push_str(&format!("  Files registered: {}\n", self.files.len()));
        out.push_str(&format!("  Blueprints:       {}\n", self.blueprints.len()));
        out.push_str(&format!("  Total risk:       {:.2}\n", self.total_risk));
        for zone in [Zone::Green, Zone::Yellow, Zone::Red, Zone::Critical, Zone::Forbidden] {
            let count = self.zone_counts.get(&zone).unwrap_or(&0);
            out.push_str(&format!("  {:<10} {:>4} files\n", format!("{}:", zone), count));
        }
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_for_hydra_is_forbidden() {
        assert_eq!(zone_for_path("src/hydra/kill_switch.rs"), Zone::Forbidden);
        assert_eq!(zone_for_path("src/hydra/breach_alarm.rs"), Zone::Forbidden);
    }

    #[test]
    fn zone_for_pq_is_critical() {
        assert_eq!(zone_for_path("src/pq/dsa.rs"), Zone::Critical);
        assert_eq!(zone_for_path("src/crypto_signer.rs"), Zone::Critical);
    }

    #[test]
    fn zone_for_orchestrator_is_red() {
        assert_eq!(zone_for_path("src/orchestrator.rs"), Zone::Red);
        assert_eq!(zone_for_path("src/workflow_gate.rs"), Zone::Red);
    }

    #[test]
    fn zone_for_parallel_is_yellow() {
        assert_eq!(zone_for_path("src/parallel_patterns.rs"), Zone::Yellow);
        assert_eq!(zone_for_path("src/dynamic_spawner.rs"), Zone::Yellow);
    }

    #[test]
    fn zone_for_misc_is_green() {
        assert_eq!(zone_for_path("src/hex_util.rs"), Zone::Green);
        assert_eq!(zone_for_path("tests/foo.rs"), Zone::Green);
    }

    #[test]
    fn forbidden_blocks_blueprint() {
        let mut h = SelfHarness::new();
        assert!(h.blueprint("src/hydra/kill_switch.rs", "test").is_none());
    }

    #[test]
    fn blueprint_generated_for_yellow() {
        let mut h = SelfHarness::new();
        let bp = h.blueprint("src/parallel_patterns.rs", "add caching");
        assert!(bp.is_some());
        assert_eq!(bp.unwrap().zone, Zone::Yellow);
    }

    #[test]
    fn predict_safe_green() {
        let h = SelfHarness::new();
        assert!(h.predict_safe("src/hex_util.rs").is_true());
        assert!(h.predict_safe("src/hydra/kill_switch.rs").is_false());
    }

    #[test]
    fn dashboard_contains_sections() {
        let h = SelfHarness::new();
        let d = h.dashboard();
        assert!(d.contains("Self-Harness Dashboard"));
    }

    #[test]
    fn impact_analysis_finds_deps() {
        let mut h = SelfHarness::new();
        h.register_file("src/foo.rs", 100, 5, vec!["bar".to_string()], vec!["Foo".to_string()]);
        h.register_file("src/bar.rs", 200, 10, vec![], vec!["Bar".to_string()]);
        let impact = h.analyze_impact("bar");
        assert!(!impact.direct_deps.is_empty());
    }
}
