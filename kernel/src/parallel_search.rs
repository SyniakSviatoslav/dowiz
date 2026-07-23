//! `kernel::parallel_search` — indexed parallel search across multiple indexes.
//!
//! Splits a query across N search indexes (BM25, trigram, spectral, etc.),
//! runs them in parallel (via execution plans), fuses results with weighted
//! scoring. Each index is independently queryable; the parallel search
//! orchestrates fan-out → per-index search → fan-in → fusion.
//!
//! # Architecture
//! ```text
//! ParallelSearchEngine
//! +-- SearchIndex[] (BM25, trigram, spectral, custom)
//! +-- PidController (adjusts parallelism based on latency)
//! +-- SearchCache (cached query results, invalidated on index update)
//! +-- fan_out(query) -> per-index search tasks
//! +-- fan_in(results) -> fused + ranked results
//! +-- ascii_dashboard() -> live diagnostics
//! ```

use crate::orchestrator::PidController;

/// EMA alpha for search latency tracking.
pub const SEARCH_LATENCY_ALPHA: f64 = 0.3;

/// Maximum number of search indexes.
pub const MAX_INDEXES: usize = 16;

/// Maximum results per index before truncation.
pub const MAX_PER_INDEX: usize = 100;

// ─── Search Index Trait ──────────────────────────────────────────────────

/// A single search index that can be queried.
#[derive(Debug, Clone)]
pub enum SearchIndexKind {
    BM25,
    Trigram,
    Spectral,
    Custom(u32),
}

/// A search hit from a single index.
#[derive(Debug, Clone)]
pub struct IndexHit {
    /// Document/section ID.
    pub doc_id: u32,
    /// Score from this index (0.0..1.0 normalized).
    pub score: f64,
    /// Which index produced this hit.
    pub index_kind: SearchIndexKind,
    /// Raw score before normalization.
    pub raw_score: f64,
}

/// A fused search result from parallel search.
#[derive(Debug, Clone)]
pub struct FusedResult {
    /// Document/section ID.
    pub doc_id: u32,
    /// Fused score (weighted combination across indexes).
    pub fused_score: f64,
    /// Per-index breakdown.
    pub index_scores: Vec<(SearchIndexKind, f64)>,
    /// How many indexes contributed to this result.
    pub contributing_indexes: usize,
}

// ─── Search Configuration ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Number of parallel search workers.
    pub worker_count: usize,
    /// Maximum results to return.
    pub top_k: usize,
    /// Cache TTL (microseconds).
    pub cache_ttl_us: u64,
    /// Minimum score threshold (results below this are dropped).
    pub min_score: f64,
    /// Target batch latency (microseconds) for PID feedback.
    pub target_batch_latency_us: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            worker_count: 4,
            top_k: 10,
            cache_ttl_us: 100_000,
            min_score: 0.01,
            target_batch_latency_us: 50_000,
        }
    }
}

// ─── Index Weights ───────────────────────────────────────────────────────

/// Weight configuration for fusing results from different indexes.
#[derive(Debug, Clone)]
pub struct IndexWeights {
    pub bm25: f64,
    pub trigram: f64,
    pub spectral: f64,
    pub custom: f64,
}

impl Default for IndexWeights {
    fn default() -> Self {
        IndexWeights { bm25: 0.5, trigram: 0.2, spectral: 0.2, custom: 0.1 }
    }
}

// ─── Search Cache ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SearchCacheEntry {
    pub query_hash: [u8; 32],
    pub results: Vec<FusedResult>,
    pub computed_us: u64,
    pub latency_us: u64,
}

#[derive(Debug, Clone)]
pub struct SearchCache {
    pub entries: Vec<SearchCacheEntry>,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
}

impl SearchCache {
    pub fn new(max_entries: usize) -> Self {
        SearchCache { entries: Vec::with_capacity(max_entries), max_entries, hits: 0, misses: 0 }
    }

    pub fn lookup(&mut self, query_hash: [u8; 32], now_us: u64, ttl_us: u64) -> Option<&SearchCacheEntry> {
        let entry = self.entries.iter().find(|e| e.query_hash == query_hash && now_us.saturating_sub(e.computed_us) < ttl_us);
        if entry.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        entry
    }

    pub fn insert(&mut self, query_hash: [u8; 32], results: Vec<FusedResult>, computed_us: u64, latency_us: u64) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(SearchCacheEntry { query_hash, results, computed_us, latency_us });
    }

    pub fn invalidate(&mut self) { self.entries.clear(); }
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

// ─── Search Metrics ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SearchMetrics {
    pub total_queries: u64,
    pub total_results: u64,
    pub avg_latency_us: f64,
    pub avg_results_per_query: f64,
    pub index_hit_counts: Vec<(SearchIndexKind, u64)>,
}

impl SearchMetrics {
    pub fn new() -> Self {
        SearchMetrics { total_queries: 0, total_results: 0, avg_latency_us: 0.0, avg_results_per_query: 0.0, index_hit_counts: Vec::new() }
    }

    pub fn record_query(&mut self, results: usize, latency_us: u64, index_counts: &[(SearchIndexKind, u64)]) {
        self.total_queries += 1;
        self.total_results += results as u64;
        let n = self.total_queries as f64;
        self.avg_latency_us = (self.avg_latency_us * (n - 1.0) + latency_us as f64) / n;
        self.avg_results_per_query = (self.avg_results_per_query * (n - 1.0) + results as f64) / n;
        for (kind, count) in index_counts {
            if let Some(entry) = self.index_hit_counts.iter_mut().find(|(k, _)| std::mem::discriminant(k) == std::mem::discriminant(kind)) {
                entry.1 += count;
            } else {
                self.index_hit_counts.push((kind.clone(), *count));
            }
        }
    }
}

// ─── Parallel Search Engine ──────────────────────────────────────────────

/// Indexed parallel search engine with PID-controlled parallelism and caching.
pub struct ParallelSearchEngine {
    config: SearchConfig,
    weights: IndexWeights,
    pid: PidController,
    cache: SearchCache,
    metrics: SearchMetrics,
    /// Number of registered indexes.
    index_count: usize,
}

impl ParallelSearchEngine {
    pub fn new(config: SearchConfig) -> Self {
        ParallelSearchEngine {
            pid: PidController::new_min_max(1, config.worker_count),
            cache: SearchCache::new(64),
            config,
            weights: IndexWeights::default(),
            metrics: SearchMetrics::new(),
            index_count: 0,
        }
    }

    pub fn with_weights(config: SearchConfig, weights: IndexWeights) -> Self {
        let mut e = Self::new(config);
        e.weights = weights;
        e
    }

    /// Register an index (returns its kind for querying).
    pub fn register_index(&mut self, kind: SearchIndexKind) -> SearchIndexKind {
        self.index_count += 1;
        kind
    }

    /// Fan-out: split query across indexes, produce per-index search tasks.
    pub fn fan_out(&self, query: &str, index_kinds: &[SearchIndexKind]) -> Vec<SearchTask> {
        let parallelism = self.pid.recommended().min(index_kinds.len()).max(1);
        index_kinds.iter().enumerate().map(|(i, kind)| {
            SearchTask {
                index_kind: kind.clone(),
                query: query.to_string(),
                max_results: MAX_PER_INDEX,
                worker_id: i % parallelism,
            }
        }).collect()
    }

    /// Fan-in: fuse per-index results into ranked output.
    pub fn fan_in(&self, per_index_results: Vec<Vec<IndexHit>>, top_k: usize) -> Vec<FusedResult> {
        // Collect all unique doc_ids.
        let mut doc_map: std::collections::HashMap<u32, Vec<(SearchIndexKind, f64)>> = std::collections::HashMap::new();
        for hits in &per_index_results {
            for hit in hits {
                doc_map.entry(hit.doc_id).or_default().push((hit.index_kind.clone(), hit.score));
            }
        }

        // Fuse scores.
        let mut results: Vec<FusedResult> = doc_map.into_iter().map(|(doc_id, scores)| {
            let fused_score: f64 = scores.iter().map(|(kind, score)| {
                let weight = match kind {
                    SearchIndexKind::BM25 => self.weights.bm25,
                    SearchIndexKind::Trigram => self.weights.trigram,
                    SearchIndexKind::Spectral => self.weights.spectral,
                    SearchIndexKind::Custom(_) => self.weights.custom,
                };
                weight * score
            }).sum();
            let contributing = scores.len();
            FusedResult { doc_id, fused_score, index_scores: scores, contributing_indexes: contributing }
        }).collect();

        results.sort_by(|a, b| b.fused_score.partial_cmp(&a.fused_score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// Execute parallel search (fan-out → fan-in).
    pub fn search(&mut self, query: &str, index_kinds: &[SearchIndexKind], search_fn: &dyn Fn(&str, &SearchIndexKind, usize) -> Vec<IndexHit>, now_us: u64) -> Vec<FusedResult> {
        let query_hash = crate::event_log::sha3_256(query.as_bytes());

        // Cache check.
        if let Some(entry) = self.cache.lookup(query_hash, now_us, self.config.cache_ttl_us) {
            return entry.results.clone();
        }

        let start_us = now_us;
        let tasks = self.fan_out(query, index_kinds);

        // Execute per-index searches.
        let per_index: Vec<Vec<IndexHit>> = tasks.iter().map(|task| {
            let hits = search_fn(&task.query, &task.index_kind, task.max_results);
            // Normalize scores to 0..1.
            let max_raw = hits.iter().map(|h| h.raw_score).fold(0.0f64, f64::max);
            hits.into_iter().map(|mut h| {
                if max_raw > 0.0 { h.score = h.raw_score / max_raw; }
                h
            }).collect()
        }).collect();

        // Track index hit counts.
        let index_counts: Vec<(SearchIndexKind, u64)> = per_index.iter().zip(tasks.iter())
            .map(|(hits, task)| (task.index_kind.clone(), hits.len() as u64))
            .collect();

        // Fan-in fusion.
        let results = self.fan_in(per_index, self.config.top_k);

        // Filter by min score.
        let results: Vec<FusedResult> = results.into_iter()
            .filter(|r| r.fused_score >= self.config.min_score)
            .collect();

        let latency = now_us.saturating_sub(start_us);
        self.metrics.record_query(results.len(), latency, &index_counts);

        // PID feedback.
        self.pid.update(self.config.target_batch_latency_us as f64, latency as f64);

        // Cache.
        self.cache.insert(query_hash, results.clone(), now_us, latency);

        results
    }

    /// ASCII dashboard.
    pub fn ascii_dashboard(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("ParallelSearch Dashboard\n");
        out.push_str(&format!("  Indexes:     {}\n", self.index_count));
        out.push_str(&format!("  Workers:     {} (PID recommended={})\n", self.config.worker_count, self.pid.recommended()));
        out.push_str(&format!("  Queries:     {} total\n", self.metrics.total_queries));
        out.push_str(&format!("  Results:     {:.1} avg per query\n", self.metrics.avg_results_per_query));
        out.push_str(&format!("  Latency:     {:.0} us avg\n", self.metrics.avg_latency_us));
        out.push_str(&format!("  Cache:       {:.0}% hit rate ({} hits / {} misses)\n",
            self.cache.hit_rate() * 100.0, self.cache.hits, self.cache.misses));
        out.push_str(&format!("  Top-k:       {}\n", self.config.top_k));
        if !self.metrics.index_hit_counts.is_empty() {
            out.push_str("  Index hits:\n");
            for (kind, count) in &self.metrics.index_hit_counts {
                let name = match kind {
                    SearchIndexKind::BM25 => "BM25",
                    SearchIndexKind::Trigram => "Trigram",
                    SearchIndexKind::Spectral => "Spectral",
                    SearchIndexKind::Custom(id) => &format!("Custom({})", id),
                };
                out.push_str(&format!("    {:<12} {}\n", name, count));
            }
        }
        out
    }

    pub fn config(&self) -> &SearchConfig { &self.config }
    pub fn weights(&self) -> &IndexWeights { &self.weights }
    pub fn metrics(&self) -> &SearchMetrics { &self.metrics }
    pub fn cache(&self) -> &SearchCache { &self.cache }
    pub fn cache_mut(&mut self) -> &mut SearchCache { &mut self.cache }
    pub fn pid_output(&self) -> f64 { self.pid.output() }
}

/// A search task for a single index.
#[derive(Debug, Clone)]
pub struct SearchTask {
    pub index_kind: SearchIndexKind,
    pub query: String,
    pub max_results: usize,
    pub worker_id: usize,
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_search(_query: &str, kind: &SearchIndexKind, max: usize) -> Vec<IndexHit> {
        (0..max.min(5)).map(|i| IndexHit {
            doc_id: i as u32,
            score: 1.0 - i as f64 * 0.1,
            index_kind: kind.clone(),
            raw_score: 100.0 - i as f64 * 10.0,
        }).collect()
    }

    #[test]
    fn fan_out_produces_tasks() {
        let engine = ParallelSearchEngine::new(SearchConfig::default());
        let tasks = engine.fan_out("test", &[SearchIndexKind::BM25, SearchIndexKind::Trigram]);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn fan_in_fuses_results() {
        let engine = ParallelSearchEngine::new(SearchConfig::default());
        let per_index = vec![
            vec![IndexHit { doc_id: 0, score: 0.8, index_kind: SearchIndexKind::BM25, raw_score: 80.0 }],
            vec![IndexHit { doc_id: 0, score: 0.6, index_kind: SearchIndexKind::Trigram, raw_score: 60.0 }],
        ];
        let results = engine.fan_in(per_index, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].contributing_indexes, 2);
        assert!(results[0].fused_score > 0.0);
    }

    #[test]
    fn fan_in_empty_input() {
        let engine = ParallelSearchEngine::new(SearchConfig::default());
        let results = engine.fan_in(vec![], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn search_with_cache() {
        let mut engine = ParallelSearchEngine::new(SearchConfig { cache_ttl_us: 100_000, ..Default::default() });
        let kinds = vec![SearchIndexKind::BM25];
        let r1 = engine.search("test", &kinds, &dummy_search, 1000);
        let r2 = engine.search("test", &kinds, &dummy_search, 1000);
        assert_eq!(r1.len(), r2.len());
        assert!(engine.cache().hit_rate() > 0.0);
    }

    #[test]
    fn search_top_k_limits_results() {
        let mut engine = ParallelSearchEngine::new(SearchConfig { top_k: 3, ..Default::default() });
        let kinds = vec![SearchIndexKind::BM25, SearchIndexKind::Trigram];
        let results = engine.search("test", &kinds, &dummy_search, 1000);
        assert!(results.len() <= 3);
    }

    #[test]
    fn search_min_score_filter() {
        let mut engine = ParallelSearchEngine::new(SearchConfig { min_score: 0.5, ..Default::default() });
        let kinds = vec![SearchIndexKind::BM25];
        let results = engine.search("test", &kinds, &dummy_search, 1000);
        for r in &results {
            assert!(r.fused_score >= 0.5);
        }
    }

    #[test]
    fn cache_invalidation() {
        let mut cache = SearchCache::new(10);
        cache.insert([1u8; 32], vec![], 1000, 100);
        assert!(cache.lookup([1u8; 32], 1500, 100_000).is_some());
        cache.invalidate();
        assert!(cache.lookup([1u8; 32], 1500, 100_000).is_none());
    }

    #[test]
    fn cache_hit_rate() {
        let mut cache = SearchCache::new(10);
        cache.insert([1u8; 32], vec![], 1000, 100);
        let _ = cache.lookup([1u8; 32], 1500, 100_000); // hit
        let _ = cache.lookup([2u8; 32], 1500, 100_000); // miss
        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn metrics_record_query() {
        let mut m = SearchMetrics::new();
        m.record_query(5, 200, &[(SearchIndexKind::BM25, 3), (SearchIndexKind::Trigram, 2)]);
        assert_eq!(m.total_queries, 1);
        assert_eq!(m.total_results, 5);
        assert!((m.avg_latency_us - 200.0).abs() < 0.01);
    }

    #[test]
    fn dashboard_contains_sections() {
        let engine = ParallelSearchEngine::new(SearchConfig::default());
        let d = engine.ascii_dashboard();
        assert!(d.contains("ParallelSearch Dashboard"));
        assert!(d.contains("Indexes:"));
        assert!(d.contains("Cache:"));
    }

    #[test]
    fn register_index_increments_count() {
        let mut engine = ParallelSearchEngine::new(SearchConfig::default());
        engine.register_index(SearchIndexKind::BM25);
        engine.register_index(SearchIndexKind::Trigram);
        assert_eq!(engine.index_count, 2);
    }

    #[test]
    fn weight_default() {
        let w = IndexWeights::default();
        assert!((w.bm25 + w.trigram + w.spectral + w.custom - 1.0).abs() < 0.01);
    }

    #[test]
    fn different_queries_different_hashes() {
        let h1 = crate::event_log::sha3_256(b"hello");
        let h2 = crate::event_log::sha3_256(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn search_deduplicates_across_indexes() {
        let mut engine = ParallelSearchEngine::new(SearchConfig::default());
        let kinds = vec![SearchIndexKind::BM25, SearchIndexKind::Trigram];
        // Both indexes return doc_id=0.
        let results = engine.search("test", &kinds, &dummy_search, 1000);
        // Should be deduplicated into a single fused result.
        let count_zero = results.iter().filter(|r| r.doc_id == 0).count();
        assert!(count_zero <= 1, "doc_id 0 should appear at most once: {}", count_zero);
    }
}
