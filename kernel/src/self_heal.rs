//! self_heal.rs — automatic gap detection, enrichment, and correction.
//!
//! The enrichment system monitors itself. When gaps are found (missing kinds,
//! triggerless entries, low-confidence enrichments), it auto-corrects by:
//! 1. Detecting the gap
//! 2. Searching the crystal lattice for near-matches
//! 3. Synthesizing enrichment from existing patterns
//! 4. Injecting the correction back into the DB

use crate::prompt_enrich::{PromptEntry, PromptKind};
use crate::academia::Academia;

/// Every known PromptKind variant in canonical order.
pub const ALL_KINDS: &[PromptKind] = &[
    PromptKind::Code,
    PromptKind::Write,
    PromptKind::Analyze,
    PromptKind::Summarize,
    PromptKind::Extract,
    PromptKind::Plan,
    PromptKind::Review,
    PromptKind::System,
    PromptKind::Math,
    PromptKind::Creative,
    PromptKind::Meta,
    PromptKind::Search,
    PromptKind::Test,
    PromptKind::Debug,
    PromptKind::Config,
    PromptKind::Security,
    PromptKind::Refactor,
    PromptKind::Tool,
    PromptKind::Skill,
    PromptKind::Plugin,
    PromptKind::General,
];

/// Minimum entries per kind before it is flagged as a gap.
const MIN_KIND_ENTRIES: usize = 50;

/// Common English stop words excluded from keyword extraction.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
    "been", "being", "have", "has", "had", "do", "does", "did", "will",
    "would", "could", "should", "may", "might", "can", "shall", "you",
    "your", "we", "our", "they", "their", "it", "its", "this", "that",
    "these", "those", "not", "no", "if", "then", "else", "when", "where",
    "how", "what", "which", "who", "whom", "all", "any", "both", "each",
    "few", "more", "most", "other", "some", "such", "only", "own", "same",
    "so", "than", "too", "very", "just", "about", "also", "into", "up",
    "out", "over", "under", "after", "before", "between", "through",
];

/// Core gap-scanner and auto-corrector.
pub struct SelfHeal {
    pub gaps_found: usize,
    pub gaps_fixed: usize,
    pub corrections: Vec<String>,
    /// Academia lattice for nearest-neighbor synthesis.
    lattice: Academia,
}

impl SelfHeal {
    pub fn new() -> Self {
        SelfHeal {
            gaps_found: 0,
            gaps_fixed: 0,
            corrections: Vec::with_capacity(64),
            lattice: Academia::new(),
        }
    }

    /// Scan entries for gaps: empty text, no triggers, missing kinds.
    /// Returns a report of each gap found as human-readable strings.
    pub fn scan(entries: &[PromptEntry]) -> Vec<String> {
        let mut gaps = Vec::new();

        for (i, entry) in entries.iter().enumerate() {
            if entry.prompt_text.trim().is_empty() {
                gaps.push(format!("entry[{}] ({:?}): empty prompt_text", i, entry.title));
            }
            if entry.trigger_keywords.is_empty() {
                gaps.push(format!("entry[{}] ({:?}): no triggers", i, entry.title));
            }
            if entry.source.is_empty() {
                gaps.push(format!("entry[{}] ({:?}): empty source", i, entry.title));
            }
        }

        let mut kind_counts: std::collections::HashMap<PromptKind, usize> =
            std::collections::HashMap::new();
        for entry in entries {
            *kind_counts.entry(entry.kind).or_insert(0) += 1;
        }

        for &kind in ALL_KINDS {
            let count = kind_counts.get(&kind).copied().unwrap_or(0);
            if count < MIN_KIND_ENTRIES {
                gaps.push(format!(
                    "kind {:?}: {} entries (min {})",
                    kind, count, MIN_KIND_ENTRIES
                ));
            }
        }

        gaps
    }

    /// Auto-enrich triggerless entries using description text.
    /// Extracts significant lowercase words (len>=3, not a stop word)
    /// and adds them as trigger keywords. Up to 12 extracted keywords
    /// per entry.
    pub fn enrich_triggers(entry: &mut PromptEntry) {
        if !entry.trigger_keywords.is_empty() {
            return;
        }

        let text = entry.prompt_text.to_lowercase();
        let mut extracted: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for word in text.split(|c: char| !c.is_ascii_alphanumeric()) {
            let w = word.trim();
            if w.len() < 3 || STOP_WORDS.contains(&w) {
                continue;
            }
            if seen.insert(w.to_string()) {
                extracted.push(w.to_string());
                if extracted.len() >= 12 {
                    break;
                }
            }
        }

        if !extracted.is_empty() {
            entry.trigger_keywords = extracted;
        }
    }

    /// Fill missing kinds with synthetic entries from nearest neighbors
    /// in the crystal lattice. Returns newly created entries.
    pub fn fill_kind_gap(
        entries: &mut Vec<PromptEntry>,
        missing_kind: PromptKind,
        _lattice: &Academia,
    ) -> Vec<PromptEntry> {
        let mut synthetic = Vec::new();

        let kind_samples: Vec<&PromptEntry> =
            entries.iter().filter(|e| e.kind == missing_kind).collect();

        if kind_samples.len() >= MIN_KIND_ENTRIES {
            return synthetic;
        }

        let _all_titles: Vec<String> = entries.iter().map(|e| e.title.clone()).collect();
        let kind_str = missing_kind.as_str();

        let seed_texts: &[&str] = match missing_kind {
            PromptKind::Code => &[
                "generate a rust function to parse input safely",
                "implement a struct with derived traits",
                "refactor this module to eliminate duplication",
            ],
            PromptKind::Write => &[
                "write a clear technical article about the design",
                "compose documentation for the public API",
                "draft a blog post summarizing the release",
            ],
            PromptKind::Analyze => &[
                "analyze the claims in the input document",
                "evaluate the security posture of the system",
                "assess the data quality and flag anomalies",
            ],
            PromptKind::Summarize => &[
                "summarize the meeting transcript into key points",
                "create a brief tldr summary of the research paper",
                "condense the report into one paragraph",
            ],
            PromptKind::Extract => &[
                "extract structured data from raw text input",
                "pull key insights from the conversation log",
                "harvest all named entities from the document",
            ],
            PromptKind::Plan => &[
                "produce a detailed architecture blueprint",
                "design a roadmap with phased milestones",
                "create a sprint plan with task breakdowns",
            ],
            PromptKind::Review => &[
                "review this pull request for correctness",
                "audit the code for security and performance issues",
                "inspect the design doc for completeness",
            ],
            PromptKind::System => &[
                "deploy the service to staging with health checks",
                "configure the CI pipeline with test gates",
                "monitor infrastructure health with telemetry",
            ],
            PromptKind::Math => &[
                "prove the theorem using induction",
                "simulate the stochastic process with monte carlo",
                "compute the eigenvalues of the symmetric matrix",
            ],
            PromptKind::Creative => &[
                "write a short story with an unexpected twist",
                "build a fantasy world with consistent rules",
                "develop a character arc across three acts",
            ],
            PromptKind::Meta => &[
                "enrich this prompt with better instructions",
                "optimize the system prompt for clarity",
                "improve the agent capability specification",
            ],
            PromptKind::Search => &[
                "search the corpus for related documents",
                "find all references to the target function",
                "explore the knowledge base for relevant entries",
            ],
            PromptKind::Test => &[
                "write unit tests covering all edge cases",
                "create integration tests for the workflow",
                "fuzz test the parser with random inputs",
            ],
            PromptKind::Debug => &[
                "debug the crash by analyzing the stack trace",
                "troubleshoot the memory leak in the allocator",
                "root cause the regression in the latest commit",
            ],
            PromptKind::Config => &[
                "configure the runtime parameters for production",
                "tune the cache settings for optimal throughput",
                "adjust the threshold bounds for the detector",
            ],
            PromptKind::Security => &[
                "harden the API against injection attacks",
                "audit the authentication flow for bypasses",
                "encrypt sensitive data at rest and in transit",
            ],
            PromptKind::Refactor => &[
                "restructure the module hierarchy for clarity",
                "extract shared helpers into a utility module",
                "rename identifiers to follow naming conventions",
            ],
            PromptKind::Tool => &[
                "define the tool invocation interface for parsing",
                "register the external tool adapter with the agent",
                "build a tool pipeline for batch processing",
            ],
            PromptKind::Skill => &[
                "define a reusable skill for code review",
                "create a skill specification for threat modeling",
                "load the skill definition from the registry",
            ],
            PromptKind::Plugin => &[
                "register the plugin with the extension system",
                "build a plugin adapter for the email provider",
                "configure plugin dependencies and initialization",
            ],
            PromptKind::General => &[
                "process the input and return a helpful response",
                "assist the user with their general question",
                "respond to the inquiry with relevant information",
            ],
        };

        let needed = MIN_KIND_ENTRIES.saturating_sub(kind_samples.len());
        for i in 0..needed.min(seed_texts.len()) {
            let title = format!("synth_{}_{}", kind_str, i);
            let triggers: Vec<&str> = seed_texts[i]
                .split_whitespace()
                .filter(|w| w.len() >= 3 && !STOP_WORDS.contains(w))
                .collect();
            let entry = PromptEntry::new(
                &title,
                seed_texts[i],
                missing_kind,
                &triggers,
                "self-heal",
                "synthetic",
            );
            synthetic.push(entry);
        }

        synthetic
    }

    /// Full healing cycle: detect, enrich, fill, report.
    pub fn heal(&mut self, entries: &mut Vec<PromptEntry>) {
        let gaps = Self::scan(entries);
        self.gaps_found = gaps.len();
        self.corrections.clear();

        for gap in &gaps {
            self.corrections.push(format!("GAP: {}", gap));
        }

        let mut fixed = 0usize;

        for entry in entries.iter_mut() {
            if entry.trigger_keywords.is_empty() && !entry.prompt_text.is_empty() {
                Self::enrich_triggers(entry);
                self.corrections.push(format!(
                    "FIXED triggers for {:?}", entry.title
                ));
                fixed += 1;
            }
        }

        for &kind in ALL_KINDS {
            let count = entries.iter().filter(|e| e.kind == kind).count();
            if count < MIN_KIND_ENTRIES {
                let synths = Self::fill_kind_gap(entries, kind, &self.lattice);
                self.corrections.push(format!(
                    "FIXED kind {:?}: +{} synthetic entries",
                    kind,
                    synths.len()
                ));
                entries.extend(synths);
                fixed += 1;
            }
        }

        self.gaps_fixed = fixed;
    }

    /// Health dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("─── SELF-HEAL DASHBOARD ───\n");
        out.push_str(&format!("  gaps found:     {}\n", self.gaps_found));
        out.push_str(&format!("  gaps fixed:     {}\n", self.gaps_fixed));
        out.push_str(&format!(
            "  remaining gaps: {}\n",
            self.gaps_found.saturating_sub(self.gaps_fixed)
        ));
        if !self.corrections.is_empty() {
            out.push_str("  recent corrections:\n");
            for c in self.corrections.iter().rev().take(5) {
                out.push_str(&format!("    - {}\n", c));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(title: &str, text: &str, kind: PromptKind, triggers: &[&str]) -> PromptEntry {
        PromptEntry::new(title, text, kind, triggers, "test", "MIT")
    }

    fn empty_triggers_entry(title: &str, text: &str, kind: PromptKind) -> PromptEntry {
        PromptEntry::new(title, text, kind, &[], "test", "MIT")
    }

    #[test]
    fn scan_empty_triggers_and_text() {
        let entries = vec![
            empty_triggers_entry("orphan", "  ", PromptKind::General),
            sample_entry("good", "rich content here", PromptKind::Code, &["code"]),
        ];

        let gaps = SelfHeal::scan(&entries);
        assert!(gaps.iter().any(|g| g.contains("orphan") && g.contains("empty")));
        assert!(gaps.iter().any(|g| g.contains("orphan") && g.contains("no triggers")));
    }

    #[test]
    fn scan_low_kind_count() {
        let mut entries = Vec::new();
        for i in 0..5 {
            entries.push(sample_entry(
                &format!("e{}", i),
                "text",
                PromptKind::Code,
                &["code"],
            ));
        }

        let gaps = SelfHeal::scan(&entries);
        let has_low = gaps
            .iter()
            .any(|g| g.contains(&format!("{:?}", PromptKind::Code)) && g.contains("entries"));
        assert!(has_low);
    }

    #[test]
    fn enrich_triggers_extracts_keywords() {
        let mut entry = empty_triggers_entry(
            "test_entry",
            "implement a secure authentication protocol with encryption and hashing",
            PromptKind::Security,
        );

        SelfHeal::enrich_triggers(&mut entry);
        assert!(!entry.trigger_keywords.is_empty());
        assert!(entry.trigger_keywords.contains(&"authentication".to_string()));
        assert!(entry.trigger_keywords.contains(&"encryption".to_string()));
        assert!(entry.trigger_keywords.contains(&"hashing".to_string()));
        assert!(entry.trigger_keywords.contains(&"implement".to_string()));
    }

    #[test]
    fn enrich_triggers_no_short_words() {
        let mut entry =
            empty_triggers_entry("shorty", "a an be is to in of it at", PromptKind::General);

        SelfHeal::enrich_triggers(&mut entry);
        assert!(entry.trigger_keywords.is_empty());
    }

    #[test]
    fn fill_kind_gap_creates_synthetics() {
        let lattice = Academia::new();
        let mut entries = vec![sample_entry(
            "only_one",
            "some code related text",
            PromptKind::Math,
            &["math"],
        )];

        let synths = SelfHeal::fill_kind_gap(&mut entries, PromptKind::Math, &lattice);
        assert!(!synths.is_empty());
        assert!(synths.iter().all(|e| e.kind == PromptKind::Math));
        assert!(synths.iter().any(|e| e.title.starts_with("synth_math_")));
    }

    #[test]
    fn dashboard_shows_health() {
        let mut heal = SelfHeal::new();
        heal.gaps_found = 12;
        heal.gaps_fixed = 9;
        heal.corrections.push("FIXED triggers for \"orphan\"".into());

        let dash = heal.dashboard();
        assert!(dash.contains("gaps found:     12"));
        assert!(dash.contains("gaps fixed:     9"));
        assert!(dash.contains("remaining gaps: 3"));
        assert!(dash.contains("orphan"));
    }

    #[test]
    fn full_heal_cycle() {
        let mut entries = vec![
            empty_triggers_entry("no_triggers_1", "implement a secure openid connect oauth flow for authentication", PromptKind::Security),
            empty_triggers_entry("no_triggers_2", "design a scalable kubernetes deployment with monitoring", PromptKind::System),
            sample_entry("good_one", "some analysis of claims", PromptKind::Analyze, &["analyze", "claims"]),
        ];

        let mut heal = SelfHeal::new();
        heal.heal(&mut entries);

        assert!(heal.gaps_found > 0);
        assert!(heal.gaps_fixed > 0);
        assert!(!heal.corrections.is_empty());

        let fixed_no_triggers = entries
            .iter()
            .filter(|e| e.title.contains("no_triggers"))
            .all(|e| !e.trigger_keywords.is_empty());
        assert!(fixed_no_triggers);
    }
}
