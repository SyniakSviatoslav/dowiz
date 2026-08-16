//! `lm_graph` — build a living-memory graph from a Rust source tree, persist it
//! crash-safely, and query it (keyword + vector navigation). The native runtime
//! for "use living memory instead of grep".
//!
//!   lm_graph build SRC_DIR STORE    # extract symbols -> living memory, persist
//!   lm_graph search STORE TEXT      # keyword + vector top-k over the palace
//!   lm_graph nodes STORE            # list symbols (name + kind) in the graph

use dowiz_core::code_graph::{CodeGraph, NodeKind};
use dowiz_core::living_memory::MemoryKind;
use dowiz_kernel::living_memory_store::LivingMemoryStore;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lm_graph build SRC_DIR STORE | search STORE TEXT | nodes STORE");
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
                collect_rs(p, out);
            } else if p.extension().map_or(false, |x| x == "rs") {
                out.push(p);
            }
        }
    }
}

/// Match a line to (symbol name, node kind), or `None`.
fn match_symbol(line: &str) -> (Option<String>, NodeKind) {
    let t = line.trim();
    for (prefix, kind) in [
        ("pub const fn ", NodeKind::Function),
        ("pub fn ", NodeKind::Function),
        ("pub struct ", NodeKind::Struct),
        ("pub enum ", NodeKind::Enum),
        ("pub trait ", NodeKind::Trait),
        ("pub mod ", NodeKind::Module),
        ("pub const ", NodeKind::Other),
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let name = rest
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() && name != "fn" && name != "struct" {
                return (Some(name), kind);
            }
        }
    }
    (None, NodeKind::Other)
}
