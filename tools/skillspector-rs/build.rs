//! build.rs — regenerate src/rules.rs from the upstream Python sources when they change.
//! Keeps the native rules byte-identical to tools/skillspector without manual copies.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Re-run if the Python analyzer sources change.
    let py_sources = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("skillspector")
        .join("src")
        .join("skillspector")
        .join("nodes")
        .join("analyzers");
    println!("cargo:rerun-if-changed={}", py_sources.display());

    // Invoke the AST generator (pure python stdlib).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gen = manifest.join("gen_rules.py");
    let status = Command::new("python3")
        .arg(&gen)
        .current_dir(&manifest)
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => {
            // Generator failed — but rules.rs may already exist from a prior run.
            // Only hard-fail if it's missing (truly cannot build).
            let rules = manifest.join("src").join("rules.rs");
            if !rules.exists() {
                eprintln!("cargo:warning=gen_rules.py failed and src/rules.rs is missing");
                std::process::exit(1);
            }
            eprintln!("cargo:warning=gen_rules.py failed; using existing src/rules.rs");
        }
    }
}
