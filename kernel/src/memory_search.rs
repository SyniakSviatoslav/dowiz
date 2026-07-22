//! `kernel::memory_search` — unified memory search engine (indexed vector + spectral navigation).
//!
//! Composes the existing retrieval primitives into a single API for searching
//! structured documents (like MEMORY.md) by section:
//!   - **Indexed vector**: BM25 lexical scoring + TrigramIndex candidate narrowing
//!   - **Spectral navigation**: CSR graph + PPR for "what relates to X" diffusion
//!   - **Fusion**: weighted combination of lexical + spectral signals
//!
//! Replaces all `grep -rn "pattern" docs/` / `grep -F` / `awk` extraction
//! with a single kernel-native call. Zero new deps, pure `std`.
//!
//! # Architecture
//! ```text
//! MemorySearchEngine
//! +-- Lexical Layer
//! |   +-- BM25 (retrieval/bm25.rs) -- Okapi scoring
//! |   +-- TrigramIndex (retrieval/index.rs) -- candidate narrowing
//! +-- Spectral Layer
//! |   +-- CSR graph (csr.rs) -- section adjacency
//! |   +-- PPR (retrieval/ppr.rs) -- personalized PageRank navigation
//! +-- Fusion
//!     +-- score = w_bm25 * bm25_norm + w_ppr * ppr_norm + w_tri * tri_norm
//! ```

use crate::retrieval::bm25::{self, Bm25, Document};
use crate::retrieval::index::TrigramIndex;
use crate::retrieval::ppr::Ppr;

/// Weight for BM25 lexical signal in fusion.
pub const W_BM25: f64 = 0.5;
/// Weight for PPR spectral signal in fusion.
pub const W_PPR: f64 = 0.3;
/// Weight for trigram exact-match signal in fusion.
pub const W_TRIGRAM: f64 = 0.2;

/// PPR teleport probability.
pub const PPR_ALPHA: f64 = 0.15;
/// PPR fixed iteration count.
pub const PPR_K: usize = 20;

/// A section parsed from a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// 0-based section index (order of appearance).
    pub id: usize,
    /// Section header text (without `## ` prefix).
    pub header: String,
    /// Full section text (header + body).
    pub text: String,
}

/// A search hit from the memory engine.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// Section index in the corpus.
    pub section_id: usize,
    /// Section header.
    pub header: String,
    /// Fused relevance score (0.0–1.0 range, higher = more relevant).
    pub score: f64,
    /// BM25 lexical score component.
    pub bm25_score: f64,
    /// PPR spectral score component.
    pub ppr_score: f64,
    /// Trigram exact-match score component.
    pub trigram_score: f64,
}

/// The unified memory search engine.
///
/// Constructed from a Markdown document (split into `##` sections), then
/// queried with natural-language or keyword queries. The engine fuses three
/// signals — BM25 lexical scoring, trigram exact narrowing, and PPR spectral
/// navigation — into a single ranked result list.
pub struct MemorySearchEngine {
    /// Parsed sections from the document.
    sections: Vec<Section>,
    /// BM25 lexical index over section texts.
    bm25: Bm25,
    /// Trigram inverted index over section texts.
    trigram: TrigramIndex,
    /// Section adjacency CSR graph (built from term co-occurrence).
    graph: crate::csr::Csr,
    /// Row-stochastic transition matrix for PPR (derived from CSR).
    transition: Vec<Vec<f64>>,
}

impl MemorySearchEngine {
    /// Build the engine from a Markdown document string.
    ///
    /// Splits on `## ` headers to define sections, builds BM25 + trigram indices,
    /// constructs a term-overlap adjacency graph, and pre-computes the PPR
    /// transition matrix.
    pub fn new(document: &str) -> Self {
        let sections = Self::split_sections(document);
        let n = sections.len();

        // Build BM25 index over section texts
        let docs: Vec<Document> = sections
            .iter()
            .map(|s| Document::from_text(&s.text))
            .collect();
        let bm25 = Bm25::new(docs);

        // Build trigram index over section texts
        let doc_strs: Vec<&str> = sections.iter().map(|s| s.text.as_str()).collect();
        let trigram = TrigramIndex::new(&doc_strs);

        // Build section adjacency graph (sections sharing terms are connected)
        let graph = Self::build_adjacency_graph(&sections);
        let transition = Self::csr_to_transition(&graph);

        MemorySearchEngine {
            sections,
            bm25,
            trigram,
            graph,
            transition,
        }
    }

    /// Search the memory for relevant sections. Returns results sorted by
    /// fused score (descending). Empty query returns empty results.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let n = self.sections.len();
        if n == 0 {
            return Vec::new();
        }

        // Layer 1: BM25 lexical scoring
        let bm25_hits = self.bm25.rank_text(query);
        let bm25_max = bm25_hits.first().map(|h| h.score).unwrap_or(0.0);

        // Layer 2: Trigram exact-match candidates
        let tri_candidates = self.trigram.query_literal(query);

        // Layer 3: PPR spectral navigation from top BM25 hit as seed
        let ppr_scores = if !bm25_hits.is_empty() {
            let seed = bm25_hits[0].doc_id;
            self.ppr_from_seed(seed)
        } else {
            vec![0.0; n]
        };
        let ppr_max = ppr_scores.iter().copied().fold(0.0f64, f64::max);

        // Fusion: combine all three signals
        let mut results: Vec<SearchResult> = (0..n)
            .map(|i| {
                let bm25_norm = if bm25_max > 0.0 {
                    self.bm25_score(i, &bm25_hits) / bm25_max
                } else {
                    0.0
                };
                let ppr_norm = if ppr_max > 0.0 {
                    ppr_scores[i] / ppr_max
                } else {
                    0.0
                };
                let tri_norm = if tri_candidates.contains(&(i as u32)) {
                    1.0
                } else {
                    0.0
                };

                let fused = W_BM25 * bm25_norm + W_PPR * ppr_norm + W_TRIGRAM * tri_norm;

                SearchResult {
                    section_id: i,
                    header: self.sections[i].header.clone(),
                    score: fused,
                    bm25_score: bm25_norm,
                    ppr_score: ppr_norm,
                    trigram_score: tri_norm,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.section_id.cmp(&b.section_id))
        });
        results
    }

    /// Top-k search results.
    pub fn search_top_k(&self, query: &str, k: usize) -> Vec<SearchResult> {
        let mut r = self.search(query);
        r.truncate(k);
        r
    }

    /// PPR spectral navigation from a seed section.
    /// Returns PPR scores for all sections.
    fn ppr_from_seed(&self, seed: usize) -> Vec<f64> {
        let ppr = Ppr::new(self.transition.clone());
        ppr.rank(seed, PPR_ALPHA, PPR_K)
    }

    /// Get the number of sections in the corpus.
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Get a section by index.
    pub fn section(&self, id: usize) -> Option<&Section> {
        self.sections.get(id)
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Split a Markdown document into sections by `## ` headers.
    fn split_sections(document: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut current_header = String::new();
        let mut current_body = String::new();
        let mut id = 0;

        for line in document.lines() {
            if let Some(header) = line.strip_prefix("## ") {
                // Save previous section (if any)
                if !current_header.is_empty() || !current_body.is_empty() {
                    let mut text = current_header.clone();
                    if !current_body.is_empty() {
                        text.push('\n');
                        text.push_str(&current_body);
                    }
                    sections.push(Section {
                        id,
                        header: current_header.clone(),
                        text,
                    });
                    id += 1;
                }
                current_header = header.trim().to_string();
                current_body.clear();
            } else if !current_header.is_empty() {
                if !current_body.is_empty() {
                    current_body.push('\n');
                }
                current_body.push_str(line);
            }
        }

        // Push final section
        if !current_header.is_empty() {
            let mut text = current_header.clone();
            if !current_body.is_empty() {
                text.push('\n');
                text.push_str(&current_body);
            }
            sections.push(Section {
                id,
                header: current_header,
                text,
            });
        }

        sections
    }

    /// Build a CSR adjacency graph connecting sections that share terms.
    /// Two sections are connected if they share at least one significant term
    /// (length >= 3, not a stop word). Edge weight = number of shared terms.
    fn build_adjacency_graph(sections: &[Section]) -> crate::csr::Csr {
        let n = sections.len();
        if n == 0 {
            return crate::csr::Csr {
                row_ptr: vec![0],
                col_idx: Vec::new(),
                val: Vec::new(),
            };
        }

        // Tokenize each section and collect significant terms (len >= 3)
        let section_terms: Vec<Vec<String>> = sections
            .iter()
            .map(|s| {
                bm25::tokenize(&s.text)
                    .into_iter()
                    .filter(|t| t.len() >= 3)
                    .collect()
            })
            .collect();

        // Build term -> section-ids inverted index
        let mut term_index: std::collections::HashMap<String, Vec<u32>> =
            std::collections::HashMap::new();
        for (sid, terms) in section_terms.iter().enumerate() {
            let mut seen = std::collections::HashSet::new();
            for t in terms {
                if seen.insert(t.clone()) {
                    term_index
                        .entry(t.clone())
                        .or_default()
                        .push(sid as u32);
                }
            }
        }

        // Compute shared-term counts for each pair
        let mut edge_counts: std::collections::HashMap<(u32, u32), u32> =
            std::collections::HashMap::new();
        for sids in term_index.values() {
            for i in 0..sids.len() {
                for j in (i + 1)..sids.len() {
                    let key = if sids[i] < sids[j] {
                        (sids[i], sids[j])
                    } else {
                        (sids[j], sids[i])
                    };
                    *edge_counts.entry(key).or_insert(0) += 1;
                }
            }
        }

        // Convert to edges (undirected: both directions)
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();
        for ((a, b), count) in &edge_counts {
            let w = *count as f64;
            edges.push((*a as usize, *b as usize, w));
            edges.push((*b as usize, *a as usize, w));
        }

        crate::csr::Csr::from_edges(n, &edges)
    }

    /// Convert a CSR graph to a row-stochastic transition matrix for PPR.
    fn csr_to_transition(csr: &crate::csr::Csr) -> Vec<Vec<f64>> {
        let n = csr.nrows();
        let mut w = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            let start = csr.row_ptr[i];
            let end = csr.row_ptr[i + 1];
            let row_sum: f64 = csr.val[start..end].iter().sum();
            if row_sum > 0.0 {
                for idx in start..end {
                    w[i][csr.col_idx[idx]] = csr.val[idx] / row_sum;
                }
            } else {
                // Dangling node: uniform distribution
                let inv = 1.0 / n as f64;
                for j in 0..n {
                    w[i][j] = inv;
                }
            }
        }
        w
    }

    /// Get the BM25 score for a specific section from the ranked hits.
    fn bm25_score(&self, section_id: usize, hits: &[crate::retrieval::bm25::Scored]) -> f64 {
        hits.iter()
            .find(|h| h.doc_id == section_id)
            .map(|h| h.score)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DOC: &str = r#"## Project Overview
dowiz is a sovereign kernel for a delivery-order system.
Architecture: kernel plus agents plus tools plus intake adapters.
Primary language: Rust.

## Architecture
Kernel in Rust with zero external dependencies.
FDR is the flight-data recorder.
Agent facade handles LLM interface.

## Testing Rules
Run cargo test before every commit.
TDD with RED then GREEN.
No external test frameworks.

## Security
Hydra closure is NEVER.
P103 supervisor is dual-witness.
Intake firewall prevents unauthorized calls.

## Mesh Swarm
Agents self-organize as decentralized mesh swarm.
No hierarchical orchestration.
Skills selected from living memory.
"#;

    #[test]
    fn section_splitting() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        assert_eq!(engine.section_count(), 5);
        assert_eq!(engine.section(0).unwrap().header, "Project Overview");
        assert_eq!(engine.section(1).unwrap().header, "Architecture");
        assert_eq!(engine.section(2).unwrap().header, "Testing Rules");
        assert_eq!(engine.section(3).unwrap().header, "Security");
        assert_eq!(engine.section(4).unwrap().header, "Mesh Swarm");
    }

    #[test]
    fn search_finds_relevant_section() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search("cargo test");
        assert!(!results.is_empty(), "must find at least one result");
        // "Testing Rules" mentions cargo test → should rank highly
        let top = &results[0];
        assert_eq!(top.section_id, 2); // Testing Rules section
        assert!(top.score > 0.0);
    }

    #[test]
    fn search_hydra_finds_security() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search("hydra");
        assert!(!results.is_empty());
        assert_eq!(results[0].section_id, 3); // Security section
    }

    #[test]
    fn search_mesh_swarm() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search("mesh swarm");
        assert!(!results.is_empty());
        assert_eq!(results[0].section_id, 4); // Mesh Swarm section
    }

    #[test]
    fn search_empty_query() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        assert!(engine.search("").is_empty());
    }

    #[test]
    fn search_returns_scored_results() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search("kernel rust");
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.score >= 0.0, "score must be non-negative");
            assert!(r.score <= 1.0 + 1e-9, "score must be <= 1.0");
        }
    }

    #[test]
    fn search_top_k_limits_results() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search_top_k("kernel", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn graph_has_edges_for_shared_terms() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        // The graph should have edges between sections sharing terms
        assert!(engine.graph.nnz() > 0, "adjacency graph must have edges");
    }

    #[test]
    fn ppr_returns_valid_distribution() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let scores = engine.ppr_from_seed(0);
        assert_eq!(scores.len(), 5);
        let total: f64 = scores.iter().sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "PPR mass must be ~1.0, got {total}"
        );
    }

    #[test]
    fn deterministic_results() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let a = engine.search("delivery order");
        let b = engine.search("delivery order");
        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.section_id, rb.section_id);
            assert!((ra.score - rb.score).abs() < 1e-12);
        }
    }

    #[test]
    fn fusion_components_are_non_negative() {
        let engine = MemorySearchEngine::new(TEST_DOC);
        let results = engine.search("security");
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.bm25_score >= 0.0);
            assert!(r.ppr_score >= 0.0);
            assert!(r.trigram_score >= 0.0);
        }
    }
}

// ─── Topological-Chronological Parametric Memory Surface ───────────────
// Пам'ять як параметрична поверхня в неевклідовому просторі.
// Топологія = зв'язки між спогадами, хронологія = час.
// Поверхня S(u,v): u = топологічна координата, v = хронологічна.

/// Параметрична поверхня в R^n: S(u,v) -> R^n.
#[derive(Debug, Clone)]
pub struct ParametricSurface {
    /// Контрольні точки поверхні (кожна = спогад у R^n).
    pub control_points: Vec<Vec<f64>>,
    /// Топологічна координата u для кожної точки.
    pub u_coords: Vec<f64>,
    /// Хронологічна координата v для кожної точки.
    pub v_coords: Vec<f64>,
    /// Вага (важливість) кожної точки.
    pub weights: Vec<f64>,
    /// Час створення кожної точки.
    pub timestamps: Vec<u64>,
}

impl ParametricSurface {
    pub fn new() -> Self {
        ParametricSurface {
            control_points: Vec::new(), u_coords: Vec::new(), v_coords: Vec::new(),
            weights: Vec::new(), timestamps: Vec::new(),
        }
    }

    /// Додати точку на поверхню: топологія + хронологія + координати.
    pub fn add_point(&mut self, coords: Vec<f64>, u: f64, v: f64, weight: f64, now: u64) {
        self.control_points.push(coords);
        self.u_coords.push(u);
        self.v_coords.push(v);
        self.weights.push(weight);
        self.timestamps.push(now);
    }

    /// Інтерполяція поверхні: обчислити точку S(u,v).
    pub fn evaluate(&self, u: f64, v: f64, dims: usize) -> Vec<f64> {
        let n = self.control_points.len();
        if n == 0 { return vec![0.0; dims]; }
        let mut result = vec![0.0; dims];
        let mut total_weight = 0.0;
        // Зважена сума найближчих точок (гаусове ядро)
        for i in 0..n {
            let du = u - self.u_coords[i];
            let dv = v - self.v_coords[i];
            let dist2 = du * du + dv * dv;
            if dist2 > 100.0 { continue; }
            let w = self.weights[i] * (-dist2 * 0.5).exp();
            total_weight += w;
            for j in 0..dims.min(self.control_points[i].len()) {
                result[j] += w * self.control_points[i][j];
            }
        }
        if total_weight > 0.0 {
            for j in 0..dims {
                result[j] /= total_weight;
            }
        }
        result
    }

    /// Геодезична відстань по поверхні (сума сегментів через найближчі точки).
    pub fn geodesic_distance(&self, u1: f64, v1: f64, u2: f64, v2: f64) -> f64 {
        let n = self.control_points.len();
        if n < 2 { return ((u2 - u1).powi(2) + (v2 - v1).powi(2)).sqrt(); }
        // Знайти найближчі точки до (u1,v1) та (u2,v2)
        let mut dist = 0.0;
        let mut prev_u = u1;
        let mut prev_v = v1;
        for _ in 0..3 { // 3 сегменти апроксимації геодезичної
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for i in 0..n {
                let d = (self.u_coords[i] - u2).powi(2) + (self.v_coords[i] - v2).powi(2);
                if d < best_d { best_d = d; best = i; }
            }
            let nu = self.u_coords[best];
            let nv = self.v_coords[best];
            dist += ((nu - prev_u).powi(2) + (nv - prev_v).powi(2)).sqrt();
            prev_u = nu; prev_v = nv;
            if best_d < 0.1 { break; }
        }
        dist += ((u2 - prev_u).powi(2) + (v2 - prev_v).powi(2)).sqrt();
        dist
    }

    /// Кривина Гауса в точці: K = (det(H) / (1 + grad²)²).
    pub fn gaussian_curvature(&self, u: f64, v: f64, eps: f64) -> f64 {
        let d = self.control_points[0].len();
        let suu = self.evaluate(u + eps, v, d);
        let su = self.evaluate(u - eps, v, d);
        let sv = self.evaluate(u, v - eps, d);
        let svv = self.evaluate(u, v + eps, d);
        let s = self.evaluate(u, v, d);
        let du: Vec<f64> = su.iter().zip(&s).map(|(a, b)| a - b).collect();
        let dv: Vec<f64> = sv.iter().zip(&s).map(|(a, b)| a - b).collect();
        let duu: Vec<f64> = suu.iter().zip(&su).map(|(a, b)| a - b).collect();
        let dvv: Vec<f64> = svv.iter().zip(&sv).map(|(a, b)| a - b).collect();
        let e_f64: f64 = du.iter().map(|x| x * x).sum();
        let e = e_f64.sqrt();
        let f_f64: f64 = du.iter().zip(&dv).map(|(a, b)| a * b).sum();
        let g_f64: f64 = dv.iter().map(|x| x * x).sum();
        let g = g_f64.sqrt();
        let l_f64: f64 = duu.iter().map(|x| x * x).sum();
        let l = l_f64.sqrt();
        let m: f64 = 0.0;
        let n_f64: f64 = dvv.iter().map(|x| x * x).sum();
        let n = n_f64.sqrt();
        let denom: f64 = e * g - f_f64 * f_f64;
        let denom_abs: f64 = if denom < 0.0 { -denom } else { denom };
        if denom_abs < 1e-12 { 0.0 } else { (l * n - m * m) / denom_abs }
    }
}

/// Тополого-хронологічна пам'ять: поверхня в неевклідовому просторі.
#[derive(Debug)]
pub struct TopoChronoMemory {
    /// Поверхня пам'яті.
    pub surface: ParametricSurface,
    /// Кількість вимірів простору.
    pub dims: usize,
    /// Метрика простору.
    pub metric: crate::academia_p2p::MetricTensor,
    /// Поточний час.
    pub tick: u64,
    /// Назви спогадів.
    pub labels: Vec<String>,
}

impl TopoChronoMemory {
    pub fn new(dims: usize) -> Self {
        TopoChronoMemory {
            surface: ParametricSurface::new(),
            dims,
            metric: crate::academia_p2p::MetricTensor::euclidean(),
            tick: 0,
            labels: Vec::new(),
        }
    }

    /// Записати спогад: текст + топологічна вага.
    pub fn record(&mut self, label: &str, text: &str, topology: f64, weight: f64) {
        let hash = crate::event_log::sha3_256(text.as_bytes());
        let coords: Vec<f64> = hash.iter().take(self.dims).map(|&b| b as f64 / 255.0 * 20.0 - 10.0).collect();
        let chrono = self.tick as f64 / 100.0;
        self.surface.add_point(coords, topology, chrono, weight, self.tick);
        self.labels.push(label.to_string());
        self.tick += 1;
    }

    /// Знайти спогади за топологічно-хронологічною близькістю.
    pub fn retrieve(&self, topology: f64, time: f64, k: usize) -> Vec<(String, f64, f64)> {
        let mut scores: Vec<(usize, f64)> = (0..self.surface.control_points.len()).map(|i| {
            let du = topology - self.surface.u_coords[i];
            let dv = time - self.surface.v_coords[i];
            (i, (-(du * du + dv * dv) * 0.5).exp())
        }).collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores.iter().map(|&(i, s)| {
            (self.labels[i].clone(), self.surface.u_coords[i], self.surface.v_coords[i])
        }).collect()
    }

    /// Асоціативний пошук: знайти спогади, пов'язані з даним.
    pub fn associate(&self, idx: usize, k: usize) -> Vec<(String, f64)> {
        if idx >= self.surface.control_points.len() { return vec![]; }
        let u = self.surface.u_coords[idx];
        let v = self.surface.v_coords[idx];
        let mut dists: Vec<(usize, f64)> = (0..self.surface.control_points.len()).filter(|&i| i != idx).map(|i| {
            let d = self.surface.geodesic_distance(u, v, self.surface.u_coords[i], self.surface.v_coords[i]);
            (i, d)
        }).collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        dists.truncate(k);
        dists.iter().map(|&(i, d)| (self.labels[i].clone(), d)).collect()
    }

    /// Еволюція пам'яті: зміна ваг з часом (забування).
    pub fn evolve(&mut self, decay_rate: f64) {
        for w in &mut self.surface.weights {
            *w *= (1.0 - decay_rate).max(0.0);
            if *w < 0.01 { *w = 0.01; }
        }
        self.tick += 1;
    }

    /// Підкріплення: збільшити вагу спогаду (повторення).
    pub fn reinforce(&mut self, idx: usize, amount: f64) {
        if idx < self.surface.weights.len() {
            self.surface.weights[idx] += amount;
        }
    }

    pub fn dashboard(&self) -> String {
        format!(
            "TopoChrono Memory (Parametric Surface in R^{})\n  Records:  {}\n  Dims:     {}\n  Tick:     {}\n  Curvature: {:.6}",
            self.dims, self.surface.control_points.len(), self.dims, self.tick,
            if self.surface.control_points.len() > 3 {
                self.surface.gaussian_curvature(0.0, 0.0, 0.1)
            } else { 0.0 }
        )
    }
}

#[cfg(test)]
mod topo_tests {
    use super::*;

    #[test]
    fn surface_add_point() {
        let mut s = ParametricSurface::new();
        s.add_point(vec![1.0, 2.0], 0.0, 0.0, 1.0, 0);
        assert_eq!(s.control_points.len(), 1);
    }

    #[test]
    fn surface_evaluate_interpolates() {
        let mut s = ParametricSurface::new();
        s.add_point(vec![0.0], 0.0, 0.0, 1.0, 0);
        s.add_point(vec![10.0], 1.0, 1.0, 1.0, 1);
        let mid = s.evaluate(0.5, 0.5, 1);
        assert!(mid[0] > 0.0 && mid[0] < 10.0);
    }

    #[test]
    fn topo_chrono_record_retrieve() {
        let mut mem = TopoChronoMemory::new(4);
        mem.record("alpha", "first memory", 0.0, 1.0);
        mem.record("beta", "second memory", 1.0, 1.0);
        let results = mem.retrieve(0.0, 0.0, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn topo_chrono_associate() {
        let mut mem = TopoChronoMemory::new(4);
        mem.record("A", "mem A", 0.0, 1.0);
        mem.record("B", "mem B", 1.0, 1.0);
        mem.record("C", "mem C", 2.0, 1.0);
        let assoc = mem.associate(0, 2);
        assert_eq!(assoc.len(), 2);
    }

    #[test]
    fn topo_chrono_evolve_decay() {
        let mut mem = TopoChronoMemory::new(4);
        mem.record("test", "test memory", 0.0, 1.0);
        let w0 = mem.surface.weights[0];
        mem.evolve(0.1);
        assert!(mem.surface.weights[0] < w0);
    }

    #[test]
    fn topo_chrono_reinforce() {
        let mut mem = TopoChronoMemory::new(4);
        mem.record("test", "test memory", 0.0, 1.0);
        let w0 = mem.surface.weights[0];
        mem.reinforce(0, 0.5);
        assert!(mem.surface.weights[0] > w0);
    }

    #[test]
    fn topo_chrono_dashboard() {
        let mem = TopoChronoMemory::new(4);
        let d = mem.dashboard();
        assert!(d.contains("TopoChrono"));
    }

    #[test]
    fn surface_geodesic_distance() {
        let mut s = ParametricSurface::new();
        s.add_point(vec![0.0], 0.0, 0.0, 1.0, 0);
        s.add_point(vec![10.0], 1.0, 1.0, 1.0, 1);
        let d = s.geodesic_distance(0.0, 0.0, 1.0, 1.0);
        assert!(d > 0.0);
    }
}
