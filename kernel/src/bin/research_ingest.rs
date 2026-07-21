//! `research_ingest` — Kernel research paper ingestion + pattern analysis CLI.
//!
//! Uses kernel-native JSON parser (no serde_json dependency).

use dowiz_kernel::json::{self, Value};
use dowiz_kernel::research::{Paper, ResearchEngine, };
use dowiz_kernel::TriState;
use std::fs;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: research_ingest <papers.jsonl> [--output patterns.jsonl]");
        return Ok(());
    }

    let input_file = &args[1];
    let mut output_file = String::from("patterns.jsonl");

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => { i += 1; output_file = args[i].clone(); }
            _ => { eprintln!("Unknown: {}", args[i]); }
        }
        i += 1;
    }

    eprintln!("Reading papers from: {}", input_file);
    let file = fs::File::open(input_file)?;
    let reader = io::BufReader::new(file);

    let mut engine = ResearchEngine::new();
    let mut count = 0u64;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }

        match parse_paper_json(&line) {
            Ok(paper) => {
                engine.ingest_papers(vec![paper]);
                count += 1;
                if count % 1000 == 0 {
                    eprintln!("  Ingested {} papers...", count);
                }
            }
            Err(e) => {
                eprintln!("  WARNING: parse error at line {}: {}", count + 1, e);
            }
        }
    }

    eprintln!("\nIngested {} papers. Running pattern extraction...", count);

    let patterns = engine.extract_patterns();
    eprintln!("  Found {} patterns", patterns.len());
    let high_cross = engine.high_lift_cross_patterns(1.5);
    eprintln!("  Found {} cross-patterns (lift >= 1.5)", high_cross.len());

    let mut out = fs::File::create(&output_file)?;

    for p in &patterns {
        writeln!(out, "{{\"pattern\":\"{}\",\"domain\":{},\"kind\":{},\"confidence\":{:.4},\"papers\":{}}}",
            p.name, p.domain as u32, p.kind as u32, p.confidence, p.paper_ids.len())?;
    }
    for cp in high_cross {
        writeln!(out, "{{\"cross\":\"{}\",\"lift\":{:.4},\"count\":{}}}",
            cp.description, cp.lift, cp.co_occurrence_count)?;
    }

    eprintln!("\nOutput written to: {}", output_file);
    eprintln!("Engine dashboard:\n{}", engine.dashboard());

    Ok(())
}

fn parse_paper_json(line: &str) -> Result<Paper, String> {
    let val = json::parse(line).map_err(|e| format!("JSON parse error: {}", e))?;

    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let title = val.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let abstract_text = val.get("abstract").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let authors: Vec<String> = val.get("authors")
        .map(|a| match a {
            Value::Array(arr) => arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        })
        .unwrap_or_default();

    let categories: Vec<String> = val.get("categories")
        .map(|c| match c {
            Value::Array(arr) => arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        })
        .unwrap_or_default();

    let year = val.get("year").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
    let citation_count = val.get("citation_count").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
    let arxiv_id = val.get("arxiv_id").and_then(|v| v.as_str().map(|s| s.to_string()));
    let doi = val.get("doi").and_then(|v| v.as_str().map(|s| s.to_string()));

    let hash = dowiz_kernel::event_log::sha3_256(title.as_bytes());

    Ok(Paper {
        id, title, authors, abstract_text, categories,
        year, citation_count, arxiv_id, doi,
        paper_hash: hash,
        full_text_accessible: TriState::True,
        embedding: vec![],
    })
}
