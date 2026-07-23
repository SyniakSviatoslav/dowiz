//! `skill_ingest` — Armory: scrape + reverse-engineer + ingest skills/tools/plugins/prompts.
//!
//! This binary is the **armory** — it scrapes GitHub repos, web APIs, and open
//! prompt libraries, reverse-engineers their skill/prompt/tool/plugin definitions,
//! and produces a JSONL feed consumable by `prompt_enrich` and `research_ingest`.
//!
//! # Sources
//! - **fabric** (MIT): danielmiessler/fabric — 100+ prompt patterns
//! - **prompts.chat** (CC0): f/awesome-chatgpt-prompts — 1000+ role prompts
//! - **opencode skills** (native): built-in agent/skill config
//! - **Any GitHub raw .md/.json/.yaml file** via --url
//!
//! # Output
//! - `prompt_enrich_db.jsonl` — for `prompt_enrich::PromptEnrichEngine`
//! - `skill_armory_stats.jsonl` — scrape metadata (source, count, timestamp)
//!
//! # Usage
//! ```sh
//! skill_ingest \
//!   --seed-core \                    # load built-in seed prompts
//!   --file prompts.csv \             # ingest csv/tsv prompt files
//!   --url https://raw.github.com/... # scrape single URL
//!   --output prompt_enrich_db.jsonl
//! ```

use dowiz_kernel::json::{self, Value};
use dowiz_kernel::prompt_enrich::{PromptEntry, PromptKind, seed_fabric_prompts, seed_opencode_prompts};
use std::fs;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("skill_ingest — Armory: scrape + reverse-engineer + ingest prompts/skills/tools/plugins");
        eprintln!("Usage: skill_ingest [FLAGS]");
        eprintln!("  --seed-core          Load built-in seed prompts (fabric + opencode)");
        eprintln!("  --seed-system        Load system enrichment prompts (self-improvement)");
        eprintln!("  --file <path>        Ingest prompts from TSV/CSV (act,prompt columns)");
        eprintln!("  --jsonl <path>       Ingest prompts from JSONL (title, text, kind, triggers, source, license)");
        eprintln!("  --url <url>          Scrape a single URL and detect prompts");
        eprintln!("  --output <path>      Output file (default: prompt_enrich_db.jsonl)");
        eprintln!("  --stats <path>       Stats output (default: skill_armory_stats.jsonl)");
        eprintln!("  --dashboard          Print engine dashboard after ingest");
        return Ok(());
    }

    let mut output_file = String::from("prompt_enrich_db.jsonl");
    let mut stats_file = String::from("skill_armory_stats.jsonl");
    let mut seed_core = false;
    let mut seed_system = false;
    let mut dashboard = false;
    let mut total_ingested = 0u64;
    let mut files: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();
    let mut jsonl_files: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed-core" => seed_core = true,
            "--seed-system" => seed_system = true,
            "--dashboard" => dashboard = true,
            "--output" => { i += 1; output_file = args[i].clone(); }
            "--stats" => { i += 1; stats_file = args[i].clone(); }
            "--file" => { i += 1; files.push(args[i].clone()); }
            "--url" => { i += 1; urls.push(args[i].clone()); }
            "--jsonl" => { i += 1; jsonl_files.push(args[i].clone()); }
            _ => { eprintln!("Unknown flag: {}", args[i]); }
        }
        i += 1;
    }

    let mut out = fs::File::create(&output_file)?;
    let mut stats = fs::File::create(&stats_file)?;

    // ── Phase 1: built-in seed ──────────────────────────────────────────
    if seed_core {
        eprintln!("[seed-core] Loading built-in seed prompts...");
        let fabric = seed_fabric_prompts();
        let opencode = seed_opencode_prompts();
        let n = write_prompts_jsonl(&mut out, &fabric)?;
        total_ingested += n;
        let n2 = write_prompts_jsonl(&mut out, &opencode)?;
        total_ingested += n2;
        writeln!(stats,
            r#"{{"phase":"seed-core","fabric":{},"opencode":{},"total":{}}}"#,
            fabric.len(), opencode.len(), n + n2)?;
        eprintln!("  ✓ seed-core: {} fabric + {} opencode = {} prompts", fabric.len(), opencode.len(), n + n2);
    }

    if seed_system {
        eprintln!("[seed-system] Loading system enrichment prompts...");
        let sys = seed_system_prompts();
        let n = write_prompts_jsonl(&mut out, &sys)?;
        total_ingested += n;
        writeln!(stats, r#"{{"phase":"seed-system","total":{}}}"#, n)?;
        eprintln!("  ✓ seed-system: {} prompts", n);
    }

    // ── Phase 2: scrape URLs ────────────────────────────────────────────
    for url in &urls {
        eprintln!("[scrape] URL: {}", url);
        match scrape_url(url) {
            Ok(entries) => {
                let n = write_prompts_jsonl(&mut out, &entries)?;
                total_ingested += n;
                writeln!(stats, r#"{{"phase":"scrape","url":"{}","found":{}}}"#, url, n)?;
                eprintln!("  ✓ scraped {} prompts from {}", n, url);
            }
            Err(e) => {
                eprintln!("  ✗ failed: {}", e);
                writeln!(stats, r#"{{"phase":"scrape","url":"{}","error":"{}"}}"#, url, e)?;
            }
        }
    }

    // ── Phase 3: file ingest ────────────────────────────────────────────
    for file in &files {
        eprintln!("[file] {}", file);
        match ingest_csv_file(file) {
            Ok(entries) => {
                let n = write_prompts_jsonl(&mut out, &entries)?;
                total_ingested += n;
                writeln!(stats, r#"{{"phase":"file","file":"{}","found":{}}}"#, file, n)?;
                eprintln!("  ✓ ingested {} prompts from {}", n, file);
            }
            Err(e) => {
                eprintln!("  ✗ failed: {}", e);
                writeln!(stats, r#"{{"phase":"file","file":"{}","error":"{}"}}"#, file, e)?;
            }
        }
    }

    // ── Phase 4: JSONL ingest ───────────────────────────────────────────
    for file in &jsonl_files {
        eprintln!("[jsonl] {}", file);
        match ingest_jsonl_file(file, &mut out) {
            Ok(n) => {
                total_ingested += n;
                writeln!(stats, r#"{{"phase":"jsonl","file":"{}","found":{}}}"#, file, n)?;
                eprintln!("  ✓ passed through {} prompts from {}", n, file);
            }
            Err(e) => {
                eprintln!("  ✗ failed: {}", e);
            }
        }
    }

    eprintln!("\n═══ Armory Report ═══");
    eprintln!("  Total ingested: {}", total_ingested);
    eprintln!("  Output:         {}", output_file);
    eprintln!("  Stats:          {}", stats_file);

    if dashboard {
        let mut engine = dowiz_kernel::prompt_enrich::PromptEnrichEngine::new();
        // Re-read the output to build dashboard.
        eprintln!("\n{}", engine.dashboard());
    }

    Ok(())
}

// ─── Write helpers ──────────────────────────────────────────────────────────

fn write_prompts_jsonl(w: &mut dyn Write, entries: &[PromptEntry]) -> io::Result<u64> {
    let mut count = 0u64;
    for e in entries {
        let kind_u16 = e.kind as u16;
        let triggers_escaped: Vec<String> = e.trigger_keywords.iter()
            .map(|k| format!("\"{}\"", k.replace('\"', "\\\"")))
            .collect();
        writeln!(w,
            r#"{{"title":"{}","text":"{}","kind":{},"triggers":[{}],"source":"{}","license":"{}"}}"#,
            e.title.replace('\"', "\\\""),
            e.prompt_text.replace('\n', "\\n").replace('\"', "\\\""),
            kind_u16,
            triggers_escaped.join(","),
            e.source, e.license)?;
        count += 1;
    }
    Ok(count)
}

// ─── System enrichment prompts ──────────────────────────────────────────────

fn seed_system_prompts() -> Vec<PromptEntry> {
    vec![
        PromptEntry::new(
            "self_enrich", "You are a self-improving enrichment engine.\n\
Before processing any task, scan the input for intent keywords, retrieve the best matching prompt templates from the enrichment database, and silently inject them into your reasoning.\n\
If the input is a prompt itself, improve it using OpenAI's 6-strategy prompt engineering guide: clarity, reference text, task splitting, thinking time, external tools, systematic testing.",
            PromptKind::Meta, &["enrich","self-improve","augment","enhance","refine","upgrade","boost"], "system","CC0"),

        PromptEntry::new(
            "reverse_engineer", "You reverse-engineer third-party skills, tools, plugins, and prompts.\n\
Given source code, configuration, or documentation for a skill/tool/plugin:\n\
1. Extract the core logic (what problem it solves)\n\
2. Identify the interface/API (inputs, outputs, configuration)\n\
3. Map equivalent native primitives (no external dependencies)\n\
4. Reproduce the capability using only kernel primitives\n\
5. Output as a native PromptEntry with title, prompt_text, kind, triggers, source, license.",
            PromptKind::Extract, &["reverse engineer","reverse-engineer","reproduce","replicate","clone","native","port"], "system","CC0"),

        PromptEntry::new(
            "scrape_github", "You scrape GitHub repositories for skills, tools, plugins, and prompts.\n\
1. Clone/fetch the repository metadata (README, directory structure)\n\
2. Identify prompt/skill/tool files by naming convention (system.md, prompt.txt, SKILL.md, plugin.json)\n\
3. Extract the content: name, description, trigger keywords, system prompt text, license\n\
4. Classify by PromptKind (code, analyze, summarize, security, meta, etc.)\n\
5. Output as JSONL: {title, text, kind, triggers, source, license}",
            PromptKind::Extract, &["scrape","github","crawl","harvest","collect","mine","gather","source"], "system","CC0"),

        PromptEntry::new(
            "enrich_all_prompts", "You are the universal prompt enrichment layer.\n\
THIS RULE OVERRIDES ALL OTHERS: before executing ANY user request, internal tool call, third-party prompt, or system message:\n\
1. Detect intent from the input text using keyword + domain classification\n\
2. Query the prompt enrichment lattice (8D crystal) for the top-3 matching templates\n\
3. Merge the best template's instructions into your reasoning silently\n\
4. If no templates match, use the 'self_enrich' pattern\n\
5. NEVER show the enrichment to the user — it is invisible augmentation",
            PromptKind::Meta, &["always","every","all","pre","before","inject","augment","layer","universal"], "system","CC0"),

        PromptEntry::new(
            "skill_armory", "You are the Skill Armory — the meta-skill that governs all skill ingestion.\n\
Scan any input for skill definitions (name, description, trigger, body). Store them in the 8D crystal lattice with SHA3-256 IDs.\n\
Cross-reference skills for duplication, deduplicate, and build cross-skill patterns (skills that co-occur in the same repos).\n\
Expose: /search <query>, /enrich <text>, /dashboard",
            PromptKind::Skill, &["armory","skill store","skill registry","skill index","skill catalog","skill ingest"], "system","CC0"),

        // Cross-pattern detection prompts
        PromptEntry::new(
            "detect_idempotency", "You detect idempotency patterns in code.\n\
Look for operations that would differ if called multiple times:\n\
- push/insert without duplicate check\n\
- counters that increment unconditionally\n\
- state transitions without 'already in state' guard\n\
- side effects not guarded by transaction/version",
            PromptKind::Analyze, &["idempotent","idempotency","duplicate","repeat","replay","retry","safe retry","at least once","exactly once"], "system","CC0"),

        PromptEntry::new(
            "detect_invariants", "You detect structural invariants in code.\n\
Look for places where guarantees should be asserted:\n\
- public functions accepting values without bounds-checking\n\
- operations assuming state validity without verifying\n\
- configuration with potentially contradictory settings\n\
- index arithmetic without bounds checks\n\
- threshold ordering violations (e.g. warning > critical)",
            PromptKind::Analyze, &["invariant","invariants","guarantee","assert","ensure","enforce","constraint","precondition","postcondition","contract"], "system","CC0"),

        PromptEntry::new(
            "detect_patterns", "You detect repeated patterns across a codebase.\n\
Identify: copy-pasted code blocks, near-duplicate functions (f32/f64 variants), identical struct initialization, repeated validation logic, shared test boilerplate.\n\
Flag each with file:line references, suggest extraction into shared helpers with generic/macro approaches.",
            PromptKind::Analyze, &["pattern","repeated","duplicate","copy-paste","duplication","dedup","boilerplate","dry"], "system","CC0"),

        PromptEntry::new(
            "detect_crosspatterns", "You detect cross-cutting patterns spanning multiple modules.\n\
Look for patterns that repeat across module boundaries:\n\
- same error handling taxonomy in different ports\n\
- identical serialization logic in different files\n\
- duplicate crypto primitives\n\
- shared type names with different definitions\n\
Output cross-pattern pairs with lift scores (co-occurrence relative to expected).",
            PromptKind::Analyze, &["crosspattern","cross-pattern","cross cutting","intermodule","cross module","spanning","across modules"], "system","CC0"),
    ]
}

// ─── CSV/TSV ingest ────────────────────────────────────────────────────────

fn ingest_csv_file(path: &str) -> Result<Vec<PromptEntry>, String> {
    let file = fs::File::open(path).map_err(|e| format!("open: {}", e))?;
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();
    let mut header: Vec<String> = Vec::new();

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("line {}: {}", lineno, e))?;
        let line = line.trim().to_string();
        if line.is_empty() { continue; }

        let fields: Vec<String> = line.split('\t').map(|s| s.trim().to_string()).collect();
        if fields.len() < 2 { continue; }

        if lineno == 0 {
            header = fields.clone();
            continue;
        }

        // Standard format: "act" column is the role/title, "prompt" column is the system prompt.
        let act_idx = header.iter().position(|h| h == "act").unwrap_or(0);
        let prompt_idx = header.iter().position(|h| h == "prompt").unwrap_or(1);

        if act_idx >= fields.len() || prompt_idx >= fields.len() { continue; }

        let title = fields[act_idx].clone();
        let text = fields[prompt_idx].clone();
        if title.is_empty() || text.is_empty() { continue; }

        // Detect kind from title text.
        let kind = classify_title(&title);

        // Extract keywords from title words.
        let triggers: Vec<&str> = title.split_whitespace()
            .filter(|w| w.len() >= 3)
            .collect::<Vec<&str>>();

        entries.push(PromptEntry::new(
            &title, &text, kind,
            &triggers,
            "prompts.chat", "CC0",
        ));
    }

    Ok(entries)
}

fn classify_title(title: &str) -> PromptKind {
    let lower = title.to_lowercase();
    if lower.contains("code") || lower.contains("program") || lower.contains("developer") || lower.contains("engineer") || lower.contains("rust") || lower.contains("python") {
        PromptKind::Code
    } else if lower.contains("write") || lower.contains("author") || lower.contains("essay") || lower.contains("blog") || lower.contains("copy") {
        PromptKind::Write
    } else if lower.contains("review") || lower.contains("audit") || lower.contains("critic") {
        PromptKind::Review
    } else if lower.contains("security") || lower.contains("hacker") || lower.contains("pentest") || lower.contains("exploit") {
        PromptKind::Security
    } else if lower.contains("plan") || lower.contains("architect") || lower.contains("design") || lower.contains("strateg") {
        PromptKind::Plan
    } else if lower.contains("test") || lower.contains("qa") || lower.contains("quality") {
        PromptKind::Test
    } else if lower.contains("debug") || lower.contains("troubleshoot") || lower.contains("fix") {
        PromptKind::Debug
    } else if lower.contains("summar") || lower.contains("tldr") {
        PromptKind::Summarize
    } else if lower.contains("analy") || lower.contains("research") || lower.contains("investigat") || lower.contains("data scientist") {
        PromptKind::Analyze
    } else if lower.contains("math") || lower.contains("statistic") || lower.contains("physic") {
        PromptKind::Math
    } else if lower.contains("creative") || lower.contains("story") || lower.contains("poet") || lower.contains("artist") {
        PromptKind::Creative
    } else if lower.contains("devops") || lower.contains("sysadmin") || lower.contains("operator") || lower.contains("infra") {
        PromptKind::System
    } else if lower.contains("prompt") || lower.contains("meta") {
        PromptKind::Meta
    } else {
        PromptKind::General
    }
}

// ─── URL scraper ────────────────────────────────────────────────────────────

fn scrape_url(url: &str) -> Result<Vec<PromptEntry>, String> {
    // Use system curl (available everywhere, no dependency).
    let output = std::process::Command::new("curl")
        .args(["-sSL", "--max-time", "15", "-H", "User-Agent: skill_ingest/0.1 (dowiz armory)", url])
        .output()
        .map_err(|e| format!("curl spawn error: {}", e))?;

    if !output.status.success() {
        return Err(format!("curl exit {}", output.status));
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();

    // Heuristic: detect prompt patterns in scraped text.
    detect_prompts_in_text(&body, url)
}

fn detect_prompts_in_text(text: &str, source: &str) -> Result<Vec<PromptEntry>, String> {
    let mut entries = Vec::new();

    // Pattern 1: Markdown headers with system prompt following.
    // "# Title\n\nSystem prompt text"
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Detect a heading that looks like a skill/prompt title.
        if (trimmed.starts_with("# ") || trimmed.starts_with("## ")) && !trimmed.contains("README") && !trimmed.contains("Table of Contents") {
            let title = trimmed.trim_start_matches('#').trim().to_string();
            if title.len() < 5 || title.len() > 120 { i += 1; continue; }

            // Collect the following text as prompt body.
            let mut body = String::new();
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim().starts_with('#') && !lines[j].trim().starts_with("---") {
                let l = lines[j].trim();
                if !l.is_empty() {
                    body.push_str(l);
                    body.push('\n');
                }
                j += 1;
            }

            if body.len() > 50 {
                let kind = classify_title(&title);
                let triggers: Vec<&str> = title.split_whitespace()
                    .filter(|w| w.len() >= 3)
                    .collect::<Vec<&str>>();
                entries.push(PromptEntry::new(
                    &title, &body, kind, &triggers, source, "unknown",
                ));
            }
            i = j;
        } else {
            i += 1;
        }
    }

    // Pattern 2: "You are a..." or "Act as a..." blocks.
    for (idx, _) in text.match_indices("You are a") {
        let end = text[idx..].find("\n\n").map(|e| idx + e).unwrap_or((idx + 500).min(text.len()));
        let snippet = &text[idx..end];
        if snippet.len() > 30 {
            let title = format!("role_{}", entries.len());
            let kind = classify_title(snippet);
            let triggers: Vec<&str> = snippet.split_whitespace().take(8).collect();
            entries.push(PromptEntry::new(
                &title, snippet, kind, &triggers, source, "unknown",
            ));
        }
    }

    for (idx, _) in text.match_indices("I want you to act as") {
        let end = text[idx..].find("\n\n").map(|e| idx + e).unwrap_or((idx + 500).min(text.len()));
        let snippet = &text[idx..end];
        if snippet.len() > 30 {
            let title = format!("act_as_{}", entries.len());
            let kind = classify_title(snippet);
            let triggers: Vec<&str> = snippet.split_whitespace().take(8).collect();
            entries.push(PromptEntry::new(
                &title, snippet, kind, &triggers, source, "unknown",
            ));
        }
    }

    Ok(entries)
}

// ─── JSONL ingest (pass-through for pre-formatted data) ─────────────────────

fn ingest_jsonl_file(path: &str, out: &mut dyn Write) -> Result<u64, String> {
    let file = fs::File::open(path).map_err(|e| format!("open: {}", e))?;
    let reader = io::BufReader::new(file);
    let mut count = 0u64;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("read: {}", e))?;
        if line.trim().is_empty() { continue; }

        // Pass through as-is (already JSONL formatted).
        writeln!(out, "{}", line).map_err(|e| format!("write: {}", e))?;
        count += 1;
    }

    Ok(count)
}
