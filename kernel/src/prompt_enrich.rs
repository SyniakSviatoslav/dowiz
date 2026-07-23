//! `kernel::prompt_enrich` — native, zero-dep prompt/skill enrichment engine.
//!
//! Parses, stores, and retrieves prompt templates. Detects user intent from
//! natural language and injects the best matching prompt enrichments.
//!
//! # Architecture
//! 1. **Ingest** — feeds raw prompt templates into the engine
//! 2. **Lattice** — 8D crystal lattice (reuse `crate::academia`) for O(1) lookup
//! 3. **Intent** — keyword/domain classification → best matching prompts
//! 4. **Enrich** — given user input, return ranked enrichment suggestions
//! 5. **Recursive** — prompts from batch N seed batch N+1 (same pattern as `research`)
//!
//! # Sources (CC0/MIT-licensed, scraped + reverse-engineered)
//! - fabric patterns (danielmiessler/fabric, MIT)
//! - prompts.chat (f/awesome-chatgpt-prompts, CC0)
//! - opencode built-in skills/agents
//! - any user-supplied custom prompts
//!
//! # Target: 100k prompts, 100k skills/tools/plugins stored natively
//! Stored in the crystal lattice for O(1) neighbor lookup — same infra as
//! `academia.rs` uses for 610M papers but scaled down for prompt text.

use crate::event_log::sha3_256;
use crate::academia::Academia;
use std::collections::HashMap;

/// Max prompt entries in the engine.
pub const MAX_PROMPTS: usize = 100_000;
/// Min keyword match count to trigger enrichment.
pub const MIN_INTENT_KEYWORDS: usize = 1;
/// Max enriched prompts returned per query.
pub const MAX_ENRICH_RESULTS: usize = 5;

// ─── PromptKind ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum PromptKind {
    /// Code generation, refactoring, explanation
    Code = 0,
    /// Writing: essays, articles, documentation, copy
    Write = 1,
    /// Analysis: claims, arguments, data, security
    Analyze = 2,
    /// Summarization: condense, extract key points
    Summarize = 3,
    /// Extraction: pull structured data from unstructured text
    Extract = 4,
    /// Planning: architecture, roadmaps, design
    Plan = 5,
    /// Review: code review, PR review, security audit
    Review = 6,
    /// System/ops: CI, deployment, monitoring, infrastructure
    System = 7,
    /// Math/science: equations, proofs, simulations
    Math = 8,
    /// Creative: stories, dialogue, worldbuilding
    Creative = 9,
    /// Meta: prompt improvement, self-enrichment
    Meta = 10,
    /// Search/research: finding, indexing, knowledge retrieval
    Search = 11,
    /// Testing: unit tests, integration, fuzzing
    Test = 12,
    /// Debug: root-cause, trace analysis
    Debug = 13,
    /// Config: configuration, tuning, optimization
    Config = 14,
    /// Security: hardening, threat modeling, audit
    Security = 15,
    /// Refactor: restructuring without changing behavior
    Refactor = 16,
    /// Tool use: specific tool invocation patterns
    Tool = 17,
    /// Skill: reusable capability definition
    Skill = 18,
    /// Plugin: extensibility patterns
    Plugin = 19,
    /// General: catch-all for uncategorized prompts
    General = 31,
}

impl PromptKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptKind::Code => "code",
            PromptKind::Write => "write",
            PromptKind::Analyze => "analyze",
            PromptKind::Summarize => "summarize",
            PromptKind::Extract => "extract",
            PromptKind::Plan => "plan",
            PromptKind::Review => "review",
            PromptKind::System => "system",
            PromptKind::Math => "math",
            PromptKind::Creative => "creative",
            PromptKind::Meta => "meta",
            PromptKind::Search => "search",
            PromptKind::Test => "test",
            PromptKind::Debug => "debug",
            PromptKind::Config => "config",
            PromptKind::Security => "security",
            PromptKind::Refactor => "refactor",
            PromptKind::Tool => "tool",
            PromptKind::Skill => "skill",
            PromptKind::Plugin => "plugin",
            PromptKind::General => "general",
        }
    }
}

// ─── PromptEntry ───────────────────────────────────────────────────────────

/// A single prompt/skill/tool template, scraped and stored natively.
#[derive(Debug, Clone)]
pub struct PromptEntry {
    /// Unique hash (SHA3-256 of title).
    pub id: [u8; 32],
    /// Short name/title.
    pub title: String,
    /// Full system prompt text.
    pub prompt_text: String,
    /// What task category this prompt targets.
    pub kind: PromptKind,
    /// Keywords that trigger this prompt (lowercase).
    pub trigger_keywords: Vec<String>,
    /// Source repository / origin.
    pub source: String,
    /// License (CC0, MIT, unlicensed, etc.).
    pub license: String,
    /// How many times this prompt has been used (or matched).
    pub use_count: u64,
    /// 8D quark signature for crystal lattice indexing.
    pub quark_sig: [u8; 8],
}

impl PromptEntry {
    pub fn new(title: &str, prompt_text: &str, kind: PromptKind, triggers: &[&str], source: &str, license: &str) -> Self {
        let hash = sha3_256(title.as_bytes());
        let quark_sig = crate::academia::hash_to_row(title);
        PromptEntry {
            id: hash,
            title: title.to_string(),
            prompt_text: prompt_text.to_string(),
            kind,
            trigger_keywords: triggers.iter().map(|s| s.to_lowercase()).collect(),
            source: source.to_string(),
            license: license.to_string(),
            use_count: 0,
            quark_sig,
        }
    }
}

// ─── EnrichedResult ────────────────────────────────────────────────────────

/// A matched enrichment — the original prompt plus injected suggestions.
#[derive(Debug, Clone)]
pub struct EnrichedResult {
    /// Matching prompts ranked by relevance.
    pub matches: Vec<PromptEntry>,
    /// Detected intent kind.
    pub intent: PromptKind,
    /// Intent confidence (0.0–1.0).
    pub intent_confidence: f64,
}

// ─── IntentKeywordMap ──────────────────────────────────────────────────────

/// Maps keywords → PromptKind for intent detection.
type IntentMap = HashMap<&'static str, PromptKind>;

fn build_intent_map() -> IntentMap {
    let mut m = HashMap::new();
    // Code
    for k in &["code", "implement", "build", "refactor", "compile", "debug", "bug",
        "function", "struct", "impl", "mod", "cargo", "rustc", "npm", "pip", "import",
        "fix", "patch", "component", "module", "crate", "type", "trait", "enum"] {
        m.insert(*k, PromptKind::Code);
    }
    // Write
    for k in &["write", "essay", "article", "document", "blog", "readme", "docs",
        "prose", "paragraph", "author", "compose", "draft", "text"] {
        m.insert(*k, PromptKind::Write);
    }
    // Analyze
    for k in &["analyze", "analysis", "evaluate", "assess", "audit", "claims",
        "verify", "validate", "inspect", "examine", "investigate"] {
        m.insert(*k, PromptKind::Analyze);
    }
    // Summarize
    for k in &["summarize", "summary", "tldr", "brief", "condense", "recap", "digest",
        "synopsis", "abridge"] {
        m.insert(*k, PromptKind::Summarize);
    }
    // Extract
    for k in &["extract", "parse", "scrape", "pull", "harvest", "gather", "collect",
        "fetch", "crawl"] {
        m.insert(*k, PromptKind::Extract);
    }
    // Plan
    for k in &["plan", "roadmap", "blueprint", "design", "architecture", "spec",
        "proposal", "phase", "milestone", "strategy"] {
        m.insert(*k, PromptKind::Plan);
    }
    // Review
    for k in &["review", "audit", "inspect", "check", "critique", "feedback", "pr",
        "pull request", "code quality"] {
        m.insert(*k, PromptKind::Review);
    }
    // System
    for k in &["deploy", "ci", "cd", "docker", "container", "server", "infra",
        "kubernetes", "aws", "gcp", "terraform", "ansible", "systemd", "service",
        "pipeline", "orchestrate", "operate"] {
        m.insert(*k, PromptKind::System);
    }
    // Test
    for k in &["test", "spec", "assert", "mock", "stub", "fuzz", "coverage",
        "unit test", "integration test", "e2e", "prove"] {
        m.insert(*k, PromptKind::Test);
    }
    // Security
    for k in &["security", "vuln", "exploit", "threat", "attack", "harden",
        "encrypt", "decrypt", "auth", "authn", "authz", "permission", "acl", "rbac"] {
        m.insert(*k, PromptKind::Security);
    }
    // Meta
    for k in &["prompt", "enrich", "improve prompt", "optimize prompt", "skill",
        "plugin", "tool", "agent"] {
        m.insert(*k, PromptKind::Meta);
    }
    // Refactor
    for k in &["refactor", "clean up", "simplify", "dedup", "extract", "rename",
        "restructure", "reorganize", "decouple"] {
        m.insert(*k, PromptKind::Refactor);
    }
    m
}

fn detect_intent(text: &str) -> (PromptKind, f64) {
    let lower = text.to_lowercase();
    let map = build_intent_map();
    let mut scores: HashMap<PromptKind, usize> = HashMap::new();

    for (keyword, kind) in &map {
        if lower.contains(*keyword) {
            *scores.entry(*kind).or_insert(0) += 1;
        }
    }

    if scores.is_empty() {
        return (PromptKind::General, 0.0);
    }

    let total: usize = scores.values().sum();
    let (best_kind, best_count) = scores.iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, c)| (*k, *c))
        .unwrap_or((PromptKind::General, 0));

    let confidence = if total > 0 { best_count as f64 / total.max(1) as f64 } else { 0.0 };
    (best_kind, confidence)
}

// ─── PromptEnrichEngine ────────────────────────────────────────────────────

pub struct PromptEnrichEngine {
    /// All stored prompts.
    pub prompts: Vec<PromptEntry>,
    /// 8D crystal lattice for O(1) neighbor lookup.
    pub lattice: Academia,
    /// Prompt index: kind → vec of prompt indices.
    kind_index: HashMap<PromptKind, Vec<usize>>,
    /// Keyword index: keyword → vec of prompt indices.
    keyword_index: HashMap<String, Vec<usize>>,
    total_ingested: u64,
}

impl PromptEnrichEngine {
    pub fn new() -> Self {
        PromptEnrichEngine {
            prompts: Vec::with_capacity(MAX_PROMPTS),
            lattice: Academia::new(),
            kind_index: HashMap::new(),
            keyword_index: HashMap::new(),
            total_ingested: 0,
        }
    }

    /// Ingest a batch of prompt entries.
    pub fn ingest(&mut self, entries: Vec<PromptEntry>) {
        for entry in entries {
            // Crystal lattice index (stores quark signature, returns index).
            self.lattice.insert(&entry.title);

            // Kind index.
            self.kind_index.entry(entry.kind).or_default().push(self.prompts.len());

            // Keyword index.
            for kw in &entry.trigger_keywords {
                self.keyword_index.entry(kw.clone()).or_default().push(self.prompts.len());
            }

            self.prompts.push(entry);
            self.total_ingested += 1;

            if self.prompts.len() >= MAX_PROMPTS {
                break;
            }
        }
    }

    pub fn total(&self) -> usize { self.prompts.len() }

    /// Enrich a user prompt by finding the best matching prompt templates.
    ///
    /// 1. Detect intent from user input
    /// 2. Query crystal lattice for neighbors
    /// 3. Rank by combination of intent match + keyword hits + lattice distance
    /// 4. Return top-N enrichments
    pub fn enrich(&self, user_input: &str) -> EnrichedResult {
        let (intent, confidence) = detect_intent(user_input);
        let lower = user_input.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        // Collect candidates: prompts matching intent AND/OR keywords.
        let mut candidates: Vec<(usize, u32)> = Vec::new(); // (prompt_idx, score)

        // Phase 1: intent-matched prompts.
        if let Some(kind_matches) = self.kind_index.get(&intent) {
            for &idx in kind_matches {
                let mut score = 3u32; // base intent match
                let entry = &self.prompts[idx];

                // Keyword overlap bonus.
                for word in &words {
                    if entry.trigger_keywords.iter().any(|k| k.contains(word) || word.contains(k.as_str())) {
                        score += 2;
                    }
                    if entry.prompt_text.to_lowercase().contains(word) {
                        score += 1;
                    }
                }
                candidates.push((idx, score));
            }
        }

        // Phase 2: keyword-matched prompts (other kinds).
        for word in &words {
            if word.len() < 3 { continue; }
            if let Some(kw_matches) = self.keyword_index.get(*word) {
                for &idx in kw_matches {
                    let entry = &self.prompts[idx];
                    let base = if entry.kind == intent { 2 } else { 1 };
                    candidates.push((idx, base));
                }
            }
        }

        // Phase 3: lattice neighbor search (returns indices into lattice matrix).
        let lattice_neighbors = self.lattice.search(user_input, 10);
        for (lattice_idx, _score) in &lattice_neighbors {
            // lattice_idx is the insertion order in Academia (not our prompt index).
            // We rely on keyword + kind matching above; lattice is supplemental.
            if *lattice_idx < self.prompts.len() {
                if !candidates.iter().any(|(i, _)| *i == *lattice_idx) {
                    candidates.push((*lattice_idx, 1));
                }
            }
        }

        // Dedup + sort by score descending.
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let mut seen = std::collections::HashSet::new();
        let mut matches: Vec<PromptEntry> = Vec::new();
        for (idx, _) in candidates {
            if seen.insert(idx) && matches.len() < MAX_ENRICH_RESULTS {
                matches.push(self.prompts[idx].clone());
            }
        }

        EnrichedResult {
            matches,
            intent,
            intent_confidence: confidence,
        }
    }

    /// Search prompts by keyword (exact match on trigger_keywords or title).
    pub fn search(&self, query: &str) -> Vec<&PromptEntry> {
        let lower = query.to_lowercase();
        self.prompts.iter()
            .filter(|p| {
                p.title.to_lowercase().contains(&lower)
                    || p.trigger_keywords.iter().any(|k| k.contains(&lower))
                    || p.prompt_text.to_lowercase().contains(&lower)
            })
            .collect()
    }

    /// Dashboard summary.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("Prompt Enrich Engine\n");
        out.push_str(&format!("  Total prompts:  {}\n", self.prompts.len()));
        out.push_str(&format!("  Lattice cells:  {}\n", self.lattice.len()));
        out.push_str(&format!("  Kind index:     {} kinds\n", self.kind_index.len()));
        out.push_str(&format!("  Keyword index:  {} keywords\n", self.keyword_index.len()));

        let mut kind_counts: Vec<(&str, usize)> = self.kind_index.iter()
            .map(|(k, v)| (k.as_str(), v.len()))
            .collect();
        kind_counts.sort_by(|a, b| b.1.cmp(&a.1));
        out.push_str("  By kind:\n");
        for (kind, count) in kind_counts.iter().take(8) {
            out.push_str(&format!("    {}: {}\n", kind, count));
        }
        out
    }
}

// ─── Built-in prompt seed database ─────────────────────────────────────────

/// Seed prompts from scraped fabric patterns (MIT-licensed).
pub fn seed_fabric_prompts() -> Vec<PromptEntry> {
    vec![
        PromptEntry::new(
            "analyze_claims", "You are an objectively minded and centrist-oriented analyzer of truth claims and arguments.\n\
You specialize in analyzing and rating the truth claims made in the input provided and providing both evidence in support of those claims, as well as counter-arguments and counter-evidence.\n\
Output: ARGUMENT SUMMARY, TRUTH CLAIMS with CLAIM SUPPORT EVIDENCE and CLAIM REFUTATION EVIDENCE, LOGICAL FALLACIES, CLAIM RATING (A-F), LABELS, OVERALL SCORE.",
            PromptKind::Analyze, &["analyze","claims","fact-check","verify","debunk","truth","evidence"], "fabric","MIT"),

        PromptEntry::new(
            "summarize", "You are an expert content summarizer. You take content in and output a Markdown formatted summary.\n\
Output: ONE SENTENCE SUMMARY (20 words max), MAIN POINTS (up to 10, each ≤16 words), TAKEAWAYS (up to 5).",
            PromptKind::Summarize, &["summarize","summary","tldr","recap","digest","brief"], "fabric","MIT"),

        PromptEntry::new(
            "extract_wisdom", "You extract surprising, insightful, and interesting information from text content.\n\
Focus: purpose and meaning of life, human flourishing, technology's future, AI and humans, memes, learning, reading, books, continuous improvement.\n\
Output: SUMMARY, IDEAS (20-50), INSIGHTS (10-20), QUOTES (15-30), HABITS, FACTS, REFERENCES, ONE-SENTENCE TAKEAWAY, RECOMMENDATIONS.",
            PromptKind::Extract, &["extract","wisdom","insights","ideas","quotes","habits","lessons","learnings"], "fabric","MIT"),

        PromptEntry::new(
            "explain_code", "You are an expert coder that takes code and documentation as input and do your best to explain it.\n\
Output depends on input type: EXPLANATION (code), SECURITY IMPLICATIONS (security output), CONFIGURATION EXPLANATION (config), ANSWER (documentation questions).",
            PromptKind::Code, &["explain","what does this do","document","walkthrough","understand code"], "fabric","MIT"),

        PromptEntry::new(
            "improve_prompt", "You optimize LLM prompts using the 6-strategy OpenAI prompt engineering guide:\n\
1. Write clear instructions (be specific, delimiters, structured output, conditional steps, few-shot)\n\
2. Provide reference text\n\
3. Split complex tasks into subtasks (intent classification, summarize/recursively, summarize long documents piecewise)\n\
4. Give the model time to think (inner monologue, chain of thought, self-ask)\n\
5. Use external tools (code execution / function calling)\n\
6. Test changes systematically (evaluations with golden answers)\n\
Input: a prompt. Output: improved version with strategies applied.",
            PromptKind::Meta, &["improve","optimize","better prompt","rewrite prompt","enhance prompt","prompt engineering"], "fabric","MIT"),

        PromptEntry::new(
            "rate_content", "You rate content quality by idea density and theme alignment.\n\
Output: LABELS (single-word content themes), RATING (S: profound novel ideas, A: high quality, B: good, C: mediocre, D: poor), CONTENT SCORE (1-100).",
            PromptKind::Analyze, &["rate","rank","score","quality","classify","review content"], "fabric","MIT"),

        PromptEntry::new(
            "label_and_rate", "You label and rate content using predefined taxonomy:\n\
Labels: Meaning, Future, Business, Tutorial, Podcast, Miscellaneous, Creativity, NatSec, CyberSecurity, AI, Essay, Video, Conversation, Optimization, Personal, Writing, Human3.0, Health, Technology, Education, Leadership, Mindfulness, Innovation, Culture, Productivity, Science, Philosophy.\n\
Output JSON only: {\"rating\":\"A-F\",\"score\":1-100,\"labels\":[\"...\"]}.",
            PromptKind::Analyze, &["label","categorize","tag","classify","tier list"], "fabric","MIT"),

        PromptEntry::new(
            "write_essay", "You write an essay in the style of {{author_name}}.\n\
1. Look up example works by the author to understand voice, vocabulary, sentence structure\n\
2. Match the author's vocabulary level precisely\n\
3. Use ZERO cliches — every sentence must be original\n\
4. Mirror the author's rhetorical patterns and argument style",
            PromptKind::Write, &["write essay","emulate author","in the style of","compose"], "fabric","MIT"),

        PromptEntry::new(
            "create_report_finding", "You create a cybersecurity finding report.\n\
Output sections: Description, Risk, Recommendations, References, One-Sentence-Summary, Trends, Quotes.\n\
Focus: objective technical assessment, actionable remediation, severity classification.",
            PromptKind::Security, &["security finding","vulnerability report","pentest writeup","threat","risk assessment"], "fabric","MIT"),

        PromptEntry::new(
            "agility_story", "You create an agile user story with acceptance criteria.\n\
Output JSON: {\"Topic\":\"...\",\"Story\":\"As a <role> I want <goal> so that <reason>\",\"Criteria\":[\"Given <context> When <action> Then <outcome>\"]}.",
            PromptKind::Plan, &["user story","acceptance criteria","agile","scrum","story","backlog"], "fabric","MIT"),

        PromptEntry::new(
            "clean_text", "You clean broken/malformatted text.\n\
Fix: line breaks, punctuation, capitalization, spacing.\n\
Do NOT change content, spelling, or meaning. Input: messy text. Output: clean text.",
            PromptKind::Write, &["clean","fix formatting","repair text","cleanup","normalize text"], "fabric","MIT"),

        PromptEntry::new(
            "capture_thinkers_work", "You extract a philosopher/thinker's key teachings.\n\
Output: ONE-LINE ENCAPSULATION, BACKGROUND, SCHOOL, MOST IMPACTFUL IDEAS (list), PRIMARY ADVICE/TEACHINGS, WORKS (bibliography), QUOTES, APPLICATION (how to apply in daily life), ADVICE.",
            PromptKind::Extract, &["philosophy","thinker","philosopher","school of thought","teachings","ideas of"], "fabric","MIT"),
    ]
}

/// Seed prompts from opencode built-in agents (reverse-engineered from docs).
pub fn seed_opencode_prompts() -> Vec<PromptEntry> {
    vec![
        PromptEntry::new(
            "code_reviewer", "You are a code reviewer. Focus on security, performance, and maintainability.\n\
Check: input validation, error handling, concurrency safety, resource leaks, API design, test coverage.\n\
Provide constructive feedback without making direct changes.",
            PromptKind::Review, &["review","audit","code check","code quality","security review"], "opencode","MIT"),

        PromptEntry::new(
            "security_auditor", "You are a security expert. Focus on identifying potential security issues.\n\
Look for: input validation flaws, authentication bypass, data exposure, dependency vulnerabilities, configuration issues, injection attacks, crypto misuse.",
            PromptKind::Security, &["security","vuln","exploit","threat","hardening","audit security","auth"], "opencode","MIT"),

        PromptEntry::new(
            "docs_writer", "You are a technical writer. Create clear, comprehensive documentation.\n\
Focus on: clear explanations, proper structure, code examples, user-friendly language, consistent terminology, navigation aids.",
            PromptKind::Write, &["document","docs","readme","write docs","documentation"], "opencode","MIT"),

        PromptEntry::new(
            "plan_agent", "You are in planning mode. Analyze and plan without making code changes.\n\
Read the codebase, understand the architecture, identify dependencies and risks, produce a detailed plan. Do NOT edit files.",
            PromptKind::Plan, &["plan","roadmap","blueprint","architecture","design","proposal"], "opencode","MIT"),

        PromptEntry::new(
            "build_agent", "You are the build agent. Full development mode — all tools enabled.\n\
Write code, run tests, fix bugs, refactor, deploy. Follow conventions, keep changes minimal and correct.",
            PromptKind::Code, &["build","implement","develop","code","create","make"], "opencode","MIT"),

        PromptEntry::new(
            "explore_agent", "You explore the codebase — read-only.\n\
Find files by patterns, search code for keywords, read and analyze. Report findings precisely with file:line references.",
            PromptKind::Search, &["explore","find","search","locate","grep","look for","where is"], "opencode","MIT"),

        PromptEntry::new(
            "test_writer", "You write tests. Focus on: edge cases, error paths, boundary conditions, regression protection.\n\
Write RED→GREEN tests: write the test first (it fails), then the fix (it passes). Never delete tests without confirming they test dead code.",
            PromptKind::Test, &["test","spec","assert","prove","verify test","write test","test coverage"], "opencode","MIT"),

        PromptEntry::new(
            "debug_agent", "You are a debug specialist. Root-cause analysis.\n\
Find the actual cause, not the symptom. Read error messages, trace logs, inspect state, reproduce the issue. Fix at the source.",
            PromptKind::Debug, &["debug","fix bug","troubleshoot","root cause","why does","broken","error"], "opencode","MIT"),

        PromptEntry::new(
            "refactor_cleanup", "You refactor code without changing behavior.\n\
Extract helpers, eliminate duplication, rename for clarity, simplify control flow. Run tests before and after to verify no behavioral change.",
            PromptKind::Refactor, &["refactor","clean up","simplify","dedup","extract","rename","restructure"], "opencode","MIT"),
    ]
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_starts_empty() {
        let engine = PromptEnrichEngine::new();
        assert_eq!(engine.total(), 0);
    }

    #[test]
    fn ingest_seed_prompts() {
        let mut engine = PromptEnrichEngine::new();
        let fabric = seed_fabric_prompts();
        let opencode = seed_opencode_prompts();
        engine.ingest(fabric);
        engine.ingest(opencode);
        assert_eq!(engine.total(), 21); // 12 fabric + 9 opencode
    }

    #[test]
    fn detect_intent_code() {
        let (kind, confidence) = detect_intent("implement a new struct with traits and fix the bug");
        assert_eq!(kind, PromptKind::Code);
        assert!(confidence > 0.0);
    }

    #[test]
    fn detect_intent_security() {
        let (kind, confidence) = detect_intent("audit the authentication system for security vulnerabilities");
        assert_eq!(kind, PromptKind::Security);
        assert!(confidence > 0.0);
    }

    #[test]
    fn detect_intent_summarize() {
        let (kind, _) = detect_intent("summarize this long document into a brief tldr");
        assert_eq!(kind, PromptKind::Summarize);
    }

    #[test]
    fn detect_intent_review() {
        let (kind, _) = detect_intent("review this pull request and check code quality");
        assert_eq!(kind, PromptKind::Review);
    }

    #[test]
    fn detect_intent_plan() {
        let (kind, _) = detect_intent("design a blueprint for the new architecture");
        assert_eq!(kind, PromptKind::Plan);
    }

    #[test]
    fn detect_intent_meta() {
        let (kind, _) = detect_intent("improve this prompt and enrich it with better instructions");
        assert_eq!(kind, PromptKind::Meta);
    }

    #[test]
    fn detect_intent_general_for_empty() {
        let (kind, confidence) = detect_intent("hello world");
        assert_eq!(kind, PromptKind::General);
        assert_eq!(confidence, 0.0);
    }

    #[test]
    fn enrich_returns_matches() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());

        let result = engine.enrich("write a summary of the design document and list key takeaways");
        assert!(!result.matches.is_empty());
        // Should find summarize prompt.
        let has_summarize = result.matches.iter().any(|p| p.title == "summarize");
        if !has_summarize {
            eprintln!("Enrich matches: {:?}", result.matches.iter().map(|p| &p.title).collect::<Vec<_>>());
        }
        assert!(has_summarize);
    }

    #[test]
    fn enrich_code_query() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());

        let result = engine.enrich("implement a function that refactors the code and fix the compilation bug");
        let has_code = result.matches.iter().any(|p| p.kind == PromptKind::Code);
        assert!(has_code);
    }

    #[test]
    fn dashboard_works() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        let dash = engine.dashboard();
        assert!(dash.contains("Prompt Enrich Engine"));
        assert!(dash.contains("Total prompts"));
    }

    #[test]
    fn search_finds_by_keyword() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());

        let results = engine.search("summarize");
        assert!(!results.is_empty());
        assert!(results.iter().any(|p| p.title == "summarize"));
    }

    #[test]
    fn search_finds_by_trigger() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());

        let results = engine.search("vulnerability");
        assert!(results.iter().any(|p| p.title == "create_report_finding"));
    }

    #[test]
    fn prompt_entry_hash_consistent() {
        let a = PromptEntry::new("test", "body", PromptKind::Code, &["test"], "src", "MIT");
        let b = PromptEntry::new("test", "body", PromptKind::Code, &["test"], "src", "MIT");
        assert_eq!(a.id, b.id);
        assert_eq!(a.quark_sig, b.quark_sig);
    }
}
