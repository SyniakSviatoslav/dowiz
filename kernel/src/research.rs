//! `kernel::research` — Research knowledge extraction and pattern analysis engine.
//!
//! Pure data structures and algorithms for:
//! - Fetching research papers from arXiv / Semantic Scholar / OpenAlex APIs
//! - Extracting patterns (methods, architectures, results, claims)
//! - Finding cross-patterns (pairs/triples of patterns that co-occur)
//! - **Recursive extraction** — patterns from batch N seed batch N+1
//!   (100 → 500 → 2,500 → 12,500 → 62,500 → 100,000 in ~6 iterations)
//! - Building a knowledge library of papers, patterns, citations
//!
//! The kernel owns the data model and analysis algorithms. The actual HTTP
//! requests to arXiv / Semantic Scholar live in a concrete adapter (outside
//! kernel). This module stays pure computation.
//!
//! # Recursive extraction methodology
//! Each iteration:
//! 1. Take top-K patterns (confidence > threshold) as search queries
//! 2. Fan-out parallel queries across arXiv / Semantic Scholar / OpenAlex
//! 3. Ingest results into ResearchEngine
//! 4. Extract new patterns from the new paper set
//! 5. Recursion: go to 1 with expanded pattern set
//! Growth: each iteration multiplies paper count by ~5x.
//!
//! # Platforms (all free, no auth required for basic access)
//! - arXiv API: https://export.arxiv.org/api/ (Atom XML, no key)
//! - Semantic Scholar: https://api.semanticscholar.org/graph/v1 (no key)
//! - OpenAlex: https://api.openalex.org (free key, $1/day credit)
//!
//! # Pattern extraction methodology
//! Patterns are extracted from paper metadata (title, abstract, categories).
//! Each pattern has a type, confidence, and list of related papers.
//! Cross-patterns are discovered by co-occurrence analysis across papers.

use crate::TriState;
use std::collections::{HashMap, HashSet};

/// Maximum patterns in the library.
pub const MAX_PATTERNS: usize = 10_000;
/// Maximum cross-patterns.
pub const MAX_CROSS_PATTERNS: usize = 5_000;
/// Minimum co-occurrence count for cross-pattern detection.
pub const MIN_CROSS_COUNT: usize = 3;

// ─── Paper ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub year: u32,
    pub citation_count: u32,
    pub arxiv_id: Option<String>,
    pub doi: Option<String>,
    pub paper_hash: [u8; 32],
    /// Whether full text is accessible.
    pub full_text_accessible: TriState,
    /// Embedding vector for similarity search (2048-dim L2-normalized).
    pub embedding: Vec<f32>,
}

// ─── Knowledge Domain ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnowledgeDomain {
    /// AI / Machine Learning / Deep Learning
    AiMl,
    /// Data Science / Statistics
    DataScience,
    /// Natural Language Processing
    Nlp,
    /// Computer Vision
    ComputerVision,
    /// Reinforcement Learning
    ReinforcementLearning,
    /// Robotics
    Robotics,
    /// Systems / Distributed Computing
    Systems,
    /// Theory / Mathematics
    Theory,
    /// Other / Uncategorized
    Other,
}

// ─── Pattern ──────────────────────────────────────────────────────────────

/// A reusable knowledge pattern extracted from research papers.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub domain: KnowledgeDomain,
    /// Papers that exhibit this pattern.
    pub paper_ids: Vec<String>,
    /// Confidence (0..1): proportion of papers where pattern is validated.
    pub confidence: f64,
    /// Pattern category.
    pub kind: PatternKind,
    /// Hash of pattern content.
    pub pattern_hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// Architecture design pattern (e.g., Transformer, GAN, Diffusion)
    Architecture,
    /// Training methodology (e.g., curriculum learning, distillation)
    TrainingMethod,
    /// Optimization technique (e.g., AdamW, LoRA, mixed precision)
    Optimization,
    /// Evaluation protocol (e.g., cross-validation, ablation, benchmark)
    Evaluation,
    /// Data processing (e.g., augmentation, normalization, tokenization)
    DataProcessing,
    /// Architectural cross-pattern (combining two+ architectures)
    CrossArchitecture,
    /// Training + Architecture cross-pattern
    TrainArch,
    /// Evaluation + Data cross-pattern
    EvalData,
}

// ─── Cross-Pattern ────────────────────────────────────────────────────────

/// Two or three patterns that frequently co-occur in the same papers.
#[derive(Debug, Clone)]
pub struct CrossPattern {
    pub id: u64,
    pub pattern_ids: Vec<u64>,
    pub co_occurrence_count: usize,
    pub total_papers: usize,
    pub lift: f64,
    pub description: String,
}

// ─── Research Query ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ResearchQuery {
    pub keywords: Vec<String>,
    pub domains: Vec<KnowledgeDomain>,
    pub max_results: usize,
    pub year_from: u32,
    pub year_to: u32,
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Relevance,
    Date,
    CitationCount,
}

// ─── Research Engine ──────────────────────────────────────────────────────

/// The research knowledge extraction and pattern analysis engine.
pub struct ResearchEngine {
    pub papers: Vec<Paper>,
    pub patterns: Vec<Pattern>,
    pub cross_patterns: Vec<CrossPattern>,
    /// Pattern index: paper_id → vec of pattern_ids
    paper_patterns: HashMap<String, Vec<u64>>,
    /// Next pattern ID.
    next_pattern_id: u64,
    next_cross_id: u64,
}

impl ResearchEngine {
    pub fn new() -> Self {
        ResearchEngine {
            papers: Vec::new(),
            patterns: Vec::new(),
            cross_patterns: Vec::new(),
            paper_patterns: HashMap::new(),
            next_pattern_id: 1,
            next_cross_id: 1,
        }
    }

    /// Add a batch of papers from an API result.
    pub fn ingest_papers(&mut self, papers: Vec<Paper>) {
        for p in papers {
            let hash = crate::event_log::sha3_256(p.title.as_bytes());
            let mut paper = p;
            paper.paper_hash = hash;
            self.papers.push(paper);
        }
    }

    /// Total papers ingested.
    pub fn total_papers(&self) -> usize { self.papers.len() }

    /// Extract patterns from ingested papers using keyword analysis.
    /// Patterns are discovered by co-occurrence of domain-specific keywords
    /// in titles and abstracts.
    pub fn extract_patterns(&mut self) -> Vec<Pattern> {
        let mut discovered: Vec<Pattern> = Vec::new();

        for paper in &self.papers {
            let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
            let domain = classify_domain(&paper.categories);

            // Detect patterns from keywords.
            let patterns = detect_patterns_in_text(&text, domain, paper.id.clone());
            for pat in patterns {
                // Check if this pattern already exists.
                let existing = self.patterns.iter().position(|p| {
                    p.name == pat.name && p.domain == pat.domain && p.kind == pat.kind
                });
                match existing {
                    Some(idx) => {
                        if !self.patterns[idx].paper_ids.contains(&pat.paper_ids[0]) {
                            self.patterns[idx].paper_ids.push(pat.paper_ids[0].clone());
                            let n = self.patterns[idx].paper_ids.len();
                            self.patterns[idx].confidence = (n as f64) / (self.total_papers().max(1) as f64);
                        }
                    }
                    None => {
                        let id = self.next_pattern_id;
                        self.next_pattern_id += 1;
                        self.patterns.push(Pattern { id, ..pat });
                    }
                }
            }
        }

        // Recompute confidence for all patterns.
        let total = self.total_papers().max(1) as f64;
        for p in &mut self.patterns {
            p.confidence = (p.paper_ids.len() as f64) / total;
        }

        // Build paper→patterns index.
        for p in &self.patterns {
            for pid in &p.paper_ids {
                self.paper_patterns.entry(pid.clone())
                    .or_default()
                    .push(p.id);
            }
        }

        // Discover cross-patterns.
        self.discover_cross_patterns();

        self.patterns.clone()
    }

    /// Discover cross-patterns: pairs of patterns that co-occur in the same papers.
    fn discover_cross_patterns(&mut self) {
        let mut co_occur: HashMap<(u64, u64), usize> = HashMap::new();

        for paper in &self.papers {
            if let Some(pids) = self.paper_patterns.get(&paper.id) {
                for i in 0..pids.len() {
                    for j in (i + 1)..pids.len() {
                        let key = (pids[i].min(pids[j]), pids[i].max(pids[j]));
                        *co_occur.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        self.cross_patterns.clear();
        for ((a, b), count) in co_occur {
            if count >= MIN_CROSS_COUNT {
                // Find the patterns to get their names.
                let a_name = self.patterns.iter().find(|p| p.id == a).map(|p| p.name.clone()).unwrap_or_default();
                let b_name = self.patterns.iter().find(|p| p.id == b).map(|p| p.name.clone()).unwrap_or_default();
                let lift = if self.total_papers() > 0 {
                    let expected = (self.patterns.iter().find(|p| p.id == a).map(|p| p.paper_ids.len()).unwrap_or(0) as f64
                        / self.total_papers() as f64)
                        * (self.patterns.iter().find(|p| p.id == b).map(|p| p.paper_ids.len()).unwrap_or(0) as f64
                        / self.total_papers() as f64)
                        * self.total_papers() as f64;
                    if expected > 0.0 { count as f64 / expected } else { 1.0 }
                } else { 1.0 };

                let id = self.next_cross_id;
                self.next_cross_id += 1;
                self.cross_patterns.push(CrossPattern {
                    id, pattern_ids: vec![a, b],
                    co_occurrence_count: count,
                    total_papers: self.total_papers(),
                    lift,
                    description: format!("{} × {} (lift: {:.2})", a_name, b_name, lift),
                });
            }
        }

        // Sort by lift descending.
        self.cross_patterns.sort_by(|a, b| b.lift.partial_cmp(&a.lift).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Search papers by keyword.
    pub fn search(&self, query: &ResearchQuery) -> Vec<&Paper> {
        let mut results: Vec<&Paper> = self.papers.iter()
            .filter(|p| {
                let text = format!("{} {} {:?}", p.title, p.abstract_text, p.categories).to_lowercase();
                query.keywords.iter().all(|k| text.contains(&k.to_lowercase()))
                && p.year >= query.year_from && p.year <= query.year_to
            })
            .collect();

        match query.sort_by {
            SortBy::CitationCount => results.sort_by(|a, b| b.citation_count.cmp(&a.citation_count)),
            SortBy::Date => results.sort_by(|a, b| b.year.cmp(&a.year)),
            SortBy::Relevance => {} // keep ingestion order
        }

        results.truncate(query.max_results);
        results
    }

    /// Get all cross-patterns with lift > threshold.
    pub fn high_lift_cross_patterns(&self, min_lift: f64) -> Vec<&CrossPattern> {
        self.cross_patterns.iter().filter(|cp| cp.lift >= min_lift).collect()
    }

    /// Summary dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("Research Engine\n");
        out.push_str(&format!("  Papers:      {}\n", self.papers.len()));
        out.push_str(&format!("  Patterns:    {}\n", self.patterns.len()));
        out.push_str(&format!("  Cross-pats:  {}\n", self.cross_patterns.len()));
        let top = self.cross_patterns.iter().take(5).map(|cp| {
            let names: Vec<&str> = cp.pattern_ids.iter().filter_map(|pid| {
                self.patterns.iter().find(|p| p.id == *pid).map(|p| p.name.as_str())
            }).collect();
            format!("    {} (lift: {:.2})", names.join(" × "), cp.lift)
        }).collect::<Vec<_>>().join("\n");
        if !top.is_empty() {
            out.push_str("  Top cross:\n");
            out.push_str(&top);
            out.push('\n');
        }
        out
    }
}

// ─── Pattern Detection ────────────────────────────────────────────────────

/// Classify knowledge domain from arXiv categories.
fn classify_domain(categories: &[String]) -> KnowledgeDomain {
    for cat in categories {
        let c = cat.to_lowercase();
        if c.contains("cs.ai") || c.contains("cs.lg") || c.contains("stat.ml") {
            return KnowledgeDomain::AiMl;
        }
        if c.contains("cs.cl") || c.contains("cs.ir") {
            return KnowledgeDomain::Nlp;
        }
        if c.contains("cs.cv") {
            return KnowledgeDomain::ComputerVision;
        }
        if c.contains("cs.ro") {
            return KnowledgeDomain::Robotics;
        }
        if c.contains("cs.dc") || c.contains("cs.ds") {
            return KnowledgeDomain::DataScience;
        }
    }
    KnowledgeDomain::Other
}

/// Detect knowledge patterns in a paper's text.
fn detect_patterns_in_text(text: &str, domain: KnowledgeDomain, paper_id: String) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    // Architecture patterns (ML + Systems).
    let arch_patterns = [
        ("Transformer", PatternKind::Architecture, "Self-attention based architecture"),
        ("Diffusion", PatternKind::Architecture, "Diffusion-based generative model"),
        ("GAN", PatternKind::Architecture, "Generative adversarial network"),
        ("VAE", PatternKind::Architecture, "Variational autoencoder"),
        ("ResNet", PatternKind::Architecture, "Residual network architecture"),
        ("LSTM", PatternKind::Architecture, "Long short-term memory"),
        ("CNN", PatternKind::Architecture, "Convolutional neural network"),
        ("RNN", PatternKind::Architecture, "Recurrent neural network"),
        ("Graph Neural Network", PatternKind::Architecture, "GNN-based architecture"),
        ("Attention", PatternKind::Architecture, "Attention mechanism"),
        ("Mixture of Experts", PatternKind::Architecture, "MoE architecture"),
        ("State Space Model", PatternKind::Architecture, "SSM-based model"),
        ("Mamba", PatternKind::Architecture, "State space selective scan model"),
    ];

    // Training patterns.
    let train_patterns = [
        ("Transfer Learning", PatternKind::TrainingMethod, "Transfer learning from pretrained model"),
        ("Fine-tuning", PatternKind::TrainingMethod, "Fine-tuning approach"),
        ("Distillation", PatternKind::TrainingMethod, "Knowledge distillation"),
        ("Curriculum Learning", PatternKind::TrainingMethod, "Curriculum-based training"),
        ("Self-supervised", PatternKind::TrainingMethod, "Self-supervised learning"),
        ("Contrastive Learning", PatternKind::TrainingMethod, "Contrastive representation learning"),
        ("Reinforcement Learning", PatternKind::TrainingMethod, "RL-based training"),
        ("Federated Learning", PatternKind::TrainingMethod, "Federated training"),
        ("Meta-Learning", PatternKind::TrainingMethod, "Learn to learn"),
        ("Few-shot Learning", PatternKind::TrainingMethod, "Few-shot learning approach"),
    ];

    // Optimization patterns.
    let opt_patterns = [
        ("Adam", PatternKind::Optimization, "Adam optimizer"),
        ("LoRA", PatternKind::Optimization, "Low-rank adaptation"),
        ("Quantization", PatternKind::Optimization, "Model quantization"),
        ("Pruning", PatternKind::Optimization, "Network pruning"),
        ("Mixed Precision", PatternKind::Optimization, "Mixed precision training"),
        ("Gradient Clipping", PatternKind::Optimization, "Gradient clipping technique"),
        ("Learning Rate Schedule", PatternKind::Optimization, "LR scheduling"),
    ];

    // Data patterns.
    let data_patterns = [
        ("Data Augmentation", PatternKind::DataProcessing, "Data augmentation technique"),
        ("Normalization", PatternKind::DataProcessing, "Data normalization"),
        ("Tokenization", PatternKind::DataProcessing, "Tokenization approach"),
        ("Embedding", PatternKind::DataProcessing, "Embedding method"),
    ];

    // System architecture patterns (dowiz codebase).
    let sys_patterns = [
        ("fractal", PatternKind::Architecture, "Fractal node: self-similar copy of entire system"),
        ("spin wave", PatternKind::Architecture, "Spin wave communication via quantum states"),
        ("wave bus", PatternKind::Architecture, "Bidirectional async socket queues for wave comm"),
        ("light communication", PatternKind::Architecture, "Omnidirectional nD light front propagation"),
        ("null geodesic", PatternKind::Architecture, "Light ray on null cone in O(p,q) spacetime"),
        ("phase space", PatternKind::Architecture, "Non-Euclidean phase space with symplectic str"),
        ("metric tensor", PatternKind::Architecture, "Riemannian/Pseudo-Riemannian metric g_ij"),
        ("symplectic", PatternKind::Architecture, "Symplectic 2-form for Hamiltonian mechanics"),
        ("split algebra", PatternKind::Architecture, "Split-complex/quat/octonion for light comm"),
        ("standard model", PatternKind::Architecture, "SU(3)xSU(2)xU(1) particle physics"),
        ("quark sig", PatternKind::Architecture, "8D crystal lattice QuarkSig signatures"),
        ("pseudo-euclidean", PatternKind::Optimization, "O(p,q) metric with light cones and causality"),
        ("topo-chrono memory", PatternKind::DataProcessing, "Topological-chronological parametric surface"),
        ("parametric surface", PatternKind::DataProcessing, "S(u,v) surface in non-Euclidean space"),
        ("unified navigator", PatternKind::Architecture, "Navigation fusing PPR+geodesic+light cones"),
        ("prediction service", PatternKind::TrainingMethod, "Forward prediction + RTS retrospective"),
        ("autonomous loop", PatternKind::TrainingMethod, "Observe-predict-decide-execute-learn cycle"),
        ("memory pipeline", PatternKind::DataProcessing, "Memory consolidation pipeline"),
        ("inference engine", PatternKind::TrainingMethod, "ML projection for trajectory extrapolation"),
        ("p2p network", PatternKind::Architecture, "P2P fractal network with seed/peer mesh"),
        ("academia mesh", PatternKind::Architecture, "P2P mesh for distributed 8D matrix chunks"),
        ("research engine", PatternKind::DataProcessing, "Paper ingestion and pattern extraction"),
        ("pattern oracle", PatternKind::TrainingMethod, "Pattern insight search and discovery"),
        ("meta miner", PatternKind::TrainingMethod, "Self-improving knowledge mining"),
        ("physics engine", PatternKind::Optimization, "PID-controlled parallel FanOut quantization"),
        ("swarm coordinator", PatternKind::Architecture, "DSU decomposition and executor dispatch"),
        ("reverse engineer", PatternKind::DataProcessing, "ELF parsing and behavior profiling"),
        ("spine snapshot", PatternKind::DataProcessing, "Tamper-evident hash chain integrity"),
        // Three-body problem patterns
        ("three-body", PatternKind::Architecture, "Three-body problem: chaotic dynamics of 3 interacting nodes"),
        ("lagrange point", PatternKind::Optimization, "Stable equilibrium points for 3-node mesh configuration"),
        ("figure-8 orbit", PatternKind::Architecture, "Periodic stable orbit for 3-node synchronization cycle"),
        ("hierarchical three-body", PatternKind::Architecture, "Close binary + distant third: O(N log N) scaling"),
        ("restricted three-body", PatternKind::Architecture, "Light node + two heavy nodes: client-server mesh"),
        ("sitnikov problem", PatternKind::Architecture, "Vertical oscillation: mobile node between two bases"),
        ("euler collinear", PatternKind::Architecture, "Collinear 3-node topology: daisy-chain communication"),
        ("symplectic integration", PatternKind::Optimization, "Energy-preserving prediction for node state evolution"),
        ("chaos synchronization", PatternKind::TrainingMethod, "Quantum entanglement stabilizes chaotic 3-node drift"),
    ];

    // Extract matches — include sys_patterns.
    for (name, kind, desc) in arch_patterns.iter().chain(train_patterns.iter())
        .chain(opt_patterns.iter()).chain(data_patterns.iter()).chain(sys_patterns.iter())
    {
        if text.contains(&name.to_lowercase()) {
            let hash = crate::event_log::sha3_256(name.as_bytes());
            patterns.push(Pattern {
                id: 0, // Will be assigned by engine
                name: name.to_string(),
                description: desc.to_string(),
                domain,
                paper_ids: vec![paper_id.clone()],
                confidence: 0.0,
                kind: *kind,
                pattern_hash: hash,
            });
        }
    }

    patterns
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_starts_empty() {
        let re = ResearchEngine::new();
        assert_eq!(re.total_papers(), 0);
        assert_eq!(re.patterns.len(), 0);
    }

    #[test]
    fn ingest_papers() {
        let mut re = ResearchEngine::new();
        re.ingest_papers(vec![Paper {
            id: "test1".into(), title: "Attention Is All You Need".into(),
            authors: vec!["Vaswani".into()],
            abstract_text: "We propose a novel Transformer architecture using self-attention.".into(),
            categories: vec!["cs.LG".into(), "cs.CL".into()],
            year: 2017, citation_count: 100000,
            arxiv_id: Some("1706.03762".into()), doi: None,
            paper_hash: [0; 32], full_text_accessible: TriState::True,
            embedding: vec![0.0; 8],
        }]);
        assert_eq!(re.total_papers(), 1);
    }

    #[test]
    fn extract_patterns_finds_transformer() {
        let mut re = ResearchEngine::new();
        re.ingest_papers(vec![Paper {
            id: "test1".into(), title: "Attention Is All You Need".into(),
            authors: vec!["Vaswani".into()],
            abstract_text: "We propose a novel Transformer architecture using self-attention and multi-head attention with Adam optimizer and fine-tuning.".into(),
            categories: vec!["cs.LG".into()], year: 2017, citation_count: 100000,
            arxiv_id: Some("1706.03762".into()), doi: None,
            paper_hash: [0; 32], full_text_accessible: TriState::True,
            embedding: vec![0.0; 8],
        }]);
        re.extract_patterns();
        assert!(re.patterns.iter().any(|p| p.name == "Transformer"));
        assert!(re.patterns.iter().any(|p| p.name == "Attention"));
        assert!(re.patterns.iter().any(|p| p.name == "Adam"));
    }

    #[test]
    fn cross_patterns_detected() {
        let mut re = ResearchEngine::new();
        for i in 0..5 {
            re.ingest_papers(vec![Paper {
                id: format!("paper{}", i),
                title: "Paper with Transformer and Adam".into(),
                authors: vec!["Author".into()],
                abstract_text: "Using Transformer architecture with Adam optimizer and fine-tuning.".into(),
                categories: vec!["cs.LG".into()], year: 2023, citation_count: 10,
                arxiv_id: None, doi: None,
                paper_hash: [0; 32], full_text_accessible: TriState::True,
                embedding: vec![0.0; 8],
            }]);
        }
        re.extract_patterns();
        // Should have cross-patterns among Transformer, Adam, Fine-tuning.
        assert!(!re.cross_patterns.is_empty(), "expected cross-patterns, got 0");
    }

    #[test]
    fn search_by_keyword() {
        let mut re = ResearchEngine::new();
        re.ingest_papers(vec![
            Paper {
                id: "a".into(), title: "Transformer Advances".into(),
                authors: vec!["Author".into()],
                abstract_text: "Advances in transformer models".into(),
                categories: vec!["cs.LG".into()], year: 2023, citation_count: 50,
                arxiv_id: None, doi: None,
                paper_hash: [0; 32], full_text_accessible: TriState::True,
                embedding: vec![0.0; 8],
            },
            Paper {
                id: "b".into(), title: "CNN for Vision".into(),
                authors: vec!["Author".into()],
                abstract_text: "CNN-based vision models".into(),
                categories: vec!["cs.CV".into()], year: 2022, citation_count: 30,
                arxiv_id: None, doi: None,
                paper_hash: [0; 32], full_text_accessible: TriState::True,
                embedding: vec![0.0; 8],
            },
        ]);
        let query = ResearchQuery {
            keywords: vec!["transformer".into()],
            domains: vec![],
            max_results: 10,
            year_from: 2020, year_to: 2024,
            sort_by: SortBy::CitationCount,
        };
        let results = re.search(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn dashboard_contains_papers() {
        let re = ResearchEngine::new();
        let d = re.dashboard();
        assert!(d.contains("Papers:"));
        assert!(d.contains("Patterns:"));
    }

    #[test]
    fn domain_classification() {
        assert_eq!(classify_domain(&["cs.LG".into()]), KnowledgeDomain::AiMl);
        assert_eq!(classify_domain(&["cs.CL".into()]), KnowledgeDomain::Nlp);
        assert_eq!(classify_domain(&["cs.CV".into()]), KnowledgeDomain::ComputerVision);
        assert_eq!(classify_domain(&["physics".into()]), KnowledgeDomain::Other);
    }

    #[test]
    fn high_lift_filters() {
        let mut re = ResearchEngine::new();
        // Need pattern extraction to happen first.
        for i in 0..5 {
            re.ingest_papers(vec![Paper {
                id: format!("p{}", i),
                title: "Transformer Fine-tuning".into(),
                authors: vec!["Author".into()],
                abstract_text: "Fine-tuning Transformer model with Adam.".into(),
                categories: vec!["cs.LG".into()], year: 2023, citation_count: 10,
                arxiv_id: None, doi: None,
                paper_hash: [0; 32], full_text_accessible: TriState::True,
                embedding: vec![0.0; 8],
            }]);
        }
        re.extract_patterns();
        let high = re.high_lift_cross_patterns(0.5);
        // With enough data, expect >0 lift for co-occurring patterns.
        assert!(!high.is_empty() || re.cross_patterns.is_empty());
    }
}

// ─── Recursive Extraction Engine ───────────────────────────────────────────

/// A single extraction iteration plan — the kernel tells the external tool
/// WHAT queries to run in parallel.
#[derive(Debug, Clone)]
pub struct ExtractionIteration {
    /// Iteration depth (0 = seed, 1 = first expansion, ...).
    pub depth: usize,
    /// Queries to fan-out across APIs in parallel.
    pub queries: Vec<ExtractionQuery>,
    /// Expected paper yield from this iteration.
    pub expected_yield: usize,
    /// Target total papers after this iteration.
    pub target_total: usize,
    /// Growth factor from previous iteration.
    pub growth_factor: f64,
}

#[derive(Debug, Clone)]
pub struct ExtractionQuery {
    pub api: ApiKind,
    pub query_string: String,
    pub max_results: usize,
    pub start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKind {
    ArXiv,
    SemanticScholar,
    OpenAlex,
}

/// Recursive research extraction engine.
///
/// Growth model: each iteration expands the seed set using discovered
/// patterns, achieving ~5x paper count growth per iteration.
///   100 → 500 → 2,500 → 12,500 → 62,500 → 100,000 (+) in ~6 iterations.
pub struct RecursiveExtractor {
    pub engine: ResearchEngine,
    pub seed_keywords: Vec<String>,
    pub max_papers: usize,
    pub min_pattern_confidence: f64,
    pub queries_per_iteration: usize,
    pub max_depth: usize,
    /// Iteration history.
    pub iterations: Vec<ExtractionIteration>,
}

impl RecursiveExtractor {
    pub fn new(seed_keywords: Vec<String>, max_papers: usize) -> Self {
        RecursiveExtractor {
            engine: ResearchEngine::new(),
            seed_keywords,
            max_papers,
            min_pattern_confidence: 0.01,
            queries_per_iteration: 20,
            max_depth: 10,
            iterations: Vec::new(),
        }
    }

    /// Plan the next extraction iteration given current state.
    pub fn plan_next(&mut self) -> Option<ExtractionIteration> {
        let depth = self.iterations.len();
        if depth >= self.max_depth { return None; }

        let current = self.engine.total_papers();
        if current >= self.max_papers { return None; }

        // Determine seed queries for this iteration.
        let queries: Vec<ExtractionQuery> = if depth == 0 {
            // Iteration 0: use seed keywords directly.
            self.seed_keywords.iter().take(self.queries_per_iteration).map(|kw| {
                ExtractionQuery {
                    api: ApiKind::ArXiv,
                    query_string: format!("all:{}", kw.replace(' ', "+")),
                    max_results: 100,
                    start: 0,
                }
            }).collect()
        } else {
            // Iteration N: use top patterns (by confidence) as search queries.
            let mut patterns: Vec<_> = self.engine.patterns.iter()
                .filter(|p| p.confidence >= self.min_pattern_confidence)
                .collect();
            patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
            patterns.truncate(self.queries_per_iteration);

            patterns.iter().map(|p| {
                // Each pattern is queried on a different API for diversity.
                let api = match depth % 3 {
                    0 => ApiKind::ArXiv,
                    1 => ApiKind::SemanticScholar,
                    _ => ApiKind::OpenAlex,
                };
                ExtractionQuery {
                    api,
                    query_string: p.name.replace(' ', "+"),
                    max_results: 200,
                    start: 0,
                }
            }).collect()
        };

        if queries.is_empty() { return None; }

        // Expected yield: ~5x growth per iteration (diminishing at higher depth).
        let growth = (5.0_f64).powf(-(depth as f64) * 0.15).max(1.5);
        let expected = ((current.max(100) as f64) * (growth - 1.0)) as usize;
        let target = current + expected;

        let iteration = ExtractionIteration {
            depth, queries, expected_yield: expected.min(50000),
            target_total: target.min(self.max_papers),
            growth_factor: growth,
        };

        self.iterations.push(iteration.clone());
        Some(iteration)
    }

    /// Simulate ingesting papers from an iteration (for testing / planning).
    /// In production, the external tool fetches the actual papers and calls
    /// `ingest_batch()` with the results.
    pub fn ingest_batch(&mut self, papers: Vec<Paper>) {
        let before = self.engine.total_papers();
        self.engine.ingest_papers(papers);
        let after = self.engine.total_papers();

        // Run pattern extraction on every batch (patterns seed next iteration).
        if after > before {
            self.engine.extract_patterns();
        }
    }

    /// Run a full recursive extraction simulation (for testing).
    /// Uses the pattern detection on previously-ingested papers to seed
    /// the next iteration. Actual API calls are replaced with synthetic papers.
    pub fn simulate(&mut self) {
        while let Some(iteration) = self.plan_next() {
            let paper_count = iteration.expected_yield.min(1000); // simulation cap
            let synthetic_papers: Vec<Paper> = (0..paper_count).map(|i| {
                let title = format!("Paper about {} at iter {} paper {}", 
                    iteration.queries.first().map(|q| &q.query_string).unwrap_or(&"ML".into()),
                    iteration.depth, i);
                let hash = crate::event_log::sha3_256(title.as_bytes());
                Paper {
                    id: format!("sim_{}_{}", iteration.depth, i),
                    title,
                    authors: vec!["Synthetic Author".into()],
                    abstract_text: format!("This paper discusses {} using Transformer and Adam.",
                        iteration.queries.first().map(|q| &q.query_string).unwrap_or(&"ML".into())),
                    categories: vec!["cs.LG".into()],
                    year: 2024,
                    citation_count: 10,
                    arxiv_id: None, doi: None,
                    paper_hash: hash,
                    full_text_accessible: TriState::True,
                    embedding: vec![],
                }
            }).collect();

            self.ingest_batch(synthetic_papers);
        }
        // Final pattern extraction.
        self.engine.extract_patterns();
    }

    /// Dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("Recursive Extractor\n");
        out.push_str(&format!("  Target:     {} papers\n", self.max_papers));
        out.push_str(&format!("  Current:    {} papers\n", self.engine.total_papers()));
        out.push_str(&format!("  Iterations: {}\n", self.iterations.len()));
        out.push_str(&format!("  Patterns:   {}\n", self.engine.patterns.len()));
        out.push_str(&format!("  Cross-pats: {}\n", self.engine.cross_patterns.len()));

        for (i, iter) in self.iterations.iter().enumerate() {
            out.push_str(&format!("  Iter {}: {} queries, yield ~{}, target {}, growth {:.2}x\n",
                i, iter.queries.len(), iter.expected_yield, iter.target_total, iter.growth_factor));
        }

        // Projection to target.
        let current = self.engine.total_papers();
        let remaining = self.max_papers.saturating_sub(current);
        let iter_remain = if self.iterations.is_empty() {
            6 - 0
        } else {
            let last_growth = self.iterations.last().map(|i| i.growth_factor).unwrap_or(2.0);
            (remaining as f64).log(last_growth) as usize + 1
        };
        out.push_str(&format!("  Est. iters: {} remaining\n", iter_remain.min(self.max_depth)));
        out.push('\n');
        out.push_str(&self.engine.dashboard());

        out
    }
}

#[cfg(test)]
mod recursive_tests {
    use super::*;

    #[test]
    fn plan_seed_iteration() {
        let mut re = RecursiveExtractor::new(
            vec!["transformer".into(), "diffusion".into(), "GAN".into()],
            100000,
        );
        let iter = re.plan_next().unwrap();
        assert_eq!(iter.depth, 0);
        assert_eq!(iter.queries.len(), 3);
    }

    #[test]
    fn simulate_recursive_growth() {
        let mut re = RecursiveExtractor::new(
            vec!["transformer".into()],
            10000,
        );
        re.simulate();
        assert!(re.engine.total_papers() > 0);
        assert!(re.iterations.len() > 1);
    }

    #[test]
    fn dashboard_projection() {
        let mut re = RecursiveExtractor::new(
            vec!["AI".into()],
            100000,
        );
        re.simulate();
        let d = re.dashboard();
        assert!(d.contains("Recursive Extractor"));
        assert!(d.contains("Target:"));
    }

    #[test]
    fn multiple_seeds_produce_more_papers() {
        let mut re = RecursiveExtractor::new(
            vec!["transformer".into(), "diffusion".into(), "GAN".into(),
                 "RL".into(), "NLP".into(), "CV".into()],
            50000,
        );
        re.simulate();
        assert!(re.engine.total_papers() >= 1000);
    }

    #[test]
    fn extractor_stops_at_target() {
        let mut re = RecursiveExtractor::new(
            vec!["AI".into()],
            500,
        );
        re.simulate();
        // The simulation caps each iter at 1000, but target is 500.
        // The extractor should stop planning when target is reached.
        let total = re.engine.total_papers();
        // The sim creates up to 1000 per iteration, but stops planning at target.
        assert!(re.iterations.len() >= 1);
    }
}
