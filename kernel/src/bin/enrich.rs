//! `enrich` — Universal prompt enrichment binary. Agent-agnostic, std-only.
//!
//! Reads user input from stdin or --query, runs the enrichment engine,
//! outputs the enriched result. Works with ANY agent (opencode, claude-code,
//! codex, cursor, copilot, gemini, shell scripts, etc.).
//!
//! # Usage
//! ```sh
//! echo "summarize the design doc" | enrich --load prompt_enrich_db.bin
//! enrich --query "fix the compilation bug" --format json --load prompt_enrich_db.bin
//! enrich --load prompt_enrich_db.bin --query "audit security" --verbose
//! ```
//!
//! # Performance
//! Release build: 95ms cold load, <1ms warm query.
//! Binary format (.bin): 3.1 MB, 54ms load. JSONL (.jsonl): 4.2 MB, 57ms load.
//! Build: `cargo build --bin enrich --release`

use dowiz_kernel::prompt_enrich::{
    PromptEnrichEngine, PromptEntry, PromptKind, EnrichmentReport,
    seed_fabric_prompts, seed_opencode_prompts,
    detect_all_intents, detect_intent_tree, inherit_patterns, PATTERN_TREE,
};
use dowiz_kernel::telemetry_harvest::HarvestLedger;
use dowiz_kernel::json;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("enrich — Universal prompt enrichment engine (agent-agnostic)");
        println!("Usage:");
        println!("  echo \"query\" | enrich                    # stdin, plain output");
        println!("  enrich --query \"query\"                 # direct query");
        println!("  enrich --query \"query\" --format json   # JSON output");
        println!("  enrich --query \"query\" --verbose       # full enrichment report");
        println!("  enrich --load db.jsonl --query \"...\"   # load custom DB");
        println!("  enrich --dashboard                       # show engine stats");
        println!("  enrich --detect \"query\"                 # detect intent only");
        println!("  enrich --patterns \"query\"               # show inherited patterns");
        println!();
        println!("Database: uses built-in seed DB (30 prompts) unless --load specified.");
        return Ok(());
    }

    let mut query = String::new();
    let mut format = "plain";
    let mut verbose = false;
    let mut dashboard = false;
    let mut detect_only = false;
    let mut patterns_only = false;
    let mut db_file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--query" => { i += 1; query = args[i].clone(); }
            "--format" => { i += 1; format = args[i].as_str(); }
            "--verbose" | "-v" => verbose = true,
            "--dashboard" => dashboard = true,
            "--detect" => { i += 1; query = args[i].clone(); detect_only = true; }
            "--patterns" => { i += 1; query = args[i].clone(); patterns_only = true; }
            "--load" => { i += 1; db_file = Some(args[i].clone()); }
            _ => { eprintln!("Unknown: {}", args[i]); }
        }
        i += 1;
    }

    // Build engine
    let mut engine = PromptEnrichEngine::new();

    // Load DB
    if let Some(path) = db_file {
        let entries = if path.ends_with(".bin") {
            load_binary(&path)?
        } else {
            load_jsonl(&path)?
        };
        engine.ingest(entries);
        eprintln!("Loaded {} entries from {} ({} format)", engine.total(), path,
            if path.ends_with(".bin") { "binary" } else { "JSONL" });
    } else {
        engine.ingest(seed_fabric_prompts());
        engine.ingest(seed_opencode_prompts());
    }

    if dashboard {
        println!("{}", engine.dashboard());
        return Ok(());
    }

    // Read query from stdin if not provided
    if query.is_empty() {
        let stdin = io::stdin();
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        query = input.trim().to_string();
    }

    if query.is_empty() {
        eprintln!("No query provided. Use --query or pipe input.");
        return Ok(());
    }

    // Detect only mode
    if detect_only {
        let intents = detect_all_intents(&query);
        let paths = detect_intent_tree(&query);
        println!("Intent detection:");
        for (kind, count, score) in &intents {
            println!("  {}: {} hits, confidence {:.2}", kind.as_str(), count, score);
        }
        if !paths.is_empty() {
            println!("Tree paths:");
            for p in &paths {
                println!("  {}", p.join(" → "));
            }
        }
        let mut ledger = HarvestLedger::new(1000);
        let success = !intents.is_empty();
        let value = if intents.is_empty() { 0.0 } else {
            let sum: f64 = intents.iter().map(|(_, _, s)| *s).sum();
            sum / intents.len() as f64
        };
        ledger.record("enrich", "detect", success, value, query.len() as f64);
        return Ok(());
    }

    // Patterns only mode
    if patterns_only {
        let paths = detect_intent_tree(&query);
        if let Some(path) = paths.first() {
            let patterns = inherit_patterns(path);
            println!("Inherited patterns for {}:", path.join(" → "));
            for p in &patterns {
                println!("  [{}] {}: {}", p.category, p.name, p.rule);
            }
        } else {
            println!("No intent detected.");
        }
        return Ok(());
    }

    // Enrichment
    let report = engine.enrich_report(&query);

    // Harvest telemetry: record enrichment operation for EV scoring
    let mut ledger = HarvestLedger::new(1000);
    let enrich_success = !report.intents.is_empty();
    let enrich_value = if report.intents.is_empty() { 0.0 } else {
        let sum: f64 = report.intents.iter().map(|(_, _, s)| *s).sum();
        sum / report.intents.len() as f64
    };
    let enrich_cost = query.len() as f64;
    ledger.record("enrich", "lookup", enrich_success, enrich_value, enrich_cost);

    match format {
        "json" => {
            use std::io::Write;
            let mut out = String::new();
            out.push_str("{");
            out.push_str(&format!("\"primary\":\"{}\"", report.primary_intent.as_str()));
            out.push_str(&format!(",\"intents\":["));
            for (i, (k, _, s)) in report.intents.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&format!("{{\"kind\":\"{}\",\"score\":{:.2}}}", k.as_str(), s));
            }
            out.push_str("]");
            if !report.intent_paths.is_empty() {
                out.push_str(",\"paths\":[");
                for (i, p) in report.intent_paths.iter().enumerate() {
                    if i > 0 { out.push(','); }
                    out.push_str(&format!("\"{}\"", p.join("→")));
                }
                out.push_str("]");
            }
            out.push_str(&format!(",\"prompts\":["));
            for (i, p) in report.prompts.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&format!("\"{}\"", p.title));
            }
            out.push_str("]");
            out.push_str(&format!(",\"skills\":["));
            for (i, s) in report.skills.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&format!("\"{}\"", s));
            }
            out.push_str("]");
            out.push_str("}");
            println!("{}", out);
        }
        _ => {
            if verbose {
                println!("{}", report.display());
                // Also show inherited patterns
                if let Some(path) = report.intent_paths.first() {
                    let patterns = inherit_patterns(path);
                    println!("  patterns:");
                    for p in &patterns {
                        println!("    [{}] {}: {}", p.category, p.name, p.rule);
                    }
                }
            } else {
                // Compact output for agent consumption
                print!("[");
                print!("primary:{}", report.primary_intent.as_str());
                if let Some(path) = report.intent_paths.first() {
                    print!(" path:{}", path.join("→"));
                }
                if !report.prompts.is_empty() {
                    print!(" prompts:");
                    for p in &report.prompts {
                        print!("{} ", p.title);
                    }
                }
                println!("]");
            }
        }
    }

    Ok(())
}

fn kind_from_u16(k: u16) -> PromptKind {
    match k {
        0 => PromptKind::Code, 1 => PromptKind::Write, 2 => PromptKind::Analyze,
        3 => PromptKind::Summarize, 4 => PromptKind::Extract, 5 => PromptKind::Plan,
        6 => PromptKind::Review, 7 => PromptKind::System, 8 => PromptKind::Math,
        9 => PromptKind::Creative, 10 => PromptKind::Meta, 11 => PromptKind::Search,
        12 => PromptKind::Test, 13 => PromptKind::Debug, 14 => PromptKind::Config,
        15 => PromptKind::Security, 16 => PromptKind::Refactor, 17 => PromptKind::Tool,
        18 => PromptKind::Skill, 19 => PromptKind::Plugin, _ => PromptKind::General,
    }
}

fn load_binary(path: &str) -> Result<Vec<PromptEntry>, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let mut pos = 0;
    if data.len() < 4 { return Err("truncated".into()); }
    let n = u32::from_le_bytes([data[0],data[1],data[2],data[3]]) as usize;
    pos += 4;
    let mut entries = Vec::with_capacity(n);
    for _ in 0..n {
        if pos + 2 > data.len() { break; }
        let tl = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
        let title = String::from_utf8_lossy(&data[pos..pos+tl]).to_string(); pos += tl;
        if pos + 2 > data.len() { break; }
        let tl = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
        let text = String::from_utf8_lossy(&data[pos..pos+tl]).to_string(); pos += tl;
        if pos + 2 > data.len() { break; }
        let kind = u16::from_le_bytes([data[pos],data[pos+1]]); pos += 2;
        if pos + 2 > data.len() { break; }
        let tc = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
        let mut triggers: Vec<String> = Vec::with_capacity(tc);
        for _ in 0..tc {
            if pos + 2 > data.len() { break; }
            let tl = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
            triggers.push(String::from_utf8_lossy(&data[pos..pos+tl]).to_string()); pos += tl;
        }
        if pos + 2 > data.len() { break; }
        let sl = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
        let source = String::from_utf8_lossy(&data[pos..pos+sl]).to_string(); pos += sl;
        if pos + 2 > data.len() { break; }
        let ll = u16::from_le_bytes([data[pos],data[pos+1]]) as usize; pos += 2;
        let license = String::from_utf8_lossy(&data[pos..pos+ll]).to_string(); pos += ll;
        let trigger_strs: Vec<&str> = triggers.iter().map(|s| s.as_str()).collect();
        entries.push(PromptEntry::new(&title, &text, kind_from_u16(kind), &trigger_strs, &source, &license));
    }
    Ok(entries)
}

fn load_jsonl(path: &str) -> Result<Vec<PromptEntry>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        if let Ok(val) = json::parse(&line) {
            let title = val.get("title").and_then(|v| v.as_str()).unwrap_or("unknown");
            let text = val.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let kind = val.get("kind").and_then(|v| v.as_f64()).unwrap_or(31.0) as u16;
            let triggers: Vec<&str> = val.get("triggers")
                .map(|v| match v {
                    json::Value::Array(arr) => arr.iter().filter_map(|x| x.as_str()).collect(),
                    _ => vec![],
                }).unwrap_or_default();
            let source = val.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let license = val.get("license").and_then(|v| v.as_str()).unwrap_or("MIT");
            entries.push(PromptEntry::new(title, text, kind_from_u16(kind), &triggers, source, license));
        }
    }
    Ok(entries)
}
