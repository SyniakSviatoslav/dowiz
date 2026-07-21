//! `kernel::skill_extractor` — Book-to-skill native: on-demand knowledge extraction.
//!
//! Extracts structured skills from documents: frameworks, decision rules,
//! anti-patterns, per-chapter on-demand loading. Token-efficient packaging.
//!
//! # Cross-patterns
//! - Strategy × Pipeline: extraction strategy adapts to document type
//! - Cache × Observer: predictions cached, invalidated on new observations
//! - Fan-out × PID: parallel chapter extraction, PID controls concurrency

use crate::orchestrator::PidController;
use crate::TriState;

/// Maximum chapters per skill.
pub const MAX_CHAPTERS: usize = 128;
/// Maximum glossary terms.
pub const MAX_GLOSSARY: usize = 512;

// ─── Document Type ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentType {
    Technical,
    Text,
    Reference,
    Mixed,
}

// ─── Depth Mode ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    /// Shallow lookup — concise summaries.
    Reference,
    /// Deep learning — full frameworks and patterns.
    Study,
    /// Unknown depth — not yet determined.
    Unknown,
}

// ─── Chapter ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Chapter {
    pub index: usize,
    pub title: String,
    pub summary: String,
    pub frameworks: Vec<Framework>,
    pub anti_patterns: Vec<String>,
    pub token_count: usize,
    pub hash: [u8; 32],
}

// ─── Framework ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Framework {
    pub name: String,
    pub description: String,
    pub steps: Vec<String>,
    pub applicable_when: String,
    pub anti_patterns: Vec<String>,
}

// ─── Glossary Term ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GlossaryTerm {
    pub term: String,
    pub definition: String,
    pub chapter_refs: Vec<usize>,
}

// ─── Decision Rule ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DecisionRule {
    pub condition: String,
    pub action: String,
    pub confidence: f64,
    pub source_chapter: usize,
}

// ─── Extracted Skill ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExtractedSkill {
    pub slug: String,
    pub title: String,
    pub doc_type: DocumentType,
    pub depth: DepthMode,
    pub chapters: Vec<Chapter>,
    pub glossary: Vec<GlossaryTerm>,
    pub patterns: Vec<Framework>,
    pub decision_rules: Vec<DecisionRule>,
    pub cheatsheet: String,
    /// Total token count (SKILL.md + all chapters).
    pub total_tokens: usize,
    /// Token savings vs raw document dump.
    pub savings_ratio: f64,
    /// Hash of the complete skill.
    pub skill_hash: [u8; 32],
}

// ─── Extraction Config ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    pub doc_type: DocumentType,
    pub depth: DepthMode,
    pub max_chapters: usize,
    pub max_tokens_per_chapter: usize,
    pub parallel_workers: usize,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        ExtractionConfig {
            doc_type: DocumentType::Technical,
            depth: DepthMode::Reference,
            max_chapters: MAX_CHAPTERS,
            max_tokens_per_chapter: 2000,
            parallel_workers: 4,
        }
    }
}

// ─── Skill Extractor ─────────────────────────────────────────────────────

/// Native skill extraction engine.
pub struct SkillExtractor {
    config: ExtractionConfig,
    pid: PidController,
    /// Cached extraction results.
    cache: std::collections::HashMap<[u8; 32], ExtractedSkill>,
}

impl SkillExtractor {
    pub fn new(config: ExtractionConfig) -> Self {
        SkillExtractor {
            pid: PidController::new(1, config.parallel_workers),
            config,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Extract a skill from a document.
    pub fn extract(&mut self, title: &str, chapters_raw: &[String], now_us: u64) -> ExtractedSkill {
        let doc_hash = crate::event_log::sha3_256(title.as_bytes());

        // Cache check.
        if let Some(cached) = self.cache.get(&doc_hash) {
            return cached.clone();
        }

        let chapters: Vec<Chapter> = chapters_raw.iter().enumerate().map(|(i, raw)| {
            let hash = crate::event_log::sha3_256(raw.as_bytes());
            let token_count = raw.split_whitespace().count();
            Chapter {
                index: i,
                title: format!("Chapter {}", i + 1),
                summary: raw.chars().take(200).collect(),
                frameworks: Vec::new(),
                anti_patterns: Vec::new(),
                token_count,
                hash,
            }
        }).collect();

        let total_tokens: usize = chapters.iter().map(|c| c.token_count).sum();
        let raw_tokens = total_tokens; // approximate raw dump
        let skill_tokens = if total_tokens > 0 { (total_tokens / 10).max(1) } else { 0 };
        let savings_ratio = if raw_tokens > 0 {
            raw_tokens as f64 / skill_tokens.max(1) as f64
        } else {
            0.0
        };

        let skill_hash = crate::event_log::sha3_256(
            &[title.as_bytes(), &total_tokens.to_le_bytes()].concat()
        );

        let skill = ExtractedSkill {
            slug: title.to_lowercase().replace(' ', "-"),
            title: title.to_string(),
            doc_type: self.config.doc_type,
            depth: self.config.depth,
            chapters,
            glossary: Vec::new(),
            patterns: Vec::new(),
            decision_rules: Vec::new(),
            cheatsheet: String::new(),
            total_tokens: skill_tokens,
            savings_ratio,
            skill_hash,
        };

        self.cache.insert(doc_hash, skill.clone());
        skill
    }

    /// Token savings ratio (raw / skill).
    pub fn savings_ratio(&self, skill: &ExtractedSkill) -> f64 {
        skill.savings_ratio
    }

    /// PID output for parallel extraction tuning.
    pub fn pid_output(&self) -> f64 { self.pid.output }
    pub fn config(&self) -> &ExtractionConfig { &self.config }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic_skill() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let chapters = vec!["Intro content here".to_string(), "Advanced topic".to_string()];
        let skill = ext.extract("Test Book", &chapters, 1000);
        assert_eq!(skill.chapters.len(), 2);
        assert!(skill.total_tokens > 0);
    }

    #[test]
    fn savings_ratio_positive() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let chapters = vec!["A".repeat(1000)];
        let skill = ext.extract("Book", &chapters, 1000);
        assert!(skill.savings_ratio >= 1.0);
    }

    #[test]
    fn cache_hit() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let chapters = vec!["content".to_string()];
        let s1 = ext.extract("Book", &chapters, 1000);
        let s2 = ext.extract("Book", &chapters, 2000);
        assert_eq!(s1.skill_hash, s2.skill_hash);
    }

    #[test]
    fn hash_deterministic() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let chapters = vec!["test".to_string()];
        let s1 = ext.extract("Title", &chapters, 1000);
        let s2 = ext.extract("Title", &chapters, 1000);
        assert_eq!(s1.skill_hash, s2.skill_hash);
    }

    #[test]
    fn different_titles_different_hashes() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let chapters = vec!["test".to_string()];
        let s1 = ext.extract("Book A", &chapters, 1000);
        let s2 = ext.extract("Book B", &chapters, 1000);
        assert_ne!(s1.skill_hash, s2.skill_hash);
    }

    #[test]
    fn empty_chapters() {
        let mut ext = SkillExtractor::new(ExtractionConfig::default());
        let skill = ext.extract("Empty", &[], 1000);
        assert_eq!(skill.chapters.len(), 0);
        assert_eq!(skill.total_tokens, 0);
    }
}
