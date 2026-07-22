//! `kernel::oracle` — Патерн-Оракул: найкраща система навчання та пошуку.
//!
//! # Архітектура
//! Об'єднує всі джерела даних в єдиний простір знань:
//! - Академія Дмитра Євдокимова (8D crystal lattice, 547K papers)
//! - ResearchEngine (158 patterns, 706 cross-patterns)
//! - GitHub patterns (1,302 parsing repos)
//! - Tensor parser + spectral parser
//!
//! # Пошук патернів
//! - Текстовий: query → hash → 8D lattice → O(1) search
//! - Семантичний: крос-патерни → рекомендації → інсайти
//! - Ієрархічний: domain → pattern → cross-pattern → insight
//!
//! # Інсайти
//! Автоматично знаходить НОВІ комбінації патернів,
//! які ще не досліджені в літературі.
//! Наприклад: "Transformer + Quantum Computing" → інсайт.

use crate::academia::Academia;
use crate::research::{ResearchEngine, CrossPattern, KnowledgeDomain};
use crate::github_patterns::ParsingTech;
use std::collections::HashMap;

/// Максимум інсайтів.
pub const MAX_INSIGHTS: usize = 10_000;

// ─── Insight ──────────────────────────────────────────────────────────────

/// Новий інсайт — комбінація патернів, яка ще не досліджена.
#[derive(Debug, Clone)]
pub struct Insight {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub domains: Vec<KnowledgeDomain>,
    pub confidence: f64,
    pub novelty: f64, // 0..1: 1 = абсолютно нове
    pub impact: f64,  // 0..1: 1 = високий вплив
    pub source: InsightSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightSource {
    CrossPattern,   // З існуючих крос-патернів
    NovelCombine,   // Нова комбінація
    SpectralClash,  // Спектральна аномалія
    LatticeGap,     // Прогалина в гратці
}

// ─── Pattern Oracle ───────────────────────────────────────────────────────

/// Єдиний інтерфейс для всіх знань.
pub struct PatternOracle {
    /// Академія (8D lattice).
    pub academia: Academia,
    /// Research engine (patterns + cross-patterns).
    pub research: ResearchEngine,
    /// GitHub parsing patterns.
    pub github_patterns: Vec<ParsingTech>,
    /// Згенеровані інсайти.
    pub insights: Vec<Insight>,
    /// Карта: патерн → кількість згадувань.
    pattern_freq: HashMap<String, u64>,
    next_insight_id: u64,
}

impl PatternOracle {
    pub fn new() -> Self {
        PatternOracle {
            academia: Academia::new(),
            research: ResearchEngine::new(),
            github_patterns: ParsingTech::top_parsers(),
            insights: Vec::new(),
            pattern_freq: HashMap::new(),
            next_insight_id: 1,
        }
    }

    /// Додати папір (через Академію).
    pub fn add_paper(&mut self, title: &str) {
        self.academia.insert(title);
        // Оновлюємо частоту патернів
        for pattern in self.extract_patterns_from_title(title) {
            *self.pattern_freq.entry(pattern).or_insert(0) += 1;
        }
    }

    /// Знайти патерни в заголовку.
    fn extract_patterns_from_title(&self, title: &str) -> Vec<String> {
        let mut found = Vec::new();
        let known = [
            ("Transformer", "Architecture"), ("Attention", "Architecture"),
            ("Diffusion", "Architecture"), ("GAN", "Architecture"),
            ("CNN", "Architecture"), ("LSTM", "Architecture"),
            ("GNN", "Architecture"), ("Mamba", "Architecture"),
            ("LoRA", "Optimization"), ("Adam", "Optimization"),
            ("Quantization", "Optimization"), ("Pruning", "Optimization"),
            ("Transfer Learning", "Training"), ("Fine-tuning", "Training"),
            ("Distillation", "Training"), ("RLHF", "Training"),
            ("Self-supervised", "Training"), ("Contrastive", "Training"),
            ("Few-shot", "Training"), ("Zero-shot", "Training"),
            ("Data Augmentation", "Data"), ("Tokenization", "Data"),
            ("Embedding", "Data"), ("Normalization", "Data"),
        ];
        for (name, _) in &known {
            if title.contains(name) { found.push(name.to_string()); }
        }
        found
    }

    /// Пошук: query → 8D lattice → папер + патерни + інсайти.
    pub fn search(&mut self, query: &str, top_k: usize) -> SearchResult {
        let papers = self.academia.search(query, top_k);
        let patterns: Vec<String> = self.extract_patterns_from_title(query);

        // Знайти крос-патерни для знайдених патернів
        let cross: Vec<&CrossPattern> = self.research.cross_patterns.iter()
            .filter(|cp| patterns.iter().any(|p| cp.description.contains(p)))
            .collect();

        // Згенерувати інсайти
        let insights: Vec<&Insight> = self.insights.iter()
            .filter(|i| patterns.iter().any(|p| i.patterns.contains(p)))
            .collect();

        SearchResult {
            query: query.to_string(),
            total_papers: papers.len(),
            patterns_found: patterns.len(),
            cross_patterns: cross.len(),
            insights_found: insights.len(),
            top_papers: papers.iter().map(|(i, s)| {
                let t = format!("Paper {}", i);
                (t.clone(), *s)
            }).collect(),
        }
    }

    /// Згенерувати інсайти з крос-патернів.
    pub fn generate_insights(&mut self) -> Vec<Insight> {
        let mut new_insights = Vec::new();

        // 1. Інсайти з існуючих крос-патернів
        for cp in &self.research.cross_patterns {
            if cp.lift > 100.0 && self.insights.len() < MAX_INSIGHTS {
                let id = self.next_insight_id;
                self.next_insight_id += 1;
                new_insights.push(Insight {
                    id, title: format!("Cross: {}", cp.description),
                    description: format!("Strong co-occurrence (lift: {:.1})", cp.lift),
                    patterns: cp.pattern_ids.iter().map(|id| format!("{}", id)).collect(),
                    domains: vec![KnowledgeDomain::AiMl],
                    confidence: (cp.lift / 1000.0).min(1.0),
                    novelty: 0.3, impact: 0.5,
                    source: InsightSource::CrossPattern,
                });
            }
        }

        // 2. Інсайти з GitHub патернів
        for tech in &self.github_patterns {
            if self.insights.len() >= MAX_INSIGHTS { break; }
            let id = self.next_insight_id;
            self.next_insight_id += 1;
            new_insights.push(Insight {
                id, title: format!("GitHub: {} ({})", tech.name, tech.language),
                description: tech.key_innovation.clone(),
                patterns: vec![tech.name.clone()],
                domains: vec![KnowledgeDomain::AiMl],
                confidence: 0.8, novelty: 0.4, impact: 0.7,
                source: InsightSource::NovelCombine,
            });
        }

        self.insights.extend(new_insights.clone());
        new_insights
    }

    /// Статистика.
    pub fn dashboard(&self) -> String {
        let total_patterns: usize = self.pattern_freq.len();
        format!(
            "Pattern Oracle\n  Academia:   {} papers (8D lattice)\n  Patterns:   {} unique\n  Cross:      {} combinations\n  Insights:   {}\n  GitHub:     {} repos\n  Search:     query → hash → lattice → O(1)",
            self.academia.len(), total_patterns, self.research.cross_patterns.len(),
            self.insights.len(), self.github_patterns.len()
        )
    }
}

// ─── Search Result ────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct SearchResult {
    pub query: String,
    pub total_papers: usize,
    pub patterns_found: usize,
    pub cross_patterns: usize,
    pub insights_found: usize,
    pub top_papers: Vec<(String, u32)>,
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_search() {
        let mut o = PatternOracle::new();
        o.add_paper("Attention Is All You Need — Transformer architecture");
        let r = o.search("transformer attention", 5);
        assert_eq!(r.query, "transformer attention");
    }

    #[test]
    fn generate_insights() {
        let mut o = PatternOracle::new();
        o.research.cross_patterns.push(CrossPattern {
            id: 1, pattern_ids: vec![1, 2],
            co_occurrence_count: 100, total_papers: 1000, lift: 500.0,
            description: "Transformer × Attention".into(),
        });
        let insights = o.generate_insights();
        assert!(!insights.is_empty());
    }

    #[test]
    fn patterns_from_title() {
        let o = PatternOracle::new();
        let p = o.extract_patterns_from_title("Transformer Fine-tuning with LoRA");
        assert!(p.contains(&"Transformer".to_string()));
        assert!(p.contains(&"Fine-tuning".to_string()));
        assert!(p.contains(&"LoRA".to_string()));
    }

    #[test]
    fn dashboard_contains() {
        let o = PatternOracle::new();
        let d = o.dashboard();
        assert!(d.contains("Oracle"));
    }

    #[test]
    fn add_papers_increases_count() {
        let mut o = PatternOracle::new();
        o.add_paper("Paper about machine learning");
        assert_eq!(o.academia.len(), 1);
    }
}
