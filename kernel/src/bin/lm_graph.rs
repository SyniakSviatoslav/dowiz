//! `lm_graph` — build a living-memory graph from a Rust source tree, persist it
//! crash-safely (Markdown `.md` + binary `.idx` code sidecar), and query it
//! (keyword + vector navigation + shift-invariant NTT convolution). The native
//! runtime for "use living memory instead of grep".
//!
//!   lm_graph build SRC_DIR STORE      # extract symbols -> living memory, persist
//!   lm_graph search STORE TEXT        # keyword + vector top-k over the palace
//!   lm_graph conv STORE TEXT          # shift-invariant (NTT) re-rank, top-k
//!   lm_graph nodes STORE              # list symbols (name + kind) in the graph

use dowiz_core::code_graph::{CodeGraph, NodeKind};
use dowiz_core::living_memory::MemoryKind;
use dowiz_core::prompt_enrich::{
    seed_fabric_prompts, seed_opencode_prompts, PromptEnrichEngine, PromptKind,
};
use dowiz_core::agent_orchestrator::TaskOracle;
use dowiz_core::dynamic_spawner::{DynamicSpawner, SpawnBatchConfig};
use dowiz_kernel::living_memory_store::LivingMemoryStore;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: lm_graph build SRC STORE | search STORE TEXT | conv STORE TEXT | enrich TEXT | armory | seed-armory STORE | serve STORE | nodes STORE"
        );
        exit(2);
    }
    match args[1].as_str() {
        "build" => {
            if args.len() < 4 {
                eprintln!("lm_graph build SRC_DIR STORE");
                exit(2);
            }
            let mut store = LivingMemoryStore::open(&args[3]).expect("open store");
            let n = extract(&args[2], &mut store);
            println!("built graph: {n} symbols -> {}", args[3]);
        }
        "search" => {
            if args.len() < 4 {
                eprintln!("lm_graph search STORE TEXT");
                exit(2);
            }
            let store = LivingMemoryStore::open(&args[2]).expect("open store");
            let q = &args[3];
            println!("== keyword ==");
            for id in store.memory().search(q) {
                let r = store.memory().recall(id).unwrap();
                println!("  [{id}] {} :: {}", r.key, r.summary);
            }
            println!("== vector ==");
            for (id, score) in store.memory().vector_search(q, 5) {
                let r = store.memory().recall(id).unwrap();
                println!("  [{id}] {:.4} {} :: {}", score, r.key, r.summary);
            }
        }
        "conv" => {
            if args.len() < 4 {
                eprintln!("lm_graph conv STORE TEXT");
                exit(2);
            }
            let store = LivingMemoryStore::open(&args[2]).expect("open store");
            let q = &args[3];
            println!("== convolution (shift-invariant NTT) ==");
            for (id, score) in store.memory().convolution_search(q, 5) {
                let r = store.memory().recall(id).unwrap();
                println!("  [{id}] {:.4} {} :: {}", score, r.key, r.summary);
            }
        }
        "nodes" => {
            if args.len() < 3 {
                eprintln!("lm_graph nodes STORE");
                exit(2);
            }
            let store = LivingMemoryStore::open(&args[2]).expect("open store");
            let g: &CodeGraph = store.memory().code_graph();
            for (i, n) in g.nodes().iter().enumerate() {
                println!("{i}\t{:?}\t{}", n.kind, n.name);
            }
        }
        "serve" => {
            // In-memory query server: load once, answer many queries in-process.
            // Each query after the first is ~microseconds (no cold-start, no
            // re-parse) — the "100x faster than cold-start" runtime path.
            if args.len() < 3 {
                eprintln!("lm_graph serve STORE");
                exit(2);
            }
            let store = LivingMemoryStore::open(&args[2]).expect("open store");
            eprintln!(
                "lm_graph: serving {} records — 'search TEXT' | 'vector TEXT' | 'conv TEXT' | 'quit'",
                store.memory().record_count()
            );
            use std::io::BufRead;
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.expect("read stdin");
                let t = line.trim();
                if t.is_empty() {
                    continue;
                }
                if t == "quit" || t == "exit" {
                    break;
                }
                let mut parts = t.splitn(2, ' ');
                let cmd = parts.next().unwrap_or("");
                let arg = parts.next().unwrap_or("");
                match cmd {
                    "search" => {
                        for id in store.memory().search(arg) {
                            let r = store.memory().recall(id).unwrap();
                            println!("[{id}] {} :: {}", r.key, r.summary);
                        }
                    }
                    "vector" => {
                        for (id, score) in store.memory().vector_search(arg, 5) {
                            let r = store.memory().recall(id).unwrap();
                            println!("[{id}] {score:.4} {} :: {}", r.key, r.summary);
                        }
                    }
                    "conv" => {
                        for (id, score) in store.memory().convolution_search(arg, 5) {
                            let r = store.memory().recall(id).unwrap();
                            println!("[{id}] {score:.4} {} :: {}", r.key, r.summary);
                        }
                    }
                    "read" => {
                        for id in store.memory().search(arg).into_iter().take(1) {
                            let r = store.memory().recall(id).unwrap();
                            println!("=== [{id}] {} ({} / {}) ===", r.key, r.wing, r.room);
                            println!("{}", r.content);
                        }
                    }
                    other => eprintln!("lm_graph serve: unknown command `{other}`"),
                }
            }
        }
        "enrich" => {
            // Runtime armory: intent detection + enriched prompts/skills from
            // the seed corpus (fabric patterns + opencode skills/agents).
            if args.len() < 3 {
                eprintln!("lm_graph enrich TEXT");
                exit(2);
            }
            let engine = build_armory_engine();
            let report = engine.enrich_report(&args[2]);
            println!("== intents ==");
            for (kind, hits, score) in &report.intents {
                println!("  {} ({} hits, {:.2})", kind.as_str(), hits, score);
            }
            println!("== intent paths ==");
            for p in &report.intent_paths {
                println!("  {}", p.join(" -> "));
            }
            println!("== enriched prompts ==");
            for p in &report.prompts {
                println!("  [{}] {}  <{} / {}>", p.kind.as_str(), p.title, p.source, p.license);
            }
            println!("== skills ==");
            for s in &report.skills {
                println!("  - {s}");
            }
            println!(
                "armory: {} prompts, {} skills matched",
                report.total_prompts, report.total_skills
            );
        }
        "armory" => {
            // List the in-memory armory (seed corpus) by kind.
            let engine = build_armory_engine();
            println!("{}", engine.dashboard());
        }
        "seed-armory" => {
            // Persist the armory (agents/skills/enriched prompts) INTO living
            // memory so it is searchable, crash-safe, and survives sessions.
            if args.len() < 3 {
                eprintln!("lm_graph seed-armory STORE");
                exit(2);
            }
            let engine = build_armory_engine();
            let mut store = LivingMemoryStore::open(&args[2]).expect("open store");
            let mut n = 0usize;
            for p in engine.all_entries() {
                let room = match p.kind {
                    PromptKind::Skill => "skills",
                    PromptKind::Plugin => "plugins",
                    PromptKind::Tool => "tools",
                    PromptKind::Meta => "dynamic-skills",
                    _ => "prompts",
                };
                let summary = format!("{} — {}", p.kind.as_str(), p.trigger_keywords.join(", "));
                store.memory_mut().remember_full(
                    MemoryKind::Procedural,
                    "armory",
                    room,
                    &p.title,
                    &summary,
                    &p.prompt_text,
                    None,
                );
                n += 1;
            }
            store.persist().expect("persist armory");
            println!("armory seeded: {n} entries -> {}", args[2]);
        }
        "read" => {
            // Read via living memory: return full content of the best matches.
            if args.len() < 4 {
                eprintln!("lm_graph read STORE QUERY");
                exit(2);
            }
            let store = LivingMemoryStore::open(&args[2]).expect("open store");
            let q = args[3..].join(" ");
            // Rank matches by relevance: a key/path hit first (exact file), then a
            // summary hit, then a content-only hit. `search` returns insertion
            // order, which is wrong for "read the file named X".
            let mut ids = store.memory().search(&q);
            ids.sort_by_key(|id| {
                let r = store.memory().recall(*id).unwrap();
                if r.key.contains(&q) {
                    0
                } else if r.summary.contains(&q) {
                    1
                } else {
                    2
                }
            });
            for id in ids.iter().take(3) {
                let r = store.memory().recall(*id).unwrap();
                println!("=== [{id}] {} ({} / {}) ===", r.key, r.wing, r.room);
                println!("{}", r.content);
                println!();
            }
        }
        "dispatch" => {
            // Runtime orchestration: intent -> skills -> specialized agent ->
            // spawn batch, all chained from one prompt.
            if args.len() < 3 {
                eprintln!("lm_graph dispatch PROMPT");
                exit(2);
            }
            let prompt = args[2..].join(" ");
            let engine = build_armory_engine();
            let report = engine.enrich_report(&prompt);
            let oracle = TaskOracle::new();
            let (class, conf) = oracle.predict(&prompt);
            let mut spawner = DynamicSpawner::new(SpawnBatchConfig::default());
            let batch = spawner.compute_batch(0, 1);
            println!("== intent ==  {}", report.primary_intent.as_str());
            for (kind, hits, score) in &report.intents {
                println!("  {} ({} hits, {:.2})", kind.as_str(), hits, score);
            }
            println!("== skills ==");
            for s in &report.skills {
                println!("  - {s}");
            }
            println!("== agent ==  class={class:?} confidence={conf:.2}");
            println!("== spawn ==  count={} reason={:?}", batch.count, batch.reason);
        }
        other => {
            eprintln!("lm_graph: unknown subcommand `{other}`");
            exit(2);
        }
    }
}

/// Extract `pub fn/struct/enum/mod/const` symbols from `*.rs` files under `dir`
/// into the store (code_graph node + a semantic memory record). Returns the
/// number of symbols extracted.
fn extract(dir: &str, store: &mut LivingMemoryStore) -> usize {
    let mut count = 0;
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_rs(PathBuf::from(dir), &mut paths);
    for path in paths {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let file = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        for line in text.lines() {
            let (name, kind) = match_symbol(line);
            if let Some(name) = name {
                // Code-graph node (navigation).
                store.memory_mut().code_graph_mut().add_node(&name, kind);
                // Semantic memory record (recall/search).
                let content = format!("{}: {}", name, line.trim());
                store.memory_mut().remember_full(
                    MemoryKind::Semantic,
                    "code",
                    &file,
                    &name,
                    line.trim(),
                    &content,
                    None,
                );
                count += 1;
            }
        }
        // File record: full source content, so "read" goes through living
        // memory too (navigation + search + read share one graph).
        let rel = path.to_string_lossy().to_string();
        let lines = text.lines().count();
        store.memory_mut().remember_full(
            MemoryKind::LongTerm,
            "source",
            "files",
            &rel,
            &format!("{file} · {lines} lines"),
            &text,
            None,
        );
    }
    // One durable snapshot at the end (not per-symbol — that was O(n²)).
    store.persist().expect("persist");
    count
}

fn collect_rs(dir: PathBuf, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                // Skip generated/build/hidden trees — only real source.
                if name == "target" || name == ".git" || name == "node_modules" {
                    continue;
                }
                collect_rs(p, out);
            } else if p.extension().map_or(false, |x| x == "rs") {
                out.push(p);
            }
        }
    }
}

/// Match a line to (symbol name, node kind), or `None`. Covers pub AND non-pub
/// declarations (`fn`/`struct`/`enum`/`trait`/`mod`/`const`/`static`/`type`/
/// `impl`), so the living-memory store is a grep-complete index of the source
/// tree, not just the public API surface.
fn match_symbol(line: &str) -> (Option<String>, NodeKind) {
    let t = line.trim();
    const PREFIXES: &[(&str, NodeKind)] = &[
        ("pub async fn ", NodeKind::Function),
        ("pub const fn ", NodeKind::Function),
        ("pub fn ", NodeKind::Function),
        ("pub struct ", NodeKind::Struct),
        ("pub enum ", NodeKind::Enum),
        ("pub trait ", NodeKind::Trait),
        ("pub mod ", NodeKind::Module),
        ("pub static ", NodeKind::Other),
        ("pub type ", NodeKind::Other),
        ("pub const ", NodeKind::Other),
        ("async fn ", NodeKind::Function),
        ("const fn ", NodeKind::Function),
        ("fn ", NodeKind::Function),
        ("struct ", NodeKind::Struct),
        ("enum ", NodeKind::Enum),
        ("trait ", NodeKind::Trait),
        ("impl ", NodeKind::Struct),
        ("mod ", NodeKind::Module),
        ("static ", NodeKind::Other),
        ("type ", NodeKind::Other),
        ("const ", NodeKind::Other),
    ];
    for &(prefix, kind) in PREFIXES {
        if let Some(rest) = t.strip_prefix(prefix) {
            let name = rest
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("")
                .to_string();
            // Skip bare keywords (e.g. `impl` with no name, `fn` with no name).
            const KEYWORDS: &[&str] = &[
                "fn", "struct", "enum", "trait", "impl", "mod", "const", "static", "type", "async",
                "use", "where", "for", "let",
            ];
            if !name.is_empty() && !KEYWORDS.contains(&name.as_str()) {
                return (Some(name), kind);
            }
        }
    }
    (None, NodeKind::Other)
}

/// Build the in-memory armory engine: fabric patterns (MIT) + opencode
/// skills/agents (MIT), ingested into the crystal-lattice prompt engine.
fn build_armory_engine() -> PromptEnrichEngine {
    let mut engine = PromptEnrichEngine::new();
    engine.ingest(seed_fabric_prompts());
    engine.ingest(seed_opencode_prompts());
    engine
}
