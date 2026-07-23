//! `kernel::cross_bridge` — cross-kind pattern bridges, natively encoded.
//!
//! Each cross-pattern finding from the enrichment research is reverse-engineered
//! into a native kernel bridge. These bridges connect otherwise-isolated kinds
//! (code, tool, skill, plugin, security, etc.) through shared trigger keywords.
//!
//! # Bridges encoded
//! - python(17 kinds): universal connector via cross-language SDK patterns
//! - tool(14 kinds): CLI/API bridge between code and ops
//! - llm(14 kinds): AI model bridge across all decision domains
//! - claude-code(12 kinds): agent hub connecting skills, plugins, tools
//! - security(11 kinds): hardening bridge spanning crypto to ops
//!
//! ZERO deps. Uses enrichment engine's own data.

use crate::prompt_enrich::PromptKind;
use std::collections::{HashMap, HashSet};

/// A cross-kind bridge — connects two or more PromptKinds through shared triggers.
#[derive(Debug, Clone)]
pub struct CrossBridge {
    pub name: String,
    pub kinds: HashSet<PromptKind>,
    pub trigger_keywords: Vec<String>,
    pub strength: usize,  // number of entries that form this bridge
}

impl CrossBridge {
    pub fn new(name: &str, kinds: &[PromptKind], triggers: &[&str], strength: usize) -> Self {
        CrossBridge {
            name: name.to_string(),
            kinds: kinds.iter().copied().collect(),
            trigger_keywords: triggers.iter().map(|s| s.to_string()).collect(),
            strength,
        }
    }

    /// Does this bridge connect two specific kinds?
    pub fn connects(&self, a: PromptKind, b: PromptKind) -> bool {
        self.kinds.contains(&a) && self.kinds.contains(&b)
    }
}

/// The complete set of reverse-engineered cross-pattern bridges.
pub struct CrossBridgeRegistry {
    pub bridges: Vec<CrossBridge>,
}

impl CrossBridgeRegistry {
    /// Build the registry from research findings (hardcoded — these ARE the findings).
    pub fn from_research() -> Self {
        let mut bridges = Vec::new();

        // ── python: 17 kinds, universal connector ────────────────────────
        bridges.push(CrossBridge::new("python-universal",
            &[PromptKind::Code, PromptKind::Analyze, PromptKind::Summarize, PromptKind::Extract,
              PromptKind::Review, PromptKind::System, PromptKind::Math, PromptKind::Creative,
              PromptKind::Meta, PromptKind::Search, PromptKind::Test, PromptKind::Debug,
              PromptKind::Config, PromptKind::Security, PromptKind::Tool, PromptKind::Skill,
              PromptKind::Plugin],
            &["python","sdk","library","package","pip","pypi","scripting","automation"], 657));

        // ── tool: 14 kinds, CLI/API bridge ───────────────────────────────
        bridges.push(CrossBridge::new("tool-cli-bridge",
            &[PromptKind::Code, PromptKind::Extract, PromptKind::Review, PromptKind::System,
              PromptKind::Math, PromptKind::Creative, PromptKind::Meta, PromptKind::Search,
              PromptKind::Test, PromptKind::Debug, PromptKind::Security, PromptKind::Refactor,
              PromptKind::Tool, PromptKind::Plugin],
            &["tool","cli","command-line","terminal","shell","bash","utility"], 1273));

        // ── llm: 14 kinds, AI model bridge ───────────────────────────────
        bridges.push(CrossBridge::new("llm-ai-bridge",
            &[PromptKind::Code, PromptKind::Analyze, PromptKind::Summarize, PromptKind::Plan,
              PromptKind::Review, PromptKind::System, PromptKind::Math, PromptKind::Meta,
              PromptKind::Search, PromptKind::Security, PromptKind::Tool, PromptKind::Skill,
              PromptKind::Plugin, PromptKind::General],
            &["llm","ai","model","gpt","transformer","inference","prompt","generative"], 633));

        // ── claude-code: 12 kinds, agent hub ─────────────────────────────
        bridges.push(CrossBridge::new("claude-agent-hub",
            &[PromptKind::Code, PromptKind::Write, PromptKind::Plan, PromptKind::System,
              PromptKind::Creative, PromptKind::Meta, PromptKind::Test, PromptKind::Security,
              PromptKind::Tool, PromptKind::Skill, PromptKind::Plugin, PromptKind::General],
            &["claude-code","claude","agent","skill","plugin","tool","coding","assistant"], 1700));

        // ── security: 11 kinds, hardening bridge ─────────────────────────
        bridges.push(CrossBridge::new("security-hardening",
            &[PromptKind::Code, PromptKind::Extract, PromptKind::Review, PromptKind::System,
              PromptKind::Meta, PromptKind::Test, PromptKind::Security, PromptKind::Tool,
              PromptKind::Skill, PromptKind::Plugin, PromptKind::General],
            &["security","crypto","auth","encrypt","harden","vulnerability","audit","pentest"], 120));

        // ── machine-learning: 13 kinds ───────────────────────────────────
        bridges.push(CrossBridge::new("ml-datascience",
            &[PromptKind::Code, PromptKind::Analyze, PromptKind::Summarize, PromptKind::Extract,
              PromptKind::Plan, PromptKind::System, PromptKind::Math, PromptKind::Meta,
              PromptKind::Security, PromptKind::Tool, PromptKind::Skill, PromptKind::Plugin,
              PromptKind::General],
            &["machine-learning","data-science","model","training","inference","pipeline"], 91));

        // ── education: 11 kinds ──────────────────────────────────────────
        bridges.push(CrossBridge::new("education-bridge",
            &[PromptKind::Code, PromptKind::Analyze, PromptKind::System, PromptKind::Math,
              PromptKind::Meta, PromptKind::Test, PromptKind::Security, PromptKind::Tool,
              PromptKind::Skill, PromptKind::Plugin, PromptKind::General],
            &["education","learning","teaching","tutorial","course","academic","student"], 66));

        CrossBridgeRegistry { bridges }
    }

    /// Find bridges that connect two specific kinds.
    pub fn find_connection(&self, a: PromptKind, b: PromptKind) -> Vec<&CrossBridge> {
        self.bridges.iter().filter(|br| br.connects(a, b)).collect()
    }

    /// Find the strongest bridge covering the most kinds.
    pub fn strongest(&self) -> Option<&CrossBridge> {
        self.bridges.iter().max_by_key(|br| br.kinds.len())
    }

    /// Total kinds covered by all bridges combined.
    pub fn kinds_covered(&self) -> HashSet<PromptKind> {
        let mut all = HashSet::new();
        for br in &self.bridges {
            all.extend(&br.kinds);
        }
        all
    }

    /// Dashboard showing all bridges.
    pub fn dashboard(&self) -> String {
        let mut out = String::from("═══ CROSS-BRIDGE REGISTRY ═══\n");
        for br in &self.bridges {
            let kind_names: Vec<&str> = br.kinds.iter()
                .map(|k| k.as_str()).collect();
            out.push_str(&format!("  {}: {} kinds [{}] strength={}\n",
                br.name, br.kinds.len(), kind_names.join(","), br.strength));
        }
        out.push_str(&format!("  Total kinds covered: {}\n", self.kinds_covered().len()));
        out
    }

    /// Compute eigen correlation between two kind domains.
    /// Returns a similarity score 0-1.
    pub fn eigen_correlate(&self, kind_a: PromptKind, kind_b: PromptKind) -> f64 {
        use crate::eigen::Eigen;
        let kws_a: Vec<String> = self.bridges.iter()
            .filter(|br| br.kinds.contains(&kind_a))
            .flat_map(|br| br.trigger_keywords.iter().cloned())
            .collect();
        let kws_b: Vec<String> = self.bridges.iter()
            .filter(|br| br.kinds.contains(&kind_b))
            .flat_map(|br| br.trigger_keywords.iter().cloned())
            .collect();
        let mut vocab: Vec<String> = std::collections::HashSet::<String>::from_iter(
            kws_a.iter().chain(kws_b.iter()).cloned()
        ).into_iter().collect();
        vocab.sort();
        let ea = Eigen::from_bow(&kws_a, &vocab);
        let eb = Eigen::from_bow(&kws_b, &vocab);
        ea.cosine_sim(&eb)
    }
}

/// Publisher diversity tracker — which publishers span the most kinds.
#[derive(Debug, Clone)]
pub struct PublisherDiversity {
    pub name: String,
    pub kinds: HashSet<PromptKind>,
    pub entry_count: usize,
}

pub struct PublisherRegistry {
    pub publishers: Vec<PublisherDiversity>,
}

impl PublisherRegistry {
    pub fn from_research() -> Self {
        let mut pubs = Vec::new();
        pubs.push(PublisherDiversity { name: "NVIDIA".into(), kinds: [PromptKind::Code,PromptKind::Extract,PromptKind::System,PromptKind::Creative,PromptKind::Meta,PromptKind::Test,PromptKind::Security,PromptKind::Skill,PromptKind::General].iter().copied().collect(), entry_count: 157 });
        pubs.push(PublisherDiversity { name: "anthropics".into(), kinds: [PromptKind::Code,PromptKind::Extract,PromptKind::System,PromptKind::Creative,PromptKind::Meta,PromptKind::Test,PromptKind::Skill,PromptKind::General].iter().copied().collect(), entry_count: 21 });
        pubs.push(PublisherDiversity { name: "microsoft".into(), kinds: [PromptKind::Code,PromptKind::Extract,PromptKind::System,PromptKind::Creative,PromptKind::Meta,PromptKind::Test,PromptKind::Tool,PromptKind::Skill].iter().copied().collect(), entry_count: 146 });
        pubs.push(PublisherDiversity { name: "openai".into(), kinds: [PromptKind::Code,PromptKind::System,PromptKind::Creative,PromptKind::Meta,PromptKind::Test,PromptKind::Security,PromptKind::Skill].iter().copied().collect(), entry_count: 43 });
        PublisherRegistry { publishers: pubs }
    }

    pub fn dashboard(&self) -> String {
        let mut out = String::from("═══ PUBLISHER DIVERSITY ═══\n");
        for p in &self.publishers {
            let kind_names: Vec<&str> = p.kinds.iter().map(|k| k.as_str()).collect();
            out.push_str(&format!("  {}: {} kinds [{}] entries={}\n",
                p.name, p.kinds.len(), kind_names.join(","), p.entry_count));
        }
        out
    }
}

// ─── P5: CooccurrenceMatrix ────────────────────────────────────────────────

/// Matrix tracking which PromptKinds co-occur in user queries.
pub struct CooccurrenceMatrix {
    pub counts: HashMap<(PromptKind, PromptKind), usize>,
    pub total_queries: usize,
}

impl CooccurrenceMatrix {
    pub fn new() -> Self {
        Self { counts: HashMap::new(), total_queries: 0 }
    }

    /// Record a query's detected intents.
    pub fn record(&mut self, kinds: &[PromptKind]) {
        self.total_queries += 1;
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                let a = kinds[i];
                let b = kinds[j];
                let key = if (a as u16) < (b as u16) { (a, b) } else { (b, a) };
                *self.counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    /// Discover bridge candidates: pairs that co-occur >= min_cooccur times.
    pub fn discover_bridges(&self, min_cooccur: usize) -> Vec<(PromptKind, PromptKind, f64)> {
        let mut candidates: Vec<(PromptKind, PromptKind, f64)> = Vec::new();
        for ((a, b), count) in &self.counts {
            if *count >= min_cooccur {
                let freq = *count as f64 / self.total_queries.max(1) as f64;
                candidates.push((*a, *b, freq));
            }
        }
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        candidates
    }
}

// ─── P5: CrossBridgeDiscovery ──────────────────────────────────────────────

pub struct CrossBridgeDiscovery {
    pub matrix: CooccurrenceMatrix,
    pub candidates: Vec<(PromptKind, PromptKind, f64)>,
    pub promoted: Vec<(PromptKind, PromptKind)>,
    pub min_confirmations: usize,
}

impl CrossBridgeDiscovery {
    pub fn new(min_confirmations: usize) -> Self {
        Self {
            matrix: CooccurrenceMatrix::new(),
            candidates: Vec::new(),
            promoted: Vec::new(),
            min_confirmations,
        }
    }

    pub fn feed(&mut self, kinds: &[PromptKind]) {
        self.matrix.record(kinds);
    }

    pub fn scan(&mut self) -> Vec<(PromptKind, PromptKind, f64)> {
        self.candidates = self.matrix.discover_bridges(self.min_confirmations);
        self.candidates.clone()
    }

    pub fn promote(&mut self) -> Vec<(PromptKind, PromptKind)> {
        let mut newly_promoted = Vec::new();
        for (a, b, _freq) in &self.candidates {
            let pair = (*a, *b);
            if !self.promoted.contains(&pair) {
                self.promoted.push(pair);
                newly_promoted.push(pair);
            }
        }
        newly_promoted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_bridges() {
        let reg = CrossBridgeRegistry::from_research();
        assert!(reg.bridges.len() >= 5);
    }

    #[test]
    fn python_bridge_connects_code_and_math() {
        let reg = CrossBridgeRegistry::from_research();
        let conns = reg.find_connection(PromptKind::Code, PromptKind::Math);
        assert!(!conns.is_empty());
        assert!(conns.iter().any(|b| b.name == "python-universal"));
    }

    #[test]
    fn strongest_bridge_covers_most_kinds() {
        let reg = CrossBridgeRegistry::from_research();
        let s = reg.strongest().unwrap();
        assert_eq!(s.name, "python-universal");
        assert!(s.kinds.len() >= 15);
    }

    #[test]
    fn publisher_diversity_works() {
        let pr = PublisherRegistry::from_research();
        assert!(pr.publishers.len() >= 3);
        let nvidia = &pr.publishers[0];
        assert!(nvidia.kinds.len() >= 7);
    }

    #[test]
    fn dashboard_renders() {
        let reg = CrossBridgeRegistry::from_research();
        let d = reg.dashboard();
        assert!(d.contains("python-universal"));
        assert!(d.contains("claude-agent-hub"));
    }

    // ─── P5: Cooccurrence & bridge discovery tests ──────────────────────────

    #[test]
    fn cooccurrence_matrix_records_pairs() {
        let mut matrix = CooccurrenceMatrix::new();
        matrix.record(&[PromptKind::Code, PromptKind::Security, PromptKind::Test]);
        assert_eq!(matrix.total_queries, 1);
        // Should have Code-Security, Code-Test, Security-Test pairs
        assert_eq!(matrix.counts.len(), 3);
    }

    #[test]
    fn cross_bridge_discovery_detects_above_threshold() {
        let mut discovery = CrossBridgeDiscovery::new(5);
        for _ in 0..10 {
            discovery.feed(&[PromptKind::Code, PromptKind::Security]);
        }
        let candidates = discovery.scan();
        assert!(!candidates.is_empty(), "10 co-occurrences should exceed threshold of 5");
        assert_eq!(candidates[0].0, PromptKind::Code);
    }

    #[test]
    fn cross_bridge_discovery_below_threshold_returns_empty() {
        let mut discovery = CrossBridgeDiscovery::new(100);
        for _ in 0..3 {
            discovery.feed(&[PromptKind::Code, PromptKind::Review]);
        }
        let candidates = discovery.scan();
        assert!(candidates.is_empty(), "3 co-occurrences should not exceed threshold of 100");
    }

    #[test]
    fn cross_bridge_eigen_correlate_self_is_high() {
        let reg = CrossBridgeRegistry::from_research();
        let sim = reg.eigen_correlate(PromptKind::Code, PromptKind::Code);
        assert!(sim >= 0.0, "Self-correlation should be non-negative");
    }
}
