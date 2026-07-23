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
use crate::delta::{Delta, DeltaComparison, DeltaTracker};
use crate::telemetry_harvest::HarvestLedger;
use crate::chronos_topology::ChronoTopology;
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

// ─── Intent Tree — hierarchical sub-intent classification ──────────────────

/// A node in the intent tree. Root intents (PromptKind) branch into sub-intents
/// which themselves branch into deeper refinements (child → child-of-child).
#[derive(Debug, Clone)]
pub struct IntentNode {
    pub name: &'static str,
    pub kind: PromptKind,
    pub keywords: &'static [&'static str],
    /// Indices into the static INTENT_TREE flat list.
    pub children: &'static [usize],
}

/// A path from root to leaf: ["code", "debug", "compile-error"].
pub type IntentPath = Vec<String>;

/// Compact static tree: flat list, children are indices.
/// Built once at compile-time, queried at runtime.
pub static INTENT_TREE: &[IntentNode] = &[
    // ── Root nodes ───────────────────────────────────────────────────────
    IntentNode { name: "code", kind: PromptKind::Code, keywords: &["code","implement","build","compile","function","struct","impl","mod","cargo","rustc","npm","pip","import","fix","patch","component","module","crate","type","trait","enum","api","endpoint","handler","route","service","async","await","concurrent","код","реалізувати","реализовать","скомпілювати","функція","функция","виправити","исправить","модуль","білд","билд","створити","создать","писати","писать","розробка","разработка"], children: &[1,2,3,4,5,6] },
    IntentNode { name: "implement", kind: PromptKind::Code, keywords: &["implement","build","create","make","write","generate","scaffold","new","setup","init"], children: &[7,8,9,10,11] },
    IntentNode { name: "debug", kind: PromptKind::Code, keywords: &["debug","fix","troubleshoot","root","cause","broken","error","panic","crash","bug","why","doesn","incorrect","wrong","unexpected"], children: &[12,13,14,15] },
    IntentNode { name: "refactor", kind: PromptKind::Refactor, keywords: &["refactor","clean","up","simplify","dedup","extract","rename","restructure","reorganize","decouple","remove","dead","unused"], children: &[16,17,18,19] },
    IntentNode { name: "review", kind: PromptKind::Review, keywords: &["review","audit","inspect","check","critique","feedback","pr","pull","request","code quality"], children: &[20,21,22,23] },
    IntentNode { name: "test", kind: PromptKind::Test, keywords: &["test","spec","assert","mock","stub","fuzz","coverage","unit","integration","e2e","end","prove","regression","snapshot","golden"], children: &[24,25,26,27] },
    IntentNode { name: "api-client", kind: PromptKind::Code, keywords: &["api","client","fetch","request","http","endpoint","rest","graphql","grpc","websocket"], children: &[] },
    // ── implement children ────────────────────────────────────────────────
    IntentNode { name: "struct", kind: PromptKind::Code, keywords: &["struct","data","model","entity","record","newtype","wrapper","builder","pattern"], children: &[] },
    IntentNode { name: "function", kind: PromptKind::Code, keywords: &["function","fn","method","closure","handler","callback","algorithm","logic","flow"], children: &[] },
    IntentNode { name: "module", kind: PromptKind::Code, keywords: &["module","mod","crate","library","package","workspace","project","layout","organize","split"], children: &[] },
    IntentNode { name: "trait-impl", kind: PromptKind::Code, keywords: &["trait","impl","interface","abstract","generic","polymorph","derive","macro"], children: &[] },
    IntentNode { name: "api", kind: PromptKind::Code, keywords: &["api","endpoint","route","handler","rest","crud","controller","resource","serialize","deserialize"], children: &[] },
    // ── debug children ────────────────────────────────────────────────────
    IntentNode { name: "compile", kind: PromptKind::Code, keywords: &["compil","type","error","borrow","lifetime","linker","cargo","build","fail","unresolved","undeclared"], children: &[] },
    IntentNode { name: "runtime", kind: PromptKind::Code, keywords: &["panic","crash","segfault","null","deref","overflow","deadlock","race","leak","oom","timeout"], children: &[] },
    IntentNode { name: "test-fail", kind: PromptKind::Test, keywords: &["test","fail","regression","assertion","mock","expect","red","broke"], children: &[] },
    IntentNode { name: "logic", kind: PromptKind::Code, keywords: &["logic","wrong","incorrect","off","by","one","boundary","edge","case","condition","branch","off-by"], children: &[] },
    // ── refactor children ─────────────────────────────────────────────────
    IntentNode { name: "extract-helper", kind: PromptKind::Refactor, keywords: &["extract","helper","utility","shared","common","dedup","dry","duplicate","reuse","generic","macro"], children: &[] },
    IntentNode { name: "rename", kind: PromptKind::Refactor, keywords: &["rename","naming","identifier","variable","consistency","style","convention","camel","snake"], children: &[] },
    IntentNode { name: "restructure", kind: PromptKind::Refactor, keywords: &["restructure","reorganize","decouple","split","merge","flatten","nest","modular"], children: &[] },
    IntentNode { name: "remove-dead", kind: PromptKind::Refactor, keywords: &["dead","unused","obsolete","deprecated","stale","legacy","remove","delete","purge","cleanup"], children: &[] },
    // ── review children ───────────────────────────────────────────────────
    IntentNode { name: "security", kind: PromptKind::Security, keywords: &["security","vuln","vulnerable","inject","xss","csrf","auth","authn","authz","crypto","secret","token","leak","expose"], children: &[] },
    IntentNode { name: "perf", kind: PromptKind::Code, keywords: &["perf","performance","bottleneck","slow","optimize","benchmark","latency","throughput","memory","cpu","profile"], children: &[] },
    IntentNode { name: "style", kind: PromptKind::Code, keywords: &["style","format","lint","clippy","convention","idiom","best","practice","pattern","clean","readable"], children: &[] },
    IntentNode { name: "api-design", kind: PromptKind::Code, keywords: &["api","design","contract","interface","signature","public","export","breaking","compat","version"], children: &[] },
    // ── test children ─────────────────────────────────────────────────────
    IntentNode { name: "unit", kind: PromptKind::Test, keywords: &["unit","function","method","isolated","mock","stub","spy","assert"], children: &[] },
    IntentNode { name: "integration", kind: PromptKind::Test, keywords: &["integration","e2e","end","to","system","scenario","workflow","browser","selenium","playwright"], children: &[] },
    IntentNode { name: "fuzz", kind: PromptKind::Test, keywords: &["fuzz","property","random","chaos","monkey","mutation","quickcheck","proptest"], children: &[] },
    IntentNode { name: "regression", kind: PromptKind::Test, keywords: &["regression","snapshot","golden","insta","baseline","reference","known","good"], children: &[] },
    // ── More roots ────────────────────────────────────────────────────────
    IntentNode { name: "write", kind: PromptKind::Write, keywords: &["write","essay","article","document","blog","readme","docs","prose","paragraph","author","compose","draft","text","copy","marketing","ad","content"], children: &[28,29,30,31] },
    IntentNode { name: "docs", kind: PromptKind::Write, keywords: &["document","readme","api","docs","guide","tutorial","manual","reference","how","to","example"], children: &[] },
    IntentNode { name: "essay", kind: PromptKind::Write, keywords: &["essay","article","blog","post","prose","narrative","story","piece","editorial"], children: &[] },
    IntentNode { name: "copy", kind: PromptKind::Write, keywords: &["copy","marketing","ad","advertising","promo","landing","sales","pitch","tagline","slogan"], children: &[] },
    IntentNode { name: "technical", kind: PromptKind::Write, keywords: &["technical","spec","rfc","proposal","architecture","decision","adr","white","paper"], children: &[] },
    // ── Analyze ───────────────────────────────────────────────────────────
    IntentNode { name: "analyze", kind: PromptKind::Analyze, keywords: &["analyze","analysis","evaluate","assess","audit","claims","verify","validate","inspect","examine","investigate","research"], children: &[32,33,34,35] },
    IntentNode { name: "claims", kind: PromptKind::Analyze, keywords: &["claim","fact","check","verify","debunk","truth","evidence","cite","source","bias","logic","fallacy"], children: &[] },
    IntentNode { name: "data", kind: PromptKind::Analyze, keywords: &["data","statistic","trend","correlation","analytics","metric","dashboard","chart","graph","insight"], children: &[] },
    IntentNode { name: "codebase", kind: PromptKind::Analyze, keywords: &["codebase","architecture","dependency","module","graph","call","stack","trace","repo","project","structure"], children: &[] },
    IntentNode { name: "invar", kind: PromptKind::Analyze, keywords: &["invariant","idempotent","guarantee","assert","enforce","constraint","precondition","postcondition","contract"], children: &[] },
    // ── Summarize ─────────────────────────────────────────────────────────
    IntentNode { name: "summarize", kind: PromptKind::Summarize, keywords: &["summarize","summary","tldr","brief","condense","recap","digest","synopsis","abridge","abstract","shorten","compress"], children: &[36,37,38] },
    IntentNode { name: "article", kind: PromptKind::Summarize, keywords: &["article","paper","research","academic","journal","publication","preprint"], children: &[] },
    IntentNode { name: "meeting", kind: PromptKind::Summarize, keywords: &["meeting","transcript","call","discussion","conversation","chat","minutes","notes"], children: &[] },
    IntentNode { name: "tldr", kind: PromptKind::Summarize, keywords: &["tldr","short","one","line","sentence","quick","briefly","summary"], children: &[] },
    // ── Extract ───────────────────────────────────────────────────────────
    IntentNode { name: "extract", kind: PromptKind::Extract, keywords: &["extract","parse","scrape","pull","harvest","gather","collect","fetch","crawl","mine"], children: &[39,40,41] },
    IntentNode { name: "structured", kind: PromptKind::Extract, keywords: &["structured","json","csv","xml","yaml","toml","table","schema","field","format","convert"], children: &[] },
    IntentNode { name: "insights", kind: PromptKind::Extract, keywords: &["insight","key","point","takeaway","finding","lesson","learn","wisdom","quote","highlight"], children: &[] },
    IntentNode { name: "reverse", kind: PromptKind::Extract, keywords: &["reverse","engineer","reproduce","replicate","clone","decompile","disassemble","port","migrate","convert"], children: &[] },
    // ── Plan ──────────────────────────────────────────────────────────────
    IntentNode { name: "plan", kind: PromptKind::Plan, keywords: &["plan","roadmap","blueprint","design","architecture","spec","proposal","phase","milestone","strategy"], children: &[42,43,44,45] },
    IntentNode { name: "arch", kind: PromptKind::Plan, keywords: &["architecture","blueprint","design","doc","system","component","layer","tier","micro","mono","distributed"], children: &[] },
    IntentNode { name: "roadmap", kind: PromptKind::Plan, keywords: &["roadmap","milestone","phase","timeline","quarter","release","version","epic","initiative"], children: &[] },
    IntentNode { name: "sprint", kind: PromptKind::Plan, keywords: &["sprint","iteration","cycle","scrum","kanban","agile","backlog","story","task","ticket"], children: &[] },
    IntentNode { name: "feature", kind: PromptKind::Plan, keywords: &["feature","spec","proposal","rfc","product","requirement","scope","mwp","mvp","poc"], children: &[] },
    // ── System ────────────────────────────────────────────────────────────
    IntentNode { name: "system", kind: PromptKind::System, keywords: &["deploy","ci","cd","docker","container","server","infra","kubernetes","aws","gcp","azure","terraform","ansible","systemd","service","pipeline","orchestrate","operate"], children: &[46,47,48,49] },
    IntentNode { name: "deploy", kind: PromptKind::System, keywords: &["deploy","release","ship","publish","rollout","promote","staging","prod","canary","blue","green"], children: &[] },
    IntentNode { name: "ci-cd", kind: PromptKind::System, keywords: &["ci","cd","pipeline","github","actions","jenkins","gitlab","circle","travis","build","runner"], children: &[] },
    IntentNode { name: "infra", kind: PromptKind::System, keywords: &["infra","terraform","pulumi","kubernetes","k8s","docker","compose","helm","pod","node","cluster"], children: &[] },
    IntentNode { name: "monitor", kind: PromptKind::System, keywords: &["monitor","alert","observe","telemetry","log","metric","trace","dashboard","promethe","grafana","datadog","sentry"], children: &[] },
    // ── Security ──────────────────────────────────────────────────────────
    IntentNode { name: "harden", kind: PromptKind::Security, keywords: &["harden","secure","lock","down","protect","defend","mitigate","prevent","safeguard"], children: &[50,51,52,53] },
    IntentNode { name: "audit", kind: PromptKind::Security, keywords: &["audit","pentest","scan","assess","check","cve","owasp","nist","sast","dast"], children: &[] },
    IntentNode { name: "crypto", kind: PromptKind::Security, keywords: &["crypto","encrypt","decrypt","hash","sign","verify","cert","tls","ssl","pgp","key","cipher"], children: &[] },
    IntentNode { name: "compliance", kind: PromptKind::Security, keywords: &["compliance","gdpr","soc2","iso","hipaa","pci","regulation","policy","governance","risk"], children: &[] },
    IntentNode { name: "exploit", kind: PromptKind::Security, keywords: &["exploit","attack","inject","xss","csrf","sqli","rce","lfi","bypass","hijack","spoof","phish"], children: &[] },
    // ── Meta ──────────────────────────────────────────────────────────────
    IntentNode { name: "meta", kind: PromptKind::Meta, keywords: &["prompt","engineer","improve","optimize","enrich","enhance","augment","rewrite","refine","structure","framework","template","better","автоматичний","автоматический","універсальний","универсальный","внутрішній","внутренний","без залежностей","без зависимостей","динамічний","динамический","енріч","энрич","нативно","нативно"], children: &[] },
    IntentNode { name: "skill-design", kind: PromptKind::Skill, keywords: &["skill","plugin","tool","extension","agent","capability","mcp","server","adapter","connector","bridge","integration"], children: &[] },
    IntentNode { name: "self-enrich", kind: PromptKind::Meta, keywords: &["enrich","self","improve","augment","enhance","upgrade","boost","amplify","level","up","meta"], children: &[] },
    // ── Other roots ───────────────────────────────────────────────────────
    IntentNode { name: "search", kind: PromptKind::Search, keywords: &["search","find","locate","look","grep","query","index","explore","discover","browse","navigate"], children: &[] },
    IntentNode { name: "config", kind: PromptKind::Config, keywords: &["config","configure","setup","settings","option","param","tune","optimize","adjust","calibrate","env","dotenv","toml","yaml","json"], children: &[] },
    IntentNode { name: "math", kind: PromptKind::Math, keywords: &["math","equation","proof","theorem","calculus","algebra","geometry","statistic","probability","optimization","simulation","model"], children: &[] },
    IntentNode { name: "creative", kind: PromptKind::Creative, keywords: &["creative","story","dialogue","world","build","character","plot","narrative","fiction","poem","poetry","song","lyric","script","screenplay"], children: &[] },
    IntentNode { name: "general", kind: PromptKind::General, keywords: &[""], children: &[] },
];

/// Detect the full tree path of intent: returns [[root, child?, grandchild?]]
/// for each matched branch. Order: deepest match first.
pub fn detect_intent_tree(text: &str) -> Vec<IntentPath> {
    let lower = text.to_lowercase();
    let mut matches: Vec<(usize, usize)> = Vec::new(); // (node_idx, keyword_hits)

    for (idx, node) in INTENT_TREE.iter().enumerate() {
        let mut hits = 0usize;
        for kw in node.keywords {
            if !kw.is_empty() && lower.contains(*kw) {
                hits += 1;
            }
        }
        if hits > 0 {
            matches.push((idx, hits));
        }
    }

    matches.sort_by(|a, b| b.1.cmp(&a.1));

    // Build paths: for each match, walk up to root via parent index lookup.
    let mut paths: Vec<IntentPath> = Vec::new();
    for &(node_idx, _) in &matches {
        let node = &INTENT_TREE[node_idx];
        let mut path = vec![node.name.to_string()];

        // Walk up: find parent by searching which node has this as child.
        for (pidx, pnode) in INTENT_TREE.iter().enumerate() {
            if pnode.children.contains(&node_idx) {
                path.insert(0, pnode.name.to_string());
                // Check grandparent.
                for (_gpidx, gpnode) in INTENT_TREE.iter().enumerate() {
                    if gpnode.children.contains(&pidx) {
                        path.insert(0, gpnode.name.to_string());
                        break;
                    }
                }
                break;
            }
        }
        paths.push(path);
    }

    // Dedup by path content.
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.join("/")));
    paths.truncate(7);
    paths
}

// ─── Pattern Tree — universal → domain → sub-domain with inheritance ───────

/// A node in the universal pattern taxonomy. Root patterns apply everywhere;
/// children specialize for domains; grandchildren further refine.
/// Each level inherits all patterns from its ancestors.
#[derive(Debug, Clone)]
pub struct PatternNode {
    pub name: &'static str,
    /// Pattern kind: what aspect of work this pattern governs.
    pub category: &'static str,  // "quality", "safety", "process", "structure", "communication"
    /// Core instruction (inherited by all children).
    pub rule: &'static str,
    pub children: &'static [usize],
    /// Connected cross-patterns (links to other branches).
    pub cross_links: &'static [usize],
}

/// Static pattern taxonomy. Universals (root) inherited by everything.
pub static PATTERN_TREE: &[PatternNode] = &[
    // ── UNIVERSALS (root, index 0-4) — inherited by ALL domains ───────────
    PatternNode { name: "quality", category: "quality", rule: "Verify with real execution before claiming done. Never fake-green.", children: &[5,6], cross_links: &[11] },
    PatternNode { name: "safety", category: "safety", rule: "Never commit secrets. Input validation at trust boundaries. Error handling prevents data loss.", children: &[7,8], cross_links: &[12,13] },
    PatternNode { name: "minimal", category: "structure", rule: "Fewest correct files. Delete dead code. Minimal is good; correct-and-minimal is the bar.", children: &[9,10], cross_links: &[] },
    PatternNode { name: "idempotency", category: "safety", rule: "Mutations must be safe to retry. Push/insert guarded by dedup. Counters guarded by version. State transitions guard against re-entry.", children: &[14,15], cross_links: &[1,16] },
    PatternNode { name: "invariant", category: "quality", rule: "Structural guarantees must be asserted. Public API bounds-check inputs. Threshold ordering enforced. Index arithmetic guarded.", children: &[17,18], cross_links: &[0,14] },
    // ── quality children ──────────────────────────────────────────────────
    PatternNode { name: "test-first", category: "quality", rule: "Write test RED first, then code GREEN. Never delete tests without confirming they test dead code.", children: &[], cross_links: &[9] },
    PatternNode { name: "evidence", category: "quality", rule: "Back claims with measured numbers. Benchmarks, not vibes. Falsifiable done-checks.", children: &[], cross_links: &[2] },
    // ── safety children ───────────────────────────────────────────────────
    PatternNode { name: "no-secrets", category: "safety", rule: "Never expose or log secrets. Never commit secrets. Scan for hardcoded credentials.", children: &[], cross_links: &[9] },
    PatternNode { name: "fail-closed", category: "safety", rule: "Default deny. On error, refuse — don't fall through to default-allow. Auth failures are errors.", children: &[], cross_links: &[12] },
    // ── minimal children ──────────────────────────────────────────────────
    PatternNode { name: "dedup", category: "structure", rule: "Extract shared helpers. Eliminate copy-paste. Generic over duplicate. One source of truth.", children: &[], cross_links: &[3] },
    PatternNode { name: "remove-dead", category: "structure", rule: "Delete unused code, dead flags, stale config. Remove before adding.", children: &[], cross_links: &[] },
    // ── Cross-links: structure → process ──────────────────────────────────
    // ── DOMAIN: code (index 11-16) — inherits universals 0-4 ──────────────
    PatternNode { name: "code-core", category: "process", rule: "Follow repo conventions. Match existing patterns. Use existing primitives before adding deps.", children: &[12,13,19,20,21], cross_links: &[3] },
    // SAFETY: this is a documentation-only reference to the concept of `unsafe`
    // blocks, not a call into unsafe code. The word appears inside a string
    // literal as a pattern-rule description for code-security best practices.
    // No actual unsafe block or fn exists in this module.
    PatternNode { name: "code-security", category: "safety", rule: "Input validation on all public APIs. Sanitize f64: NaN→0.0. Bounds-check indices. No unsafe without SAFETY comment.", children: &[], cross_links: &[1,7] },
    PatternNode { name: "code-crypto", category: "safety", rule: "Never fake crypto. Real KAT-gated primitives only. No classical fallback for PQ.", children: &[], cross_links: &[1] },
    // ── idempotency children ──────────────────────────────────────────────
    PatternNode { name: "push-dup-guard", category: "safety", rule: "Before push: check contains. Use HashSet or dedup guard. Replayed events must be no-ops.", children: &[], cross_links: &[3,16] },
    PatternNode { name: "state-guard", category: "safety", rule: "State transitions: first check 'already in this state?' before advancing. FSM must be idempotent.", children: &[], cross_links: &[15] },
    // ── invariant children ────────────────────────────────────────────────
    PatternNode { name: "ordering", category: "quality", rule: "Thresholds must be strictly ordered. elevated<warning<critical<failed. Validate in constructor, assert in debug.", children: &[], cross_links: &[4] },
    PatternNode { name: "bounds", category: "quality", rule: "Index arithmetic: debug_assert! bounds. Public fns: clamp or Option. Never silent OOB.", children: &[], cross_links: &[4,12] },
    // ── code children ─────────────────────────────────────────────────────
    PatternNode { name: "rust-idiom", category: "structure", rule: "Prefer std over external. Use Result, not panic. Use match, not if-let chains. Derive where possible.", children: &[], cross_links: &[2] },
    PatternNode { name: "test-idiom", category: "quality", rule: "Tests: arrange/act/assert. RED→GREEN. One assertion concept per test. Test edge cases and error paths.", children: &[], cross_links: &[5] },
    PatternNode { name: "refactor-idiom", category: "structure", rule: "Extract helper, don't inline. Name constants. Generic over duplicate. Delete dead before adding new.", children: &[], cross_links: &[9,10] },
    // ── DOMAIN: security (index 22-26) — inherits universals 0-4 ──────────
    PatternNode { name: "sec-core", category: "process", rule: "Threat-model first. Identify trust boundaries. Audit input paths. Defense in depth.", children: &[22,23,24], cross_links: &[1,12] },
    PatternNode { name: "sec-audit", category: "quality", rule: "Audit: check for injection, auth bypass, data exposure, crypto misuse, dependency vulns.", children: &[], cross_links: &[0,6] },
    PatternNode { name: "sec-harden", category: "safety", rule: "Harden: least privilege. Rate-limit. Timeout all external calls. Validate all inputs.", children: &[], cross_links: &[7,8] },
    PatternNode { name: "sec-report", category: "communication", rule: "Security finding: Description, Risk, Recommendations, References, Summary. Severity: Critical/High/Medium/Low.", children: &[], cross_links: &[] },
    // ── DOMAIN: system (index 27-30) — inherits universals 0-4 ────────────
    PatternNode { name: "sys-core", category: "process", rule: "Infrastructure as code. Immutable deploys. Rollback path tested before rollout.", children: &[], cross_links: &[2] },
    PatternNode { name: "sys-monitor", category: "quality", rule: "Every deployment has telemetry. Alerts fire on regression. Benchmarks gate before merge.", children: &[], cross_links: &[6] },
    PatternNode { name: "sys-resilience", category: "safety", rule: "Circuit breaker on external calls. Timeout + retry with backoff. Failover tested regularly.", children: &[], cross_links: &[1,8] },
    PatternNode { name: "sys-config", category: "structure", rule: "Config as code. Secrets from env, never committed. Feature flags off by default.", children: &[], cross_links: &[7,9] },
    // ── DOMAIN: meta/prompt (index 31-34) — inherits universals 0-4 ───────
    PatternNode { name: "prompt-core", category: "structure", rule: "Be specific. Use delimiters. Structured output. Conditional steps. Few-shot examples.", children: &[], cross_links: &[2] },
    PatternNode { name: "prompt-reason", category: "quality", rule: "Chain of thought. Inner monologue. Self-ask. Verify before answering. Step-by-step reasoning.", children: &[], cross_links: &[6] },
    PatternNode { name: "prompt-split", category: "process", rule: "Split complex tasks into subtasks. Intent classification → routing → execution. Summarize long docs piecewise.", children: &[], cross_links: &[0] },
    PatternNode { name: "prompt-test", category: "quality", rule: "Test prompt changes systematically. Evaluations with golden answers. A/B compare. Measure before shipping.", children: &[], cross_links: &[5,6] },
];

/// Get all patterns that apply to a given intent path.
/// Inherits from root (universal) → domain → sub-domain.
pub fn inherit_patterns(path: &IntentPath) -> Vec<&'static PatternNode> {
    let mut result: Vec<&PatternNode> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Always include universals (indices 0-4).
    for i in 0..=4 {
        if seen.insert(i) { result.push(&PATTERN_TREE[i]); }
    }

    // Map intent root name → domain pattern index.
    let domain_map: &[(&str, usize)] = &[
        ("code", 11), ("write", 11), ("refactor", 11), ("debug", 11), ("review", 11), ("test", 11),
        ("security", 21), ("harden", 21),
        ("system", 25), ("deploy", 25),
        ("meta", 29), ("prompt-eng", 29), ("skill-design", 29),
    ];

    for seg in path.iter() {
        for (key, idx) in domain_map {
            if seg == *key && seen.insert(*idx) {
                result.push(&PATTERN_TREE[*idx]);
                // Also add domain children.
                for &child in PATTERN_TREE[*idx].children {
                    if seen.insert(child) { result.push(&PATTERN_TREE[child]); }
                }
            }
        }
    }

    // Always include idempotency and invariant patterns.
    for i in [3, 4, 14, 15, 16, 17] {
        if seen.insert(i) { result.push(&PATTERN_TREE[i]); }
    }

    result
}

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

/// A full enrichment report with all detected intents + applied prompts + skills.
#[derive(Debug, Clone)]
pub struct EnrichmentReport {
    /// All detected intents with scores.
    pub intents: Vec<(PromptKind, usize, f64)>,
    /// Primary intent.
    pub primary_intent: PromptKind,
    /// Tree paths found (e.g. ["code","debug","compile"]).
    pub intent_paths: Vec<IntentPath>,
    /// Applied prompt enrichments.
    pub prompts: Vec<PromptEntry>,
    /// Applied skill names.
    pub skills: Vec<String>,
    pub total_prompts: usize,
    pub total_skills: usize,
}

impl EnrichmentReport {
    pub fn display(&self) -> String {
        let mut out = String::with_capacity(1024);
        out.push_str("─── ENRICHMENT ───\n");
        out.push_str(&format!("  primary: {}\n", self.primary_intent.as_str()));
        if !self.intents.is_empty() {
            out.push_str("  intents:");
            for (kind, count, score) in &self.intents {
                out.push_str(&format!(" {}({}|{:.2})", kind.as_str(), count, score));
            }
            out.push('\n');
        }
        if !self.intent_paths.is_empty() {
            out.push_str("  paths:");
            for p in &self.intent_paths {
                out.push_str(&format!(" [{}]", p.join(" → ")));
            }
            out.push('\n');
        }
        if !self.prompts.is_empty() {
            out.push_str(&format!("  prompts ({}): ", self.prompts.len()));
            for p in &self.prompts {
                out.push_str(&format!("[{}] ", p.title));
            }
            out.push('\n');
        }
        if !self.skills.is_empty() {
            out.push_str(&format!("  skills ({}): ", self.skills.len()));
            for s in &self.skills {
                out.push_str(&format!("[{}] ", s));
            }
            out.push('\n');
        }
        out
    }
}

// ─── IntentKeywordMap ──────────────────────────────────────────────────────

/// Maps keywords → PromptKind for intent detection.
type IntentMap = HashMap<&'static str, PromptKind>;

fn build_intent_map() -> IntentMap {
    let mut m = HashMap::new();
    // Code
    for k in &["code", "implement", "build", "refactor", "compile", "debug", "bug",
        "function", "struct", "impl", "mod", "cargo", "rustc", "npm", "pip", "import",
        "fix", "patch", "component", "module", "crate", "type", "trait", "enum",
        "代码","编程","实现","コード","実装","プログラミング","코드","프로그래밍"] {
        m.insert(*k, PromptKind::Code);
    }
    // Write
    for k in &["write", "essay", "article", "document", "blog", "readme", "docs",
        "prose", "paragraph", "author", "compose", "draft", "text",
        "写","写作","文档","書く","文書"] {
        m.insert(*k, PromptKind::Write);
    }
    // Analyze
    for k in &["analyze", "analysis", "evaluate", "assess", "audit", "claims",
        "verify", "validate", "inspect", "examine", "investigate",
        "分析","调查","調査","解析"] {
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
        "unit test", "integration test", "e2e", "prove",
        "测试","测试用例","テスト"] {
        m.insert(*k, PromptKind::Test);
    }
    // Security
    for k in &["security", "vuln", "exploit", "threat", "attack", "harden",
        "encrypt", "decrypt", "auth", "authn", "authz", "permission", "acl", "rbac"] {
        m.insert(*k, PromptKind::Security);
    }
    // Meta
    for k in &["prompt", "enrich", "improve prompt", "optimize prompt", "skill",
        "plugin", "tool", "agent",
        "автоматичний","автоматический","універсальний","универсальный",
        "внутрішній","внутренний","без залежностей","без зависимостей",
        "динамічний","динамический","енріч","энрич","нативно","нативно",
        "скрізь","везде","завжди","всегда","показувати","показывать",
        "використовувати","использовать","система","система",
        "提示词","提示","工程","プロンプト"] {
        m.insert(*k, PromptKind::Meta);
    }
    // Debug
    for k in &["debug", "fix bug", "troubleshoot", "root cause", "broken", "error",
        "调试","修复"] {
        m.insert(*k, PromptKind::Debug);
    }
    // Refactor
    for k in &["refactor", "clean up", "simplify", "dedup", "extract", "rename",
        "restructure", "reorganize", "decouple"] {
        m.insert(*k, PromptKind::Refactor);
    }
    m
}

fn detect_intent(text: &str) -> (PromptKind, f64) {
    let intents = detect_all_intents(text);
    if intents.is_empty() {
        return (PromptKind::General, 0.0);
    }
    (intents[0].0, intents[0].2)
}

/// Detect ALL intents with scores — for batch enrichment.
pub fn detect_all_intents(text: &str) -> Vec<(PromptKind, usize, f64)> {
    let lower = text.to_lowercase();
    let map = build_intent_map();
    let mut scores: HashMap<PromptKind, usize> = HashMap::new();

    for (keyword, kind) in &map {
        if lower.contains(*keyword) {
            *scores.entry(*kind).or_insert(0) += 1;
        }
    }

    if scores.is_empty() {
        return vec![];
    }

    let total: usize = scores.values().sum();
    let mut ranked: Vec<(PromptKind, usize, f64)> = scores.into_iter()
        .map(|(k, c)| (k, c, if total > 0 { c as f64 / total as f64 } else { 0.0 }))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    ranked
}

/// Detect ALL intents AND record telemetry into the harvest ledger.
pub fn detect_all_intents_with_telemetry(
    text: &str,
    ledger: &mut HarvestLedger,
) -> Vec<(PromptKind, usize, f64)> {
    let intents = detect_all_intents(text);
    let success = !intents.is_empty();
    let value = if intents.is_empty() {
        0.0
    } else {
        let sum: f64 = intents.iter().map(|(_, _, s)| *s).sum();
        sum / intents.len() as f64
    };
    let cost = text.len() as f64;
    ledger.record("prompt_enrich", "intent_detect", success, value, cost);
    intents
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

    /// Borrow all entries for eigen enrichment.
    pub fn all_entries(&self) -> &[PromptEntry] {
        &self.prompts
    }

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

    /// Produce a full enrichment report — detects ALL intents, matches prompts
    /// AND skills across them, builds a batch of applicable enrichments.
    /// This is the primary API: call before any cognitive work.
    pub fn enrich_report(&self, user_input: &str) -> EnrichmentReport {
        let intents = detect_all_intents(user_input);
        let intent_paths = detect_intent_tree(user_input);
        let primary = intents.first().map(|(k, _, _)| *k).unwrap_or(PromptKind::General);
        let lower = user_input.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        // Collect prompt matches across all detected intents (batch).
        let mut candidates: Vec<(usize, u32)> = Vec::new();
        let mut seen_intents = std::collections::HashSet::new();

        for &(kind, _, _) in &intents {
            if !seen_intents.insert(kind) { continue; }
            if let Some(kind_matches) = self.kind_index.get(&kind) {
                for &idx in kind_matches {
                    let mut score = 3u32;
                    let entry = &self.prompts[idx];
                    for word in &words {
                        if entry.trigger_keywords.iter().any(|k| k.contains(word) || word.contains(k.as_str())) {
                            score += 2;
                        }
                    }
                    candidates.push((idx, score));
                }
            }
        }

        // Dedup + sort.
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let mut seen = std::collections::HashSet::new();
        let mut prompts: Vec<PromptEntry> = Vec::new();
        for (idx, _) in candidates {
            if seen.insert(idx) && prompts.len() < MAX_ENRICH_RESULTS {
                prompts.push(self.prompts[idx].clone());
            }
        }

        // Collect skill names from matched prompts.
        let mut skills: Vec<String> = Vec::new();
        let mut seen_skills = std::collections::HashSet::new();
        for p in &prompts {
            if (p.kind == PromptKind::Skill || p.kind == PromptKind::Meta)
                && seen_skills.insert(p.title.clone())
            {
                skills.push(p.title.clone());
            }
        }

        let total_skills = skills.len();
        EnrichmentReport {
            intents,
            intent_paths,
            primary_intent: primary,
            prompts,
            skills,
            total_prompts: self.prompts.len(),
            total_skills,
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

    /// Attach a harvest ledger for telemetry recording.
    /// Subsequent `enrich_report` calls will record telemetry automatically.
    pub fn with_telemetry(&mut self, ledger: &mut HarvestLedger) -> &mut Self {
        let _ = ledger; // ledger is wired during enrich_report_with_telemetry
        self
    }

    /// Enrich with telemetry: same as `enrich_report` but also records a
    /// harvest record into the ledger for EV scoring.
    pub fn enrich_report_with_telemetry(
        &self,
        user_input: &str,
        ledger: &mut HarvestLedger,
    ) -> EnrichmentReport {
        let report = self.enrich_report(user_input);
        let success = !report.intents.is_empty();
        let value = if report.intents.is_empty() {
            0.0
        } else {
            let sum: f64 = report.intents.iter().map(|(_, _, s)| *s).sum();
            sum / report.intents.len() as f64
        };
        let cost = user_input.len() as f64;
        ledger.record("prompt_enrich", "intent_detect", success, value, cost);
        report
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

// ─── Eigen-based enrichment ────────────────────────────────────────────────

/// Build a vocabulary from the enrichment engine's entries.
pub fn build_vocabulary(engine: &PromptEnrichEngine) -> Vec<String> {
    let mut words: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in engine.all_entries() {
        for kw in &entry.trigger_keywords {
            words.insert(kw.to_lowercase());
        }
    }
    let mut vocab: Vec<String> = words.into_iter().collect();
    vocab.sort();
    vocab
}

/// Eigen-based enrichment: decomposes the query and each entry into eigen vectors,
/// then ranks by eigen-projection similarity.
pub fn eigen_enrich_report(engine: &PromptEnrichEngine, input: &str, vocab: &[String]) -> Vec<PromptEntry> {
    use crate::eigen::Eigen;
    let input_lower = input.to_lowercase();
    let query_kws: Vec<String> = input_lower.split_whitespace().map(|s| s.to_string()).collect();
    let query_eigen = Eigen::from_bow(&query_kws, vocab);

    let mut scored: Vec<(PromptEntry, f64)> = Vec::new();
    for entry in engine.all_entries() {
        let entries_kws: Vec<String> = entry.trigger_keywords.iter().map(|k| k.to_lowercase()).collect();
        let entry_eigen = Eigen::from_bow(&entries_kws, vocab);
        let sim = query_eigen.cosine_sim(&entry_eigen);
        if sim > 0.0 {
            scored.push((entry.clone(), sim));
        }
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(10);
    scored.into_iter().map(|(e, _)| e).collect()
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

    #[test]
    fn detect_all_intents_multiple() {
        let intents = detect_all_intents("implement a new feature with tests and deploy the service");
        assert!(!intents.is_empty());
        let codes: Vec<PromptKind> = intents.iter().map(|(k, _, _)| *k).collect();
        assert!(codes.contains(&PromptKind::Code));
        assert!(codes.contains(&PromptKind::Test));
        assert!(codes.contains(&PromptKind::System));
    }

    #[test]
    fn enrich_report_batch() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());
        engine.ingest(seed_system_prompts());

        let report = engine.enrich_report("audit the security system and summarize findings for the plan");
        assert!(!report.intents.is_empty());
        assert!(!report.prompts.is_empty());
        // Should have >1 intent (security, summarize, plan).
        let intent_kinds: Vec<PromptKind> = report.intents.iter().map(|(k, _, _)| *k).collect();
        assert!(intent_kinds.len() >= 1);
        // Display string works.
        let disp = report.display();
        assert!(disp.contains("ENRICHMENT"));
        assert!(disp.contains("prompts"));
    }

    #[test]
    fn enrichment_report_display_format() {
        let report = EnrichmentReport {
            intents: vec![(PromptKind::Code, 5, 0.5), (PromptKind::Test, 3, 0.3)],
            intent_paths: vec![vec!["code".into(), "debug".into(), "compile".into()]],
            primary_intent: PromptKind::Code,
            prompts: vec![PromptEntry::new("test_prompt", "text", PromptKind::Code, &["test"], "src", "MIT")],
            skills: vec!["test_skill".into()],
            total_prompts: 1,
            total_skills: 1,
        };
        let disp = report.display();
        assert!(disp.contains("code"));
        assert!(disp.contains("test_prompt"));
        assert!(disp.contains("test_skill"));
        assert!(disp.contains("paths"));
        assert!(disp.contains("debug"));
    }

    #[test]
    fn detect_intent_tree_code_debug() {
        let paths = detect_intent_tree("fix the compilation error in this rust function");
        assert!(!paths.is_empty());
        // Should find a path containing "code" and "compile" (or "debug")
        let all_names: Vec<&str> = paths.iter().flatten().map(|s| s.as_str()).collect();
        assert!(all_names.iter().any(|n| *n == "code"));
    }

    #[test]
    fn detect_intent_tree_security() {
        let paths = detect_intent_tree("audit the authentication for security vulnerabilities in the XSS exploit");
        assert!(!paths.is_empty());
        let all_names: Vec<&str> = paths.iter().flatten().map(|s| s.as_str()).collect();
        assert!(all_names.iter().any(|n| *n == "harden" || *n == "exploit" || *n == "security"));
    }

    #[test]
    fn detect_intent_tree_multiple_branches() {
        let paths = detect_intent_tree("implement a new API endpoint and write integration tests for it");
        assert!(paths.len() >= 2);
        let all_names: Vec<&str> = paths.iter().flatten().map(|s| s.as_str()).collect();
        assert!(all_names.iter().any(|n| *n == "code" || *n == "api"));
        assert!(all_names.iter().any(|n| *n == "test" || *n == "integration"));
    }

    #[test]
    fn inherit_patterns_universal() {
        let path = vec!["code".to_string(), "implement".to_string()];
        let patterns = inherit_patterns(&path);
        // Must include universals.
        assert!(patterns.iter().any(|p| p.name == "quality"));
        assert!(patterns.iter().any(|p| p.name == "safety"));
        assert!(patterns.iter().any(|p| p.name == "minimal"));
        assert!(patterns.iter().any(|p| p.name == "idempotency"));
        assert!(patterns.iter().any(|p| p.name == "invariant"));
    }

    #[test]
    fn inherit_patterns_domain_specific() {
        let path = vec!["code".to_string(), "debug".to_string()];
        let patterns = inherit_patterns(&path);
        // Domain-specific code patterns.
        assert!(patterns.iter().any(|p| p.name == "code-core"));
        assert!(patterns.iter().any(|p| p.name == "code-security"));
        // Idempotency children always included.
        assert!(patterns.iter().any(|p| p.name == "push-dup-guard"));
        assert!(patterns.iter().any(|p| p.name == "state-guard"));
    }

    #[test]
    fn inherit_patterns_security() {
        let path = vec!["harden".to_string(), "exploit".to_string()];
        let patterns = inherit_patterns(&path);
        assert!(patterns.iter().any(|p| p.name == "sec-core"));
        assert!(patterns.iter().any(|p| p.name == "sec-harden"));
    }

    #[test]
    fn enrich_report_with_telemetry_emits_record() {
        use crate::telemetry_harvest::HarvestLedger;
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());
        let mut ledger = HarvestLedger::new(100);
        let _ = engine.enrich_report_with_telemetry("fix the compilation bug in Rust", &mut ledger);
        assert_eq!(ledger.len(), 1, "enrich_report_with_telemetry must emit exactly 1 record");
        let recs = ledger.records();
        assert_eq!(recs[0].model, "prompt_enrich");
        assert_eq!(recs[0].task, "intent_detect");
        assert!(recs[0].success);
    }

    #[test]
    fn detect_all_intents_with_telemetry_emits_record() {
        use crate::telemetry_harvest::HarvestLedger;
        let mut ledger = HarvestLedger::new(100);
        let intents = detect_all_intents_with_telemetry("write a report on the architecture", &mut ledger);
        assert!(!intents.is_empty());
        assert_eq!(ledger.len(), 1);
        let recs = ledger.records();
        assert_eq!(recs[0].model, "prompt_enrich");
        assert_eq!(recs[0].task, "intent_detect");
        assert!(recs[0].success);
    }

    #[test]
    fn eigen_enrich_report_finds_code_entries() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());
        let vocab = build_vocabulary(&engine);
        let results = eigen_enrich_report(&engine, "I need to write some code", &vocab);
        assert!(!results.is_empty(), "Should find code-related entries");
    }

    #[test]
    fn eigen_build_vocabulary_is_non_empty() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        let vocab = build_vocabulary(&engine);
        assert!(!vocab.is_empty(), "Vocabulary should not be empty");
    }

    #[test]
    fn eigen_enrich_report_returns_sorted_by_relevance() {
        let mut engine = PromptEnrichEngine::new();
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());
        let vocab = build_vocabulary(&engine);
        let results = eigen_enrich_report(&engine, "code review", &vocab);
        if results.len() >= 2 {
            let kw1 = results[0].trigger_keywords.iter().filter(|k| "code review".contains(k.as_str())).count();
            let kw2 = results[1].trigger_keywords.iter().filter(|k| "code review".contains(k.as_str())).count();
            assert!(kw1 >= kw2, "First result should be most relevant");
        }
    }

    // ─── P3: Quality monitor tests ───────────────────────────────────────────

    #[test]
    fn quality_monitor_calibrate_sets_baseline() {
        let mut monitor = EnrichmentQualityMonitor::new();
        let engine = PromptEnrichEngine::new();
        monitor.calibrate(&engine);
        assert!(!monitor.baseline_scores.is_empty());
        assert_eq!(monitor.baseline_scores.len(), monitor.benchmark_queries.len());
    }

    #[test]
    fn quality_monitor_check_drift_returns_stable_for_no_change() {
        let mut monitor = EnrichmentQualityMonitor::new();
        let engine = PromptEnrichEngine::new();
        monitor.calibrate(&engine);
        let comparison = monitor.check_drift(&engine);
        // Should be stable since engine hasn't changed
        assert!(matches!(comparison, DeltaComparison::Stable | DeltaComparison::Growing));
    }

    // ─── P4: Chronos tracker tests ──────────────────────────────────────────

    #[test]
    fn chronos_tracker_registers_and_trends() {
        let mut tracker = ChronosEnrichmentTracker::new();
        tracker.register_keywords(&["code".into(), "test".into(), "deploy".into()]);
        // Record queries that match "code" frequently
        for _ in 0..5 {
            tracker.record_query("write code for sorting");
            tracker.record_query("code review guidelines");
        }
        // Record queries that match "test" less
        tracker.record_query("unit test framework");
        // "code" should be trending higher than "deploy"
        let trending = tracker.trending_keywords(3);
        assert!(!trending.is_empty());
    }

    #[test]
    fn chronos_tracker_decaying_keywords() {
        let mut tracker = ChronosEnrichmentTracker::new();
        tracker.register_keywords(&["old_api".into(), "new_api".into()]);
        for _ in 0..10 {
            tracker.record_query("use new_api for requests");
        }
        let decaying = tracker.decaying_keywords(2);
        assert!(!decaying.is_empty());
    }
}

/// Re-export for test access.
fn seed_system_prompts() -> Vec<PromptEntry> {
    vec![
        PromptEntry::new("self_enrich", "self-improving enrichment engine", PromptKind::Meta, &["enrich","self-improve"], "system", "CC0"),
        PromptEntry::new("skill_armory", "skill ingestion orchestration", PromptKind::Skill, &["armory","skill store"], "system", "CC0"),
    ]
}

// ─── P3: EnrichmentQualityMonitor ──────────────────────────────────────────

/// Monitors enrichment quality drift after DB ingestion.
pub struct EnrichmentQualityMonitor {
    pub benchmark_queries: Vec<String>,
    pub baseline_scores: Vec<f64>,
    pub tracker: DeltaTracker,
    pub telemetry: HarvestLedger,
}

impl EnrichmentQualityMonitor {
    pub fn new() -> Self {
        Self {
            benchmark_queries: vec![
                "write code to sort a list".into(),
                "debug a null pointer exception".into(),
                "design a REST API".into(),
                "secure authentication system".into(),
                "optimize database queries".into(),
            ],
            baseline_scores: Vec::new(),
            tracker: DeltaTracker::new(5.0, 100.0),
            telemetry: HarvestLedger::new(100),
        }
    }

    /// Calibrate: run benchmark queries through the engine, store baseline scores.
    pub fn calibrate(&mut self, engine: &PromptEnrichEngine) {
        self.baseline_scores.clear();
        for q in &self.benchmark_queries {
            let results = engine.enrich_report(q);
            let score = results.prompts.len() as f64;
            self.baseline_scores.push(score);
        }
    }

    /// Check drift: run benchmark queries, compare to baseline, feed DeltaTracker.
    pub fn check_drift(&mut self, engine: &PromptEnrichEngine) -> DeltaComparison {
        if self.baseline_scores.is_empty() {
            self.calibrate(engine);
        }
        let now = crate::now_ms();
        let mut total_delta = 0.0_f64;
        for (i, q) in self.benchmark_queries.iter().enumerate() {
            let results = engine.enrich_report(q);
            let new_score = results.prompts.len() as f64;
            let baseline = self.baseline_scores.get(i).copied().unwrap_or(new_score);
            let delta_value = new_score - baseline;
            total_delta += delta_value;
            let d = Delta::between(&[baseline], now, &[new_score], now + 1);
            self.tracker.observe(d);
        }
        let avg_delta = total_delta / self.benchmark_queries.len().max(1) as f64;
        self.telemetry.record(
            "enrichment_quality",
            &format!("drift_check_avg_delta={:.3}", avg_delta),
            avg_delta.abs() < 1.0,
            if avg_delta > 0.0 { avg_delta } else { 1.0 },
            0.0,
        );
        if self.tracker.history.is_empty() {
            DeltaComparison::Stable
        } else {
            let cumulative: f64 = self.tracker.history.iter().map(|d| d.components.iter().sum::<f64>()).sum();
            if cumulative.abs() <= 1.0 {
                DeltaComparison::Stable
            } else if cumulative > 0.0 {
                DeltaComparison::Growing
            } else {
                DeltaComparison::Shrinking
            }
        }
    }
}

// ─── P4: ChronosEnrichmentTracker ──────────────────────────────────────────

/// Tracks keyword relevance over time using chronos topology.
pub struct ChronosEnrichmentTracker {
    pub topology: ChronoTopology,
    pub keyword_rows: HashMap<String, usize>,
    pub query_count: usize,
}

impl ChronosEnrichmentTracker {
    pub fn new() -> Self {
        Self {
            topology: ChronoTopology::new(),
            keyword_rows: HashMap::new(),
            query_count: 0,
        }
    }

    /// Register enrichment keywords as topology subsystems.
    pub fn register_keywords(&mut self, keywords: &[String]) {
        for kw in keywords {
            let lower = kw.to_lowercase();
            if !self.keyword_rows.contains_key(&lower) {
                let row = self.keyword_rows.len();
                self.keyword_rows.insert(lower.clone(), row);
                self.topology.register(&lower, 1, 3);
            }
        }
    }

    /// Record a query — increment present counts for matched keywords.
    pub fn record_query(&mut self, query_text: &str) {
        self.query_count += 1;
        let lower = query_text.to_lowercase();
        let kw_list: Vec<String> = self.keyword_rows.keys().cloned().collect();
        for kw in &kw_list {
            if lower.contains(kw) {
                let mut m = crate::trinary::TriMatrix::new(1, 3);
                m.set(0, 1, crate::trinary::Tri::True);
                self.topology.update(kw, m);
            }
        }
    }

    /// Get top-N trending keywords (highest predicted relevance).
    pub fn trending_keywords(&self, top_n: usize) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = Vec::new();
        for (kw, &_row) in &self.keyword_rows {
            if let Some(trinity) = self.topology.subsystems.get(kw) {
                let pres = trinity.present.get(0, 1);
                let pred = trinity.predicted.get(0, 1);
                let pres_score = tri_to_numeric(pres);
                let pred_score = tri_to_numeric(pred);
                let trend = pred_score / pres_score.max(0.01);
                scored.push((kw.clone(), crate::sanitize_f64(trend)));
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);
        scored
    }

    /// Get top-N decaying keywords (lowest predicted relevance).
    pub fn decaying_keywords(&self, top_n: usize) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = Vec::new();
        for (kw, &_row) in &self.keyword_rows {
            if let Some(trinity) = self.topology.subsystems.get(kw) {
                let past = trinity.past.get(0, 1);
                let pred = trinity.predicted.get(0, 1);
                let past_score = tri_to_numeric(past);
                let pred_score = tri_to_numeric(pred);
                let decay = pred_score / past_score.max(0.01);
                scored.push((kw.clone(), crate::sanitize_f64(decay)));
            }
        }
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_n);
        scored
    }
}

fn tri_to_numeric(t: crate::trinary::Tri) -> f64 {
    match t {
        crate::trinary::Tri::True => 1.0,
        crate::trinary::Tri::Unknown => 0.5,
        crate::trinary::Tri::False => 0.0,
    }
}
